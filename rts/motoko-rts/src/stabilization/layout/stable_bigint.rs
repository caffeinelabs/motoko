use crate::bigint::mp_calloc;
use crate::memory::Memory;
use crate::stabilization::deserialization::stable_memory_access::StableMemoryAccess;
use crate::stabilization::serialization::SerializationContext;
use crate::stabilization::serialization::stable_memory_stream::{
    ScanStream, StableMemoryStream, WriteStream,
};
use crate::tommath_bindings::mp_digit;
use crate::types::{BigInt, Bytes, TAG_BIGINT, Value, size_of};

use super::{
    Serializer, StableObjectKind, StableToSpace, StableValue, StaticScanner, round_to_u64,
};

// Tom's math library, as configured for Motoko RTS with 64-bit enhanced orthogonal persistence,
// encodes a big numbers as an array of 64-bit elements, where each element stores 60 bits of the
// number while its highest 4 bits are zero.
// Similar to the little endian encoding, the element array starts with the least significant 60 bits,
// with each subsequent element covering the next higher 60 bits.

// Tom's math library functions `mp_to_sbin` and `mp_from_sbin` imply temporary object allocations
// which could be problematic when memory is short.
// Therefore, a custom serialization to the defined portable stable bignum format is implemented.

#[repr(C)]
pub struct StableBigInt {
    is_negative: bool, // Sign bit.
    number_of_bits: Bits, // Number of used bits in the payload.

                       // Dynamically sized payload of the big integer number as
                       // little endian encoded series of 60 bits packed in 64-bit elements.
}

type ElementType = u64;
const USED_BITS_PER_ELEMENT: u32 = 60;
const ELEMENT_SIZE: usize = core::mem::size_of::<ElementType>();

// Assumes 64-bit representation for both Tom's math library and main memory.
const _: () = assert!(usize::BITS == ElementType::BITS);
const _: () = assert!(core::mem::size_of::<mp_digit>() == core::mem::size_of::<ElementType>());

#[repr(C)]
struct Bits(u64);

impl Bits {
    fn to_bytes(&self) -> Bytes<usize> {
        let bytes = ceiling_div(self.0, u8::BITS as u64);
        Bytes(bytes as usize)
    }
}

impl StableBigInt {
    fn is_negative(main_object: *mut BigInt) -> bool {
        unsafe { (*main_object).mp_int.sign != 0 }
    }

    fn serialized_length(main_object: *mut BigInt) -> Bits {
        let used_elements = unsafe { (*main_object).mp_int.used } as usize;
        if used_elements == 0 {
            return Bits(0);
        }
        let last_element = unsafe { *main_object.payload_addr().add(used_elements - 1) };
        debug_assert_ne!(last_element, 0);
        debug_assert_eq!(last_element >> USED_BITS_PER_ELEMENT, 0);
        let last_bits = ElementType::BITS - last_element.leading_zeros();
        Bits((used_elements - 1) as u64 * USED_BITS_PER_ELEMENT as u64 + last_bits as u64)
    }

    fn deserialized_elements(&self) -> usize {
        ceiling_div(self.number_of_bits.0, USED_BITS_PER_ELEMENT as u64) as usize
    }
}

fn ceiling_div(dividend: u64, divisor: u64) -> u64 {
    (dividend + divisor - 1) / divisor
}

impl StaticScanner<StableValue> for StableBigInt {}

impl Serializer<BigInt> for StableBigInt {
    unsafe fn serialize_static_part(
        _stable_memory: &mut StableMemoryStream,
        main_object: *mut BigInt,
    ) -> Self {
        let is_negative = Self::is_negative(main_object);
        let number_of_bits = Self::serialized_length(main_object);
        StableBigInt {
            is_negative,
            number_of_bits,
        }
    }

    unsafe fn serialize_dynamic_part(
        stable_memory: &mut StableMemoryStream,
        main_object: *mut BigInt,
    ) {
        let total_elements = (*main_object).mp_int.used as usize;
        let total_bytes = Self::serialized_length(main_object).to_bytes().as_usize();
        let payload = main_object.payload_addr();
        let mut written_bytes = 0;
        let mut element_index = 0;
        let mut element: ElementType = 0;
        let mut pending_bits = 0;
        let mut next_element: ElementType = 0;
        let mut next_pending_bits = 0;

        while written_bytes < total_bytes {
            while pending_bits < ElementType::BITS && element_index < total_elements {
                debug_assert_eq!(next_pending_bits, 0);
                next_element = *payload.add(element_index);
                element_index += 1;
                debug_assert_eq!(next_element >> USED_BITS_PER_ELEMENT, 0);
                element |= next_element << pending_bits;
                let consumed_bits =
                    core::cmp::min(USED_BITS_PER_ELEMENT, ElementType::BITS - pending_bits);
                pending_bits += consumed_bits;
                next_element >>= consumed_bits;
                next_pending_bits = USED_BITS_PER_ELEMENT - consumed_bits;
            }
            stable_memory.write(&element); // little endian
            written_bytes += ELEMENT_SIZE;
            element = next_element;
            pending_bits = next_pending_bits;
            next_pending_bits = 0;
        }
        debug_assert!(pending_bits == 0 || element == 0);
        debug_assert_eq!(element_index, total_elements);
        debug_assert_eq!(written_bytes % core::mem::size_of::<u64>(), 0);
    }

    fn scan_serialized_dynamic<
        'a,
        M,
        F: Fn(&mut SerializationContext<'a, M>, StableValue) -> StableValue,
    >(
        &self,
        context: &mut SerializationContext<'a, M>,
        _translate: &F,
    ) {
        let rounded_length = round_to_u64(self.number_of_bits.to_bytes().as_usize() as u64);
        context
            .serialization
            .to_space()
            .skip(rounded_length as usize);
    }

    unsafe fn allocate_deserialized<M: Memory>(
        &self,
        main_memory: &mut M,
        object_kind: StableObjectKind,
    ) -> Value {
        debug_assert_eq!(object_kind, StableObjectKind::BigInt);
        let elements = self.deserialized_elements();
        let payload = mp_calloc(main_memory, elements, Bytes(ELEMENT_SIZE)) as *mut mp_digit;
        let bigint = BigInt::from_payload(payload);
        Value::from_ptr(bigint as usize)
    }

    unsafe fn deserialize_static_part(
        &self,
        target_bigint: *mut BigInt,
        object_kind: StableObjectKind,
    ) {
        debug_assert_eq!(object_kind, StableObjectKind::BigInt);
        debug_assert_eq!((*target_bigint).header.tag, TAG_BIGINT);
        let elements = self.deserialized_elements();
        debug_assert_eq!((*target_bigint).mp_int.alloc as usize, elements);
        (*target_bigint).mp_int.sign = if self.is_negative { 1 } else { 0 };
        (*target_bigint).mp_int.used = elements as i32;
    }

    unsafe fn deserialize_dynamic_part<M: Memory>(
        &self,
        _main_memory: &mut M,
        stable_memory: &StableMemoryAccess,
        stable_object: StableValue,
        target_bigint: *mut BigInt,
    ) {
        let stable_address = stable_object.payload_address();
        let source_payload =
            stable_address + size_of::<StableBigInt>().to_bytes().as_usize() as u64;
        let target_payload = target_bigint.payload_addr();
        let total_bytes = self.number_of_bits.to_bytes().as_usize();
        let total_elements = self.deserialized_elements();

        let mut read_offset = 0;
        let mut element_index = 0;
        let mut element: ElementType = 0;
        let mut pending_bits = 0;
        let mut next_byte = 0;
        let mut next_pending_bits = 0;

        while element_index < total_elements {
            while pending_bits < USED_BITS_PER_ELEMENT && read_offset < total_bytes {
                debug_assert_eq!(next_pending_bits, 0);
                next_byte = stable_memory.read::<u8>(source_payload + read_offset as u64);
                read_offset += 1;
                element |= (next_byte as ElementType) << pending_bits;
                element &= (1 << USED_BITS_PER_ELEMENT) - 1;
                let consumed_bits = core::cmp::min(u8::BITS, USED_BITS_PER_ELEMENT - pending_bits);
                pending_bits += consumed_bits;
                next_byte = if consumed_bits < u8::BITS {
                    next_byte >> consumed_bits
                } else {
                    0
                };
                next_pending_bits = u8::BITS - consumed_bits;
            }
            *target_payload.add(element_index) = element; // little endian
            element_index += 1;
            element = next_byte as ElementType;
            pending_bits = next_pending_bits;
            next_pending_bits = 0;
        }

        debug_assert!(pending_bits == 0 || element == 0);
        debug_assert_eq!(element_index, self.deserialized_elements());
    }
}
