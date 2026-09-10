// This module is only enabled when compiling the RTS for IC or WASI.

#[incremental_gc]
pub mod partitioned_memory;

#[enhanced_orthogonal_persistence]
pub mod enhanced_memory;

use motoko_rts_macros::{enhanced_orthogonal_persistence, incremental_gc};

use super::Memory;

// Provided by generated code
unsafe extern "C" {
    fn keep_memory_reserve() -> bool;
}

#[enhanced_orthogonal_persistence]
pub(crate) unsafe fn get_aligned_heap_base() -> usize {
    enhanced_memory::get_aligned_heap_base()
}

/// Provides a `Memory` implementation, to be used in functions compiled for IC or WASI. The
/// `Memory` implementation allocates in Wasm heap with Wasm `memory.grow` instruction.
pub struct IcMemory;

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
#[enhanced_orthogonal_persistence]
pub(crate) unsafe fn allocate_wasm_memory(memory_size: crate::types::Bytes<usize>) {
    enhanced_memory::allocate_wasm_memory(memory_size);
}
