use crate::{
    memory::{Memory, alloc_array},
    stabilization::{
        deserialization::stable_memory_access::StableMemoryAccess,
        graph_copy::GraphCopy,
        serialization::{
            ArraySlice, SerializationContext,
            stable_memory_stream::{ScanStream, StableMemoryStream, WriteStream},
        },
    },
    types::{Array, TAG_ARRAY_I, TAG_ARRAY_M, TAG_ARRAY_S, TAG_ARRAY_T, Tag, Value, size_of},
};

use super::{Serializer, StableObjectKind, StableToSpace, StableValue, StaticScanner};

#[repr(C)]
pub struct StableArray {
    array_length: u64,
    // Dynamically sized body with `array_length` elements, each of `StableValue`.
}

impl StaticScanner<StableValue> for StableArray {}

impl Serializer<Array> for StableArray {
    unsafe fn serialize_static_part(
        _stable_memory: &mut StableMemoryStream,
        array: *mut Array,
    ) -> Self {
        StableArray {
            array_length: array.len() as u64,
        }
    }

    unsafe fn serialize_dynamic_part(
        stable_memory: &mut StableMemoryStream,
        main_array: *mut Array,
    ) {
        for index in 0..main_array.len() {
            let main_element = main_array.get(index);
            let stable_element = StableValue::serialize(main_element);
            stable_memory.write(&stable_element);
        }
    }

    fn scan_serialized_dynamic<
        'a,
        M,
        F: Fn(&mut SerializationContext<'a, M>, StableValue) -> StableValue,
    >(
        &self,
        context: &mut SerializationContext<'a, M>,
        translate: &F,
    ) {
        debug_assert!(!context.serialization.pending_array_scanning());
        let mut index = 0;
        self.sliced_array_scan(context, translate, &mut index);
        if index < self.array_length {
            let slice = ArraySlice::new(index, self.array_length);
            context.serialization.set_array_slice(slice);
        }
    }

    unsafe fn allocate_deserialized<M: Memory>(
        &self,
        main_memory: &mut M,
        object_kind: StableObjectKind,
    ) -> Value {
        let array_tag = decode_array_tag(object_kind);
        let array_length = self.array_length as usize;
        alloc_array(main_memory, array_tag, array_length)
    }

    unsafe fn deserialize_static_part(
        &self,
        target_array: *mut Array,
        object_kind: StableObjectKind,
    ) {
        debug_assert_eq!((*target_array).header.tag, decode_array_tag(object_kind));
        debug_assert_eq!((*target_array).len as u64, self.array_length);
    }

    unsafe fn deserialize_dynamic_part<M: Memory>(
        &self,
        _main_memory: &mut M,
        stable_memory: &StableMemoryAccess,
        stable_object: StableValue,
        target_array: *mut Array,
    ) {
        let stable_address = stable_object.payload_address();
        let mut element_address =
            stable_address + size_of::<StableArray>().to_bytes().as_usize() as u64;
        for index in 0..(*target_array).len {
            let element = stable_memory.read::<StableValue>(element_address);
            target_array.set_raw(index, element.deserialize());
            element_address += size_of::<StableValue>().to_bytes().as_usize() as u64;
        }
    }
}

fn decode_array_tag(object_kind: StableObjectKind) -> Tag {
    match object_kind {
        StableObjectKind::ArrayImmutable => TAG_ARRAY_I,
        StableObjectKind::ArrayMutable => TAG_ARRAY_M,
        StableObjectKind::ArrayTuple => TAG_ARRAY_T,
        StableObjectKind::ArraySharedFunction => TAG_ARRAY_S,
        _ => unreachable!("invalid stable array tag"),
    }
}

impl StableArray {
    pub fn resume_scanning<
        'a,
        M,
        F: Fn(&mut SerializationContext<'a, M>, StableValue) -> StableValue,
    >(
        context: &mut SerializationContext<'a, M>,
        translate: &F,
    ) {
        let mut slice = context.serialization.get_array_slice();
        let stable_array = StableArray {
            array_length: slice.array_length,
        };
        stable_array.sliced_array_scan(context, translate, &mut slice.next_index);
        if slice.next_index < slice.array_length {
            context.serialization.set_array_slice(slice);
        }
    }

    fn sliced_array_scan<
        'a,
        M,
        F: Fn(&mut SerializationContext<'a, M>, StableValue) -> StableValue,
    >(
        &self,
        context: &mut SerializationContext<'a, M>,
        translate: &F,
        index: &mut u64,
    ) {
        while *index < self.array_length {
            let old_value = context.serialization.to_space().read::<StableValue>();
            let new_value = translate(context, old_value);
            context.serialization.to_space().update(&new_value);
            *index += 1;
            if context.serialization.time_over() {
                return;
            }
        }
    }
}
