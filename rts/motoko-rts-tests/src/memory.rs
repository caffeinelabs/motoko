use motoko_rts::memory::Memory;
use motoko_rts::types::{Bytes, Value, Words};
use motoko_rts_macros::{incremental_gc, non_incremental_gc};

pub struct TestMemory {
    heap: Box<[u8]>,
    hp: usize,
}

impl TestMemory {
    pub fn new(size: Words<usize>) -> TestMemory {
        let bytes = size.to_bytes().as_usize();
        let heap = vec![0u8; bytes].into_boxed_slice();
        let hp = heap.as_ptr() as usize;
        TestMemory { heap, hp }
    }

    #[incremental_gc]
    pub fn heap_base(&self) -> usize {
        self.heap.as_ptr() as usize
    }

    #[incremental_gc]
    pub fn heap_end(&self) -> usize {
        self.heap_base() + self.heap.len()
    }

    #[incremental_gc]
    pub fn heap_pointer(&self) -> usize {
        self.hp
    }

    #[incremental_gc]
    pub fn set_heap_pointer(&mut self, heap_pointer: usize) {
        assert!(heap_pointer >= self.heap_base());
        assert!(heap_pointer <= self.heap_end());
        self.hp = heap_pointer;
    }
}

impl Memory for TestMemory {
    unsafe fn alloc_words(&mut self, n: Words<usize>) -> Value {
        let bytes = n.to_bytes();

        // Update heap pointer
        let old_hp = self.hp;
        let new_hp = old_hp + bytes.as_usize();
        self.hp = new_hp;

        // Grow memory if needed
        self.grow_memory(new_hp);

        Value::from_ptr(old_hp)
    }

    unsafe fn grow_memory(&mut self, ptr: usize) {
        let heap_end = self.heap.as_ptr() as usize + self.heap.len();
        if ptr as usize > heap_end {
            // We don't allow growing memory in tests, allocate large enough for the test
            panic!(
                "TestMemory::grow_memory called: heap_end={:#x}, grow_memory argument={:#x}",
                heap_end, ptr
            );
        }
    }
}

#[incremental_gc]
pub unsafe fn initialize_test_memory() -> TestMemory {
    use motoko_rts::gc::incremental::partitioned_heap::PARTITION_SIZE;
    use motoko_rts::gc::incremental::{IncrementalGC, set_incremental_gc_state};

    let memory = TestMemory::new(Bytes(PARTITION_SIZE).to_words());
    let state = IncrementalGC::<TestMemory>::initial_gc_state(0);
    set_incremental_gc_state(Some(state));
    memory
}

#[non_incremental_gc]
pub unsafe fn initialize_test_memory() -> TestMemory {
    const TEST_MEMORY_SIZE: usize = 32 * 1024 * 1024;
    TestMemory::new(Bytes(TEST_MEMORY_SIZE).to_words())
}

#[incremental_gc]
pub unsafe fn reset_test_memory() {
    use motoko_rts::gc::incremental::set_incremental_gc_state;

    set_incremental_gc_state(None);
}

#[non_incremental_gc]
pub unsafe fn reset_test_memory() {}
