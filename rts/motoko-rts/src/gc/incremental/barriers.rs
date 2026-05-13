//! Barriers for the incremental GC used by the compiler:
//! * Write barrier
//! * Allocation barrier

use motoko_rts_macros::ic_mem_fn;

use crate::{
    memory::Memory,
    types::{Value, is_skewed},
};

use super::{
    Phase, count_allocation, get_incremental_gc_state, post_allocation_barrier, pre_write_barrier,
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn running_gc() -> bool {
    get_incremental_gc_state().phase != Phase::Pause
}

/// Write a potential pointer value with a pre-update barrier and resolving pointer forwarding.
/// Used for the incremental GC.
/// `location` (unskewed) denotes the field or array element where the value will be written to.
/// `value` (skewed if a pointer) denotes the value that will be written to the location.
/// The barrier can be conservatively called even if the stored value might not be a pointer.
/// Additional write effects:
/// * Pre-update barrier: Used during the GC mark phase to guarantee incremental snapshot-at-the-beginning marking.
/// * Resolve forwarding: Used during the GC update phase to adjust old pointers to their new forwarded addresses.
#[ic_mem_fn]
pub unsafe fn write_with_barrier<M: Memory>(mem: &mut M, location: *mut Value, value: Value) {
    debug_assert!(!is_skewed(location as usize));
    debug_assert_ne!(location, core::ptr::null_mut());

    let state = get_incremental_gc_state();
    pre_write_barrier(mem, state, *location);
    *location = value.forward_if_possible();
}

/// Allocation barrier to be called after a new object allocation.
/// The new object needs to be fully initialized, except for the payload of a blob.
/// Used for the incremental GC.
/// `new_object` is the skewed pointer of the newly allocated and initialized object.
/// Return:
/// `new_object` for convenience and better performance.
/// Effects:
/// * Mark new allocations during the GC mark and evacuation phases.
/// * Resolve pointer forwarding during the GC update phase.
/// * Keep track of concurrent allocations to adjust the GC increment time limit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocation_barrier(new_object: Value) -> Value {
    let state = get_incremental_gc_state();
    if state.phase != Phase::Pause {
        post_allocation_barrier(state, new_object);
        count_allocation(state);
    }
    new_object
}
