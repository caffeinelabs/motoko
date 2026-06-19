use crate::{
    stabilization::serialization::stable_memory_stream::StableMemoryStream,
    types::{Region, TAG_REGION, Value},
};

use super::{Serializer, StableObjectKind, StableValue, StaticScanner};

#[repr(C)]
pub struct StableRegion {
    id: u64,
    page_count: u64,
    vec_pages: StableValue, // Blob of `u16`, each denoting a page ID.
}

impl StaticScanner<StableValue> for StableRegion {
    fn update_pointers<C, F: Fn(&mut C, StableValue) -> StableValue>(
        &mut self,
        context: &mut C,
        translate: &F,
    ) -> bool {
        self.vec_pages = translate(context, self.vec_pages);
        true
    }
}

impl Serializer<Region> for StableRegion {
    unsafe fn serialize_static_part(
        _stable_memory: &mut StableMemoryStream,
        main_object: *mut Region,
    ) -> Self {
        StableRegion {
            id: (*main_object).id as u64,
            page_count: (*main_object).page_count as u64,
            vec_pages: StableValue::serialize((*main_object).vec_pages),
        }
    }

    unsafe fn deserialize_static_part(
        &self,
        target_region: *mut Region,
        object_kind: StableObjectKind,
    ) {
        debug_assert_eq!(object_kind, StableObjectKind::Region);
        (*target_region).header.tag = TAG_REGION;
        (*target_region)
            .header
            .init_forward(Value::from_ptr(target_region as usize));
        (*target_region).id = self.id;
        (*target_region).page_count = self.page_count as usize;
        (*target_region).vec_pages = self.vec_pages.deserialize();
    }
}
