//! Text iterators need to point to a specific position in the tree
//!
//! This is currently a simple triple (array):
//!
//! 1. A pointer to a leaf (must be a BLOB)
//! 2. Position in that blob (shifted by two for GC's sake)
//! 3. 0, or a pointer to a linked list of non-empty text values to do next
//!
//! The linked list is a tuple (array) with
//! 1. A pointer to the text
//! 2. 0, or a pointer to the next list entry

use crate::barriers::allocation_barrier;
use crate::memory::{Memory, alloc_array};
use crate::rts_trap_with;
use crate::text::decode_code_point;
use crate::types::{TAG_ARRAY_T, TAG_BLOB_T, TAG_CONCAT, Value};

use motoko_rts_macros::ic_mem_fn;

const TODO_TEXT_IDX: usize = 0;
const TODO_LINK_IDX: usize = 1;

/// Find the left-most leaf of a text, putting all the others onto a list. Used to enforce the
/// invariant about TEXT_ITER_BLOB to be a blob.
unsafe fn find_leaf<M: Memory>(mem: &mut M, mut text: Value, todo: *mut Value) -> Value {
    while text.tag() == TAG_CONCAT {
        let concat = text.as_concat();

        // Add right node to TODOs
        let new_todo = alloc_array(mem, TAG_ARRAY_T, 2);
        let new_todo_array = new_todo.as_array();
        // No pre-update barrier for object initialization, but do perform post-update barrier.
        new_todo_array.initialize(TODO_TEXT_IDX, (*concat).text2, mem);
        new_todo_array.initialize(TODO_LINK_IDX, *todo, mem);
        allocation_barrier(new_todo);
        *todo = new_todo;

        // Follow left node
        text = (*concat).text1;
    }

    debug_assert_eq!(text.tag(), TAG_BLOB_T);
    text
}

const ITER_BLOB_IDX: usize = 0;
const ITER_POS_IDX: usize = 1;
const ITER_TODO_IDX: usize = 2;

// Use non-pointer sentinel value as `null` to allow simpler visitor logic.
// Anlogous to the design of `continuation_table` and `persistence`.
const NO_OBJECT: Value = Value::from_scalar(0);

/// Returns a new iterator for the text
#[ic_mem_fn]
pub unsafe fn text_iter<M: Memory>(mem: &mut M, text: Value) -> Value {
    let iter = alloc_array(mem, TAG_ARRAY_T, 3);
    let array = iter.as_array();

    // Initialize the TODO field first, to be able to use it use the location to `find_leaf`
    let todo_addr = array.payload_addr().add(ITER_TODO_IDX) as *mut _;
    *todo_addr = NO_OBJECT;

    // Initialize position field
    array.initialize(ITER_POS_IDX, Value::from_scalar(0), mem);

    // Initialize blob field, no pre-update barrier, but post-update barrier.
    array.initialize(
        ITER_BLOB_IDX,
        find_leaf(mem, text, todo_addr as *mut _),
        mem,
    );
    allocation_barrier(iter)
}

/// Returns whether the iterator is finished
#[unsafe(no_mangle)]
pub unsafe extern "C" fn text_iter_done(iter: Value) -> usize {
    let array = iter.as_array();
    let pos = array.get(ITER_POS_IDX).get_scalar();
    let blob = array.get(ITER_BLOB_IDX).as_blob();
    let todo = array.get(ITER_TODO_IDX);

    if pos >= blob.len().as_usize() && todo == NO_OBJECT {
        1
    } else {
        0
    }
}

/// Returns next character in the iterator, advances the iterator
#[ic_mem_fn]
pub unsafe fn text_iter_next<M: Memory>(mem: &mut M, iter: Value) -> u32 {
    let iter_array = iter.as_array();

    let blob = iter_array.get(ITER_BLOB_IDX).as_blob();
    let pos = iter_array.get(ITER_POS_IDX).get_scalar();

    // If we are at the end of the current blob, find the next blob
    if pos >= blob.len().as_usize() {
        let todo = iter_array.get(ITER_TODO_IDX);

        if todo == NO_OBJECT {
            // Caller should check with text_iter_done
            rts_trap_with("text_iter_next: Iter already done");
        }

        let todo_array = todo.as_array();

        let text = todo_array.get(TODO_TEXT_IDX);

        if text.tag() == TAG_CONCAT {
            // If next one is a concat node re-use both the iterator and the todo objects (avoids
            // allocation)
            let concat = text.as_concat();

            todo_array.set(TODO_TEXT_IDX, (*concat).text2, mem);
            iter_array.set(ITER_POS_IDX, Value::from_scalar(0), mem);
            let todo_addr = iter_array.payload_addr().add(ITER_TODO_IDX);

            iter_array.set(
                ITER_BLOB_IDX,
                find_leaf(mem, (*concat).text1, todo_addr),
                mem,
            );

            text_iter_next(mem, iter)
        } else {
            // Otherwise remove the entry from the chain
            debug_assert_eq!(text.tag(), TAG_BLOB_T);

            iter_array.set(ITER_BLOB_IDX, text, mem);
            iter_array.set(ITER_POS_IDX, Value::from_scalar(0), mem);

            iter_array.set(ITER_TODO_IDX, todo_array.get(TODO_LINK_IDX), mem);

            text_iter_next(mem, iter)
        }
    } else {
        // We are not at the end, read the next character from the blob
        let blob_payload = blob.payload_const();
        let mut step = 0;
        let char = decode_code_point(blob_payload.add(pos), &mut step as *mut usize);
        iter_array.set(ITER_POS_IDX, Value::from_scalar(pos + step), mem);
        char
    }
}
