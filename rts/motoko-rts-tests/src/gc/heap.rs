mod enhanced;

use super::utils::{GC, ObjectIdx};

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

    /// Get the address of the variable pointing to the static root array.
    fn static_root_array_variable_address(&self) -> usize {
        self.offset_to_address(self.static_root_array_variable_offset)
    }

    /// Get the address of the variable pointing to the continuation table.
    fn continuation_table_variable_address(&self) -> usize {
        self.offset_to_address(self.continuation_table_variable_offset)
    }

    /// Get the address of the region0 pointer
    fn region0_pointer_address(&self) -> usize {
        self.offset_to_address(self.region0_pointer_variable_offset)
    }

    pub fn new(
        map: &[(ObjectIdx, Vec<ObjectIdx>)],
        roots: &[ObjectIdx],
        continuation_table: &[ObjectIdx],
        _gc: GC,
        free_space: usize,
    ) -> MotokoHeapInner {
        self::enhanced::new_heap(map, roots, continuation_table, free_space)
    }

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

struct DummyMemory {}

impl Memory for DummyMemory {
    unsafe fn alloc_words(&mut self, _n: Words<usize>) -> Value {
        unreachable!()
    }

    unsafe fn grow_memory(&mut self, _ptr: usize) {}
}

/// Compute the size of the heap to be allocated for the GC test.
fn heap_size_for_gc(gc: GC, total_heap_size_bytes: usize, _n_objects: usize) -> usize {
    match gc {
        GC::Incremental => {
            let min_partitions = 3 * motoko_rts::gc::incremental::partitioned_heap::PARTITION_SIZE;
            // Ensure enough space for the actual heap content
            min_partitions.max(total_heap_size_bytes * 2)
        }
    }
}
