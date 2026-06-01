// Naming conventions:
//
// - offset = index in the "heap" array/slice/vector
// - address = address in the process's address space
//
// To convert an offset into an address, add heap array's address to the offset.

use motoko_rts_macros::{
    classical_persistence, enhanced_orthogonal_persistence, incremental_gc, non_incremental_gc,
};

#[classical_persistence]
mod classical;
#[enhanced_orthogonal_persistence]
mod enhanced;

#[non_incremental_gc]
mod compacting;
#[non_incremental_gc]
mod generational;
#[incremental_gc]
mod incremental;

pub mod heap;
pub mod random;
pub mod utils;

use self::utils::{GC, GC_IMPLS};
use fxhash::{FxHashMap, FxHashSet};
use heap::MotokoHeap;
use utils::ObjectIdx;

pub const SEEDS_PER_CHUNK: u64 = 10;
pub const NUM_CHUNKS: u64 = 10;

pub fn test() {
    test_predefined();
    test_random_range(0, SEEDS_PER_CHUNK * NUM_CHUNKS);
    test_gc_components();
}

/// Run just the pre-defined heap tests + GC component tests (no random seeds).
pub fn test_predefined() {
    println!("Testing garbage collection ...");
    println!("  Testing pre-defined heaps...");
    for test_heap in test_heaps() {
        run_gc_tests(&test_heap);
    }
}

/// Run random heap tests for seeds in [start, end).
pub fn test_random_range(start: u64, end: u64) {
    println!("  Testing random heaps {}-{}...", start, end);
    for seed in start..end {
        print!("\r{}/{}", seed - start + 1, end - start);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        test_random_heap(seed, 180);
    }
    print!("\r");
}

#[non_incremental_gc]
pub fn test_gc_components() {
    compacting::test();
    generational::test();
}

#[incremental_gc]
pub fn test_gc_components() {
    incremental::test();
}

fn test_heaps() -> Vec<TestHeap> {
    vec![
        // Just a random test that covers a bunch of cases:
        // - Self references
        // - Unreachable objects
        // - Forward pointers
        // - Backwards pointers
        // - More than one fields in an object
        TestHeap {
            heap: vec![
                (0, vec![0, 2]),
                (1, vec![0, 1, 2, 3]),
                (2, vec![0]),
                (3, vec![3]),
            ],
            roots: vec![0, 2, 3],
            continuation_table: vec![0],
        },
        // Tests pointing to the same object in multiple fields of an object. Also has unreachable
        // objects.
        TestHeap {
            heap: vec![(0, vec![]), (1, vec![]), (2, vec![])],
            roots: vec![1],
            continuation_table: vec![0, 0],
        },
        // Root points backwards in heap. Caught a bug in mark-compact collector.
        TestHeap {
            heap: vec![(0, vec![]), (1, vec![2]), (2, vec![1])],
            roots: vec![2],
            continuation_table: vec![],
        },
    ]
}

fn test_random_heap(seed: u64, max_objects: usize) {
    let random_heap = random::generate(seed, max_objects);
    run_gc_tests(&random_heap);
}

// All fields are vectors to preserve ordering. Objects are allocated/ added to root arrays etc. in
// the same order they appear in these vectors. Each object in `heap` should have a unique index,
// which is checked when creating the heap.
#[derive(Debug)]
pub struct TestHeap {
    pub heap: Vec<(ObjectIdx, Vec<ObjectIdx>)>,
    pub roots: Vec<ObjectIdx>,
    pub continuation_table: Vec<ObjectIdx>,
}

impl TestHeap {
    pub fn build(&self, gc: GC, free_space: usize) -> MotokoHeap {
        MotokoHeap::new(
            &self.heap,
            &self.roots,
            &self.continuation_table,
            gc,
            free_space,
        )
    }
}

/// Test all GC implementations with the given heap
fn run_gc_tests(test_heap: &TestHeap) {
    for gc in &GC_IMPLS {
        test_gc(*gc, test_heap);
    }
    reset_gc();
}

fn test_gc(gc: GC, test_heap: &TestHeap) {
    let mut heap = test_heap.build(gc, 0);
    let refs = &test_heap.heap;
    let roots = &test_heap.roots;
    let continuation_table = &test_heap.continuation_table;

    initialize_gc(&mut heap);

    // Check `create_dynamic_heap` sanity
    check_dynamic_heap(
        CheckMode::Reachability,
        refs,
        roots,
        continuation_table,
        &**heap.heap(),
        heap.heap_base_offset(),
        heap.heap_ptr_offset(),
        heap.static_root_array_variable_offset(),
        heap.continuation_table_variable_offset(),
        heap.region0_pointer_variable_offset(),
    );

    for round in 0..3 {
        let check_all_reclaimed = gc.run(&mut heap, round);

        let heap_base_offset = heap.heap_base_offset();
        let heap_ptr_offset = heap.heap_ptr_offset();
        let static_array_variable_offset = heap.static_root_array_variable_offset();
        let continuation_table_variable_offset = heap.continuation_table_variable_offset();
        let region0_ptr_offset = heap.region0_pointer_variable_offset();
        check_dynamic_heap(
            if check_all_reclaimed {
                CheckMode::AllReclaimed
            } else {
                CheckMode::Reachability
            },
            refs,
            roots,
            continuation_table,
            &**heap.heap(),
            heap_base_offset,
            heap_ptr_offset,
            static_array_variable_offset,
            continuation_table_variable_offset,
            region0_ptr_offset,
        );
    }
}

#[non_incremental_gc]
fn initialize_gc(_heap: &mut MotokoHeap) {}

#[incremental_gc]
fn initialize_gc(heap: &mut MotokoHeap) {
    use motoko_rts::gc::incremental::{
        IncrementalGC, get_partitioned_heap, set_incremental_gc_state,
    };
    use motoko_rts::types::Bytes;
    unsafe {
        let state = IncrementalGC::<MotokoHeap>::initial_gc_state(heap.heap_base_address());
        set_incremental_gc_state(Some(state));
        let allocation_size = heap.heap_ptr_address() - heap.heap_base_address();

        // Synchronize the partitioned heap with one big combined allocation by starting from the base pointer as the heap pointer.
        let result = get_partitioned_heap().allocate(heap, Bytes(allocation_size).to_words());
        // Check that the heap pointer (here equals base pointer) is unchanged, i.e. no partition switch has happened.
        // This is a restriction in the unit test where `MotokoHeap` only supports contiguous bump allocation during initialization.
        assert_eq!(result.get_ptr(), heap.heap_base_address());
    }
}

#[non_incremental_gc]
fn reset_gc() {}

#[incremental_gc]
fn reset_gc() {
    use motoko_rts::gc::incremental::set_incremental_gc_state;
    unsafe {
        set_incremental_gc_state(None);
    }
}

#[derive(Debug, PartialEq)]
pub enum CheckMode {
    /// Check reachability of all necessary objects.
    Reachability,
    /// Check the reachability of all necessary objects and
    /// that all garbage objects have been reclaimed.
    AllReclaimed,
    /// Check valid dynamic heap after stabilization.
    #[cfg(feature = "enhanced_orthogonal_persistence")]
    Stabilzation,
}

#[classical_persistence]
pub fn check_dynamic_heap(
    mode: CheckMode,
    objects: &[(ObjectIdx, Vec<ObjectIdx>)],
    roots: &[ObjectIdx],
    continuation_table: &[ObjectIdx],
    heap: &[u8],
    heap_base_offset: usize,
    heap_ptr_offset: usize,
    _static_root_array_variable_offset: usize,
    continuation_table_variable_offset: usize,
    region0_ptr_offset: usize,
) {
    self::classical::check_dynamic_heap(
        mode,
        objects,
        roots,
        continuation_table,
        heap,
        heap_base_offset,
        heap_ptr_offset,
        continuation_table_variable_offset,
        region0_ptr_offset,
    );
}

#[enhanced_orthogonal_persistence]
pub fn check_dynamic_heap(
    mode: CheckMode,
    objects: &[(ObjectIdx, Vec<ObjectIdx>)],
    roots: &[ObjectIdx],
    continuation_table: &[ObjectIdx],
    heap: &[u8],
    heap_base_offset: usize,
    heap_ptr_offset: usize,
    static_root_array_variable_offset: usize,
    continuation_table_variable_offset: usize,
    region0_ptr_offset: usize,
) {
    self::enhanced::check_dynamic_heap(
        mode,
        objects,
        roots,
        continuation_table,
        heap,
        heap_base_offset,
        heap_ptr_offset,
        static_root_array_variable_offset,
        continuation_table_variable_offset,
        region0_ptr_offset,
    );
}

fn compute_reachable_objects(
    roots: &[ObjectIdx],
    continuation_table: &[ObjectIdx],
    heap: &FxHashMap<ObjectIdx, &[ObjectIdx]>,
) -> FxHashSet<ObjectIdx> {
    let root_iter = roots.iter().chain(continuation_table.iter()).copied();

    let mut closure: FxHashSet<ObjectIdx> = root_iter.clone().collect();
    let mut work_list: Vec<ObjectIdx> = root_iter.collect();

    while let Some(next) = work_list.pop() {
        let pointees = *heap
            .get(&next)
            .unwrap_or_else(|| panic!("Object {} is in the work list, but not in heap", next));

        for pointee in pointees {
            if closure.insert(*pointee) {
                work_list.push(*pointee);
            }
        }
    }

    closure
}
