use crate::{
    stabilization::serialization::stable_memory_stream::StableMemoryStream,
    types::{MutBox, TAG_MUTBOX, Value},
};

use super::{Serializer, StableObjectKind, StableValue, StaticScanner};

#[repr(C)]
pub struct StableMutBox {
    field: StableValue,
}

impl StaticScanner<StableValue> for StableMutBox {
    fn update_pointers<C, F: Fn(&mut C, StableValue) -> StableValue>(
        &mut self,
        context: &mut C,
        translate: &F,
    ) -> bool {
        self.field = translate(context, self.field);
        true
    }
}

impl Serializer<MutBox> for StableMutBox {
    unsafe fn serialize_static_part(
        _stable_memory: &mut StableMemoryStream,
        main_object: *mut MutBox,
    ) -> Self {
        StableMutBox {
            field: StableValue::serialize((*main_object).field),
        }
    }

    unsafe fn deserialize_static_part(
        &self,
        target_mutbox: *mut MutBox,
        object_kind: StableObjectKind,
    ) {
        debug_assert_eq!(object_kind, StableObjectKind::MutBox);
        (*target_mutbox).header.tag = TAG_MUTBOX;
        (*target_mutbox)
            .header
            .init_forward(Value::from_ptr(target_mutbox as usize));
        (*target_mutbox).field = self.field.deserialize();
    }
}
