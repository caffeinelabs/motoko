//! A stack for marking heap objects (for GC). Adopted from mark & compact GC.
//! Simplified to only store object pointers without tags.

use crate::gc::generational::TAG_BLOB_B;
use crate::memory::{Memory, alloc_blob};
use crate::types::{Blob, Words};

use core::ptr::null_mut;

/// Initial stack size
pub const INIT_STACK_SIZE: Words<usize> = Words(64);

/// Pointer to the `blob` object for the mark stack. Used to get the capacity of the stack.
static mut STACK_BLOB_PTR: *mut Blob = null_mut();

/// Bottom of the mark stack
pub static mut STACK_BASE: *mut usize = null_mut();

/// Top of the mark stack
pub static mut STACK_TOP: *mut usize = null_mut();

/// Next free slot in the mark stack
pub static mut STACK_PTR: *mut usize = null_mut();

/// Allocate the mark stack at the start of each GC run
pub unsafe fn alloc_mark_stack<M: Memory>(mem: &mut M) {
    assert!(STACK_BLOB_PTR.is_null());

    // Allocating an actual object here to not break dump_heap
    // No post allocation barrier as this RTS-internal blob will be collected by the GC.
    STACK_BLOB_PTR = alloc_blob(mem, TAG_BLOB_B, INIT_STACK_SIZE.to_bytes()).get_ptr() as *mut Blob;
    STACK_BASE = STACK_BLOB_PTR.payload_addr() as *mut usize;
    STACK_PTR = STACK_BASE;
    STACK_TOP = STACK_BASE.add(INIT_STACK_SIZE.as_usize());
}

/// Deallocate the mark stack after each GC run
pub unsafe fn free_mark_stack() {
    STACK_BLOB_PTR = null_mut();
    STACK_BASE = null_mut();
    STACK_PTR = null_mut();
    STACK_TOP = null_mut();
}

/// Doubles the stack size
pub unsafe fn grow_stack<M: Memory>(mem: &mut M) {
    let stack_cap: Words<usize> = STACK_BLOB_PTR.len().to_words();
    let p = mem.alloc_words(stack_cap).get_ptr() as *mut usize;

    // Make sure nothing was allocated after the stack
    assert_eq!(STACK_TOP, p);

    let new_cap: Words<usize> = stack_cap * 2;
    (*STACK_BLOB_PTR).len = new_cap.to_bytes();
    STACK_TOP = STACK_BASE.add(new_cap.as_usize());
}

/// Push a new unskewed object pointer to be marked later
pub unsafe fn push_mark_stack<M: Memory>(mem: &mut M, object: usize) {
    if STACK_PTR == STACK_TOP {
        grow_stack(mem);
    }
    *STACK_PTR = object;
    STACK_PTR = STACK_PTR.add(1);
}

/// Pop a unskewed object pointer if existend to be visited next
pub unsafe fn pop_mark_stack() -> Option<usize> {
    if STACK_PTR == STACK_BASE {
        return None;
    }
    STACK_PTR = STACK_PTR.sub(1);
    let object = *STACK_PTR;
    return Some(object);
}
