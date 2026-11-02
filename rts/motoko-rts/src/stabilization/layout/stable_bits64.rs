use crate::{
    stabilization::serialization::stable_memory_stream::StableMemoryStream,
    types::{Bits64, TAG_BITS64_F, TAG_BITS64_S, TAG_BITS64_U, Tag, Value},
};

use super::{Serializer, StableObjectKind, StableValue, StaticScanner};

#[repr(C)]
pub struct StableBits64 {
    bits: u64,
}

impl StaticScanner<StableValue> for StableBits64 {}

impl Serializer<Bits64> for StableBits64 {
    unsafe fn serialize_static_part(
        _stable_memory: &mut StableMemoryStream,
        main_object: *mut Bits64,
    ) -> Self {
        StableBits64 {
            bits: (*main_object).bits as u64,
        }
    }

    unsafe fn deserialize_static_part(
        &self,
        target_bits64: *mut Bits64,
        object_kind: StableObjectKind,
    ) {
        (*target_bits64).header.tag = decode_bits64_tag(object_kind);
        (*target_bits64)
            .header
            .init_forward(Value::from_ptr(target_bits64 as usize));
        (*target_bits64).bits = self.bits;
    }
}

fn decode_bits64_tag(object_kind: StableObjectKind) -> Tag {
    match object_kind {
        StableObjectKind::Bits64Unsigned => TAG_BITS64_U,
        StableObjectKind::Bits64Signed => TAG_BITS64_S,
        StableObjectKind::Bits64Float => TAG_BITS64_F,
        _ => unreachable!("invalid stable boxed word 64 tag"),
    }
}
