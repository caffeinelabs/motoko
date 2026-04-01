use motoko_rts_macros::{
    classical_persistence, enhanced_orthogonal_persistence, incremental_gc, non_incremental_gc,
};

#[classical_persistence]
mod classical;
#[enhanced_orthogonal_persistence]
mod enhanced;

use super::utils::{ObjectIdx, GC};

use motoko_rts::memory::Memory;
use motoko_rts::types::*;

use std::cell::{Ref, RefCell};
use std::rc::Rc;

/// Represents Motoko heaps. Reference counted (implements `Clone`) so we can clone and move values
/// of this type to GC callbacks.
#[derive(Clone)]
pub struct MotokoHeap {
    inner: Rc<RefCell<MotokoHeapInner>>,
}

impl Memory for MotokoHeap {
    unsafe fn alloc_words(&mut self, n: Words<usize>) -> Value {
        self.inner.borrow_mut().alloc_words(n)
    }

    unsafe fn grow_memory(&mut self, ptr: usize) {
        self.inner.borrow_mut().grow_memory(ptr);
    }
}

impl MotokoHeap {
    /// Create a new Motoko heap from the given object graph and roots. `GC` argument is used to
    /// allocate as little space as possible for the dynamic heap.
    ///
    /// Note that for `GC::MarkCompact` we limit the upper bound on mark stack size as
    /// `super::MAX_MARK_STACK_SIZE`. In the worst case the size would be the same as the heap
    /// size, but that's not a realistic scenario.
    pub fn new(
        map: &[(ObjectIdx, Vec<ObjectIdx>)],
        roots: &[ObjectIdx],
        continuation_table: &[ObjectIdx],
        gc: GC,
        free_space: usize,
    ) -> MotokoHeap {
        MotokoHeap {
            inner: Rc::new(RefCell::new(MotokoHeapInner::new(
                map,
                roots,
                continuation_table,
                gc,
                free_space,
            ))),
        }
    }

    /// Get the beginning of dynamic heap, as offset in the heap array
    pub fn heap_base_offset(&self) -> usize {
        self.inner.borrow().heap_base_offset
    }

    /// Get the heap pointer, as offset in the heap array
    pub fn heap_ptr_offset(&self) -> usize {
        self.inner.borrow().heap_ptr_offset
    }

    /// Get the heap pointer, as address in the current process. The address can be used to mutate
    /// the heap.
    pub fn heap_ptr_address(&self) -> usize {
        self.inner.borrow().heap_ptr_address()
    }

    /// Update the heap pointer given as an address in the current process.
    #[non_incremental_gc]
    pub fn set_heap_ptr_address(&self, address: usize) {
        self.inner.borrow_mut().set_heap_ptr_address(address)
    }

    /// Get the last heap pointer, as address in the current process. The address can be used to mutate
    /// the heap.
    #[non_incremental_gc]
    pub fn last_ptr_address(&self) -> usize {
        self.inner.borrow().last_ptr_address()
    }

    /// Update the last heap pointer given as an address in the current process.
    #[non_incremental_gc]
    pub fn set_last_ptr_address(&self, address: usize) {
        self.inner.borrow_mut().set_last_ptr_address(address)
    }

    /// Get the beginning of dynamic heap, as an address in the current process
    pub fn heap_base_address(&self) -> usize {
        self.inner.borrow().heap_base_address()
    }

    /// Get the offset of the variable pointing to the static root array.
    pub fn static_root_array_variable_offset(&self) -> usize {
        self.inner.borrow().static_root_array_variable_offset
    }

    /// Get the address of the variable pointing to the static root array.
    pub fn static_root_array_variable_address(&self) -> usize {
        self.inner.borrow().static_root_array_variable_address()
    }

    /// Get the offset of the variable pointing to the continuation table.
    pub fn continuation_table_variable_offset(&self) -> usize {
        self.inner.borrow().continuation_table_variable_offset
    }

    /// Get the address of the variable pointing to the continuation table.
    pub fn continuation_table_variable_address(&self) -> usize {
        self.inner.borrow().continuation_table_variable_address()
    }

    /// Get the offset of the variable pointing to region0.
    pub fn region0_pointer_variable_offset(&self) -> usize {
        self.inner.borrow().region0_pointer_variable_offset
    }

    /// Get the address of the variable pointing to region0
    #[incremental_gc]
    pub fn region0_pointer_variable_address(&self) -> usize {
        self.inner.borrow().region0_pointer_address()
    }

    /// Get the heap as an array. Use `offset` values returned by the methods above to read.
    pub fn heap(&self) -> Ref<'_, Box<[u8]>> {
        Ref::map(self.inner.borrow(), |heap| &heap.heap)
    }

    /// Print heap contents to stdout, for debugging purposes.
    #[allow(unused)]
    pub fn dump(&self) {
        unsafe {
            motoko_rts::debug::dump_heap(
                self.heap_base_address(),
                self.heap_ptr_address(),
                self.static_root_array_variable_address() as *mut Value,
                self.continuation_table_variable_address() as *mut Value,
            );
        }
    }
}

struct MotokoHeapInner {
    /// The heap. This is a boxed slice instead of a vector as growing this wouldn't make sense
    /// (all pointers would have to be updated).
    heap: Box<[u8]>,

    /// Where the dynamic heap starts
    heap_base_offset: usize,

    /// Last dynamic heap end, used for generational gc testing
    _heap_ptr_last: usize,

    /// Where the dynamic heap ends, i.e. the heap pointer
    heap_ptr_offset: usize,

    /// Offset of the static root array.
    ///
    /// Reminder: This location is in static memory and points to an array in the dynamic heap.
    static_root_array_variable_offset: usize,

    /// Offset of the continuation table pointer.
    ///
    /// Reminder: this location is in static heap and will have pointer to an array in dynamic
    /// heap.
    continuation_table_variable_offset: usize,

    /// Offset of the region 0 pointer.
    ///
    /// Reminder: this location is in static heap and will have pointer to an array in dynamic
    /// heap.
    region0_pointer_variable_offset: usize,
}

impl MotokoHeapInner {
    fn address_to_offset(&self, address: usize) -> usize {
        address - self.heap.as_ptr() as usize
    }

    fn offset_to_address(&self, offset: usize) -> usize {
        offset + self.heap.as_ptr() as usize
    }

    /// Get heap base in the process's address space
    fn heap_base_address(&self) -> usize {
        self.offset_to_address(self.heap_base_offset)
    }

    /// Get heap pointer (i.e. where the dynamic heap ends) in the process's address space
    fn heap_ptr_address(&self) -> usize {
        self.offset_to_address(self.heap_ptr_offset)
    }

    /// Set heap pointer
    fn set_heap_ptr_address(&mut self, address: usize) {
        self.heap_ptr_offset = self.address_to_offset(address);
    }

    /// Get last heap pointer (i.e. where the dynamic heap ends last GC run) in the process's address space
    #[non_incremental_gc]
    fn last_ptr_address(&self) -> usize {
        self.offset_to_address(self._heap_ptr_last)
    }

    /// Set last heap pointer
    #[non_incremental_gc]
    fn set_last_ptr_address(&mut self, address: usize) {
        self._heap_ptr_last = self.address_to_offset(address);
    }

    /// Get the address of the variable pointing to the static root array.
    fn static_root_array_variable_address(&self) -> usize {
        self.offset_to_address(self.static_root_array_variable_offset)
    }

    /// Get the address of the variable pointing to the continuation table.
    fn continuation_table_variable_address(&self) -> usize {
        self.offset_to_address(self.continuation_table_variable_offset)
    }

    /// Get the address of the region0 pointer
    #[incremental_gc]
    fn region0_pointer_address(&self) -> usize {
        self.offset_to_address(self.region0_pointer_variable_offset)
    }

    #[classical_persistence]
    pub fn new(
        map: &[(ObjectIdx, Vec<ObjectIdx>)],
        roots: &[ObjectIdx],
        continuation_table: &[ObjectIdx],
        gc: GC,
        _free_space: usize,
    ) -> MotokoHeapInner {
        self::classical::new_heap(map, roots, continuation_table, gc)
    }

    #[enhanced_orthogonal_persistence]
    pub fn new(
        map: &[(ObjectIdx, Vec<ObjectIdx>)],
        roots: &[ObjectIdx],
        continuation_table: &[ObjectIdx],
        _gc: GC,
        free_space: usize,
    ) -> MotokoHeapInner {
        self::enhanced::new_heap(map, roots, continuation_table, free_space)
    }

    #[non_incremental_gc]
    unsafe fn alloc_words(&mut self, n: Words<usize>) -> Value {
        self.linear_alloc_words(n)
    }

    #[incremental_gc]
    unsafe fn alloc_words(&mut self, n: Words<usize>) -> Value {
        let mut dummy_memory = DummyMemory {};
        let result =
            motoko_rts::gc::incremental::get_partitioned_heap().allocate(&mut dummy_memory, n);
        self.set_heap_ptr_address(result.get_ptr()); // realign on partition changes

        self.linear_alloc_words(n)
    }

    unsafe fn linear_alloc_words(&mut self, n: Words<usize>) -> Value {
        // Update heap pointer
        let old_hp = self.heap_ptr_address();
        let new_hp = old_hp + n.to_bytes().as_usize();
        self.heap_ptr_offset = new_hp - self.heap.as_ptr() as usize;

        // Grow memory if needed
        self.grow_memory(new_hp as usize);
        Value::from_ptr(old_hp)
    }

    unsafe fn grow_memory(&mut self, ptr: usize) {
        let heap_end = self.heap.as_ptr() as usize + self.heap.len();
        if ptr > heap_end {
            // We don't allow growing memory in tests, allocate large enough for the test
            panic!(
                "MotokoHeap::grow_memory called: heap_end={:#x}, grow_memory argument={:#x}",
                heap_end, ptr
            );
        }
    }
}

#[incremental_gc]
struct DummyMemory {}

#[incremental_gc]
impl Memory for DummyMemory {
    unsafe fn alloc_words(&mut self, _n: Words<usize>) -> Value {
        unreachable!()
    }

    unsafe fn grow_memory(&mut self, _ptr: usize) {}
}

/// Compute the size of the heap to be allocated for the GC test.
#[non_incremental_gc]
fn heap_size_for_gc(gc: GC, total_heap_size_bytes: usize, n_objects: usize) -> usize {
    use super::utils::WORD_SIZE;
    match gc {
        GC::Copying => 2 * total_heap_size_bytes,
        GC::MarkCompact => {
            let bitmap_size_bytes = {
                let heap_bytes = Bytes(total_heap_size_bytes);
                // `...to_words().to_bytes()` below effectively rounds up heap size to word size
                // then gets the bytes
                let heap_words = heap_bytes.to_words();
                let mark_bit_bytes = heap_words.to_bytes();

                // The bitmap implementation rounds up to 64-bits to be able to read as many
                // bits as possible in one instruction and potentially skip 64 words in the
                // heap with single 64-bit comparison
                (((mark_bit_bytes.as_usize() + 7) / 8) * 8)
                    + size_of::<Blob>().to_bytes().as_usize()
            };
            // In the worst case the entire heap will be pushed to the mark stack, but in tests
            // we limit the size
            let mark_stack_words = n_objects.clamp(
                motoko_rts::gc::mark_compact::mark_stack::INIT_STACK_SIZE.as_usize(),
                super::utils::MAX_MARK_STACK_SIZE,
            ) + size_of::<Blob>().as_usize();

            total_heap_size_bytes + bitmap_size_bytes as usize + (mark_stack_words * WORD_SIZE)
        }
        GC::Generational => {
            const ROUNDS: usize = 3;
            const REMEMBERED_SET_MAXIMUM_SIZE: usize = 1024 * 1024 * WORD_SIZE;
            let size = heap_size_for_gc(GC::MarkCompact, total_heap_size_bytes, n_objects);
            size + ROUNDS * REMEMBERED_SET_MAXIMUM_SIZE
        }
    }
}

#[incremental_gc]
fn heap_size_for_gc(gc: GC, _total_heap_size_bytes: usize, _n_objects: usize) -> usize {
    match gc {
        GC::Incremental => 3 * motoko_rts::gc::incremental::partitioned_heap::PARTITION_SIZE,
    }
}
