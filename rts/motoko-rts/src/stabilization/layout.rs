//! The long-term layout definition of the stable format.
//!
//! The stable object graph resides a linear stable memory space.
//!
//! Pointers are serialized as 64-bit skewed offsets in that space.
//! The stable encoding of scalars is currently identical to the
//! main memory encoding, requiring the precise scalar tag as metadata.
//! If the main memory scalar representation changes in future, the
//! values need to be explicitly translated to the expected stable format.
//!
//! Each object uses a `StableTag` as header and is followed by
//! the object payload as outlined in the corresponding Rust structs.
//! Some objects, such as `StableArray`, `StableObject`, `StableBlob`,
//! and `StableBigNum` have a dynamic payload body in addition to a static
//! header.
//!
//! Not all heap memory object types are stabilized because some
//! of them are not stable types. New object types can be added
//! with backwards compatibility but encoding changes to existing stable
//! data types must be handled with extra care to ensure backwards compatibility.
//!
//! Note: For a potential downgrade to 32-bit, the 64-bit stable pointer
//! offsets can be scaled down by a factor `8` during the destabilization
//! such that they fit into 32-bit values during Cheney's graph-copy.

use crate::{
    barriers::allocation_barrier,
    constants::WORD_SIZE,
    memory::Memory,
    rts_trap_with,
    types::{
        base_array_tag, size_of, Tag, Value, TAG_ARRAY_I, TAG_ARRAY_M, TAG_ARRAY_S,
        TAG_ARRAY_SLICE_MIN, TAG_ARRAY_T, TAG_BIGINT, TAG_BITS64_F, TAG_BITS64_S, TAG_BITS64_U,
        TAG_BLOB_A, TAG_BLOB_B, TAG_BLOB_P, TAG_BLOB_T, TAG_CONCAT, TAG_MUTBOX, TAG_OBJECT,
        TAG_REGION, TAG_SOME, TAG_VARIANT, TAG_WEAK_REF, TRUE_VALUE,
    },
};

use self::{
    stable_array::StableArray, stable_bigint::StableBigInt, stable_bits64::StableBits64,
    stable_blob::StableBlob, stable_concat::StableConcat, stable_mutbox::StableMutBox,
    stable_object::StableObject, stable_region::StableRegion, stable_some::StableSome,
    stable_variant::StableVariant, stable_weakref::StableWeakRef,
};

use super::{
    deserialization::stable_memory_access::StableMemoryAccess,
    serialization::{
        stable_memory_stream::{ScanStream, StableMemoryStream, WriteStream},
        SerializationContext,
    },
};

mod stable_array;
mod stable_bigint;
mod stable_bits64;
mod stable_blob;
mod stable_concat;
mod stable_mutbox;
mod stable_object;
mod stable_region;
mod stable_some;
mod stable_variant;
mod stable_weakref;

/// Different kinds of objects used in the stable format.
#[repr(u64)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum StableObjectKind {
    ArrayImmutable = 1,
    ArrayMutable = 2,
    ArrayTuple = 3,
    ArraySharedFunction = 4,
    MutBox = 5,
    Object = 6,
    BlobBytes = 7,
    BlobText = 8,
    BlobPrincipal = 9,
    BlobActor = 10,
    Bits64Unsigned = 11,
    Bits64Signed = 12,
    Bits64Float = 13,
    Region = 14,
    Variant = 15,
    Concat = 16,
    BigInt = 17,
    Some = 18,
    WeakRef = 19,
}

#[repr(C)]
pub struct StableTag(u64);

impl StableObjectKind {
    pub fn encode(&self) -> StableTag {
        StableTag(*self as u64)
    }
}

impl StableTag {
    pub fn decode(&self) -> StableObjectKind {
        const STABLE_TAG_ARRAY_IMMUTABLE: u64 = StableObjectKind::ArrayImmutable as u64;
        const STABLE_TAG_ARRAY_MUTABLE: u64 = StableObjectKind::ArrayMutable as u64;
        const STABLE_TAG_ARRAY_TUPLE: u64 = StableObjectKind::ArrayTuple as u64;
        const STABLE_TAG_ARRAY_SHARED_FUNCTION: u64 = StableObjectKind::ArraySharedFunction as u64;
        const STABLE_TAG_MUTBOX: u64 = StableObjectKind::MutBox as u64;
        const STABLE_TAG_OBJECT: u64 = StableObjectKind::Object as u64;
        const STABLE_TAG_BLOB_BYTES: u64 = StableObjectKind::BlobBytes as u64;
        const STABLE_TAG_BLOB_TEXT: u64 = StableObjectKind::BlobText as u64;
        const STABLE_TAG_BLOB_PRINCIPAL: u64 = StableObjectKind::BlobPrincipal as u64;
        const STABLE_TAG_BLOB_ACTOR: u64 = StableObjectKind::BlobActor as u64;
        const STABLE_TAG_BITS64_UNSIGNED: u64 = StableObjectKind::Bits64Unsigned as u64;
        const STABLE_TAG_BITS64_SIGNED: u64 = StableObjectKind::Bits64Signed as u64;
        const STABLE_TAG_BITS64_FLOAT: u64 = StableObjectKind::Bits64Float as u64;
        const STABLE_TAG_REGION: u64 = StableObjectKind::Region as u64;
        const STABLE_TAG_VARIANT: u64 = StableObjectKind::Variant as u64;
        const STABLE_TAG_CONCAT: u64 = StableObjectKind::Concat as u64;
        const STABLE_TAG_BIGINT: u64 = StableObjectKind::BigInt as u64;
        const STABLE_TAG_SOME: u64 = StableObjectKind::Some as u64;
        const STABLE_TAG_WEAK_REF: u64 = StableObjectKind::WeakRef as u64;
        match self.0 {
            STABLE_TAG_ARRAY_IMMUTABLE => StableObjectKind::ArrayImmutable,
            STABLE_TAG_ARRAY_MUTABLE => StableObjectKind::ArrayMutable,
            STABLE_TAG_ARRAY_TUPLE => StableObjectKind::ArrayTuple,
            STABLE_TAG_ARRAY_SHARED_FUNCTION => StableObjectKind::ArraySharedFunction,
            STABLE_TAG_MUTBOX => StableObjectKind::MutBox,
            STABLE_TAG_OBJECT => StableObjectKind::Object,
            STABLE_TAG_BLOB_BYTES => StableObjectKind::BlobBytes,
            STABLE_TAG_BLOB_TEXT => StableObjectKind::BlobText,
            STABLE_TAG_BLOB_PRINCIPAL => StableObjectKind::BlobPrincipal,
            STABLE_TAG_BLOB_ACTOR => StableObjectKind::BlobActor,
            STABLE_TAG_BITS64_UNSIGNED => StableObjectKind::Bits64Unsigned,
            STABLE_TAG_BITS64_SIGNED => StableObjectKind::Bits64Signed,
            STABLE_TAG_BITS64_FLOAT => StableObjectKind::Bits64Float,
            STABLE_TAG_REGION => StableObjectKind::Region,
            STABLE_TAG_VARIANT => StableObjectKind::Variant,
            STABLE_TAG_CONCAT => StableObjectKind::Concat,
            STABLE_TAG_BIGINT => StableObjectKind::BigInt,
            STABLE_TAG_SOME => StableObjectKind::Some,
            STABLE_TAG_WEAK_REF => StableObjectKind::WeakRef,
            _ => rts_trap_with("Invalid tag"),
        }
    }
}

impl StableObjectKind {
    fn deserialize(tag: Tag) -> StableObjectKind {
        match tag {
            TAG_WEAK_REF => StableObjectKind::WeakRef,
            // During the marking phase of the incremental GC, the mutator can see
            // array slice information in the object tag.
            TAG_ARRAY_I | TAG_ARRAY_M | TAG_ARRAY_T | TAG_ARRAY_S | TAG_ARRAY_SLICE_MIN.. => {
                match base_array_tag(tag) {
                    TAG_ARRAY_I => StableObjectKind::ArrayImmutable,
                    TAG_ARRAY_M => StableObjectKind::ArrayMutable,
                    TAG_ARRAY_T => StableObjectKind::ArrayTuple,
                    TAG_ARRAY_S => StableObjectKind::ArraySharedFunction,
                    _ => unreachable!("invalid array tag"),
                }
            }
            TAG_MUTBOX => StableObjectKind::MutBox,
            TAG_OBJECT => StableObjectKind::Object,
            TAG_BLOB_B => StableObjectKind::BlobBytes,
            TAG_BLOB_T => StableObjectKind::BlobText,
            TAG_BLOB_P => StableObjectKind::BlobPrincipal,
            TAG_BLOB_A => StableObjectKind::BlobActor,
            TAG_BITS64_U => StableObjectKind::Bits64Unsigned,
            TAG_BITS64_S => StableObjectKind::Bits64Signed,
            TAG_BITS64_F => StableObjectKind::Bits64Float,
            TAG_REGION => StableObjectKind::Region,
            TAG_VARIANT => StableObjectKind::Variant,
            TAG_CONCAT => StableObjectKind::Concat,
            TAG_BIGINT => StableObjectKind::BigInt,
            TAG_SOME => StableObjectKind::Some,

            _ => unreachable!("invalid tag"),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Default)]
pub struct StableValue(u64);

impl StableValue {
    fn is_ptr(&self) -> bool {
        self.0 & 0b1 == 1 && self.0 != TRUE_VALUE as u64
    }

    fn skew(address: u64) -> u64 {
        address.wrapping_sub(1)
    }

    fn unskew(pointer: u64) -> u64 {
        debug_assert!(Self::from_raw(pointer).is_ptr());
        pointer.wrapping_add(1)
    }

    pub const fn from_raw(value: u64) -> Self {
        StableValue(value)
    }

    pub fn from_stable_address(address: u64) -> Self {
        debug_assert_eq!(address % WORD_SIZE as u64, 0);
        StableValue(Self::skew(address))
    }

    pub fn to_stable_address(&self) -> u64 {
        Self::unskew(self.0)
    }

    pub fn payload_address(&self) -> u64 {
        self.to_stable_address() + size_of::<StableTag>().to_bytes().as_usize() as u64
    }

    pub fn serialize(value: Value) -> Self {
        StableValue(value.get_raw() as u64)
    }

    pub fn deserialize(&self) -> Value {
        Value::from_raw(self.0 as usize)
    }
}

/// Scan the static part of the object.
trait StaticScanner<T> {
    // Updates potential pointers in the static part of the object.
    // Returns true if values have been updated.
    fn update_pointers<C, F: Fn(&mut C, T) -> T>(
        &mut self,
        _context: &mut C,
        _translate: &F,
    ) -> bool {
        false
    }
}

pub trait StableToSpace {
    fn to_space(&mut self) -> &mut StableMemoryStream;
}

trait Serializer<T>
where
    Self: Sized + StaticScanner<StableValue>,
{
    unsafe fn serialize_static_part(
        stable_memory: &mut StableMemoryStream,
        main_object: *mut T,
    ) -> Self;
    unsafe fn serialize_dynamic_part(
        _stable_memory: &mut StableMemoryStream,
        _main_object: *mut T,
    ) {
    }

    unsafe fn serialize(stable_memory: &mut StableMemoryStream, main_object: Value) {
        let stable_tag = StableObjectKind::deserialize(main_object.tag()).encode();
        let main_object = main_object.as_obj() as *mut T;
        stable_memory.write(&stable_tag);
        unsafe {
            let static_part = Self::serialize_static_part(stable_memory, main_object);
            stable_memory.write(&static_part);
            Self::serialize_dynamic_part(stable_memory, main_object);
        }
    }

    fn scan_serialized<
        'a,
        M,
        F: Fn(&mut SerializationContext<'a, M>, StableValue) -> StableValue,
    >(
        context: &mut SerializationContext<'a, M>,
        translate: &F,
    ) {
        let mut static_part = context.serialization.to_space().read::<Self>();
        if static_part.update_pointers(context, translate) {
            context.serialization.to_space().update(&static_part);
        }
        static_part.scan_serialized_dynamic(context, translate);
    }

    fn scan_serialized_dynamic<
        'a,
        M,
        F: Fn(&mut SerializationContext<'a, M>, StableValue) -> StableValue,
    >(
        &self,
        _context: &mut SerializationContext<'a, M>,
        _translate: &F,
    ) {
    }

    unsafe fn allocate_deserialized<M: Memory>(
        &self,
        main_memory: &mut M,
        _object_kind: StableObjectKind,
    ) -> Value {
        main_memory.alloc_words(size_of::<T>())
    }

    unsafe fn deserialize_static_part(&self, target_object: *mut T, object_kind: StableObjectKind);

    unsafe fn deserialize_dynamic_part<M: Memory>(
        &self,
        _main_memory: &mut M,
        _stable_memory: &StableMemoryAccess,
        _stable_object: StableValue,
        _target_object: *mut T,
    ) {
    }

    unsafe fn deserialize<M: Memory>(
        main_memory: &mut M,
        stable_memory: &StableMemoryAccess,
        stable_object: StableValue,
        object_kind: StableObjectKind,
    ) -> Value {
        let stable_address = stable_object.payload_address();
        let stable_static_part = stable_memory.read::<Self>(stable_address);
        let target = stable_static_part.allocate_deserialized(main_memory, object_kind);
        let target_object = target.get_ptr() as *mut T;
        stable_static_part.deserialize_static_part(target_object, object_kind);
        stable_static_part.deserialize_dynamic_part(
            main_memory,
            stable_memory,
            stable_object,
            target_object,
        );
        allocation_barrier(target)
    }
}

pub fn round_to_u64(length: u64) -> u64 {
    let alignment = size_of::<u64>().to_bytes().as_usize() as u64;
    (length + alignment - 1) / alignment * alignment
}

fn write_padding_u64(stable_memory: &mut StableMemoryStream, byte_length: usize) {
    let rounded_length = round_to_u64(byte_length as u64);
    let padding = rounded_length - byte_length as u64;
    for _ in 0..padding {
        stable_memory.write(&0u8);
    }
}

pub fn scan_serialized<
    'a,
    M,
    F: Fn(&mut SerializationContext<'a, M>, StableValue) -> StableValue,
>(
    context: &mut SerializationContext<'a, M>,
    translate: &F,
) {
    if context.serialization.pending_array_scanning() {
        StableArray::resume_scanning(context, translate);
        return;
    }
    let tag = context.serialization.to_space().read::<StableTag>();
    match tag.decode() {
        StableObjectKind::ArrayImmutable
        | StableObjectKind::ArrayMutable
        | StableObjectKind::ArrayTuple
        | StableObjectKind::ArraySharedFunction => StableArray::scan_serialized(context, translate),
        StableObjectKind::MutBox => StableMutBox::scan_serialized(context, translate),
        StableObjectKind::Object => StableObject::scan_serialized(context, translate),
        StableObjectKind::BlobBytes
        | StableObjectKind::BlobText
        | StableObjectKind::BlobPrincipal
        | StableObjectKind::BlobActor => StableBlob::scan_serialized(context, translate),
        StableObjectKind::Bits64Unsigned
        | StableObjectKind::Bits64Signed
        | StableObjectKind::Bits64Float => StableBits64::scan_serialized(context, translate),
        StableObjectKind::Region => StableRegion::scan_serialized(context, translate),
        StableObjectKind::Variant => StableVariant::scan_serialized(context, translate),
        StableObjectKind::Concat => StableConcat::scan_serialized(context, translate),
        StableObjectKind::BigInt => StableBigInt::scan_serialized(context, translate),
        StableObjectKind::Some => StableSome::scan_serialized(context, translate),
        StableObjectKind::WeakRef => StableWeakRef::scan_serialized(context, translate),
    }
}

pub unsafe fn serialize(stable_memory: &mut StableMemoryStream, main_object: Value) {
    match StableObjectKind::deserialize(main_object.tag()) {
        StableObjectKind::ArrayImmutable
        | StableObjectKind::ArrayMutable
        | StableObjectKind::ArrayTuple
        | StableObjectKind::ArraySharedFunction => {
            StableArray::serialize(stable_memory, main_object)
        }
        StableObjectKind::MutBox => StableMutBox::serialize(stable_memory, main_object),
        StableObjectKind::Object => StableObject::serialize(stable_memory, main_object),
        StableObjectKind::BlobBytes
        | StableObjectKind::BlobText
        | StableObjectKind::BlobPrincipal
        | StableObjectKind::BlobActor => StableBlob::serialize(stable_memory, main_object),
        StableObjectKind::Bits64Unsigned
        | StableObjectKind::Bits64Signed
        | StableObjectKind::Bits64Float => StableBits64::serialize(stable_memory, main_object),
        StableObjectKind::Region => StableRegion::serialize(stable_memory, main_object),
        StableObjectKind::Variant => StableVariant::serialize(stable_memory, main_object),
        StableObjectKind::Concat => StableConcat::serialize(stable_memory, main_object),
        StableObjectKind::BigInt => StableBigInt::serialize(stable_memory, main_object),
        StableObjectKind::Some => StableSome::serialize(stable_memory, main_object),
        StableObjectKind::WeakRef => StableWeakRef::serialize(stable_memory, main_object),
    }
}

pub unsafe fn deserialize<M: Memory>(
    main_memory: &mut M,
    stable_memory: &mut StableMemoryAccess,
    stable_object: StableValue,
) -> Value {
    let tag = stable_memory.read::<StableTag>(stable_object.to_stable_address());
    let object_kind = tag.decode();
    match object_kind {
        StableObjectKind::ArrayImmutable
        | StableObjectKind::ArrayMutable
        | StableObjectKind::ArrayTuple
        | StableObjectKind::ArraySharedFunction => {
            StableArray::deserialize(main_memory, stable_memory, stable_object, object_kind)
        }
        StableObjectKind::MutBox => {
            StableMutBox::deserialize(main_memory, stable_memory, stable_object, object_kind)
        }
        StableObjectKind::Object => {
            StableObject::deserialize(main_memory, stable_memory, stable_object, object_kind)
        }
        StableObjectKind::BlobBytes
        | StableObjectKind::BlobText
        | StableObjectKind::BlobPrincipal
        | StableObjectKind::BlobActor => {
            StableBlob::deserialize(main_memory, stable_memory, stable_object, object_kind)
        }
        StableObjectKind::Bits64Unsigned
        | StableObjectKind::Bits64Signed
        | StableObjectKind::Bits64Float => {
            StableBits64::deserialize(main_memory, stable_memory, stable_object, object_kind)
        }
        StableObjectKind::Region => {
            StableRegion::deserialize(main_memory, stable_memory, stable_object, object_kind)
        }
        StableObjectKind::Variant => {
            StableVariant::deserialize(main_memory, stable_memory, stable_object, object_kind)
        }
        StableObjectKind::Concat => {
            StableConcat::deserialize(main_memory, stable_memory, stable_object, object_kind)
        }
        StableObjectKind::BigInt => {
            StableBigInt::deserialize(main_memory, stable_memory, stable_object, object_kind)
        }
        StableObjectKind::Some => {
            StableSome::deserialize(main_memory, stable_memory, stable_object, object_kind)
        }
        StableObjectKind::WeakRef => {
            StableWeakRef::deserialize(main_memory, stable_memory, stable_object, object_kind)
        }
    }
}
