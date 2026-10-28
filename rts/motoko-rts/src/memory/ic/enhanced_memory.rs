use crate::constants::{GB, WASM_PAGE_SIZE};
use crate::memory::ic::keep_memory_reserve;
use crate::rts_trap_with;
use crate::types::Bytes;
use core::arch::wasm64;
use core::cmp::max;

pub(crate) unsafe fn get_aligned_heap_base() -> usize {
    crate::persistence::HEAP_START
}

/// Probe the memory capacity beyond this limit by pre-allocating the reserve.
/// This is necessary because there is no function to query the IC's main memory capacity for 64-bit.
/// It is assumed that the capacity is at least the 32-bit space of 4GB, minus the last reserved
/// Wasm page (for Rust call stack overflow detection, see `compile.ml`).
const GUARANTEED_MEMORY_CAPACITY: usize = 4 * GB - WASM_PAGE_SIZE.0;

/// Assumption (not correctness-critical): The IC offers main memory in multiple of 2 GB.
/// This helps to avoid overly frequent memory probing when the heap grows.
/// The capacity granularity only serves as a heuristics for GC scheduling.
const IC_MEMORY_CAPACITY_GRANULARITY: usize = 2 * GB;

/// Page allocation. Ensures that the memory up to, but excluding, the given pointer is allocated.
/// Ensure a memory reserve of at least one Wasm page depending on the canister state.
/// `memory_reserve`: A memory reserve in bytes ensured during update and initialization calls.
/// The reserve can be used by queries and upgrade calls.
pub(crate) unsafe fn grow_memory(ptr: usize, memory_reserve: usize) {
    debug_assert!(memory_reserve <= GUARANTEED_MEMORY_CAPACITY);
    let memory_demand =
        if keep_memory_reserve() && ptr > GUARANTEED_MEMORY_CAPACITY - memory_reserve {
            // Detect overflow of `ptr + memory_reserve`.
            if ptr > usize::MAX - memory_reserve {
                rts_trap_with("Cannot grow memory");
            }
            // The reserve will be pre-allocated as a way to check the main memory capacity.
            // As the reserve can be substantial, this is only done when memory demand has
            // grown beyond `GUARANTEED_MEMORY_CAPACITY`.
            ptr + memory_reserve
        } else {
            // Either no reserve is needed or there is enough guaranteed memory capacity for the reserve,
            // we can skip the pre-allocation of a reserve.
            ptr
        };
    allocate_wasm_memory(Bytes(memory_demand));
}

/// Supposed minimum memory capacity used for GC scheduling heuristics.
/// The result may increase after time. This is because the actual capacity is
/// not known upfront and can only derived by memory allocation probing.
/// Moreover, the IC may increase the canister main memory capacity in newer versions.
pub(crate) fn minimum_memory_capacity() -> Bytes<usize> {
    let allocated_memory = wasm64::memory_size(0) * WASM_PAGE_SIZE.as_usize();
    let unrounded_capacity = max(allocated_memory, GUARANTEED_MEMORY_CAPACITY);
    Bytes(unrounded_capacity).next_multiple_of(IC_MEMORY_CAPACITY_GRANULARITY)
}

/// Grow memory without memory reserve. Used during RTS initialization and by the ordinary
/// reserve-conscious memory-grow operation (`Memory::grow_memory`).
pub(crate) unsafe fn allocate_wasm_memory(memory_size: Bytes<usize>) {
    const LAST_PAGE_LIMIT: usize = 0xFFFF_FFFF_FFFF_0000;
    debug_assert_eq!(LAST_PAGE_LIMIT, usize::MAX - WASM_PAGE_SIZE.as_usize() + 1);
    // Never allocate the last page (shadow call stack overflow detection, see `compile.ml`).
    if memory_size.as_usize() > LAST_PAGE_LIMIT {
        rts_trap_with("Cannot grow memory");
    }
    if !probe_wasm_memory(memory_size) {
        // replica signals that there is not enough memory
        rts_trap_with("Cannot grow memory");
    }
}

/// Try to allocate an amount of Wasm memory by growing the Wasm memory space if needed.
/// Returns true if the memory has been allocated and is available.
/// Otherwise, it returns false if there does not exist enough Wasm memory.
pub(crate) unsafe fn probe_wasm_memory(memory_size: Bytes<usize>) -> bool {
    let page_size = WASM_PAGE_SIZE.as_usize();
    let total_pages_needed = (memory_size.as_usize() + page_size - 1) / page_size;
    let current_pages = wasm64::memory_size(0);
    if total_pages_needed > current_pages {
        wasm64::memory_grow(0, total_pages_needed - current_pages) != usize::MAX
    } else {
        true
    }
}
