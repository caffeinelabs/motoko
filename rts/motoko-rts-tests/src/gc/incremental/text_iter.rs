//! A text iterator advanced while the incremental GC is marking.
//!
//! `text_iter_next` prepends to the iterator's todo list when it descends into a
//! concatenation. The list cell it displaces must go through the write barrier:
//! the new head is allocated during marking, so it is marked but never scanned,
//! and the displaced cell is then reachable only through it.

use motoko_rts::gc::incremental::partitioned_heap::PARTITION_SIZE;
use motoko_rts::gc::incremental::{IncrementalGC, Phase, get_incremental_gc_state};
use motoko_rts::memory::{alloc_array, alloc_blob};
use motoko_rts::text::{text_concat, text_of_str};
use motoko_rts::text_iter::{text_iter, text_iter_done, text_iter_next};
use motoko_rts::types::{Bytes, TAG_ARRAY_M, TAG_BLOB_B, Value};

use crate::gc::heap::MotokoHeap;
use crate::gc::utils::GC;
use crate::gc::{initialize_gc, reset_gc};

// Mirrors `text_iter.rs`.
const ITER_TODO_IDX: usize = 2;
const TODO_TEXT_IDX: usize = 0;
const TODO_LINK_IDX: usize = 1;

// One GC increment is bounded by 5M steps and marking an array costs about two steps
// per element, so an array of this length is still being marked after the first increment.
const BIG_ARRAY_LENGTH: usize = 4_000_000;
// Garbage that, together with the big array, cannot share a partition with the iterator.
// The iterator's partition is then not the allocation partition and gets evacuated.
const FILLER_SIZE: usize = 40 * 1024 * 1024;

unsafe fn gc_increment(heap: &mut MotokoHeap) {
    let unused_root = &mut Value::from_scalar(0) as *mut Value;
    let roots = [
        heap.static_root_array_variable_address() as *mut Value,
        heap.continuation_table_variable_address() as *mut Value,
        heap.region0_pointer_variable_address() as *mut Value,
        unused_root,
        unused_root,
        unused_root,
    ];
    IncrementalGC::instance(heap, get_incremental_gc_state()).empty_call_stack_increment(roots);
}

unsafe fn next_char(heap: &mut MotokoHeap, iter: Value) -> char {
    assert_eq!(text_iter_done(iter), 0);
    char::from_u32(text_iter_next(heap, iter)).unwrap()
}

fn partition_of(value: Value) -> usize {
    value.get_ptr() / PARTITION_SIZE
}

pub unsafe fn test() {
    println!("  Testing text iterator during marking...");

    let mut heap = MotokoHeap::new(&[], &[], &[], GC::Incremental, 6 * PARTITION_SIZE);
    initialize_gc(&mut heap);

    // x # ((a # b) # c): after `x`, the pending todo entry is a concatenation
    // whose left child is again a concatenation.
    let x = text_of_str(&mut heap, "xxxxxxxx");
    let a = text_of_str(&mut heap, "aaaaaaaa");
    let b = text_of_str(&mut heap, "bbbbbbbb");
    let c = text_of_str(&mut heap, "cccccccc");
    let ab = text_concat(&mut heap, a, b);
    let abc = text_concat(&mut heap, ab, c);
    let text = text_concat(&mut heap, x, abc);
    let iter = text_iter(&mut heap, text);

    let element = alloc_blob(&mut heap, TAG_BLOB_B, Bytes(0));
    let root = alloc_array(&mut heap, TAG_ARRAY_M, 2);
    let _filler = alloc_blob(&mut heap, TAG_BLOB_B, Bytes(FILLER_SIZE));
    let big = alloc_array(&mut heap, TAG_ARRAY_M, BIG_ARRAY_LENGTH);
    for index in 0..BIG_ARRAY_LENGTH {
        big.as_array().initialize(index, element, &mut heap);
    }
    // The mark stack is LIFO: the big array is scanned before the iterator.
    root.as_array().initialize(0, iter, &mut heap);
    root.as_array().initialize(1, big, &mut heap);
    let root_location = heap.static_root_array_variable_address() as *mut Value;
    *root_location = root;

    // Start marking. The increment ends inside the big array; the iterator is
    // marked but its fields have not been visited.
    gc_increment(&mut heap);
    assert!(get_incremental_gc_state().phase() == Phase::Mark);

    // Consume `x` and step into `(a # b) # c`. This pushes a new todo cell in front
    // of the existing one.
    let mut seen = String::new();
    for _ in 0..9 {
        seen.push(next_char(&mut heap, iter));
    }
    assert_eq!(seen, "xxxxxxxxa");
    let old_partition = partition_of(iter);

    // Finish the GC run.
    for _ in 0..16 {
        if get_incremental_gc_state().phase() == Phase::Pause {
            break;
        }
        gc_increment(&mut heap);
    }
    assert!(get_incremental_gc_state().phase() == Phase::Pause);

    // The iterator has been evacuated, and so must everything reachable from it.
    let iter = (*root_location).as_array().get(0);
    assert_ne!(
        partition_of(iter),
        old_partition,
        "test setup: the iterator's partition was not evacuated"
    );
    let mut todo = iter.as_array().get(ITER_TODO_IDX);
    let mut cells = 0;
    while !todo.is_scalar() {
        assert_ne!(
            partition_of(todo),
            old_partition,
            "todo cell {cells} was left behind in the evacuated partition"
        );
        let pending = todo.as_array().get(TODO_TEXT_IDX);
        assert_ne!(partition_of(pending), old_partition);
        todo = todo.as_array().get(TODO_LINK_IDX);
        cells += 1;
    }
    assert_eq!(cells, 2);

    while text_iter_done(iter) == 0 {
        seen.push(next_char(&mut heap, iter));
    }
    assert_eq!(seen, "xxxxxxxxaaaaaaaabbbbbbbbcccccccc");

    reset_gc();
}
