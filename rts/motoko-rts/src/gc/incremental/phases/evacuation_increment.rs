use crate::{
    gc::incremental::{
        State,
        partitioned_heap::{PartitionedHeap, PartitionedHeapIterator},
        time::BoundedTime,
    },
    mem_utils::memcpy_words,
    memory::Memory,
    stable_option::StableOption,
    types::*,
};

pub struct EvacuationIncrement<'a, M: Memory> {
    mem: &'a mut M,
    heap: &'a mut PartitionedHeap,
    iterator: &'a mut PartitionedHeapIterator,
    time: &'a mut BoundedTime,
}

impl<'a, M: Memory + 'a> EvacuationIncrement<'a, M> {
    pub unsafe fn start_phase(mem: &mut M, state: &mut State) {
        debug_assert!(state.iterator_state.is_none());
        let heap = &mut state.partitioned_heap;
        state.iterator_state = StableOption::Some(PartitionedHeapIterator::new(heap));
        heap.plan_evacuations(mem);
    }

    pub unsafe fn complete_phase(state: &mut State) {
        debug_assert!(Self::evacuation_completed(state));
        state.iterator_state = StableOption::None;
    }

    pub unsafe fn evacuation_completed(state: &State) -> bool {
        !state.iterator_state.as_ref().unwrap().has_partition()
    }

    pub unsafe fn instance(
        mem: &'a mut M,
        state: &'a mut State,
        time: &'a mut BoundedTime,
    ) -> EvacuationIncrement<'a, M> {
        let heap = &mut state.partitioned_heap;
        let iterator = state.iterator_state.as_mut().unwrap();
        EvacuationIncrement {
            mem,
            heap,
            iterator,
            time,
        }
    }

    pub unsafe fn run(&mut self) {
        while self.iterator.has_partition() {
            let partition = self.iterator.current_partition(&self.heap);
            if partition.to_be_evacuated() {
                self.evacuate_partition(partition.get_index());
                if self.time.is_over() {
                    // Resume evacuation of the same partition later.
                    break;
                }
            }
            self.iterator.next_partition(&self.heap);
        }
    }

    pub unsafe fn evacuate_partition(&mut self, partition_index: usize) {
        let partition = self.heap.get_partition(partition_index);
        debug_assert!(!partition.is_free());
        debug_assert!(!partition.has_large_content());
        while self.iterator.has_object() && !self.time.is_over() {
            let object = self.iterator.current_object();
            // Advance the iterator before evacuation since the debug mode clears the evacuating object.
            self.iterator.next_object();
            self.evacuate_object(object);
        }
    }

    unsafe fn evacuate_object(&mut self, original: *mut Obj) {
        debug_assert!(is_object_tag(original.tag()));
        debug_assert!(!original.is_forwarded());
        let size = block_size(original as usize);
        let new_address = self.mem.alloc_words(size);
        self.heap.increase_evacuated_size(size);
        let copy = new_address.get_ptr() as *mut Obj;
        memcpy_words(copy as usize, original as usize, size);
        (*copy).forward = new_address;
        (*original).forward = new_address;
        debug_assert!(!copy.is_forwarded());
        debug_assert!(original.is_forwarded());
        // Marking is necessary to ensure field updates in the copy.
        let unmarked_before = self.heap.mark_object(copy);
        debug_assert!(unmarked_before);
        // Determined by measurements in comparison to the mark and update phases.
        const TIME_FRACTION_PER_WORD: f64 = 2.7;
        self.time
            .advance(1 + (size.as_usize() as f64 / TIME_FRACTION_PER_WORD) as usize);

        #[cfg(feature = "memory_check")]
        Self::clear_object_content(original);
    }

    #[cfg(feature = "memory_check")]
    unsafe fn clear_object_content(original: *mut Obj) {
        let object_size = block_size(original as usize);
        let header_size = size_of::<Obj>();
        let payload_address = original as usize + header_size.to_bytes().as_usize();
        let payload_size = object_size - header_size;
        crate::mem_utils::memzero(payload_address, payload_size);
        // Do not clear object tag and forwarding pointer
    }
}
