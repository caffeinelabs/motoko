use motoko_rts_macros::incremental_gc;

use crate::constants::WASM_PAGE_SIZE;
use crate::memory::ic::keep_memory_reserve;
use crate::rts_trap_with;
use core::arch::wasm32;

// Provided by generated code
unsafe extern "C" {
    fn get_heap_base() -> usize;
}

pub(crate) unsafe fn get_aligned_heap_base() -> usize {
    // align to 32 bytes
    ((get_heap_base() + 31) / 32) * 32
}

/// Number of Wasm pages in main memory.
pub fn wasm_memory_size() -> usize {
    wasm32::memory_size(0)
}

/// Page allocation. Ensures that the memory up to, but excluding, the given pointer is allocated.
/// Ensure a memory reserve of at least one Wasm page depending on the canister state.
/// `memory_reserve`: A memory reserve in bytes ensured during update and initialization calls.
/// The reserve can be used by queries and upgrade calls. The reserve may vary depending on the GC
/// and the phase of the GC.
pub unsafe fn grow_memory(ptr: u64, memory_reserve: usize) {
    const LAST_PAGE_LIMIT: usize = usize::MAX - WASM_PAGE_SIZE.as_usize() + 1;
    // Spare a memory reserve during update and initialization calls for use by queries and upgrades.
    let memory_reserve = if keep_memory_reserve() {
        memory_reserve
    } else {
        0
    };
    // In any case, the last Wasm memory page is reserved to guard against shadow call stack overflows.
    // This call stack is used both by the Rust runtime system implementation and by the compiler backend,
    // see module `Stack` in `compile.ml`. This requires function activation frames to be less than the
    // Wasm page size.
    debug_assert!(memory_reserve <= LAST_PAGE_LIMIT);
    let limit = LAST_PAGE_LIMIT - memory_reserve;
    // The pointer is one byte larger than the memory size to be allocated, see the comment above.
    if ptr > limit as u64 {
        rts_trap_with("Cannot grow memory")
    };
    let page_size = WASM_PAGE_SIZE.as_usize() as u64;
    let total_pages_needed = ((ptr + page_size - 1) / page_size) as usize;
    let current_pages = wasm_memory_size();
    if total_pages_needed > current_pages {
        if wasm32::memory_grow(0, total_pages_needed - current_pages) == core::usize::MAX {
            // replica signals that there is not enough memory
            rts_trap_with("Cannot grow memory");
        }
    }
}

/// Grow memory without memory reserve (except the last WASM page).
/// Used during RTS initialization.
#[incremental_gc]
pub(crate) unsafe fn allocate_wasm_memory(memory_size: crate::types::Bytes<usize>) {
    grow_memory(memory_size.as_usize() as u64, WASM_PAGE_SIZE.as_usize());
}
