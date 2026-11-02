//! In-heap extendable scan stack to remember the deserialized objects,
//! that still need to be scanned by Cheney's algorithm.
//!
//! This data structure is necessary for the partitioned heap used by the
//! incremental GC where no linear scanning is possible because allocations
//! addresses are not always montonically growing, not even on an empty heap.
//! One example, is a large object that is allocated in a higher partition,
//! while subsequently allocated normal-sized objects are placed in lower
//! partitions.
//!
//! The scan stack cannot grow contiguously as new objects can be allocated
//! during the deserialization. This is why the stack is represented as
//! multiple tables.
//!
//! Doubly linked list of stack tables, each containing a series of entries.
//! A table is represented as a blob with the following internal layout:
//!
//! ┌──────────┬─────────┬──────────┬─────────┬──────────────┬────────┐
//! │ previous │   next  | entry[0] |  ...    | entry[top-1] | (free) |
//! └──────────┴─────────┴──────────┴─────────┴──────────────┴────────┘
//!
//! The list is doubly linked for the following purpose:
//! * `previous` to return to the previous table with preceding entries.
//! * `next` avoid repeated allocations when the stack shrinks and regrows.
//!
//! Whenever a table is full and an entry needs to be pushed on the stack,
//! a new stack table is allocated and linked, unless there already exists
//! a next table. Only the last table can have free entry space.
//!
//! NOTES:
//! * The tables are blobs, as their entries do not be analyzed by the
//! destabilization or the GC.
//! * The stack tables become garbage after a GC run and can be reclaimed.
//!
//! The design is mostly identical to the incremental GC's mark stack
//! TODO: Eliminate code duplication by unfying the stack implementation for
//! both cases.

use core::ptr::null_mut;

use crate::memory::{Memory, alloc_blob};
use crate::types::{Blob, TAG_BLOB_B, Value, size_of};

pub struct ScanStack {
    last: *mut StackTable,
    top: usize, // index of next free entry in the last stack table
}

pub const STACK_TABLE_CAPACITY: usize = 1018;

pub const STACK_EMPTY: Value = Value::from_ptr(0);

#[repr(C)]
struct StackTable {
    pub header: Blob,
    pub previous: *mut StackTable,
    pub next: *mut StackTable,
    pub entries: [Value; STACK_TABLE_CAPACITY],
}

impl ScanStack {
    pub unsafe fn new<M: Memory>(mem: &mut M) -> ScanStack {
        let table = Self::new_table(mem, null_mut());
        ScanStack {
            last: table,
            top: 0,
        }
    }

    pub unsafe fn push<M: Memory>(&mut self, mem: &mut M, value: Value) {
        debug_assert!(value != STACK_EMPTY);
        debug_assert!(self.last != null_mut());
        if self.top == STACK_TABLE_CAPACITY {
            if (*self.last).next == null_mut() {
                self.last = Self::new_table(mem, self.last);
            } else {
                self.last = (*self.last).next;
            }
            self.top = 0;
        }
        debug_assert!(self.top < STACK_TABLE_CAPACITY);
        (*self.last).entries[self.top] = value;
        self.top += 1;
    }

    /// Returns the sentinel `STACK_EMPTY` for an empty stack.
    pub unsafe fn pop(&mut self) -> Value {
        debug_assert!(self.last != null_mut());
        if self.top == 0 {
            if (*self.last).previous == null_mut() {
                return STACK_EMPTY;
            }
            self.last = (*self.last).previous;
            self.top = STACK_TABLE_CAPACITY;
        }
        debug_assert!(self.top > 0);
        self.top -= 1;
        debug_assert!(self.top < STACK_TABLE_CAPACITY);
        (*self.last).entries[self.top]
    }

    pub unsafe fn is_empty(&self) -> bool {
        debug_assert!(self.last != null_mut());
        self.top == 0 && (*self.last).previous == null_mut()
    }

    unsafe fn new_table<M: Memory>(mem: &mut M, previous: *mut StackTable) -> *mut StackTable {
        // No post allocation barrier as this RTS-internal blob will be collected by the GC.
        let table = alloc_blob(mem, TAG_BLOB_B, size_of::<StackTable>().to_bytes()).as_blob_mut()
            as *mut StackTable;
        (*table).previous = previous;
        (*table).next = null_mut();
        if previous != null_mut() {
            (*previous).next = table;
        }
        table
    }
}
