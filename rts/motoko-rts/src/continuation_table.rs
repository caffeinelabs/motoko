//! This file implements the data structure the Motoko runtime uses to keep track of outstanding
//! continuations.
//!
//! The structure is re-initialized on canister upgrades. This is why it is not part of the
//! persistent metadata, cf. `persistence::PersistentMetadata`.
//!
//! It needs to support the following operations
//!
//!  1. Adding a continuation (any heap pointer) and getting an index (usize)
//!  2. Looking up a continuation by index, which also frees it
//!  3. Peek into an existing continuation and hand back additional data
//!  4. GC must be able to traverse and move continuations in the table
//!
//! By a continuation we understand any collection of callables that help to advance
//! the actor's control flow.
//! The current implementation stores the continuations in heap-allocated Motoko array.
//!
//! To efficiently look up the next free index, we use an implicit free list: `FREE_SLOT` is the
//! index into the array payload of the next free item. Each free item contains the index of the
//! next free item, shifted 2 bits to the left (to make the index a scalar and traverse them in
//! GC).
//!
//! The last item will have scalar value `TABLE.len()`, so after adding a continuation to the last
//! free slot `FREE_SLOT` will be `table_size`, which is when we see that the array is full.
//!
//! When the table is full, we double the size, copy the existing table, and add the second half to
//! the free list. Since all indices are relative to the payload begin, they stay valid. We never
//! shrink the table.

use core::ptr::addr_of_mut;

use crate::barriers::{allocation_barrier, write_with_barrier};
use crate::memory::{Memory, alloc_array};
use crate::rts_trap_with;
use crate::types::{TAG_ARRAY_M, Value};

use motoko_rts_macros::ic_mem_fn;

const INITIAL_SIZE: usize = 256;

// The static variables are re-initialized on canister upgrades and therefore not part of the
// persistent metadata.

// Skewed pointer to the `Array` object. This needs to be a skewed pointer to be able to pass its
// location to the GC.
static mut TABLE: Value = Value::from_scalar(0);

// Number of currently live continuations
static mut N_CONTINUATIONS: usize = 0;

// Next free slot
static mut FREE_SLOT: usize = 0;

unsafe fn create_continuation_table<M: Memory>(mem: &mut M) {
    TABLE = alloc_array(mem, TAG_ARRAY_M, INITIAL_SIZE);
    FREE_SLOT = 0;
    N_CONTINUATIONS = 0;

    let table = TABLE.as_array();
    for i in 0..INITIAL_SIZE {
        table.initialize(i, Value::from_scalar(i + 1), mem);
    }
    allocation_barrier(TABLE);
}

unsafe fn double_continuation_table<M: Memory>(mem: &mut M) {
    let old_array = TABLE.as_array();
    let old_size = old_array.len();

    assert_eq!(FREE_SLOT, old_size);

    let new_size = old_size * 2;

    let new_table = alloc_array(mem, TAG_ARRAY_M, new_size);
    let new_array = new_table.as_array();

    for i in 0..old_size {
        let old_value = old_array.get(i);
        new_array.initialize(i, old_value, mem);
    }

    for i in old_size..new_size {
        new_array.initialize(i, Value::from_scalar(i + 1), mem);
    }
    allocation_barrier(new_table);

    let location = addr_of_mut!(TABLE) as *mut Value;
    write_with_barrier(mem, location, new_table);
}

pub unsafe fn table_initialized() -> bool {
    TABLE.get_raw() != 0
}

#[ic_mem_fn]
pub unsafe fn remember_continuation<M: Memory>(mem: &mut M, ptr: Value) -> usize {
    if !table_initialized() {
        create_continuation_table(mem);
    }

    if FREE_SLOT == TABLE.as_array().len() {
        double_continuation_table(mem);
    }

    // Just as a sanity check make sure the ptr is really skewed
    if ptr.is_scalar() {
        rts_trap_with("remember_continuation: Argument is not a skewed pointer");
    }

    let idx = FREE_SLOT;

    let table = TABLE.as_array();

    FREE_SLOT = table.get(idx).get_scalar();

    table.set(idx, ptr, mem);

    N_CONTINUATIONS += 1;

    idx
}

// Position of the future in explicit self-send ContinuationTable entries
// Invariant: keep this synchronised with compiler.ml (see future_array_index)
const FUTURE_ARRAY_INDEX: usize = 3;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn peek_future_continuation(idx: usize) -> Value {
    if !table_initialized() {
        rts_trap_with("peek_future_continuation: Continuation table not allocated");
    }

    if idx >= TABLE.as_array().len() {
        rts_trap_with("peek_future_continuation: Continuation index out of range");
    }

    let ptr = TABLE.as_array().get(idx);

    if ptr.is_scalar() {
        rts_trap_with("peek_future_continuation: Continuation index not in table");
    }

    ptr.as_array().get(FUTURE_ARRAY_INDEX)
}

#[ic_mem_fn]
pub unsafe fn recall_continuation<M: Memory>(mem: &mut M, idx: usize) -> Value {
    if !table_initialized() {
        rts_trap_with("recall_continuation: Continuation table not allocated");
    }

    if idx >= TABLE.as_array().len() {
        rts_trap_with("recall_continuation: Continuation index out of range");
    }

    let table = TABLE.as_array();

    let ptr = table.get(idx);

    table.set(idx, Value::from_scalar(FREE_SLOT), mem);

    FREE_SLOT = idx;

    N_CONTINUATIONS -= 1;

    if ptr.is_scalar() {
        rts_trap_with("recall_continuation: Continuation index not in table");
    }

    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn continuation_count() -> usize {
    N_CONTINUATIONS
}

#[cfg(feature = "ic")]
pub(crate) unsafe fn continuation_table_loc() -> *mut Value {
    addr_of_mut!(TABLE)
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn continuation_table_size() -> usize {
    if !table_initialized() {
        0
    } else {
        TABLE.as_array().len()
    }
}
