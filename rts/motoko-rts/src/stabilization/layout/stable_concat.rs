use crate::{
    stabilization::serialization::stable_memory_stream::StableMemoryStream,
    types::{Bytes, Concat, TAG_CONCAT, Value},
};

use super::{Serializer, StableObjectKind, StableValue, StaticScanner};

#[repr(C)]
pub struct StableConcat {
    number_of_bytes: u64,
    text1: StableValue,
    text2: StableValue,
}

impl StaticScanner<StableValue> for StableConcat {
    fn update_pointers<C, F: Fn(&mut C, StableValue) -> StableValue>(
        &mut self,
        context: &mut C,
        translate: &F,
    ) -> bool {
        self.text1 = translate(context, self.text1);
        self.text2 = translate(context, self.text2);
        true
    }
}

impl Serializer<Concat> for StableConcat {
    unsafe fn serialize_static_part(
        _stable_memory: &mut StableMemoryStream,
        main_object: *mut Concat,
    ) -> Self {
        StableConcat {
            number_of_bytes: (*main_object).n_bytes.as_usize() as u64,
            text1: StableValue::serialize(main_object.text1()),
            text2: StableValue::serialize(main_object.text2()),
        }
    }

    unsafe fn deserialize_static_part(
        &self,
        target_concat: *mut Concat,
        object_kind: StableObjectKind,
    ) {
        debug_assert_eq!(object_kind, StableObjectKind::Concat);
        let n_bytes = Bytes(self.number_of_bytes as usize);
        (*target_concat).header.tag = TAG_CONCAT;
        (*target_concat)
            .header
            .init_forward(Value::from_ptr(target_concat as usize));
        (*target_concat).n_bytes = n_bytes;
        (*target_concat).text1 = self.text1.deserialize();
        (*target_concat).text2 = self.text2.deserialize();
    }
}
