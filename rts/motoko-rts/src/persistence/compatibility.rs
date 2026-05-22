//! Persistent type compatibility check.
//! Determines whether a new actor type is compatible with the existing persistent state.
//! Engages the existing IDL subtype check functionality.

use crate::{
    barriers::write_with_barrier,
    bitrel::BitRel,
    constants::WORD_SIZE,
    idl::TypeVariance,
    memory::{Memory, alloc_blob},
    types::{TAG_BLOB_B, Value, Words},
};

const DEFAULT_VALUE: Value = Value::from_scalar(0);

/// Relocatable static type descriptor used for Candid subtypes and program upgrade compatibility checks.
/// The descriptor consists of two blobs, one for the Candid type data and one denoting a vector of types.
/// The vector only stores relative offsets in the Candid data and does not hold any absolute addresses.
/// A type descriptor is used for two cases:
/// * To store the types of the previous program version in the persistent dynamic heap, with the GC
///   potentially moving the blobs.
/// * To load the types of the current program version from passive data segments, without that absolute
///   addresses are known at compile-time.
/// The static type table is only created when calling the Candid subtype or memory compatibility check.
/// As the static table contains absolute addresses, it can only be used temporarily until the next GC increment.
pub struct TypeDescriptor {
    // Blob with candid-encoded type definitions.
    candid_data: Value,
    // Blob with a list of `usize` offsets referring to the `candid_data`.
    type_offsets: Value,
}

impl TypeDescriptor {
    pub fn default() -> Self {
        Self {
            candid_data: DEFAULT_VALUE,
            type_offsets: DEFAULT_VALUE,
        }
    }

    pub fn new(candid_data: Value, type_offsets: Value) -> Self {
        unsafe {
            Self {
                candid_data: candid_data.forward_if_possible(),
                type_offsets: type_offsets.forward_if_possible(),
            }
        }
    }

    pub fn is_default(&self) -> bool {
        self.candid_data == DEFAULT_VALUE && self.type_offsets == DEFAULT_VALUE
    }

    pub fn assert_initialized(&self) {
        assert!(self.candid_data != DEFAULT_VALUE && self.type_offsets != DEFAULT_VALUE);
    }

    pub fn candid_data(&self) -> Value {
        self.candid_data
    }

    pub fn type_offsets(&self) -> Value {
        self.type_offsets
    }

    // GC root if part of the persistent stable type
    pub fn candid_data_location(&mut self) -> *mut Value {
        &mut self.candid_data as *mut Value
    }

    // GC root when part of the persistent stable type
    pub fn type_offsets_location(&mut self) -> *mut Value {
        &mut self.type_offsets as *mut Value
    }

    pub unsafe fn assign<M: Memory>(&mut self, mem: &mut M, other: &Self) {
        let candid_data_location = &mut self.candid_data as *mut Value;
        write_with_barrier(mem, candid_data_location, other.candid_data);
        let type_offsets_location = &mut self.type_offsets as *mut Value;
        write_with_barrier(mem, type_offsets_location, other.type_offsets);
    }

    pub unsafe fn type_count(&self) -> usize {
        let blob_size = self.type_offsets.as_blob().len();
        assert_eq!(blob_size.as_usize() % WORD_SIZE, 0);
        blob_size.to_words().as_usize()
    }

    // NOTE: The resulting type table holds absolute pointers to `candid_data` and can only
    // be used during a single IC message when no GC increment is running in between.
    pub unsafe fn build_type_table<M: Memory>(&mut self, mem: &mut M) -> *mut *mut u8 {
        let type_count = self.type_count();
        let temporary_blob = alloc_blob(mem, TAG_BLOB_B, Words(type_count).to_bytes());
        let offset_table = self.type_offsets.as_blob().payload_const() as *const usize;
        let type_table = temporary_blob.as_blob_mut().payload_addr() as *mut *mut u8;
        let candid_data = self.candid_data.as_blob_mut().payload_addr();
        for index in 0..type_count {
            let offset = *offset_table.add(index);
            debug_assert!(offset < self.candid_length());
            let entry = type_table.add(index);
            *entry = candid_data.add(offset);
        }
        type_table
    }

    pub unsafe fn candid_length(&self) -> usize {
        self.candid_data.as_blob().len().as_usize()
    }

    pub unsafe fn type_table_end(&mut self) -> *mut u8 {
        self.candid_data
            .as_blob_mut()
            .payload_addr()
            .add(self.candid_length())
    }
}

unsafe fn create_type_check_cache<M: Memory>(
    mem: &mut M,
    old_type: &TypeDescriptor,
    new_type: &TypeDescriptor,
) -> BitRel {
    let old_type_count = old_type.type_count();
    let new_type_count = new_type.type_count();
    let words = Words(BitRel::words(old_type_count, new_type_count));
    let byte_length = words.to_bytes();
    let blob_value = alloc_blob(mem, TAG_BLOB_B, byte_length);
    let ptr = blob_value.as_blob_mut().payload_addr() as *mut usize;
    let end = blob_value
        .as_blob()
        .payload_const()
        .add(byte_length.as_usize()) as *mut usize;
    let cache = BitRel {
        ptr,
        end,
        size1: old_type_count,
        size2: new_type_count,
    };
    cache.init();
    cache
}

// Fix main actor type index, see `compile.ml`.
const MAIN_ACTOR_TYPE_INDEX: i32 = 0;

/// Test whether the new stable type complies with the existing old stable type.
/// This uses the existing IDL subtype test.
pub unsafe fn memory_compatible<M: Memory>(
    mem: &mut M,
    old_type: &mut TypeDescriptor,
    new_type: &mut TypeDescriptor,
) -> bool {
    let cache = create_type_check_cache(mem, old_type, new_type);

    let old_type_table = old_type.build_type_table(mem);
    let old_table_end = old_type.type_table_end();

    let new_type_table = new_type.build_type_table(mem);
    let new_table_end = new_type.type_table_end();

    crate::idl::memory_compatible(
        &cache,
        TypeVariance::Covariance,
        old_type_table,
        new_type_table,
        old_table_end,
        new_table_end,
        MAIN_ACTOR_TYPE_INDEX,
        MAIN_ACTOR_TYPE_INDEX,
        true,
    )
}
