//! Graph-copy-bazed serialization format:
//!
//! (Very first word is zeroed and backed up in last page)
//! -- Stable memory
//! Raw stable memory or region data, size S in pages
//! -- Stable variables
//! Serialized data address N:
//!   Serialized object graph, length L
//!   (possible zero padding)
//! Type descriptor address M:
//!   Candid type table
//!     Byte length (u64)
//!     Data
//!   Type offset table
//!     Byte length (u64)
//!     Data
//!   (possible zero padding)
//! -- Last physical page (metadata):
//!   (zero padding to align at page end)
//!   Upgrade statistics (instructions) (u64)
//!   Serialized data address N (u64)
//!   Serialized data length L (u64)
//!   Type descriptor address M (u64)
//!   First word of page 0
//!   Version 3, 4, 7, or 8 (u32) (match with `VERSION_GRAPH_COPY_{,V1_}{NO_REGIONS,REGIONS}` in `region.rs` and `compile.ml`.
//! -- page end
//!
//! V1 addendum (versions 7/8): a 16-byte extension block is written
//! immediately before the legacy 40-byte record, carrying extra GC roots
//! preserved across the graph-copy upgrade:
//!   Dedup table address (u64)
//!   Migrations list address (u64)
//! V0 records (versions 3/4) carry no extension; the reader clears those
//! fields back to zero after reading when it sees a V0 `Version`.

use crate::{
    barriers::allocation_barrier,
    memory::{Memory, alloc_blob},
    persistence::compatibility::TypeDescriptor,
    region::{
        VERSION_GRAPH_COPY_NO_REGIONS, VERSION_GRAPH_COPY_REGIONS,
        VERSION_GRAPH_COPY_V1_NO_REGIONS, VERSION_GRAPH_COPY_V1_REGIONS,
        VERSION_STABLE_HEAP_NO_REGIONS, VERSION_STABLE_HEAP_REGIONS,
    },
    stabilization::{StableValue, clear_stable_memory, grant_stable_space},
    stable_mem::{
        PAGE_SIZE, get_version, ic0_stable64_read, ic0_stable64_size, ic0_stable64_write, read_u32,
        read_u64, set_version, write_u32, write_u64,
    },
    types::{Bytes, TAG_BLOB_B, Tag, Value, size_of},
};

use super::performance::InstructionMeter;

#[repr(C)]
#[derive(Default)]
pub struct UpgradeStatistics {
    pub stabilization_instructions: u64,
}

/// `#[repr(C)]` laid out to mirror the full V1 last-page record in stable
/// memory: the 16-byte extension (extra GC roots) sits in the first two
/// fields, followed by the legacy 40-byte core ending with `first_word_backup`
/// and `version` at the compiler-hard-coded offsets. This ordering lets a
/// single `ic0_stable64_read`/`write` move the whole record in one go.
///
/// The extension must be at the *start* of the struct because the record is
/// anchored at the *end* of the last stable-memory page (so that `version`
/// lands at `PAGE_SIZE-4`, as the OCaml compiler hard-codes). The extension
/// therefore precedes the legacy core in stable memory too, and only a
/// struct that opens with the extension yields a 1:1 byte mapping over a
/// single `ic0_stable64_read`/`write`.
#[repr(C)]
#[derive(Default)]
pub struct LastPageRecord {
    /// Extra arguments to handle extra GC roots:
    ///     * dedup_table_address
    ///     * migrations_list_address
    /// The two fields are tracking the addresses of
    /// the saved extra GC-roots used specifically for
    /// handling the auxiliary runtime data, i.e., the blob dedup table and the migrations list.
    /// The value is 0 if not present, or a u64 integer with the address.
    pub dedup_table_address: StableValue,
    pub migrations_list_address: StableValue,
    pub statistics: UpgradeStatistics,
    serialized_data_address: u64,
    serialized_data_length: u64,
    type_descriptor_address: u64,
    first_word_backup: u32,
    version: u32,
}

pub struct StabilizationMetadata {
    pub serialized_data_start: u64,
    pub serialized_data_length: u64,
    pub type_descriptor: TypeDescriptor,
}

impl StabilizationMetadata {
    fn ensure_space(offset: u64, length: u64) {
        grant_stable_space(offset + length);
    }

    fn write_length(offset: &mut u64, length: u64) {
        Self::ensure_space(*offset, length as u64);
        write_u64(*offset, length);
        *offset += size_of::<u64>().to_bytes().as_usize() as u64;
    }

    fn write_blob(offset: &mut u64, value: Value) {
        unsafe {
            let length = value.as_blob().len().as_usize() as u64;
            Self::write_length(offset, length);
            Self::ensure_space(*offset, length as u64);
            ic0_stable64_write(
                *offset,
                value.as_blob().payload_const() as u64,
                length as u64,
            );
            *offset += length as u64;
        }
    }

    fn align_page_start(offset: &mut u64) {
        if *offset % PAGE_SIZE != 0 {
            let remainder = PAGE_SIZE - *offset % PAGE_SIZE;
            clear_stable_memory(*offset, remainder);
            *offset += remainder;
        }
    }

    fn save_type_descriptor(offset: &mut u64, descriptor: &TypeDescriptor) {
        Self::write_blob(offset, descriptor.candid_data());
        Self::write_blob(offset, descriptor.type_offsets());
    }

    fn read_length(offset: &mut u64) -> u64 {
        let length = read_u64(*offset);
        // Note: Do not use `types::size_of()` as it rounds to 64-bit words.
        clear_stable_memory(*offset, core::mem::size_of::<u32>() as u64);
        *offset += size_of::<u64>().to_bytes().as_usize() as u64;
        length
    }

    fn read_blob<M: Memory>(mem: &mut M, tag: Tag, offset: &mut u64) -> Value {
        let length = Self::read_length(offset);
        unsafe {
            let value = alloc_blob(mem, tag, Bytes(length as usize));
            ic0_stable64_read(
                value.as_blob_mut().payload_addr() as u64,
                *offset,
                length as u64,
            );
            clear_stable_memory(*offset, length as u64);
            allocation_barrier(value);
            *offset += length as u64;
            value
        }
    }

    fn load_type_descriptor<M: Memory>(mem: &mut M, offset: &mut u64) -> TypeDescriptor {
        let candid_data = Self::read_blob(mem, TAG_BLOB_B, offset);
        let type_offsets = Self::read_blob(mem, TAG_BLOB_B, offset);
        TypeDescriptor::new(candid_data, type_offsets)
    }

    fn metadata_location() -> u64 {
        let physical_pages = unsafe { ic0_stable64_size() };
        assert!(physical_pages > 0);
        let last_page_start = (physical_pages - 1) * PAGE_SIZE;
        let size = size_of::<LastPageRecord>().to_bytes().as_usize() as u64;
        assert!(size < PAGE_SIZE);
        last_page_start + (PAGE_SIZE - size)
    }

    fn write_metadata(value: &LastPageRecord) {
        let offset = Self::metadata_location();
        let size = size_of::<LastPageRecord>().to_bytes().as_usize() as u64;
        Self::ensure_space(offset, size);
        unsafe {
            ic0_stable64_write(offset, value as *const LastPageRecord as u64, size);
        }
    }

    fn read_metadata() -> LastPageRecord {
        let offset = Self::metadata_location();
        let size = size_of::<LastPageRecord>().to_bytes().as_usize() as u64;
        let mut value = LastPageRecord::default();
        unsafe {
            ic0_stable64_read(&mut value as *mut LastPageRecord as u64, offset, size);
        }
        // V0 records did not write the extension; the bytes we just read as
        // `dedup_table_address` / `migrations_list_address` are unrelated
        // data, so force them back to their "absent" sentinel.
        if matches!(
            value.version as usize,
            VERSION_GRAPH_COPY_NO_REGIONS | VERSION_GRAPH_COPY_REGIONS
        ) {
            value.dedup_table_address = StableValue::from_raw(0);
            value.migrations_list_address = StableValue::from_raw(0);
        }
        value
    }

    fn clear_metadata() {
        Self::write_metadata(&LastPageRecord::default());
    }

    pub fn store(
        &self,
        measurement: &mut InstructionMeter,
        dedup_table_address: StableValue,
        migrations_list_address: StableValue,
    ) {
        measurement.start();
        let mut offset = self.serialized_data_start + self.serialized_data_length;
        Self::align_page_start(&mut offset);
        let type_descriptor_address = offset;
        Self::save_type_descriptor(&mut offset, &self.type_descriptor);
        Self::align_page_start(&mut offset);
        let first_word_backup = read_u32(0);
        // Clear very first word that is backed up in the last page.
        // This ensures compatibility with old legacy version 0 using no
        // experimental stable memory and no regions.
        write_u32(0, 0);
        measurement.stop();
        let statistics = UpgradeStatistics {
            stabilization_instructions: measurement.total_elapsed(),
        };
        let last_page_record = LastPageRecord {
            statistics,
            serialized_data_address: self.serialized_data_start,
            serialized_data_length: self.serialized_data_length,
            type_descriptor_address,
            dedup_table_address,
            migrations_list_address,
            first_word_backup,
            version: Self::stabilization_version() as u32,
        };
        Self::write_metadata(&last_page_record);
    }

    pub fn load<M: Memory>(mem: &mut M) -> (StabilizationMetadata, LastPageRecord) {
        let last_page_record = Self::read_metadata();
        Self::clear_metadata();
        let version = last_page_record.version as usize;
        assert!(matches!(
            version,
            VERSION_GRAPH_COPY_NO_REGIONS
                | VERSION_GRAPH_COPY_REGIONS
                | VERSION_GRAPH_COPY_V1_NO_REGIONS
                | VERSION_GRAPH_COPY_V1_REGIONS
        ));
        set_version(version);
        write_u32(0, last_page_record.first_word_backup);
        let mut offset = last_page_record.type_descriptor_address;
        let type_descriptor = Self::load_type_descriptor(mem, &mut offset);
        let metadata = StabilizationMetadata {
            serialized_data_start: last_page_record.serialized_data_address,
            serialized_data_length: last_page_record.serialized_data_length,
            type_descriptor,
        };
        (metadata, last_page_record)
    }

    fn stabilization_version() -> usize {
        match get_version() {
            VERSION_STABLE_HEAP_NO_REGIONS => VERSION_GRAPH_COPY_V1_NO_REGIONS,
            VERSION_STABLE_HEAP_REGIONS => VERSION_GRAPH_COPY_V1_REGIONS,
            _ => unreachable!(),
        }
    }
}
