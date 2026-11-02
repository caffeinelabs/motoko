use crate::{
    stabilization::serialization::stable_memory_stream::StableMemoryStream,
    types::{TAG_VARIANT, Value, Variant},
};

use super::{Serializer, StableObjectKind, StableValue, StaticScanner};

#[repr(C)]
pub struct StableVariant {
    tag: u64,
    field: StableValue,
}

impl StaticScanner<StableValue> for StableVariant {
    fn update_pointers<C, F: Fn(&mut C, StableValue) -> StableValue>(
        &mut self,
        context: &mut C,
        translate: &F,
    ) -> bool {
        self.field = translate(context, self.field);
        true
    }
}

impl Serializer<Variant> for StableVariant {
    unsafe fn serialize_static_part(
        _stable_memory: &mut StableMemoryStream,
        main_object: *mut Variant,
    ) -> Self {
        StableVariant {
            tag: (*main_object).tag as u64,
            field: StableValue::serialize((*main_object).field),
        }
    }

    unsafe fn deserialize_static_part(
        &self,
        target_variant: *mut Variant,
        object_kind: StableObjectKind,
    ) {
        debug_assert_eq!(object_kind, StableObjectKind::Variant);
        (*target_variant).header.tag = TAG_VARIANT;
        (*target_variant)
            .header
            .init_forward(Value::from_ptr(target_variant as usize));
        (*target_variant).tag = self.tag as usize;
        (*target_variant).field = self.field.deserialize();
    }
}
