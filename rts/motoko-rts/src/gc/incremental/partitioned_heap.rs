//! Partitioned heap used in incremental GC for compacting evacuation.
//! The heap is divided in equal sized partitions of a large size `PARTITION_SIZE`.
//! Some partitions contain a static (pinned) heap space that is never moved.
//! This is used for static objects at the beginning of the heap or for a partition table.
//! Beyond the static space of a partition, the dynamic heap space starts with `dynamic_size`.
//!
//! Heap layout, with a dynamic number of partitions:
//! ┌───────────────┬───────────────┬───────────────┐
//! │  partition 0  │  partition 1  |      ...      |
//! └───────────────┴───────────────┴───────────────┘
//!
//! Partition layout:
//! ┌───────────────┬───────────────┬───────────────┐
//! │ static_space  │ dynamic_space |  free_space   |
//! └───────────────┴───────────────┴───────────────┘
//!
//! The heap defines an allocation partition that is the target for subsequent object allocations
//! by using efficient bump allocation inside the allocation partition.
//! Whenever a partition is full or has insufficient space to accommodate a new allocation,
//! a new empty partition is selected for allocation.
//!
//! A linked list of partition tables allows dynamic growth of the heap memory even in 64-bit address
//! space. The first partition table is placed in the record of the partitioned heap. Subsequent
//! partition tables reside in the static space of a partition.
//!
//! ┌─────────────────────┐ extension ┌─────────────────────┐ extension
//! │   Partition table   │---------->│   Partition table   │----------> ...
//! └─────────────────────┘           └─────────────────────┘
//! (in `PartitionedHeap`)              (in static space)
//!
//! On garbage collection, partitions are selected for evacuation by prioritizing high-garbage
//! partitions. The live objects of the partitions selected for evacuation are moved out to
//! other remaining partitions (through allocation). Thereby, objects from different evacuated
//! partitions can be allocated to a common partition. Allocation partitions must not be evacuated.
//!
//! To prevent that the evacuation phase runs out of free space, the number of evacuated
//! partitions is limited to the free space that is available and needed for the evacuations.
//!
//! Large allocations:
//! Huge objects with a size > PARTITION_SIZE are allocated across multiple contiguous free
//! partitions. For this purpose, a corresponding sequence of contiguous free partitions needs
//! to be searched. Huge objects stay in their partitions for their entire lifetime, i.e. they
//! are never evacuated. When becoming garbage, the underlying partitions of a huge object are
//! immediately freed. Large object allocation may be prone to external fragmentation problems,
//! i.e. that no sufficient contiguous free partitions are available on allocation. Currently,
//! this external fragmentation problem is not handled by moving other partitions which would
//! require a special blocking full GC collection. Moreover, for simplicity, the remainder
//! of the last partition of a huge object is not used for further small object allocations,
//! which implies limited internal fragmentation.

use core::{
    array::from_fn,
    iter::Iterator,
    ops::Range,
    ptr::{null, null_mut},
};

use motoko_rts_macros::{classical_persistence, enhanced_orthogonal_persistence};

use crate::{
    gc::incremental::mark_bitmap::BITMAP_ITERATION_END,
    memory::{Memory, alloc_blob},
    rts_trap_with,
    stable_option::StableOption,
    types::*,
};

use super::{
    mark_bitmap::{BITMAP_SIZE, BitmapIterator, MarkBitmap},
    sort::sort,
    time::BoundedTime,
};

/// Size of each partition.
/// Select the size of the power of two with the smallest WASM memory size in the benchmark.
/// -> Small partitions are inefficient in terms of both memory and runtime costs
///    due to the increased frequency of large object handling.
/// -> Large partitions are a waste for small programs, since the WASM memory is
///    allocated in that granularity and GC is then triggered later.
#[enhanced_orthogonal_persistence]
pub const PARTITION_SIZE: usize = 64 * 1024 * 1024;

#[classical_persistence]
pub const PARTITION_SIZE: usize = 32 * 1024 * 1024;

/// Number of entries per partition table.
/// Tables are linearly linked, allowing the usage of the entire address space.
/// Maximum contiguous space is `(PARTITION_PER_TABLE - 1) * PARTITION_SIZE`.
const PARTITIONS_PER_TABLE: usize = 128; // 8 GB of space for 64 MB partitions and 4 GB for 32 MB partitions.

/// Maximum number of partitions in the memory.
/// For simplicity, the last partition is left unused, to avoid a numeric overflow when
/// computing the end address of the last partition.
const MAX_PARTITIONS: usize = usize::MAX / PARTITION_SIZE;

/// Partitions are only evacuated if the space occupation of alive objects in the partition
/// is greater than this threshold.
/// Based on benchmark measurements, this rate is tuned to optimize the following metrics
/// in the order of occurrence:
/// 1. Lowest heap size (most reclamation).
/// 2. Lowest WASM memory size (i.e. for minimum heap size).
pub const SURVIVAL_RATE_THRESHOLD: f64 = 0.85;

/// Heap partition of size `PARTITION_SIZE`.
/// Use a long-term representation by relying on C layout.
#[repr(C)]
pub struct Partition {
    index: usize,        // Index of the partition `0..MAX_PARTITIONS`.
    free: bool,          // Denotes a free partition (which may still contain static space).
    large_content: bool, // Specifies whether a large object is contained that spans multiple partitions.
    marked_size: usize,  // Total amount of marked object space in the dynamic space.
    static_size: usize,  // Size of the static space.
    dynamic_size: usize, // Size of the dynamic space.
    bitmap: MarkBitmap,  // Mark bitmap used for marking objects inside this partition.
    temporary: bool,     // Specifies a temporary partition used during a GC run to store bitmaps.
    evacuate: bool,      // Specifies whether the partition is to be evacuated or being evacuated.
    update: bool,        // Specifies whether the pointers in the partition have to be updated.
}

/// Optimization: Avoiding `Option` or `Lazy`.
#[classical_persistence]
const UNINITIALIZED_PARTITION: Partition = Partition {
    index: usize::MAX,
    free: false,
    large_content: false,
    marked_size: 0,
    static_size: 0,
    dynamic_size: 0,
    bitmap: super::mark_bitmap::DEFAULT_MARK_BITMAP,
    temporary: false,
    evacuate: false,
    update: false,
};

impl Partition {
    pub fn get_index(&self) -> usize {
        self.index
    }

    pub fn start_address(&self) -> usize {
        self.index * PARTITION_SIZE
    }

    fn end_address(&self) -> usize {
        self.start_address() + PARTITION_SIZE
    }

    pub fn dynamic_space_start(&self) -> usize {
        self.start_address() + self.static_size
    }

    pub fn dynamic_space_end(&self) -> usize {
        self.dynamic_space_start() + self.dynamic_size
    }

    pub fn dynamic_size(&self) -> usize {
        self.dynamic_size
    }

    pub fn occuped_size(&self) -> usize {
        self.dynamic_size + self.static_size
    }

    pub fn marked_size(&self) -> usize {
        self.marked_size
    }

    pub fn free_size(&self) -> usize {
        self.end_address() - self.dynamic_space_end()
    }

    pub fn garbage_amount(&self) -> usize {
        debug_assert!(self.marked_size <= self.dynamic_size);
        self.dynamic_size - self.marked_size
    }

    pub fn is_free(&self) -> bool {
        self.free
    }

    pub fn is_evacuation_candidate(&self) -> bool {
        !self.is_free()
            && !self.has_large_content()
            && self.dynamic_size() > 0
            && self.survival_rate() <= SURVIVAL_RATE_THRESHOLD
    }

    pub fn to_be_evacuated(&self) -> bool {
        self.evacuate
    }

    pub fn to_be_updated(&self) -> bool {
        self.update
    }

    pub fn is_temporary(&self) -> bool {
        self.temporary
    }

    pub fn has_dynamic_space(&self) -> bool {
        !self.free && !self.temporary && self.static_size != PARTITION_SIZE
    }

    pub fn get_bitmap(&self) -> &MarkBitmap {
        &self.bitmap
    }

    pub fn mutable_bitmap(&mut self) -> &mut MarkBitmap {
        &mut self.bitmap
    }

    #[cfg(feature = "memory_check")]
    unsafe fn clear_free_remainder(&self) {
        use crate::constants::WORD_SIZE;
        debug_assert!(self.dynamic_space_end() <= self.end_address());
        let remaining_space = self.end_address() - self.dynamic_space_end();
        // This code assumes that the free space has been allocated (with Wasm memory grow).
        // With incremental partition allocation, this might not be the case.
        // We need to ensure that the free space is allocated if we try to clear it.
        use crate::memory::ic::allocate_wasm_memory;
        allocate_wasm_memory(Bytes(self.end_address()));

        debug_assert_eq!(remaining_space % WORD_SIZE, 0);
        debug_assert!(remaining_space <= PARTITION_SIZE);
        if remaining_space == 0 {
            return;
        }
        let block = self.dynamic_space_end() as *mut Tag;
        if remaining_space == WORD_SIZE {
            *block = TAG_ONE_WORD_FILLER;
        } else {
            *block = TAG_FREE_SPACE;
            let header_size = size_of::<FreeSpace>().to_bytes().as_usize();
            debug_assert!(remaining_space >= header_size);
            let free_space = block as *mut FreeSpace;
            (*free_space).words = Bytes(remaining_space - header_size).to_words();
            // Clear the remainder of the free space.
            let clear_start = free_space as usize + header_size;
            let clear_length = Bytes(remaining_space - header_size);
            crate::mem_utils::memzero(clear_start, clear_length.to_words());
            debug_assert_eq!(free_space.size().to_bytes().as_usize(), remaining_space);
        }
    }

    pub unsafe fn free(&mut self) {
        debug_assert!(!self.free);
        debug_assert!(self.evacuate || self.large_content || self.temporary);
        debug_assert_eq!(self.marked_size, 0);
        debug_assert!(!self.update);
        self.free = true;
        self.dynamic_size = 0;
        self.evacuate = false;
        self.large_content = false;
        self.temporary = false;

        #[cfg(feature = "memory_check")]
        self.clear_free_remainder();
    }

    pub fn survival_rate(&self) -> f64 {
        let dynamic_heap_space = PARTITION_SIZE - self.static_size;
        debug_assert!(self.marked_size <= dynamic_heap_space);
        self.marked_size as f64 / dynamic_heap_space as f64
    }

    pub fn has_large_content(&self) -> bool {
        self.large_content
    }

    pub fn is_completely_free(&self) -> bool {
        self.free && self.free_size() == PARTITION_SIZE
    }
}

/// Iterates over all partitions and their contained marked objects, by skipping
/// free partitions, the subsequent partitions of large objects, and unmarked objects.
/// Use a long-term representation by relying on C layout.
#[repr(C)]
pub struct PartitionedHeapIterator {
    partition_index: usize,
    number_of_partitions: usize,
    bitmap_iterator: StableOption<BitmapIterator>,
    visit_large_object: bool,
}

impl PartitionedHeapIterator {
    pub fn new(heap: &PartitionedHeap) -> PartitionedHeapIterator {
        let mut iterator = PartitionedHeapIterator {
            partition_index: 0,
            number_of_partitions: heap.number_of_partitions,
            bitmap_iterator: StableOption::None,
            visit_large_object: false,
        };
        iterator.skip_empty_partitions(heap);
        iterator.start_object_iteration(heap);
        iterator
    }

    fn skip_empty_partitions(&mut self, heap: &PartitionedHeap) {
        loop {
            if self.partition_index == self.number_of_partitions {
                return;
            }
            let partition = heap.get_partition(self.partition_index);
            if partition.has_dynamic_space() {
                return;
            }
            self.partition_index += 1;
        }
    }

    pub fn has_partition(&self) -> bool {
        self.partition_index < self.number_of_partitions
    }

    pub fn current_partition<'a>(&self, heap: &'a PartitionedHeap) -> &'a Partition {
        debug_assert!(self.partition_index < self.number_of_partitions);
        heap.get_partition(self.partition_index)
    }

    pub unsafe fn next_partition(&mut self, heap: &PartitionedHeap) {
        debug_assert!(self.partition_index < self.number_of_partitions);
        let partition = heap.get_partition(self.partition_index);
        let number_of_partitions = if partition.has_large_content() {
            let large_object = partition.dynamic_space_start() as *mut Obj;
            PartitionedHeap::partitions_length(large_object)
        } else {
            1
        };
        self.partition_index += number_of_partitions;
        self.skip_empty_partitions(heap);
        self.start_object_iteration(heap)
    }

    fn start_object_iteration(&mut self, heap: &PartitionedHeap) {
        debug_assert!(self.partition_index <= self.number_of_partitions);
        if self.partition_index == self.number_of_partitions {
            self.bitmap_iterator = StableOption::None;
            self.visit_large_object = false;
        } else {
            let partition = heap.get_partition(self.partition_index);
            if partition.has_large_content() {
                self.bitmap_iterator = StableOption::None;
                self.visit_large_object = partition.marked_size() > 0
            } else {
                self.bitmap_iterator = StableOption::Some(partition.get_bitmap().iterate());
                self.visit_large_object = false;
            }
        }
    }

    pub fn has_object(&self) -> bool {
        if self.bitmap_iterator.is_some() {
            let iterator = self.bitmap_iterator.as_ref().unwrap();
            let offset = iterator.current_marked_offset();
            offset != BITMAP_ITERATION_END
        } else {
            self.visit_large_object
        }
    }

    pub fn current_object(&self) -> *mut Obj {
        let partition_start = self.partition_index * PARTITION_SIZE;
        if self.bitmap_iterator.is_some() {
            let iterator = self.bitmap_iterator.as_ref().unwrap();
            let offset = iterator.current_marked_offset();
            debug_assert_ne!(offset, BITMAP_ITERATION_END);
            let address = partition_start + offset;
            address as *mut Obj
        } else {
            debug_assert!(self.visit_large_object);
            partition_start as *mut Obj
        }
    }

    pub fn next_object(&mut self) {
        if self.bitmap_iterator.is_some() {
            let iterator = self.bitmap_iterator.as_mut().unwrap();
            debug_assert_ne!(iterator.current_marked_offset(), BITMAP_ITERATION_END);
            iterator.next();
        } else {
            debug_assert!(self.visit_large_object);
            self.visit_large_object = false;
        }
    }
}

/// The first table is part of the static `PartionedHeap` record.
/// Extension tables reside in the static space of a partition.
/// Therefore, partition tables are never moved in memory.
#[repr(C)]
struct PartitionTable {
    partitions: [Partition; PARTITIONS_PER_TABLE],
    extension: *mut PartitionTable,
}

/// Optimization: Avoiding `Option` or `Lazy`.
#[classical_persistence]
const UNINITIALIZED_PARTITION_TABLE: PartitionTable = PartitionTable {
    partitions: [UNINITIALIZED_PARTITION; PARTITIONS_PER_TABLE],
    extension: null_mut(),
};

const PARTITION_TABLE_SIZE: usize = core::mem::size_of::<PartitionTable>();

impl PartitionTable {
    /// The start index is the first partition index in the table.
    /// The static space starts at the first partition entry of the partition table.
    pub fn new(start_index: usize, static_space: usize) -> PartitionTable {
        let completely_static_partitions = static_space / PARTITION_SIZE;
        let partitions = from_fn(|offset| Partition {
            index: start_index + offset,
            free: true,
            large_content: false,
            marked_size: 0,
            static_size: if offset < completely_static_partitions {
                PARTITION_SIZE
            } else if offset == completely_static_partitions {
                static_space % PARTITION_SIZE
            } else {
                0
            },
            dynamic_size: 0,
            bitmap: MarkBitmap::new(),
            temporary: false,
            evacuate: false,
            update: false,
        });
        PartitionTable {
            partitions,
            extension: null_mut(),
        }
    }

    /// The table address must point to static partition space.
    /// The start index is the first partition index in the table.
    /// The static space starts at the first partition entry of the partition table.
    pub unsafe fn allocate(
        table_address: usize,
        start_index: usize,
        static_space: usize,
    ) -> *mut PartitionTable {
        let table = table_address as *mut PartitionTable;
        *table = Self::new(start_index, static_space);
        table
    }
}

struct PartitionIterator {
    table: *mut PartitionTable,
    index: usize,
}

impl PartitionIterator {
    pub fn new(heap: &mut PartitionedHeap) -> PartitionIterator {
        let first_table = &mut heap.partition_table as *mut PartitionTable;
        PartitionIterator {
            table: first_table,
            index: 0,
        }
    }
}

impl Iterator for PartitionIterator {
    type Item = &'static mut Partition;

    fn next(&mut self) -> Option<Self::Item> {
        debug_assert_ne!(self.table, null_mut());
        if self.index > MAX_PARTITIONS {
            return None;
        }
        let table_offset = self.index % PARTITIONS_PER_TABLE;
        unsafe {
            if self.index > 0 && table_offset == 0 {
                self.table = (*self.table).extension;
                if self.table == null_mut() {
                    return None;
                }
            }
            let partition = &mut (*self.table).partitions[table_offset];
            self.index += 1;
            Some(partition)
        }
    }
}

/// Partitioned heap used by the incremental GC.
/// Use a long-term representation by relying on C layout.
#[repr(C)]
pub struct PartitionedHeap {
    partition_table: PartitionTable,
    number_of_partitions: usize,
    heap_base: usize,
    allocation_index: usize, // Index of the partition currently used for allocations.
    free_partitions: usize,  // Number of free partitions.
    evacuating: bool,
    reclaimed: u64,
    bitmap_allocation_pointer: usize, // Free pointer for allocating the next mark bitmap.
    gc_running: bool, // Create bitmaps for partitions when allocated during active GC.
    precomputed_heap_size: usize, // Occupied heap size, excluding the dynamic heap in the allocation partition.
    evacuated_size: usize, // Size of all evacuated objects during a GC run. Serves for accurate total allocation statistics.
}

/// Optimization: Avoiding `Option` or `LazyCell`.
#[classical_persistence]
pub const UNINITIALIZED_HEAP: PartitionedHeap = PartitionedHeap {
    partition_table: UNINITIALIZED_PARTITION_TABLE,
    number_of_partitions: 0,
    heap_base: 0,
    allocation_index: 0,
    free_partitions: 0,
    evacuating: false,
    reclaimed: 0,
    bitmap_allocation_pointer: 0,
    gc_running: false,
    precomputed_heap_size: 0,
    evacuated_size: 0,
};

impl PartitionedHeap {
    pub fn new(heap_base: usize) -> PartitionedHeap {
        let number_of_partitions = PARTITIONS_PER_TABLE;
        let allocation_index = heap_base / PARTITION_SIZE;
        let mut partition_table = PartitionTable::new(0, heap_base);
        for index in 0..allocation_index + 1 {
            partition_table.partitions[index].free = false;
        }
        let free_partitions = number_of_partitions - allocation_index - 1;
        PartitionedHeap {
            partition_table,
            number_of_partitions,
            heap_base,
            allocation_index,
            free_partitions,
            evacuating: false,
            reclaimed: 0,
            bitmap_allocation_pointer: 0,
            gc_running: false,
            precomputed_heap_size: heap_base,
            evacuated_size: 0,
        }
    }

    pub fn maximum_mark_bitmap_size(&self) -> usize {
        debug_assert!(self.free_partitions <= self.number_of_partitions);
        let used_partitions = self.number_of_partitions - self.free_partitions;
        used_partitions * BITMAP_SIZE
    }

    fn partitions(&mut self) -> PartitionIterator {
        PartitionIterator::new(self)
    }

    pub fn is_initialized(&self) -> bool {
        self.number_of_partitions > 0
    }

    pub fn base_address(&self) -> usize {
        self.heap_base
    }

    unsafe fn get_extension_table(&self, partition_index: usize) -> *mut PartitionTable {
        debug_assert!(partition_index >= PARTITIONS_PER_TABLE);
        let mut index = partition_index - PARTITIONS_PER_TABLE;
        let mut table = self.partition_table.extension;
        while index >= PARTITIONS_PER_TABLE {
            index -= PARTITIONS_PER_TABLE;
            debug_assert_ne!((*table).extension, null_mut());
            table = (*table).extension;
        }
        table
    }

    unsafe fn get_partition_table(&self, partition_index: usize) -> *const PartitionTable {
        if partition_index < PARTITIONS_PER_TABLE {
            let first_table = &self.partition_table as *const PartitionTable;
            debug_assert_ne!(first_table, null());
            first_table
        } else {
            self.get_extension_table(partition_index)
        }
    }

    unsafe fn mutable_partition_table(&mut self, partition_index: usize) -> *mut PartitionTable {
        self.get_partition_table(partition_index) as *mut PartitionTable
    }

    pub fn get_partition(&self, index: usize) -> &Partition {
        unsafe {
            let table = self.get_partition_table(index);
            let offset = index % PARTITIONS_PER_TABLE;
            &(*table).partitions[offset]
        }
    }

    fn mutable_partition(&mut self, index: usize) -> &mut Partition {
        unsafe {
            let table = self.mutable_partition_table(index);
            let offset = index % PARTITIONS_PER_TABLE;
            &mut (*table).partitions[offset]
        }
    }

    unsafe fn add_partition_table<M: Memory>(&mut self, mem: &mut M) {
        debug_assert!(self.number_of_partitions > 0);
        let last_table = self.mutable_partition_table(self.number_of_partitions - 1);
        debug_assert_ne!(last_table, null_mut());
        debug_assert_eq!((*last_table).extension, null_mut());
        let table_address = self.number_of_partitions * PARTITION_SIZE;
        mem.grow_memory(table_address + PARTITION_SIZE);
        (*last_table).extension = PartitionTable::allocate(
            table_address,
            self.number_of_partitions,
            PARTITION_TABLE_SIZE,
        );
        // The partition table is small enough such that all partitions contain free space.
        debug_assert!(PARTITION_TABLE_SIZE < PARTITION_SIZE);
        self.free_partitions += PARTITIONS_PER_TABLE;
        self.number_of_partitions += PARTITIONS_PER_TABLE;
        self.precomputed_heap_size += PARTITION_TABLE_SIZE;
    }

    unsafe fn allocate_partition<M: Memory, F: Fn(&mut Self) -> Option<usize>>(
        &mut self,
        mem: &mut M,
        find_partition: &F,
    ) -> usize {
        let mut result = find_partition(self);
        if result.is_none() {
            self.add_partition_table(mem);
            result = find_partition(self);
        }
        if result.is_none() {
            rts_trap_with("Cannot grow memory");
        }
        result.unwrap()
    }

    fn scan_for_temporary_partition(&mut self) -> Option<usize> {
        for partition in self.partitions() {
            if partition.is_completely_free() {
                return Some(partition.index);
            }
        }
        None
    }

    unsafe fn allocate_temporary_partition<M: Memory>(&mut self, mem: &mut M) -> &mut Partition {
        let index = self.allocate_partition(mem, &Self::scan_for_temporary_partition);
        debug_assert!(self.free_partitions > 0);
        self.free_partitions -= 1;
        let partition = self.mutable_partition(index);
        debug_assert_eq!(partition.dynamic_size, 0);
        partition.free = false;
        partition.temporary = true;
        partition
    }

    /// The returned bitmap address is guaranteed to be 64-bit-aligned.
    unsafe fn allocate_bitmap<M: Memory>(&mut self, mem: &mut M) -> *mut u8 {
        if self.bitmap_allocation_pointer % PARTITION_SIZE == 0 {
            let partition = self.allocate_temporary_partition(mem);
            mem.grow_memory(partition.end_address());
            self.bitmap_allocation_pointer = partition.start_address();
        }
        let bitmap_address = self.bitmap_allocation_pointer as *mut u8;
        self.bitmap_allocation_pointer += BITMAP_SIZE;
        debug_assert_eq!(
            bitmap_address as usize % size_of::<usize>().to_bytes().as_usize(),
            0
        );
        bitmap_address
    }

    // Optimization: Returns true if the object transitioned from unmarked to marked.
    pub unsafe fn mark_object(&mut self, object: *mut Obj) -> bool {
        let address = object as usize;
        let partition_index = address / PARTITION_SIZE;
        let partition = self.mutable_partition(partition_index);
        if partition.has_large_content() {
            return self.mark_large_object(object);
        }
        let bitmap = partition.mutable_bitmap();
        let offset = address % PARTITION_SIZE;
        if bitmap.is_marked(offset) {
            return false;
        }
        bitmap.mark(offset);
        partition.marked_size += block_size(address).to_bytes().as_usize();
        true
    }

    pub unsafe fn is_object_marked(&self, object: *mut Obj) -> bool {
        let address = object as usize;
        let partition_index = address / PARTITION_SIZE;
        let partition = self.get_partition(partition_index);
        if partition.has_large_content() {
            return self.is_large_object_marked(object);
        }
        let bitmap = partition.get_bitmap();
        let offset = address % PARTITION_SIZE;
        bitmap.is_marked(offset)
    }

    pub unsafe fn start_collection<M: Memory>(&mut self, mem: &mut M, time: &mut BoundedTime) {
        self.check_occupied_size();
        debug_assert_eq!(self.bitmap_allocation_pointer, 0);
        debug_assert!(!self.gc_running);
        self.gc_running = true;
        for partition_index in 0..self.number_of_partitions {
            let partition = self.get_partition(partition_index);
            if partition.has_dynamic_space() && !partition.has_large_content() {
                let bitmap_address = self.allocate_bitmap(mem);
                self.mutable_partition(partition_index)
                    .bitmap
                    .assign(bitmap_address);
                time.advance(Bytes(BITMAP_SIZE).to_words().as_usize());
            }
        }
    }

    pub unsafe fn plan_evacuations<M: Memory>(&mut self, mem: &mut M) {
        debug_assert_eq!(
            self.partitions()
                .filter(|partition| partition.is_free())
                .count(),
            self.free_partitions
        );
        // Do not use all free partitions for evacuation.
        // Leave a reserve for mutator allocations during a GC run.
        const EVACUATION_FRACTION: usize = 2;
        let reserved_partitions =
            (self.free_partitions + EVACUATION_FRACTION - 1) / EVACUATION_FRACTION;
        let mut evacuation_space = reserved_partitions * PARTITION_SIZE;
        let ranked_partitions = self.rank_partitions_by_garbage(mem);
        for rank in 0..self.number_of_partitions {
            let index = *ranked_partitions.add(rank);
            if index != self.allocation_index && self.get_partition(index).is_evacuation_candidate()
            {
                let partition = self.mutable_partition(index);
                if evacuation_space < partition.marked_size() {
                    // Limit the evacuations to the available free space for the current GC run.
                    return;
                }
                evacuation_space -= partition.marked_size();
                partition.evacuate = true;
                self.evacuating = true;
                debug_assert_eq!(self.evacuated_size, 0);
            }
        }
    }

    unsafe fn temporary_array<M: Memory>(mem: &mut M, length: usize) -> *mut usize {
        // No post allocation barrier as this RTS-internal blob can be collected by the GC.
        let blob = alloc_blob(mem, TAG_BLOB_B, Words(length).to_bytes());
        let payload = blob.as_blob_mut().payload_addr();
        payload as *mut usize
    }

    unsafe fn rank_partitions_by_garbage<M: Memory>(&self, mem: &mut M) -> *mut usize {
        let ranked_partitions = Self::temporary_array(mem, self.number_of_partitions);
        for index in 0..self.number_of_partitions {
            *ranked_partitions.add(index) = index;
        }
        sort(
            ranked_partitions,
            self.number_of_partitions,
            &|left, right| {
                self.get_partition(left)
                    .garbage_amount()
                    .cmp(&self.get_partition(right).garbage_amount())
                    .reverse()
            },
        );
        ranked_partitions
    }

    pub fn plan_updates(&mut self) {
        for partition in self.partitions() {
            debug_assert!(!partition.update);
            partition.update = !partition.is_free() && !partition.evacuate;
        }
    }

    pub unsafe fn complete_collection(&mut self) {
        for partition in self.partitions() {
            let marked_size = partition.marked_size;
            partition.update = false;
            partition.marked_size = 0;
            partition.bitmap.release();
            if partition.to_be_evacuated() {
                debug_assert!(partition.index != self.allocation_index);
                debug_assert!(partition.dynamic_size >= marked_size);
                self.reclaimed += (partition.dynamic_size - marked_size) as u64;
            }
            if partition.to_be_evacuated() || partition.temporary {
                self.precomputed_heap_size -= partition.dynamic_size;
                partition.free();
                self.free_partitions += 1;
            }
        }
        self.evacuating = false;
        self.evacuated_size = 0;
        self.bitmap_allocation_pointer = 0;
        debug_assert!(self.gc_running);
        self.gc_running = false;
        self.check_occupied_size();
    }

    pub fn updates_needed(&self) -> bool {
        self.evacuating
    }

    fn allocation_partition(&self) -> &Partition {
        self.get_partition(self.allocation_index)
    }

    fn mut_allocation_partition(&mut self) -> &mut Partition {
        self.mutable_partition(self.allocation_index)
    }

    pub fn is_allocation_partition(&self, index: usize) -> bool {
        self.allocation_index == index
    }

    fn scan_for_free_partition(&mut self, requested_space: usize) -> Option<usize> {
        for partition in self.partitions() {
            if partition.free && partition.free_size() >= requested_space {
                return Some(partition.index);
            }
        }
        None
    }

    unsafe fn allocate_free_partition<M: Memory>(
        &mut self,
        mem: &mut M,
        requested_space: usize,
    ) -> &mut Partition {
        let bitmap_address = if self.gc_running {
            self.allocate_bitmap(mem)
        } else {
            null_mut()
        };
        let index = self.allocate_partition(mem, &|context| {
            context.scan_for_free_partition(requested_space)
        });
        debug_assert!(self.free_partitions > 0);
        self.free_partitions -= 1;
        let partition = self.mutable_partition(index);
        debug_assert_eq!(partition.dynamic_size, 0);
        partition.free = false;
        if bitmap_address != null_mut() {
            partition.bitmap.assign(bitmap_address);
        }
        partition
    }

    fn check_occupied_size(&mut self) {
        debug_assert_eq!(
            self.partitions()
                .map(|partition| partition.static_size + partition.dynamic_size)
                .sum::<usize>(),
            self.occupied_size().as_usize()
        );
    }

    pub fn occupied_size(&self) -> Bytes<usize> {
        Bytes(self.precomputed_heap_size + self.allocation_partition().dynamic_size)
    }

    pub fn reclaimed_size(&self) -> Bytes<u64> {
        Bytes(self.reclaimed)
    }

    pub fn increase_evacuated_size(&mut self, size: Words<usize>) {
        self.evacuated_size += size.to_bytes().as_usize();
    }

    pub fn total_allocated_size(&self) -> Bytes<u64> {
        debug_assert!(self.evacuated_size <= self.occupied_size().as_usize());
        let heap_size_without_evacuations = self.occupied_size().as_usize() - self.evacuated_size;
        Bytes(heap_size_without_evacuations as u64) + self.reclaimed_size()
    }

    pub unsafe fn allocate<M: Memory>(&mut self, mem: &mut M, words: Words<usize>) -> Value {
        let size = words.to_bytes().as_usize();
        if size <= PARTITION_SIZE {
            self.allocate_normal_object(mem, size)
        } else {
            self.allocate_large_object(mem, size)
        }
    }

    unsafe fn allocate_normal_object<M: Memory>(&mut self, mem: &mut M, size: usize) -> Value {
        debug_assert!(size <= PARTITION_SIZE);
        let allocation_partition = self.mut_allocation_partition();
        debug_assert!(!allocation_partition.free);
        let heap_pointer = allocation_partition.dynamic_space_end();
        debug_assert!(size <= allocation_partition.end_address());
        if heap_pointer <= allocation_partition.end_address() - size {
            // Since the partition is allocated incrementally, we need to
            // ensure that the memory is allocated up to the end of the object,
            // which will also be the end of the dynamic_size of the partition.
            intra_partition_memory_grow(mem, heap_pointer + size);

            (*allocation_partition).dynamic_size += size;
            Value::from_ptr(heap_pointer)
        } else {
            self.allocate_in_new_partition(mem, size)
        }
    }

    // Significant performance gain by not inlining.
    #[inline(never)]
    unsafe fn allocate_in_new_partition<M: Memory>(&mut self, mem: &mut M, size: usize) -> Value {
        #[cfg(feature = "memory_check")]
        self.allocation_partition().clear_free_remainder();

        self.precomputed_heap_size += self.allocation_partition().dynamic_size;

        let new_partition = self.allocate_free_partition(mem, size);

        // Since the partition is allocated incrementally, we need to
        // ensure that the memory is allocated up to the end of the object.
        // In this case the allocation starts at the end of the
        // current dynamic_size of the partition.
        mem.grow_memory(new_partition.dynamic_space_end() + size);

        let heap_pointer = new_partition.dynamic_space_end();
        new_partition.dynamic_size += size;
        self.allocation_index = new_partition.index;
        Value::from_ptr(heap_pointer)
    }

    // Significant performance gain by not inlining.
    #[inline(never)]
    unsafe fn allocate_large_object<M: Memory>(&mut self, mem: &mut M, size: usize) -> Value {
        if size > usize::MAX - PARTITION_SIZE {
            panic!("Too large allocation");
        }
        let number_of_partitions = (size + PARTITION_SIZE - 1) / PARTITION_SIZE;
        debug_assert!(number_of_partitions > 0);

        let first_index = self.allocate_partition(mem, &|context| {
            context.scan_for_large_space(number_of_partitions)
        });
        let last_index = first_index + number_of_partitions - 1;

        debug_assert!(self.free_partitions >= number_of_partitions);
        self.free_partitions -= number_of_partitions;

        let end_address = self.get_partition(last_index).end_address();
        mem.grow_memory(end_address);
        for index in first_index..last_index + 1 {
            let partition = self.mutable_partition(index);
            debug_assert!(partition.free);
            debug_assert!(!partition.large_content);
            partition.free = false;
            partition.large_content = true;
            debug_assert_eq!(partition.static_size, 0);
            debug_assert_eq!(partition.dynamic_size, 0);
            debug_assert_eq!(partition.marked_size, 0);
            if index == last_index {
                partition.dynamic_size = size - (number_of_partitions - 1) * PARTITION_SIZE;

                #[cfg(feature = "memory_check")]
                partition.clear_free_remainder();
            } else {
                partition.dynamic_size = PARTITION_SIZE;
            }
            self.precomputed_heap_size += partition.dynamic_size;
        }
        let first_partition = self.mutable_partition(first_index);
        Value::from_ptr(first_partition.dynamic_space_start())
    }

    unsafe fn scan_for_large_space(&self, number_of_partitions: usize) -> Option<usize> {
        let mut start_of_free_range = 0;
        for index in 0..self.number_of_partitions {
            // Invariant: [start_of_free_range .. index) contains only free partitions.
            if self.get_partition(index).is_completely_free() {
                if index + 1 - start_of_free_range >= number_of_partitions {
                    return Some(start_of_free_range);
                }
            } else {
                start_of_free_range = index + 1;
            }
        }
        None
    }

    unsafe fn is_large_object_marked(&self, large_object: *mut Obj) -> bool {
        debug_assert_eq!(large_object as usize % PARTITION_SIZE, 0);
        let start_partition = large_object as usize / PARTITION_SIZE;
        self.get_partition(start_partition).marked_size > 0
    }

    unsafe fn occupied_partition_range(large_object: *mut Obj) -> Range<usize> {
        debug_assert_eq!(large_object as usize % PARTITION_SIZE, 0);
        let start_partition = large_object as usize / PARTITION_SIZE;
        let number_of_partitions = Self::partitions_length(large_object);
        start_partition..start_partition + number_of_partitions
    }

    unsafe fn partitions_length(large_object: *mut Obj) -> usize {
        let size = block_size(large_object as usize).to_bytes().as_usize();
        debug_assert!(size > PARTITION_SIZE);
        (size + PARTITION_SIZE - 1) / PARTITION_SIZE
    }

    pub unsafe fn collect_large_objects(&mut self) {
        let mut index = 0;
        while index < self.number_of_partitions {
            let partition = self.get_partition(index);
            if partition.has_large_content() {
                debug_assert!(!partition.free);
                let object = partition.dynamic_space_start() as *mut Obj;
                let number_of_partitions = Self::partitions_length(object);
                if partition.marked_size == 0 {
                    self.free_large_object(object);
                }
                index += number_of_partitions;
            } else {
                index += 1;
            }
        }
    }

    unsafe fn free_large_object(&mut self, object: *mut Obj) {
        let occupied_range = Self::occupied_partition_range(object);
        self.free_partitions += occupied_range.len();
        for index in occupied_range {
            let partition = self.mutable_partition(index);
            debug_assert!(partition.large_content);
            let size = partition.dynamic_size;
            partition.update = false;
            partition.free();
            self.reclaimed += size as u64;
            self.precomputed_heap_size -= size;
        }
    }

    // Significant performance gain by not inlining.
    // Optimization: Returns true if it has not yet been marked before.
    #[inline(never)]
    unsafe fn mark_large_object(&mut self, object: *mut Obj) -> bool {
        if self.is_large_object_marked(object) {
            return false;
        }
        let range = Self::occupied_partition_range(object);
        for index in range.start..range.end - 1 {
            self.mutable_partition(index).marked_size = PARTITION_SIZE;
        }
        let object_size = block_size(object as usize).to_bytes().as_usize();
        self.mutable_partition(range.end - 1).marked_size = object_size % PARTITION_SIZE;
        true
    }
}

#[cfg(feature = "ic")]
pub(crate) unsafe fn allocate_initial_memory(heap_base: Bytes<usize>) {
    use crate::memory::ic::allocate_wasm_memory;

    allocate_wasm_memory(heap_base);
}

#[cfg(feature = "ic")]
pub(crate) unsafe fn intra_partition_memory_grow<M: Memory>(_mem: &mut M, address: usize) {
    use crate::memory::ic::allocate_wasm_memory;

    // Allocate the memory without keeping the usual memory reserve.
    // This is because the partition may have been allocated in a moment
    // where the reserve was granted. Such a partition remains available for
    // subsequent incremental allocations even when the reserve is reactivated.
    allocate_wasm_memory(Bytes(address));
}

#[cfg(not(feature = "ic"))]
pub(crate) unsafe fn intra_partition_memory_grow<M: Memory>(mem: &mut M, address: usize) {
    // Only for test purposes
    mem.grow_memory(address);
}
