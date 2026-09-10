//! Heuristics for GC scheduling.
//! Tuned for 64-bit main memory with unknown IC memory capacity.
//!
//! Distinction of three scheduling modes:
//! * Critical: The heap exceeds 80% of the memory capacity (using capacity probing):
//!   - Schedule a GC start with a frequency of 1% of the heap growth.
//! * Medium: The heap exceeds 50% of the memory capacity:
//!   - Schedule a GC start with a frequency of 35% of the heap growth.
//! * Low: The heap is below 50% of the memory capacity.
//!   - Schedule a GC start with a frequency of 65% of the heap growth.
//!
//! A heuristics for capacity probing is used to determine the minimum amount of memory capacity.
//! This is necessary because the IC does not provide runtime information about the implemented
//! Wasm memory capacity in 64-bit. This capacity may also increase over time with newer IC versions.

use crate::gc::incremental::{get_incremental_gc_state, partitioned_heap::PARTITION_SIZE};
use crate::memory::ic::partitioned_memory::{get_heap_size, get_total_allocations};
use crate::types::Bytes;
use motoko_rts_macros::enhanced_orthogonal_persistence;

struct HeapThresholds {
    critical_heap_limit: Bytes<usize>,
    medium_heap_limit: Bytes<usize>,
}

#[enhanced_orthogonal_persistence]
impl HeapThresholds {
    /// Heuristics: Determine the threshold values of the heap size to schedule a new GC start.
    /// Note:
    /// As the maximum Wasm memory capacity supported by the IC is not known and can be increased
    /// over time, we probe the memory extension when the free space seems to be critically low.
    /// This is to avoid unnecessary heavy GC scheduling when approaching a supposed memory limit
    /// that can actually be extended.
    unsafe fn get() -> HeapThresholds {
        use crate::memory::ic::enhanced_memory::{minimum_memory_capacity, probe_wasm_memory};

        let thresholds = HeapThresholds::get_without_probing();
        let heap_size = get_heap_size();
        // Only if the heap size seems to be critically low, try to expand the Wasm memory beyond the
        // assumed current available memory.
        if heap_size > thresholds.critical_heap_limit
            && probe_wasm_memory(minimum_memory_capacity() + Bytes(1))
        {
            // Obtain the extended heap thresholds.
            HeapThresholds::get_without_probing()
        } else {
            thresholds
        }
    }

    /// Obtain the heap thresholds without memory probing:
    /// The critical limit is 80% of the currently known memory size.
    /// The medium limit is 50% of the currently known memory size.
    unsafe fn get_without_probing() -> HeapThresholds {
        use crate::memory::ic::enhanced_memory::minimum_memory_capacity;

        let available_memory = minimum_memory_capacity().as_usize();
        let critical_heap_limit = Bytes(available_memory / 10 * 8); // 80%
        let medium_heap_limit: Bytes<usize> = Bytes(available_memory / 2); // 50%
        HeapThresholds {
            critical_heap_limit,
            medium_heap_limit,
        }
    }
}

/// Determine whether a new GC run should be started based on the heap growth since
/// the last GC completion.
pub unsafe fn should_start_gc() -> bool {
    let heap_size = get_heap_size();

    const CRITICAL_GROWTH_THRESHOLD: f64 = 0.01;
    const MEDIUM_GROWTH_THRESHOLD: f64 = 0.35;
    const LOW_GROWTH_THRESHOLD: f64 = 0.65;

    let heap_thresholds = HeapThresholds::get();
    let growth_threshold = if heap_size > heap_thresholds.critical_heap_limit {
        CRITICAL_GROWTH_THRESHOLD
    } else if heap_size > heap_thresholds.medium_heap_limit {
        MEDIUM_GROWTH_THRESHOLD
    } else {
        LOW_GROWTH_THRESHOLD
    };

    let current_allocations = get_total_allocations();
    let state = get_incremental_gc_state();
    debug_assert!(current_allocations >= state.statistics.last_allocations);
    let absolute_growth = current_allocations - state.statistics.last_allocations;
    let relative_growth = absolute_growth.0 as f64 / heap_size.as_usize() as f64;
    relative_growth > growth_threshold && heap_size.as_usize() >= PARTITION_SIZE
}
