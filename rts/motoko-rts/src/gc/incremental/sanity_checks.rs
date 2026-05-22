//! Incremental GC sanity checker
#![allow(dead_code)]

use crate::gc::incremental::partitioned_heap::PARTITION_SIZE;
use crate::memory::Memory;
use crate::types::*;
use crate::visitor::visit_pointer_fields;

use crate::gc::remembered_set::RememberedSet;

use super::mark_stack::{MarkStack, STACK_EMPTY};
use super::partitioned_heap::PartitionedHeap;
use super::roots::{Roots, visit_roots};

pub unsafe fn check_memory<M: Memory>(
    mem: &mut M,
    heap: &mut PartitionedHeap,
    roots: Roots,
    mode: CheckerMode,
) {
    let mark_stack = MarkStack::new(mem);
    let visited = RememberedSet::new(mem);
    let mut checker = MemoryChecker {
        mode,
        mem,
        heap,
        roots,
        mark_stack,
        visited,
    };
    checker.run();
}

/// Sanity check modes.
pub enum CheckerMode {
    MarkCompletion,   // Check mark phase completion.
    UpdateCompletion, // Check update phase completion.
}

struct MemoryChecker<'a, M: Memory> {
    mode: CheckerMode,
    mem: &'a mut M,
    heap: &'a mut PartitionedHeap,
    roots: Roots,
    mark_stack: MarkStack,
    visited: RememberedSet,
}

impl<'a, M: Memory> MemoryChecker<'a, M> {
    // Check whether all reachable objects and pointers in the memory have a plausible state.
    // Note: The heap may contain garbage objects with stale pointers that are no longer valid.
    // Various check modes:
    // * MarkCompletion:
    ///  Check that the set of marked objects by the incremental GC is the same set or a superset
    ///  of the objects being marked by a conventional stop-the-world mark phase. The incremental
    ///  GC may mark more objects due to concurrent allocations and concurrent pointer modifications.
    // * UpdateCompletion:
    //   Check that the update phase left the heap in a consistent state, with no forwarded objects,
    //   all reachable pointers referring to valid non-moved locations, and all mark bits been cleared.
    unsafe fn run(&mut self) {
        self.check_roots();
        self.check_all_reachable();
    }

    unsafe fn check_roots(&mut self) {
        visit_roots(self.roots, self.heap.base_address(), self, |gc, field| {
            gc.check_object(*field);
        });
    }

    unsafe fn check_object(&mut self, value: Value) {
        self.check_object_header(value);
        assert!(value.get_ptr() >= self.heap.base_address());
        let object = value.as_obj();
        if let CheckerMode::MarkCompletion = self.mode {
            // The incremental GC must have marked this reachable object.
            // Mark object returns false if it has been previously marked.
            assert!(!self.heap.mark_object(object));
        }
        if !self.visited.contains(value) {
            self.visited.insert(self.mem, value);
            self.mark_stack.push(self.mem, value);
        }
    }

    unsafe fn check_all_reachable(&mut self) {
        loop {
            let value = self.mark_stack.pop();
            if value == STACK_EMPTY {
                return;
            }
            self.check_fields(value.get_ptr() as *mut Obj);
        }
    }

    unsafe fn check_fields(&mut self, object: *mut Obj) {
        visit_pointer_fields(
            self,
            object,
            object.tag(),
            0,
            |gc, field_address| {
                let value = *field_address;
                gc.check_object(value);
            },
            |_, _, array| array.len(),
        );
    }

    unsafe fn check_object_header(&self, object: Value) {
        let tag = object.tag();
        assert!(is_object_tag(tag));
        object.check_forwarding_pointer();
        if let CheckerMode::UpdateCompletion = self.mode {
            // Forwarding is no longer allowed on a completed GC.
            assert!(!object.is_forwarded());
        }
        let address = object.get_ptr();
        self.check_valid_address(address);
    }

    unsafe fn check_valid_address(&self, address: usize) {
        let partition_index = address / PARTITION_SIZE;
        let partition = self.heap.get_partition(partition_index);
        if address >= self.heap.base_address() {
            assert!(!partition.is_free());
            assert!(address >= partition.dynamic_space_start());
        }
        let size = block_size(address).to_bytes().as_usize();
        if size > PARTITION_SIZE {
            let number_of_partitions = (size + PARTITION_SIZE - 1) / PARTITION_SIZE;
            for index in partition_index..partition_index + number_of_partitions {
                assert!(self.heap.get_partition(index).has_large_content());
            }
        } else {
            assert!(
                address + block_size(address).to_bytes().as_usize()
                    <= partition.dynamic_space_end()
            );
        }
    }
}
