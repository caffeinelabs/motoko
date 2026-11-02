use crate::{
    memory::{Memory, alloc_blob},
    stabilization::{
        deserialization::stable_memory_access::StableMemoryAccess,
        serialization::{
            SerializationContext,
            stable_memory_stream::{ScanStream, StableMemoryStream, WriteStream},
        },
    },
    types::{Blob, Bytes, TAG_BLOB_A, TAG_BLOB_B, TAG_BLOB_P, TAG_BLOB_T, Tag, Value, size_of},
};

use super::{
    Serializer, StableObjectKind, StableToSpace, StableValue, StaticScanner, round_to_u64,
    write_padding_u64,
};

#[repr(C)]
#[derive(Default)]
pub struct StableBlob {
    byte_length: u64,
    // Dynamically sized body with `byte_length` bytes. No pointers to be scanned.
    // Zero padding to align to next `u64`.
    // Note: The rounding of object sizes to at least 2 bytes is necessary for the skewed pointer representation.
}

impl StableBlob {
    pub fn byte_length(&self) -> u64 {
        self.byte_length
    }
}

impl StaticScanner<StableValue> for StableBlob {}

impl Serializer<Blob> for StableBlob {
    unsafe fn serialize_static_part(
        _stable_memory: &mut StableMemoryStream,
        main_object: *mut Blob,
    ) -> Self {
        StableBlob {
            byte_length: main_object.len().as_usize() as u64,
        }
    }

    unsafe fn serialize_dynamic_part(
        stable_memory: &mut StableMemoryStream,
        main_object: *mut Blob,
    ) {
        let byte_length = main_object.len().as_usize();
        stable_memory.raw_write(main_object.payload_addr() as usize, byte_length);
        write_padding_u64(stable_memory, byte_length);
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
        let rounded_length = round_to_u64(self.byte_length);
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
        let blob_tag = decode_blob_tag(object_kind);
        let blob_length = self.byte_length as usize;
        alloc_blob(main_memory, blob_tag, Bytes(blob_length))
    }

    unsafe fn deserialize_static_part(
        &self,
        target_blob: *mut Blob,
        object_kind: StableObjectKind,
    ) {
        debug_assert_eq!((*target_blob).header.tag, decode_blob_tag(object_kind));
        debug_assert_eq!((*target_blob).len.as_usize(), self.byte_length as usize);
    }

    unsafe fn deserialize_dynamic_part<M: Memory>(
        &self,
        _main_memory: &mut M,
        stable_memory: &StableMemoryAccess,
        stable_object: StableValue,
        target_blob: *mut Blob,
    ) {
        let stable_address = stable_object.payload_address();
        let source_payload = stable_address + size_of::<StableBlob>().to_bytes().as_usize() as u64;
        let target_payload = target_blob.payload_addr() as usize;
        let blob_length = self.byte_length as usize;
        stable_memory.raw_read(source_payload, target_payload, blob_length);
    }
}

fn decode_blob_tag(object_kind: StableObjectKind) -> Tag {
    match object_kind {
        StableObjectKind::BlobBytes => TAG_BLOB_B,
        StableObjectKind::BlobText => TAG_BLOB_T,
        StableObjectKind::BlobPrincipal => TAG_BLOB_P,
        StableObjectKind::BlobActor => TAG_BLOB_A,
        _ => unreachable!("invalid stable blob tag"),
    }
}
