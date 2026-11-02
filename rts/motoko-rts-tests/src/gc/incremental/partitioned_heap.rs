use std::{
    collections::HashSet,
    io::{Write, stdout},
    mem::size_of,
};

use motoko_rts::{
    gc::incremental::{
        IncrementalGC,
        partitioned_heap::{
            PARTITION_SIZE, Partition, PartitionedHeap, PartitionedHeapIterator,
            SURVIVAL_RATE_THRESHOLD,
        },
        set_incremental_gc_state,
        time::BoundedTime,
    },
    memory::{Memory, alloc_array, alloc_blob},
    types::{
        Array, Blob, Bytes, Obj, TAG_ARRAY_I, TAG_ARRAY_M, TAG_ARRAY_S, TAG_ARRAY_T, TAG_BLOB_A,
        TAG_BLOB_B, TAG_BLOB_P, TAG_BLOB_T, Tag, Value, Words,
    },
};

use crate::{gc::utils::WORD_SIZE, memory::TestMemory};

const OBJECT_SIZE: usize = size_of::<Array>() + WORD_SIZE;
const NUMBER_OF_OBJECTS: usize = 2 * PARTITION_SIZE / OBJECT_SIZE;
const HEAP_SIZE: usize = 4 * PARTITION_SIZE;

pub unsafe fn test() {
    println!("  Testing partitioned heap...");
    test_normal_size_scenario();
    test_large_size_scenario();
}

unsafe fn test_normal_size_scenario() {
    let mut heap = create_test_heap();
    let occupied_partitions = 2 + heap.heap_pointer() / PARTITION_SIZE;
    test_allocation_partitions(&heap.inner, occupied_partitions);
    test_iteration(&heap.inner, 1024);
    test_evacuation_plan(&mut heap, occupied_partitions);
    test_freeing_partitions(&mut heap, occupied_partitions);
    test_reallocations(&mut heap);
    test_evacuation_plan(&mut heap, HEAP_SIZE / PARTITION_SIZE + 1);
    test_survival_rate(&mut heap.inner);
    test_freeing_partitions(&mut heap, HEAP_SIZE / PARTITION_SIZE + 1);
    test_close_partition(&mut heap);
    set_incremental_gc_state(None);
}

fn test_allocation_partitions(heap: &PartitionedHeap, number_of_partitions: usize) {
    println!("    Test allocation partitions...");
    assert!(number_of_partitions >= 1);
    let mut exists = false;
    for index in 0..number_of_partitions {
        if heap.is_allocation_partition(index) {
            assert!(!exists);
            exists = true;
        }
    }
    assert!(exists);
}

unsafe fn test_iteration(heap: &PartitionedHeap, break_step_size: usize) {
    println!("    Test heap iteration...");
    let mut iterator = PartitionedHeapIterator::new(heap);
    let mut set = HashSet::new();
    while set.len() < NUMBER_OF_OBJECTS {
        let mut time = BoundedTime::new(break_step_size);
        let count_before = set.len();
        iterate_heap(heap, &mut iterator, &mut set, &mut time);
        assert!(set.len() > count_before);
        assert!(time.is_over() || set.len() == NUMBER_OF_OBJECTS);
    }
    assert_eq!(set.len(), NUMBER_OF_OBJECTS);
    reset_progress();
}

unsafe fn iterate_heap(
    heap: &PartitionedHeap,
    iterator: &mut PartitionedHeapIterator,
    set: &mut HashSet<usize>,
    time: &mut BoundedTime,
) {
    while iterator.has_partition() {
        let partition = iterator.current_partition(heap);
        assert!(!partition.is_free());
        assert!(!partition.to_be_evacuated());
        assert!(!partition.has_large_content());
        assert!(!partition.is_temporary());
        iterate_partition(partition, iterator, set, time);
        if time.is_over() {
            break;
        }
        iterator.next_partition(heap);
    }
}

unsafe fn iterate_partition(
    partition: &Partition,
    iterator: &mut PartitionedHeapIterator,
    set: &mut HashSet<usize>,
    time: &mut BoundedTime,
) {
    while iterator.has_object() {
        let object = iterator.current_object();
        let array = Value::from_ptr(object as usize).as_array();
        let content = array.get(0).get_scalar();
        let inserted = set.insert(content);
        assert!(inserted);
        progress(set.len(), NUMBER_OF_OBJECTS);
        assert_eq!(partition.get_index(), object as usize / PARTITION_SIZE);
        time.tick();
        iterator.next_object();
        if time.is_over() {
            break;
        }
    }
}

unsafe fn test_evacuation_plan(heap: &mut PartitionedTestHeap, occupied_partitions: usize) {
    println!("    Test evacuation plan...");
    unmark_all_objects(heap);
    heap.inner.plan_evacuations(&mut heap.memory);
    let mut iterator = PartitionedHeapIterator::new(&heap.inner);
    while iterator.has_partition() {
        let partition = iterator.current_partition(&heap.inner);
        assert!(partition.get_index() < occupied_partitions);
        assert!(!partition.is_free());
        assert!(
            partition.to_be_evacuated()
                || heap.inner.is_allocation_partition(partition.get_index())
        );
        iterator.next_partition(&heap.inner);
    }
}

unsafe fn test_freeing_partitions(heap: &mut PartitionedTestHeap, occupied_partitions: usize) {
    println!("    Test freeing partitions...");
    heap.inner.complete_collection();
    let mut time = BoundedTime::new(0);
    heap.inner.start_collection(&mut heap.memory, &mut time);
    let mut iterator = PartitionedHeapIterator::new(&heap.inner);
    while iterator.has_partition() {
        let partition = iterator.current_partition(&heap.inner);
        assert!(partition.get_index() < occupied_partitions);
        assert!(!partition.is_free());
        assert!(!partition.to_be_evacuated());
        assert!(heap.inner.is_allocation_partition(partition.get_index()));
        iterator.next_partition(&heap.inner);
    }
    assert!(heap.inner.occupied_size().as_usize() < PARTITION_SIZE + heap.inner.base_address());
    heap.inner.complete_collection();
}

unsafe fn test_reallocations(heap: &mut PartitionedTestHeap) {
    println!("    Test reallocations...");
    let mut time = BoundedTime::new(0);
    heap.inner.start_collection(&mut heap.memory, &mut time);
    let remaining_objects = count_objects(&heap.inner);
    allocate_objects(heap);
    assert!(
        heap.inner.occupied_size().as_usize() >= NUMBER_OF_OBJECTS * OBJECT_SIZE + heap.heap_base()
    );
    let final_objects = count_objects(&heap.inner);
    assert_eq!(final_objects, remaining_objects + NUMBER_OF_OBJECTS);
}

unsafe fn count_objects(heap: &PartitionedHeap) -> usize {
    let mut count = 0;
    let mut iterator = PartitionedHeapIterator::new(heap);
    while iterator.has_partition() {
        let partition = iterator.current_partition(heap);
        assert!(!partition.is_free());
        assert!(!partition.to_be_evacuated());
        count += count_objects_in_partition(partition, &mut iterator);
        iterator.next_partition(heap);
    }
    reset_progress();
    count
}

unsafe fn count_objects_in_partition(
    partition: &Partition,
    iterator: &mut PartitionedHeapIterator,
) -> usize {
    let mut count = 0;
    let mut time = BoundedTime::new(0);
    while iterator.has_object() {
        let object = iterator.current_object();
        assert_eq!(partition.get_index(), object as usize / PARTITION_SIZE);
        let array = Value::from_ptr(object as usize).as_array();
        let content = array.get(0).get_scalar();
        assert!(content < NUMBER_OF_OBJECTS);
        time.tick();
        count += 1;
        if count <= NUMBER_OF_OBJECTS {
            progress(count, NUMBER_OF_OBJECTS);
        }
        iterator.next_object();
    }
    count
}

fn test_close_partition(heap: &mut PartitionedTestHeap) {
    println!("    Test close partition...");
    // Due to Rust borrow check restrictions, preceding `plan_evacuations`
    // allocates directly in `heap.memory` without synchronizing the heap
    // pointer in `heap`. Therefore, re-align the heap pointer of `heap`
    // with `heap.memory`.
    heap.allocate_blob(0);
    test_close_partition_multi_word(heap);
    test_close_partition_single_word(heap);
}

fn test_close_partition_multi_word(heap: &mut PartitionedTestHeap) {
    let old_heap_pointer = heap.heap_pointer();
    let old_partition = old_heap_pointer / PARTITION_SIZE;
    let remainder = PARTITION_SIZE - old_heap_pointer % PARTITION_SIZE;
    let blob = heap.allocate_blob(remainder);
    assert_ne!(heap.heap_pointer() / PARTITION_SIZE, old_partition);
    assert_ne!(blob.get_ptr(), old_heap_pointer);
}

fn test_close_partition_single_word(heap: &mut PartitionedTestHeap) {
    let old_heap_pointer = heap.heap_pointer();
    let old_partition = old_heap_pointer / PARTITION_SIZE;
    let remainder = PARTITION_SIZE - old_heap_pointer % PARTITION_SIZE;
    assert!(remainder > size_of::<Blob>());
    let old_partition_blob = heap.allocate_blob(remainder - size_of::<Blob>() - WORD_SIZE);
    assert_eq!(old_partition_blob.get_ptr(), old_heap_pointer);
    let filler_address = heap.heap_pointer();
    let new_partition_blob = heap.allocate_blob(0);
    assert_ne!(heap.heap_pointer() / PARTITION_SIZE, old_partition);
    assert_ne!(new_partition_blob.get_ptr(), filler_address);
}

unsafe fn test_survival_rate(heap: &mut PartitionedHeap) {
    println!("    Test survival rate...");
    let mut iterator = PartitionedHeapIterator::new(heap);
    while iterator.has_partition() {
        let partition = iterator.current_partition(heap);
        let dynamic_partition_size =
            PARTITION_SIZE - partition.dynamic_space_start() % PARTITION_SIZE;
        let expected_survival_rate =
            occupied_space(partition) as f64 / dynamic_partition_size as f64;
        assert!(f64::abs(partition.survival_rate() - expected_survival_rate) < 1e6);
        let expected_evacuation = !heap.is_allocation_partition(partition.get_index())
            && partition.survival_rate() <= SURVIVAL_RATE_THRESHOLD;
        assert_eq!(partition.to_be_evacuated(), expected_evacuation);
        iterator.next_partition(heap);
    }
}

unsafe fn test_large_size_scenario() {
    println!("    Test large allocations...");
    const LARGE: usize = PARTITION_SIZE + WORD_SIZE;
    const EXTRA_LARGE: usize = 2 * PARTITION_SIZE;
    test_allocation_sizes(&[8 * WORD_SIZE, PARTITION_SIZE, 4 * WORD_SIZE], 3);
    test_allocation_sizes(&[7 * WORD_SIZE, LARGE, 5 * WORD_SIZE], 3);
    test_allocation_sizes(&[6 * WORD_SIZE, LARGE, LARGE, 9 * WORD_SIZE], 5);
    test_allocation_sizes(&[6 * WORD_SIZE, EXTRA_LARGE, 4 * WORD_SIZE], 3);
    test_allocation_sizes(&[6 * WORD_SIZE, EXTRA_LARGE, LARGE, 4 * WORD_SIZE], 6);
    test_allocation_sizes(
        &[
            6 * WORD_SIZE,
            EXTRA_LARGE,
            8 * WORD_SIZE,
            LARGE,
            4 * WORD_SIZE,
        ],
        6,
    );
}

unsafe fn test_allocation_sizes(sizes: &[usize], number_of_partitions: usize) {
    let total_partitions = number_of_partitions + 1; // Plus temporary partition.
    let mut heap = PartitionedTestHeap::new(total_partitions * PARTITION_SIZE);
    let heap_base = heap.heap_base();
    let state = IncrementalGC::<PartitionedTestHeap>::initial_gc_state(heap_base);
    set_incremental_gc_state(Some(state));
    assert!(heap.inner.occupied_size().as_usize() < PARTITION_SIZE + heap.heap_base());
    let mut time = BoundedTime::new(0);
    heap.inner.start_collection(&mut heap.memory, &mut time);
    for size in sizes.iter() {
        assert_eq!(*size % WORD_SIZE, 0);
        assert!(*size >= size_of::<Blob>());
        let blob = heap.allocate_blob(*size - size_of::<Blob>());
        let object = blob.get_ptr() as *mut Obj;
        let unmarked_before = heap.inner.mark_object(object);
        assert!(unmarked_before);
    }
    assert!(
        heap.inner.occupied_size().as_usize() >= sizes.iter().sum::<usize>() + heap.heap_base()
    );
    iterate_large_objects(&heap.inner, sizes);
    unmark_all_objects(&mut heap);
    heap.inner.plan_evacuations(&mut heap.memory);
    heap.inner.collect_large_objects();
    heap.inner.complete_collection();
    heap.inner.start_collection(&mut heap.memory, &mut time);
    iterate_large_objects(&heap.inner, &[]);
    assert!(heap.inner.occupied_size().as_usize() < PARTITION_SIZE + heap.heap_base());
    set_incremental_gc_state(None);
}

unsafe fn unmark_all_objects(heap: &mut PartitionedTestHeap) {
    heap.inner.complete_collection();
    let mut time = BoundedTime::new(0);
    heap.inner.start_collection(&mut heap.memory, &mut time);
}

unsafe fn iterate_large_objects(heap: &PartitionedHeap, expected_sizes: &[usize]) {
    let mut detected_sizes = vec![];
    let mut iterator = PartitionedHeapIterator::new(heap);
    while iterator.has_partition() {
        iterate_large_partition(&mut iterator, &mut detected_sizes);
        iterator.next_partition(heap);
    }
    detected_sizes.sort();
    let mut expected_sorted = expected_sizes.to_vec();
    expected_sorted.sort();
    assert_eq!(detected_sizes, expected_sorted);
}

unsafe fn iterate_large_partition(
    iterator: &mut PartitionedHeapIterator,
    detected_sizes: &mut Vec<usize>,
) {
    let mut time = BoundedTime::new(0);
    while iterator.has_object() {
        let object = iterator.current_object();
        assert_eq!(object.tag(), TAG_BLOB_B);
        let size = block_size(object as *const Tag);
        detected_sizes.push(size);
        time.tick();
        iterator.next_object();
    }
}

unsafe fn occupied_space(partition: &Partition) -> usize {
    let mut sweep_line = partition.dynamic_space_start();
    let mut occupied_space = 0;
    while sweep_line < partition.dynamic_space_end() {
        let block = sweep_line as *const Tag;
        let size = block_size(block);
        occupied_space += size;
        sweep_line += size;
        assert!(sweep_line <= partition.dynamic_space_end());
    }
    occupied_space
}

unsafe fn create_test_heap() -> PartitionedTestHeap {
    println!("    Create test heap...");
    let mut heap = PartitionedTestHeap::new(HEAP_SIZE);
    let heap_base = heap.heap_base();
    let state = IncrementalGC::<PartitionedTestHeap>::initial_gc_state(heap_base);
    set_incremental_gc_state(Some(state));
    let mut time = BoundedTime::new(0);
    unsafe {
        heap.inner.start_collection(&mut heap.memory, &mut time);
    }
    allocate_objects(&mut heap);
    let heap_size = heap.inner.occupied_size().as_usize();
    assert_eq!(
        heap_size,
        heap.heap_base() + NUMBER_OF_OBJECTS * OBJECT_SIZE
    );
    heap
}

fn allocate_objects(heap: &mut PartitionedTestHeap) {
    for index in 0..NUMBER_OF_OBJECTS {
        progress(index + 1, NUMBER_OF_OBJECTS);
        let value = Value::from_scalar(index);
        let array = heap.allocate_array(&[value]);
        unsafe {
            let object = array.get_ptr() as *mut Obj;
            let unmarked_before = heap.inner.mark_object(object);
            assert!(unmarked_before);
            assert!(!heap.inner.mark_object(object));
        }
    }
    reset_progress();
}

fn progress(count: usize, max: usize) {
    if count % (max / 100) == 0 || count == max {
        let percentage = count * 100 / max;
        print!("{percentage}/100\r");
        Write::flush(&mut stdout()).unwrap();
    }
}

fn reset_progress() {
    print!("       \r");
    Write::flush(&mut stdout()).unwrap();
}

pub struct PartitionedTestHeap {
    memory: TestMemory,
    inner: PartitionedHeap,
}

impl PartitionedTestHeap {
    pub fn new(size: usize) -> PartitionedTestHeap {
        let memory = TestMemory::new(Bytes(size).to_words());
        let heap_base = memory.heap_base();
        let inner = PartitionedHeap::new(heap_base);
        assert_eq!(inner.base_address(), heap_base);
        PartitionedTestHeap { memory, inner }
    }

    pub fn heap_base(&self) -> usize {
        self.memory.heap_base()
    }

    pub fn heap_pointer(&self) -> usize {
        self.memory.heap_pointer()
    }

    pub fn allocate_array(&mut self, elements: &[Value]) -> Value {
        unsafe {
            let array = alloc_array(self, TAG_ARRAY_M, elements.len());
            for index in 0..elements.len() {
                let raw_array = array.as_array();
                raw_array.set(index, elements[index], self);
            }
            array
        }
    }

    pub fn allocate_blob(&mut self, size: usize) -> Value {
        unsafe { alloc_blob(self, TAG_BLOB_B, Bytes(size)) }
    }
}

unsafe fn block_size(block: *const Tag) -> usize {
    match *block {
        TAG_ARRAY_I | TAG_ARRAY_M | TAG_ARRAY_T | TAG_ARRAY_S => {
            size_of::<Array>() + (block as *const Array).len() * WORD_SIZE
        }
        TAG_BLOB_B | TAG_BLOB_T | TAG_BLOB_P | TAG_BLOB_A => {
            size_of::<Blob>() + (block as *const Blob).len().as_usize()
        }
        _ => unimplemented!(),
    }
}

impl Memory for PartitionedTestHeap {
    unsafe fn alloc_words(&mut self, size: Words<usize>) -> Value {
        let result = self.inner.allocate(&mut self.memory, size);
        self.memory
            .set_heap_pointer(result.get_ptr() + size.to_bytes().as_usize());
        result
    }

    unsafe fn grow_memory(&mut self, ptr: usize) {
        assert!(ptr <= self.memory.heap_end());
    }
}
