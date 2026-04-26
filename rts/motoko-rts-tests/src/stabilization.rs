mod layout;
mod reader_writer;
mod stable_bigints;
mod stable_memory;

use crate::{
    gc::{
        check_dynamic_heap, heap::MotokoHeap, random::generate, utils::GC, utils::WORD_SIZE,
        CheckMode, TestHeap,
    },
    memory::TestMemory,
    stabilization::stable_memory::clear_stable_memory,
};
use motoko_rts::{
    memory::{alloc_array, Memory},
    stabilization::{
        deserialization::Deserialization,
        graph_copy::GraphCopy,
        layout::StableValue,
        serialization::{Serialization, SerializationRoots},
    },
    types::{Value, Words, TAG_ARRAY_M},
};
use oorandom::Rand32;

pub unsafe fn test() {
    println!("Testing stabilization ...");
    layout::test();
    stable_bigints::test();
    reader_writer::test();
    test_stabilization();
    reset_memory();
}

#[no_mangle]
pub fn moc_stabilization_instruction_limit() -> u64 {
    u64::MAX
}

#[no_mangle]
pub fn moc_stable_memory_access_limit() -> u64 {
    u64::MAX
}

#[no_mangle]
pub fn ic0_performance_counter(_counter: u32) -> u64 {
    0
}

// This is only called for graph copy increment limit testing.
// Not used during RTS testing.
#[no_mangle]
pub fn deserialized_size() -> usize {
    0
}

fn reset_gc(heap_base_address: usize) {
    use motoko_rts::gc::incremental::{set_incremental_gc_state, IncrementalGC};

    unsafe {
        let state = IncrementalGC::<MotokoHeap>::initial_gc_state(heap_base_address);
        set_incremental_gc_state(Some(state));
    }
}

fn clear_heap(heap: &mut MotokoHeap) {
    reset_gc(heap.heap_base_address());
}

fn reset_memory() {
    clear_stable_memory();
    reset_main_memory();
}

fn reset_main_memory() {
    reset_gc(0);
}

struct RandomHeap {
    descriptor: TestHeap,
    memory: MotokoHeap,
}

impl RandomHeap {
    fn clear(&mut self) {
        clear_heap(&mut self.memory);
    }

    fn set_new_root(&mut self, stable_root: Value) {
        self.clear_continuation_table();
        self.reset_root_array(stable_root);
        // Set the stable root as sole static root pointer.
        self.reset_descriptor();
    }

    fn clear_continuation_table(&mut self) {
        let table_pointer = self.memory.continuation_table_variable_address() as *mut Value;
        unsafe {
            *table_pointer = alloc_array(&mut self.memory, TAG_ARRAY_M, 0);
        }
    }

    fn reset_root_array(&mut self, stable_root: Value) {
        let root_array_pointer = self.memory.static_root_array_variable_address() as *mut Value;
        unsafe {
            *root_array_pointer = stable_root;
        }
    }

    fn reset_descriptor(&mut self) {
        self.descriptor.continuation_table.clear();
    }

    fn old_stable_root(&self) -> Value {
        let root_array_pointer = self.memory.static_root_array_variable_address() as *mut Value;
        unsafe { *root_array_pointer }
    }

    fn check_heap(&self) {
        check_dynamic_heap(
            CheckMode::Stabilzation,
            &self.descriptor.heap,
            &self.descriptor.roots,
            &self.descriptor.continuation_table,
            &self.memory.heap().as_ref(),
            self.memory.heap_base_offset(),
            self.memory.heap_ptr_offset(),
            self.memory.static_root_array_variable_offset(),
            self.memory.continuation_table_variable_offset(),
            self.memory.region0_pointer_variable_offset(),
        )
    }
}

fn random_heap(random: &mut Rand32, max_objects: usize) -> RandomHeap {
    let descriptor = generate(random.rand_u32() as u64, max_objects);
    let pointers: usize = descriptor
        .heap
        .iter()
        .map(|(_, references)| references.len() + 1)
        .sum();
    let memory = descriptor.build(GC::Incremental, pointers * WORD_SIZE as usize);
    RandomHeap { descriptor, memory }
}

fn test_stabilization() {
    test_stabilization_small();
    test_stabilization_20k();
}

/// Derive a seed from the git hash, varying tests across commits.
fn git_seed() -> u64 {
    let hash = env!("RTS_TEST_GIT_HASH");
    hash.bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
}

pub fn test_stabilization_small() {
    let seed = git_seed();
    println!(
        "  Testing serialization and deserialization (small, seed={}) ...",
        seed
    );
    let mut random = Rand32::new(seed);
    test_serialization_deserialization(&mut random, 100, 0);
    test_serialization_deserialization(&mut random, 1000, 200);
    test_serialization_deserialization(&mut random, 10_000, 5_000);
}

pub fn test_stabilization_20k() {
    println!("  Testing serialization and deserialization (20k, seed=4711) ...");
    let mut random = Rand32::new(4711);
    test_serialization_deserialization(&mut random, 20_000, 7_000);
}

fn test_serialization_deserialization(random: &mut Rand32, max_objects: usize, stable_start: u64) {
    println!("    Testing with {max_objects} objects");
    clear_stable_memory();
    let mut heap = random_heap(random, max_objects);
    let old_stable_root = heap.old_stable_root();

    let stable_size = serialize(old_stable_root, stable_start);

    heap.clear();

    let stable_root = deserialize(&mut heap.memory, stable_start, stable_size);

    heap.set_new_root(stable_root);
    heap.check_heap();
}

fn serialize(old_stable_root: Value, stable_start: u64) -> u64 {
    let mut memory = TestMemory::new(Words(0));
    let roots = SerializationRoots {
        actor: old_stable_root,
        dedup_table: Value::from_raw(0),
        migrations_list: Value::from_raw(0),
    };
    let mut serialization = Serialization::start(&mut memory, roots, stable_start);
    serialization.copy_increment(&mut memory);
    assert!(serialization.is_completed());
    serialization.serialized_data_length()
}

fn deserialize<M: Memory>(mem: &mut M, stable_start: u64, stable_size: u64) -> Value {
    let mut deserialization = Deserialization::start(
        mem,
        stable_start,
        stable_size,
        StableValue::from_raw(0),
        StableValue::from_raw(0),
    );
    deserialization.copy_increment(mem);
    assert!(deserialization.is_completed());
    deserialization.get_stable_root()
}
