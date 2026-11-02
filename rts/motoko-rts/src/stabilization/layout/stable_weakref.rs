use crate::{
    stabilization::serialization::stable_memory_stream::StableMemoryStream,
    types::{TAG_WEAK_REF, Value},
};

use super::{Serializer, StableObjectKind, StableValue, StaticScanner};
use crate::types::WeakRef;

#[repr(C)]
pub struct StableWeakRef {
    target: StableValue,
}

impl StaticScanner<StableValue> for StableWeakRef {
    fn update_pointers<C, F: Fn(&mut C, StableValue) -> StableValue>(
        &mut self,
        context: &mut C,
        translate: &F,
    ) -> bool {
        self.target = translate(context, self.target);
        true
    }
}

impl Serializer<WeakRef> for StableWeakRef {
    unsafe fn serialize_static_part(
        _stable_memory: &mut StableMemoryStream,
        main_object: *mut WeakRef,
    ) -> Self {
        StableWeakRef {
            target: StableValue::serialize((*main_object).field),
        }
    }

    unsafe fn deserialize_static_part(
        &self,
        target_weak_ref: *mut WeakRef,
        object_kind: StableObjectKind,
    ) {
        debug_assert_eq!(object_kind, StableObjectKind::WeakRef);
        (*target_weak_ref).header.tag = TAG_WEAK_REF;
        (*target_weak_ref)
            .header
            .init_forward(Value::from_ptr(target_weak_ref as usize));
        (*target_weak_ref).field = self.target.deserialize();
    }
}
