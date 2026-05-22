//! A stack for marking heap objects (for GC). There should be no allocation after the stack
//! otherwise things will break as we push. This invariant is checked in debug builds.

use crate::gc::mark_compact::TAG_BLOB_B;
use crate::memory::{Memory, alloc_blob};
use crate::types::{Blob, Tag, Words};

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

pub unsafe fn alloc_mark_stack<M: Memory>(mem: &mut M) {
    debug_assert!(STACK_BLOB_PTR.is_null());

    // Allocating an actual object here to not break dump_heap
    // No post allocation barrier as this RTS-internal blob will be collected by the GC.
    STACK_BLOB_PTR = alloc_blob(mem, TAG_BLOB_B, INIT_STACK_SIZE.to_bytes()).get_ptr() as *mut Blob;
    STACK_BASE = STACK_BLOB_PTR.payload_addr() as *mut usize;
    STACK_PTR = STACK_BASE;
    STACK_TOP = STACK_BASE.add(INIT_STACK_SIZE.as_usize());
}

pub unsafe fn free_mark_stack() {
    STACK_BLOB_PTR = null_mut();
    STACK_BASE = null_mut();
    STACK_PTR = null_mut();
    STACK_TOP = null_mut();
}

/// Doubles the stack size
pub unsafe fn grow_stack<M: Memory>(mem: &mut M) {
    let stack_cap = STACK_BLOB_PTR.len().to_words();
    let p = mem.alloc_words(stack_cap).get_ptr() as *mut usize;

    // Make sure nothing was allocated after the stack
    debug_assert_eq!(STACK_TOP, p);

    let new_cap = stack_cap * 2;
    (*STACK_BLOB_PTR).len = new_cap.to_bytes();
    STACK_TOP = STACK_BASE.add(new_cap.as_usize());
}

pub unsafe fn push_mark_stack<M: Memory>(mem: &mut M, obj: usize, obj_tag: Tag) {
    // We add 2 words in a push, and `STACK_PTR` and `STACK_TOP` are both multiples of 2, so we can
    // do simple equality check here
    if STACK_PTR == STACK_TOP {
        grow_stack(mem);
    }

    *STACK_PTR = obj;
    *STACK_PTR.add(1) = obj_tag;
    STACK_PTR = STACK_PTR.add(2);
}

pub unsafe fn pop_mark_stack() -> Option<(usize, Tag)> {
    if STACK_PTR == STACK_BASE {
        return None;
    }
    STACK_PTR = STACK_PTR.sub(2);
    let p = *STACK_PTR;
    let tag = *STACK_PTR.add(1);
    return Some((p, tag));
}
