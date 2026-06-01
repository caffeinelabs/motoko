use crate::{
    memory::Memory,
    stabilization::{
        deserialization::stable_memory_access::StableMemoryAccess,
        layout::{StableObjectKind, stable_blob::StableBlob},
        serialization::{
            SerializationContext,
            stable_memory_stream::{ScanStream, StableMemoryStream, WriteStream},
        },
    },
    types::{FwdPtr, Object, TAG_FWD_PTR, TAG_OBJECT, Tag, Value, Words, size_of},
};

use super::{Serializer, StableTag, StableToSpace, StableValue, StaticScanner};

#[repr(C)]
pub struct StableObject {
    size: u64, // Number of fields.
    hash_blob: StableValue, // Pointer to a blob containing the `u32` hashes of the field labels.
               // Dynamically sized body with `size` fields, each of `StableValue`, ordered according to the hashes in the blob.
}

impl StaticScanner<StableValue> for StableObject {
    fn update_pointers<C, F: Fn(&mut C, StableValue) -> StableValue>(
        &mut self,
        context: &mut C,
        translate: &F,
    ) -> bool {
        self.hash_blob = translate(context, self.hash_blob);
        true
    }
}

impl Serializer<Object> for StableObject {
    unsafe fn serialize_static_part(
        stable_memory: &mut StableMemoryStream,
        main_object: *mut Object,
    ) -> Self {
        StableObject {
            size: get_object_size(stable_memory, main_object) as u64,
            hash_blob: StableValue::serialize((*main_object).hash_blob),
        }
    }

    unsafe fn serialize_dynamic_part(
        stable_memory: &mut StableMemoryStream,
        main_object: *mut Object,
    ) {
        let object_size = get_object_size(stable_memory, main_object);
        for index in 0..object_size {
            let main_field = main_object.get(index);
            let stable_field = StableValue::serialize(main_field);
            stable_memory.write(&stable_field);
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
        for _ in 0..self.size {
            let old_value = context.serialization.to_space().read::<StableValue>();
            // On a longer term, the GC could remove unnecessary fields (during evacuation) that have been
            // declared in old program versions but which name does no longer exist in a new program version.
            let new_value = translate(context, old_value);
            context.serialization.to_space().update(&new_value);
        }
    }

    unsafe fn allocate_deserialized<M: Memory>(
        &self,
        main_memory: &mut M,
        object_kind: StableObjectKind,
    ) -> Value {
        debug_assert_eq!(object_kind, StableObjectKind::Object);
        let total_size = size_of::<Object>() + Words(self.size as usize);
        main_memory.alloc_words(total_size)
    }

    unsafe fn deserialize_static_part(
        &self,
        target_object: *mut Object,
        object_kind: StableObjectKind,
    ) {
        debug_assert_eq!(object_kind, StableObjectKind::Object);
        (*target_object).header.tag = TAG_OBJECT;
        (*target_object)
            .header
            .init_forward(Value::from_ptr(target_object as usize));
        (*target_object).hash_blob = self.hash_blob.deserialize();
    }

    unsafe fn deserialize_dynamic_part<M: Memory>(
        &self,
        _main_memory: &mut M,
        stable_memory: &StableMemoryAccess,
        stable_object: StableValue,
        target_object: *mut Object,
    ) {
        let stable_address = stable_object.payload_address();
        for index in 0..self.size {
            let field_address = stable_address
                + size_of::<StableObject>().to_bytes().as_usize() as u64
                + (index * size_of::<StableValue>().to_bytes().as_usize() as u64);
            let field = stable_memory.read::<StableValue>(field_address);
            let target_field_address = target_object.payload_addr().add(index as usize);
            *target_field_address = field.deserialize();
        }
    }
}

#[repr(C)]
struct HashBlob {
    tag: StableTag,
    header: StableBlob,
}

/// Resolve object size during serialization.
/// This requires a look up in the hash blob, which may however already have been
/// serialized to stable memory.
fn get_object_size(stable_memory: &StableMemoryStream, main_object: *mut Object) -> usize {
    // Do not call tag as it resolves the forwarding pointer.
    unsafe {
        let main_hash_blob = (*main_object).hash_blob;
        let main_tag = *(main_hash_blob.get_ptr() as *const Tag);
        if main_tag == TAG_FWD_PTR {
            // The Hash blob has already been moved to stable memory.
            let target_location = (*(main_hash_blob.get_ptr() as *mut FwdPtr)).fwd;
            let stable_offset = target_location.get_ptr() as u64;
            let stable_hash_blob = stable_memory.read_preceding::<HashBlob>(stable_offset);
            assert!(stable_hash_blob.tag.decode() == StableObjectKind::BlobBytes);
            let hash_blob_length = stable_hash_blob.header.byte_length() as usize;
            let hash_entry_length = size_of::<u64>().to_bytes().as_usize();
            debug_assert_eq!(hash_blob_length % hash_entry_length, 0);
            hash_blob_length / hash_entry_length
        } else {
            main_object.size()
        }
    }
}
