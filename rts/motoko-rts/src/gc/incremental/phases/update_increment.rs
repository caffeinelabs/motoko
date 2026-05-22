use motoko_rts_macros::enhanced_orthogonal_persistence;

use crate::{
    gc::incremental::{
        Roots, State,
        array_slicing::slice_array,
        partitioned_heap::{Partition, PartitionedHeap, PartitionedHeapIterator},
        roots::visit_roots,
        time::BoundedTime,
    },
    stable_option::StableOption,
    types::*,
    visitor::visit_pointer_fields,
};

pub struct UpdateIncrement<'a> {
    time: &'a mut BoundedTime,
    heap: &'a PartitionedHeap,
    iterator: &'a mut PartitionedHeapIterator,
    updates_needed: bool,
}

impl<'a> UpdateIncrement<'a> {
    pub unsafe fn start_phase(state: &mut State) {
        debug_assert!(state.iterator_state.is_none());
        let heap = &mut state.partitioned_heap;
        state.iterator_state = StableOption::Some(PartitionedHeapIterator::new(heap));
        heap.collect_large_objects();
        heap.plan_updates();
    }

    pub unsafe fn complete_phase(state: &mut State) {
        debug_assert!(Self::update_completed(state));
        state.iterator_state = StableOption::None;
        state.partitioned_heap.complete_collection();
    }

    pub unsafe fn update_completed(state: &State) -> bool {
        !state.iterator_state.as_ref().unwrap().has_partition()
    }

    pub unsafe fn instance(state: &'a mut State, time: &'a mut BoundedTime) -> UpdateIncrement<'a> {
        let heap = &state.partitioned_heap;
        let state = state.iterator_state.as_mut().unwrap();
        let updates_needed = heap.updates_needed();
        UpdateIncrement {
            time,
            heap,
            iterator: state,
            updates_needed,
        }
    }

    pub unsafe fn update_roots(&mut self, roots: Roots) {
        visit_roots(roots, self.heap.base_address(), self, |gc, field| {
            let value = *field;

            #[enhanced_orthogonal_persistence]
            debug_assert_ne!(value, NULL_POINTER);

            if value.is_forwarded() {
                *field = value.forward_if_possible();
            }
            gc.time.tick();
        });
    }

    pub unsafe fn run(&mut self) {
        while self.iterator.has_partition() {
            let partition = self.iterator.current_partition(&self.heap);
            if partition.to_be_updated() {
                self.update_partition(partition);
                if self.time.is_over() {
                    // Resume updating the same partition later.
                    break;
                }
            }
            self.iterator.next_partition(&self.heap);
        }
    }

    pub unsafe fn update_partition(&mut self, partition: &Partition) {
        debug_assert!(!partition.is_free());
        debug_assert!(!partition.to_be_evacuated());
        while self.iterator.has_object() && !self.time.is_over() {
            let object = self.iterator.current_object();
            self.update_object(object);
            if self.time.is_over() && object.tag() >= TAG_ARRAY_SLICE_MIN {
                // Resume updating the same object later.
                break;
            }
            self.iterator.next_object();
        }
    }

    unsafe fn update_object(&mut self, object: *mut Obj) {
        debug_assert!(!object.is_forwarded());
        if self.updates_needed {
            self.update_fields(object);
        }
        if self.time.is_over() {
            // Keep mark bit and later resume updating more slices of this array.
            return;
        }
        debug_assert!(object.tag() < TAG_ARRAY_SLICE_MIN);
        self.time.tick();
    }

    unsafe fn update_fields(&mut self, object: *mut Obj) {
        loop {
            // Loop over potentially multiple array slices and return if the GC increment limit is exceeded.
            visit_pointer_fields(
                self,
                object,
                object.tag(),
                self.heap.base_address(),
                |gc, field_address| {
                    *field_address = (*field_address).forward_if_possible();
                    gc.time.tick();
                },
                |gc, slice_start, array| {
                    let length = slice_array(array);
                    gc.time.advance(1 + length - slice_start);
                    length
                },
            );

            if object.tag() < TAG_ARRAY_SLICE_MIN || self.time.is_over() {
                return;
            }
        }
    }
}
