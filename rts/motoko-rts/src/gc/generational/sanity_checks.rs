//! Extensive sanity checks for generational GC features.
//! * Write barrier coverage by memory snapshot comparisons.
//! * Memory sanity check, including a full heap scan.
#![allow(dead_code)]

use core::ptr::null_mut;

use super::write_barrier::REMEMBERED_SET;
use super::{Heap, Limits, Roots};
use crate::mem_utils::memcpy_bytes;
use crate::memory::{Memory, alloc_blob};
use crate::types::*;
use crate::visitor::{classical::pointer_to_dynamic_heap, visit_pointer_fields};

static mut SNAPSHOT: *mut Blob = null_mut();

/// Take a memory snapshot. To be initiated after GC run.
pub unsafe fn take_snapshot<M: Memory>(heap: &mut Heap<M>) {
    let length = Bytes(heap.limits.free);
    // No post allocation barrier as this RTS-internal blob will be collected by the GC.
    let blob = alloc_blob(heap.mem, TAG_BLOB_B, length).get_ptr() as *mut Blob;
    memcpy_bytes(blob.payload_addr() as usize, 0, length);
    SNAPSHOT = blob;
}

/// Verify write barrier coverage by comparing the memory against the previous snapshot.
/// To be initiated before the next GC run. No effect if no snapshpot has been taken.
pub unsafe fn verify_snapshot<M: Memory>(heap: &Heap<M>, verify_roots: bool) {
    if SNAPSHOT.is_null() {
        return;
    }
    assert!(heap.limits.base <= heap.limits.free);
    if verify_roots {
        verify_static_roots(heap.roots.static_roots.as_array(), heap.limits.free);
    }
    verify_heap(&heap.limits);
    SNAPSHOT = null_mut();
}

unsafe fn verify_static_roots(static_roots: *mut Array, last_free: usize) {
    for index in 0..static_roots.len() {
        let current = static_roots.get(index).as_obj();
        assert_eq!(current.tag(), TAG_MUTBOX); // check tag
        let mutbox = current as *mut MutBox;
        let current_field = &mut (*mutbox).field;
        if relevant_field(current_field, last_free) {
            verify_field(current_field);
        }
    }
}

unsafe fn verify_heap(limits: &Limits) {
    assert!(SNAPSHOT.len().as_usize() <= limits.free);
    let mut pointer = limits.base;
    while pointer < SNAPSHOT.len().as_usize() {
        if Value::from_ptr(pointer).is_obj() {
            let current = pointer as *mut Obj;
            let previous = (SNAPSHOT.payload_addr() as usize + pointer) as *mut Obj;
            assert!(current.tag() == previous.tag());
            visit_pointer_fields(
                &mut (),
                current,
                current.tag(),
                0,
                |_, current_field| {
                    if relevant_field(current_field, limits.last_free) {
                        verify_field(current_field);
                    }
                },
                |_, slice_start, arr| {
                    assert!(slice_start == 0);
                    arr.len()
                },
            );
        }
        pointer += block_size(pointer).to_bytes().as_usize();
    }
}

unsafe fn relevant_field(current_field: *mut Value, last_free: usize) -> bool {
    if (current_field as usize) < last_free {
        let value = *current_field;
        value.is_ptr() && value.get_ptr() as usize >= last_free
    } else {
        false
    }
}

unsafe fn verify_field(current_field: *mut Value) {
    let memory_copy = SNAPSHOT.payload_addr() as usize;
    let previous_field = (memory_copy + current_field as usize) as *mut Value;
    if *previous_field != *current_field && !recorded(current_field as usize) {
        panic!("Missing write barrier at {:#x}", current_field as usize);
    }
}

unsafe fn recorded(value: usize) -> bool {
    match REMEMBERED_SET.as_ref() {
        None => panic!("No remembered set"),
        Some(remembered_set) => remembered_set.contains(Value::from_raw(value)),
    }
}

pub struct MemoryChecker<'a> {
    limits: &'a Limits,
    roots: &'a Roots,
}

pub unsafe fn check_memory(limits: &Limits, roots: &Roots) {
    let checker = MemoryChecker { limits, roots };
    checker.check_memory();
}

impl<'a> MemoryChecker<'a> {
    unsafe fn check_memory(&self) {
        self.check_static_roots();
        if (*self.roots.continuation_table_ptr_loc).is_ptr() {
            self.check_object(*self.roots.continuation_table_ptr_loc);
        }
        self.check_heap();
    }

    unsafe fn check_static_roots(&self) {
        let root_array = self.roots.static_roots.as_array();
        for i in 0..root_array.len() {
            let obj = root_array.get(i).as_obj();
            assert_eq!(obj.tag(), TAG_MUTBOX);
            assert!((obj as usize) < self.limits.base);
            let mutbox = obj as *mut MutBox;
            let field_addr = &mut (*mutbox).field;
            if pointer_to_dynamic_heap(field_addr, self.limits.base as usize) {
                let object = *field_addr;
                self.check_object(object);
            }
        }
    }

    unsafe fn check_object(&self, object: Value) {
        self.check_object_header(object);
        visit_pointer_fields(
            &mut (),
            object.as_obj(),
            object.tag(),
            0,
            |_, field_address| {
                // Ignore null pointers used in text_iter.
                if (*field_address).get_ptr() as *mut Obj != null_mut() {
                    (&self).check_object_header(*field_address);
                }
            },
            |_, _, arr| arr.len(),
        );
    }

    unsafe fn check_object_header(&self, object: Value) {
        assert!(object.is_ptr());
        let pointer = object.get_ptr();
        assert!(pointer < self.limits.free);
        let tag = object.tag();
        assert!(tag >= TAG_OBJECT && tag <= TAG_NULL);
    }

    unsafe fn check_heap(&self) {
        let mut pointer = self.limits.base;
        while pointer < self.limits.free {
            let block = Value::from_ptr(pointer as usize);
            if block.tag() != TAG_ONE_WORD_FILLER {
                self.check_object(block);
            }
            pointer += block_size(pointer as usize).to_bytes().as_usize();
        }
    }
}
