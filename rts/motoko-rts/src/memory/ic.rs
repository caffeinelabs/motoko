// This module is only enabled when compiling the RTS for IC or WASI.

#[non_incremental_gc]
pub mod linear_memory;

#[incremental_gc]
pub mod partitioned_memory;

#[classical_persistence]
pub mod classical_memory;

#[enhanced_orthogonal_persistence]
pub mod enhanced_memory;

use motoko_rts_macros::{
    classical_persistence, enhanced_orthogonal_persistence, incremental_gc, non_incremental_gc,
};

use super::Memory;

// Provided by generated code
unsafe extern "C" {
    #[classical_persistence]
    pub(crate) fn get_static_roots() -> crate::types::Value;

    fn keep_memory_reserve() -> bool;
}

#[classical_persistence]
pub(crate) unsafe fn get_aligned_heap_base() -> usize {
    classical_memory::get_aligned_heap_base()
}

#[enhanced_orthogonal_persistence]
pub(crate) unsafe fn get_aligned_heap_base() -> usize {
    enhanced_memory::get_aligned_heap_base()
}

/// Provides a `Memory` implementation, to be used in functions compiled for IC or WASI. The
/// `Memory` implementation allocates in Wasm heap with Wasm `memory.grow` instruction.
pub struct IcMemory;

/// Number of Wasm pages in main memory.
#[classical_persistence]
#[non_incremental_gc]
fn wasm_memory_size() -> usize {
    classical_memory::wasm_memory_size()
}

/// Page allocation. Ensures that the memory up to, but excluding, the given pointer is allocated.
/// Ensure a memory reserve of at least one Wasm page depending on the canister state.
/// `memory_reserve`: A memory reserve in bytes ensured during update and initialization calls.
/// The reserve can be used by queries and upgrade calls.
#[classical_persistence]
unsafe fn grow_memory(ptr: u64, memory_reserve: usize) {
    classical_memory::grow_memory(ptr, memory_reserve);
}

/// Page allocation. Ensures that the memory up to, but excluding, the given pointer is allocated.
/// Ensure a memory reserve of at least one Wasm page depending on the canister state.
/// `memory_reserve`: A memory reserve in bytes ensured during update and initialization calls.
/// The reserve can be used by queries and upgrade calls.
#[enhanced_orthogonal_persistence]
unsafe fn grow_memory(ptr: u64, memory_reserve: usize) {
    use core::mem::size_of;
    // Statically assert the safe conversion from `u64` to `usize`.
    const _: () = assert!(size_of::<u64>() == size_of::<usize>());
    enhanced_memory::grow_memory(ptr as usize, memory_reserve);
}

/// Grow memory without memory reserve (except the last WASM page).
/// Used during RTS initialization.
#[classical_persistence]
#[incremental_gc]
pub(crate) unsafe fn allocate_wasm_memory(memory_size: crate::types::Bytes<usize>) {
    classical_memory::allocate_wasm_memory(memory_size);
}

/// Grow memory without memory reserve (except the last WASM page).
/// Used during RTS initialization.
#[enhanced_orthogonal_persistence]
pub(crate) unsafe fn allocate_wasm_memory(memory_size: crate::types::Bytes<usize>) {
    enhanced_memory::allocate_wasm_memory(memory_size);
}
