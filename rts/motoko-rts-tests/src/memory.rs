use motoko_rts::memory::Memory;
use motoko_rts::types::{Bytes, Value, Words};

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

    pub fn heap_base(&self) -> usize {
        self.heap.as_ptr() as usize
    }

    pub fn heap_end(&self) -> usize {
        self.heap_base() + self.heap.len()
    }

    pub fn heap_pointer(&self) -> usize {
        self.hp
    }

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

pub unsafe fn initialize_test_memory() -> TestMemory {
    use motoko_rts::gc::incremental::partitioned_heap::PARTITION_SIZE;
    use motoko_rts::gc::incremental::{IncrementalGC, set_incremental_gc_state};

    let memory = TestMemory::new(Bytes(PARTITION_SIZE).to_words());
    let state = IncrementalGC::<TestMemory>::initial_gc_state(0);
    set_incremental_gc_state(Some(state));
    memory
}

pub unsafe fn reset_test_memory() {
    use motoko_rts::gc::incremental::set_incremental_gc_state;

    set_incremental_gc_state(None);
}

pub unsafe fn test() {
    use motoko_rts::constants::MAX_BLOB_LENGTH;
    use motoko_rts::memory::{alloc_blob, blob_object_size};
    use motoko_rts::types::{Blob, TAG_BLOB_B, size_of};

    println!("Testing blob allocation ...");

    let header = size_of::<Blob>().as_usize();
    let object_size = |length: usize| blob_object_size(Bytes(length)).map(|words| words.as_usize());

    println!("  Testing blob object sizes");
    assert_eq!(object_size(0), Some(header));
    assert_eq!(object_size(1), Some(header + 1));
    assert_eq!(object_size(8), Some(header + 1));
    assert_eq!(object_size(9), Some(header + 2));
    assert_eq!(
        object_size(MAX_BLOB_LENGTH),
        Some(header + MAX_BLOB_LENGTH / 8)
    );

    println!("  Testing blob lengths that cannot be allocated");
    assert_eq!(object_size(MAX_BLOB_LENGTH + 1), None);
    assert_eq!(object_size(isize::MAX as usize), None);
    // Without the bound, these lengths make the object size wrap: the allocation
    // would be at most as large as the header.
    for below in 0..64 {
        assert_eq!(object_size(usize::MAX - below), None);
    }

    println!("  Testing alloc_blob");
    let mut mem = initialize_test_memory();
    let before = mem.heap_pointer();
    let blob = alloc_blob(&mut mem, TAG_BLOB_B, Bytes(13));
    assert_eq!(mem.heap_pointer() - before, (header + 2) * 8);
    assert_eq!(blob.as_blob().len(), Bytes(13));
    reset_test_memory();
}
