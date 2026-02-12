#[cfg(feature = "ic")]
pub mod ic;
use crate::{constants::MAX_ARRAY_LENGTH_FOR_ITERATOR, types::*};

use motoko_rts_macros::classical_persistence;
use motoko_rts_macros::enhanced_orthogonal_persistence;
use motoko_rts_macros::ic_mem_fn;

#[enhanced_orthogonal_persistence]
use crate::barriers::init_with_barrier;

#[cfg(feature = "ic")]
use crate::constants::MB;

#[enhanced_orthogonal_persistence]
use crate::constants::GB;

// TODO: Redesign for 64-bit support by using a dynamic partition list.
/// Currently limited to 64 GB.
#[enhanced_orthogonal_persistence]
pub const MAXIMUM_MEMORY_SIZE: Bytes<u64> = Bytes(64u64 * GB as u64);

#[classical_persistence]
pub const MAXIMUM_MEMORY_SIZE: Bytes<u64> = Bytes(usize::MAX as u64);

// Memory reserve in bytes ensured during update and initialization calls.
// For use by queries and upgrade calls.
#[cfg(feature = "ic")]
pub(crate) const GENERAL_MEMORY_RESERVE: usize = 256 * MB;

/// A trait for heap allocation. RTS functions allocate in heap via this trait.
///
/// To be able to link the RTS with moc-generated code, we implement wrappers around allocating
/// functions that pass `ic::IcMemory` for the `Memory` arguments, and export these functions with
/// the expected names for the generated code. For example, for a function like
///
/// ```
/// unsafe fn allocating_function<M: Memory>(mem: &mut M) { ... }
/// ```
///
/// we implement (or generate with a macro)
///
/// ```
/// #[no_mangle]
/// unsafe extern "C" fn export_name() { allocating_function(crate::memory::ic::IcMemory) }
/// ```
///
/// This function does not take any `Memory` arguments can be used by the generated code.
pub trait Memory {
    // General allocator working for all GC variants.
    unsafe fn alloc_words(&mut self, n: Words<usize>) -> Value;

    // Grow the allocated memory size to at least the address of `ptr`.
    unsafe fn grow_memory(&mut self, ptr: usize);
}

/// Allocate a new blob.
/// Note: After initialization, the post allocation barrier needs to be applied to all mutator objects.
/// For RTS-internal blobs that can be collected by the next GC run, the post allocation barrier can be omitted.
#[ic_mem_fn]
pub unsafe fn alloc_blob<M: Memory>(mem: &mut M, tag: Tag, size: Bytes<usize>) -> Value {
    debug_assert!(is_blob_tag(tag));
    let ptr = mem.alloc_words(size_of::<Blob>() + size.to_words());
    // NB. Cannot use `as_blob` here as we didn't write the header yet
    let blob = ptr.get_ptr() as *mut Blob;
    (*blob).header.tag = tag;
    (*blob).header.init_forward(ptr);
    (*blob).len = size;

    ptr
}

/// Allocate a new array.
/// Note: After initialization, the post allocation barrier needs to be applied to all mutator objects.
#[ic_mem_fn]
pub unsafe fn alloc_array<M: Memory>(mem: &mut M, tag: Tag, len: usize) -> Value {
    debug_assert!(is_base_array_tag(tag));
    assert!(len <= MAX_ARRAY_LENGTH_FOR_ITERATOR);

    let skewed_ptr = mem.alloc_words(size_of::<Array>() + Words(len));

    let ptr: *mut Array = skewed_ptr.get_ptr() as *mut Array;
    (*ptr).header.tag = tag;
    (*ptr).header.init_forward(skewed_ptr);
    (*ptr).len = len;

    skewed_ptr
}

/// Allocate a new weak reference.
#[enhanced_orthogonal_persistence]
#[ic_mem_fn]
pub unsafe fn alloc_weak_ref<M: Memory>(mem: &mut M, target: Value) -> Value {
    use crate::barriers::allocation_barrier;

    let weak_ref = mem.alloc_words(crate::types::size_of::<WeakRef>());
    let weak_ref_obj = weak_ref.get_ptr() as *mut WeakRef;
    (*weak_ref_obj).header.tag = TAG_WEAK_REF;
    (*weak_ref_obj).header.init_forward(weak_ref);
    init_with_barrier(mem, &mut (*weak_ref_obj).field, target);
    allocation_barrier(weak_ref)
}

/// Check if a weak reference is still live.
#[enhanced_orthogonal_persistence]
#[ic_mem_fn]
pub unsafe fn weak_ref_is_live<M: Memory>(_mem: &mut M, weak_ref: Value) -> bool {
    if !weak_ref.is_non_null_ptr() {
        crate::rts_trap_with(
            "weak_ref_is_live: Invalid WeakRef pointer. This is a bug, report to the Motoko team.",
        );
    }
    let weak_ref_obj = weak_ref.get_ptr() as *mut WeakRef;
    return (*weak_ref_obj).is_live();
}

/// Get the dedup table.
#[enhanced_orthogonal_persistence]
#[ic_mem_fn]
#[cfg(feature = "ic")]
pub unsafe fn get_dedup_table<M: Memory>(_mem: &mut M) -> Value {
    use crate::persistence::get_dedup_table_ptr;
    *get_dedup_table_ptr()
}

/// Set the dedup table.
#[enhanced_orthogonal_persistence]
#[ic_mem_fn]
#[cfg(feature = "ic")]
pub unsafe fn set_dedup_table<M: Memory>(mem: &mut M, dedup_table: Value) {
    use crate::persistence::set_dedup_table_ptr;
    if !dedup_table.is_array() {
        crate::rts_trap_with(
            "set_dedup_table: Invalid dedup table pointer. This is a bug, report to the Motoko team.",
        );
    }
    set_dedup_table_ptr(mem, dedup_table);
}

#[enhanced_orthogonal_persistence]
#[cfg(feature = "ic")]
struct PersistentArray {}
#[enhanced_orthogonal_persistence]
#[cfg(feature = "ic")]
impl PersistentArray {
    // We allow maximum 10K migrations.
    const MIGRATION_FUNCTIONS_ARRAY_SIZE: usize = 10_000;
    pub unsafe fn new<M: Memory>(mem: &mut M) -> Value {
        let array = alloc_array(mem, TAG_ARRAY_M, Self::MIGRATION_FUNCTIONS_ARRAY_SIZE);
        if !array.is_non_null_ptr() {
            crate::rts_trap_with("Failed to allocate migration functions array.");
        }
        // Fill the array with NULL_POINTER.
        let array_obj = array.as_array() as *mut Array;
        for i in 0..array_obj.len() {
            array_obj.set(i, NULL_POINTER, mem);
        }
        array
    }
    pub unsafe fn add_migration_function_hash<M: Memory>(mem: &mut M, array: Value, hash: Value) {
        let array_obj = array.as_array() as *mut Array;
        let mut index = 0;
        // Find first NULL_POINTER index.
        while index < array_obj.len() {
            let crnt_value = array_obj.get(index);
            if !crnt_value.is_non_null_ptr() {
                break;
            }
            index += 1;
        }
        let max_size = array_obj.len();
        if index >= max_size {
            crate::rts_trap_with(
                "Migration functions array is full. This is a bug, report to the Motoko team.",
            );
        }
        array_obj.set(index, hash, mem);
    }
    unsafe fn compare_blobs(blob1: *const Blob, blob2: *const Blob) -> bool {
        let len1 = blob1.len();
        let len2 = blob2.len();
        if len1 != len2 {
            return false;
        }
        for i in 0..len1.as_usize() {
            if blob1.get(i) != blob2.get(i) {
                return false;
            }
        }
        true
    }
    pub unsafe fn is_migration_function_hash_present(array: Value, hash: Value) -> bool {
        let array_obj = array.as_array() as *mut Array;
        let arr_size = array_obj.len();
        let target_hash = hash.as_blob() as *const Blob;
        for i in 0..arr_size {
            let crnt_hash = array_obj.get(i);
            if !crnt_hash.is_non_null_ptr() {
                break;
            }
            let crnt_hash_obj = crnt_hash.as_blob();
            let is_equal = Self::compare_blobs(target_hash, crnt_hash_obj);
            if is_equal {
                return true;
            }
        }
        false
    }
}

/// Register a migration function hash.
#[enhanced_orthogonal_persistence]
#[ic_mem_fn]
#[cfg(feature = "ic")]
pub unsafe fn register_migration<M: Memory>(mem: &mut M, hash: Value) {
    use crate::persistence::{
        get_migration_functions_ptr, is_migration_functions_null, set_migration_functions_ptr,
    };
    if !hash.is_blob() {
        crate::rts_trap_with(
            "register_migration: Invalid hash. This is a bug, report to the Motoko team.",
        );
    }
    let migration_functions = if is_migration_functions_null() {
        let new_migration_functions = PersistentArray::new(mem);
        set_migration_functions_ptr(mem, new_migration_functions);
        if is_migration_functions_null() {
            crate::rts_trap_with(
                "register_migration: Failed to set migration functions pointer. This is a bug, report to the Motoko team.",
            );
        }
        new_migration_functions
    } else {
        *get_migration_functions_ptr()
    };
    if !PersistentArray::is_migration_function_hash_present(migration_functions, hash) {
        PersistentArray::add_migration_function_hash(mem, migration_functions, hash);
    }
}

/// Register a migration function hash.
#[enhanced_orthogonal_persistence]
#[ic_mem_fn]
#[cfg(feature = "ic")]
pub unsafe fn was_migration_performed<M: Memory>(_mem: &mut M, hash: Value) -> bool {
    use crate::persistence::{get_migration_functions_ptr, is_migration_functions_null};
    if !hash.is_blob() {
        crate::rts_trap_with(
            "register_migration: Invalid hash. This is a bug, report to the Motoko team.",
        );
    }

    if is_migration_functions_null() {
        // This can happen if the migration functions array is not initialized.
        // This could be because the canister is being upgraded for the first time.
        return false;
    }
    let migration_functions = *get_migration_functions_ptr();

    PersistentArray::is_migration_function_hash_present(migration_functions, hash)
}
