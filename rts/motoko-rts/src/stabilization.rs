//! Graph-copy-based stabilzation on upgrades, serializing the entire stable object graph into
//! stable memory by using a defined long-term stable storage format.
//!
//! This is to support potentially radical changes of the persistent main memory layout, e.g.
//! introducing a new GC or rearranging persistent metadata. This also relies on precise value
//! tagging to allow more advanced changes that require value metadata, e.g. specializing arrays for
//! small element types or even downgrading to 32-bit heap layouts (provided that the amount of live
//! data fits into a 32-bit memory).
//!
//! A memory compatibility check similar to enhanced orthogonal persistence has to be performed.
//! For this purpose, the type table of the serialized object graph is also stored in stable memory
//! and on upgrade, compared to the new program version.
//!
//! A versioned stable storage format even permits future evolutions of the graph copy algorithm.
//!  
//! See `GraphCopyStabilization.md` for the stable format specification and the employed algorithm.

pub mod deserialization;
pub mod graph_copy;
pub mod layout;
pub mod serialization;

#[cfg(feature = "ic")]
pub mod ic;

use core::cmp::min;

use crate::{
    constants::KB,
    rts_trap_with,
    stable_mem::{self, PAGE_SIZE, ic0_stable64_write},
};

use self::layout::StableValue;

unsafe extern "C" {
    pub fn moc_stabilization_instruction_limit() -> u64;
    pub fn moc_stable_memory_access_limit() -> u64;
    fn ic0_performance_counter(number: u32) -> u64;
}

// Dummy value used for non-stable objects that are potentially reachable from
// stable variable because of structural subtyping or `Any`-subtyping.
// Must be a non-skewed value such that the GC also ignores this value.
const DUMMY_VALUE: StableValue = StableValue::from_raw(0);

/// Note: This is called incrementally in smaller chunks by the destabilization
/// as it may otherwise exceed the instruction limit.
fn clear_stable_memory(start: u64, length: u64) {
    // Optimal point for the two cost functions, according to experimental measurements:
    // * Smaller chunks cause more stable API calls that incur costs.
    // * Larger chunks cause higher chunk zero-initialization costs.
    const CHUNK_SIZE: usize = KB;
    let empty_chunk = [0u8; CHUNK_SIZE];
    let mut position = start;
    let end = start + length;
    while position < end {
        let size = min(end - position, CHUNK_SIZE as u64);
        unsafe {
            ic0_stable64_write(position, &empty_chunk as *const u8 as u64, size);
        }
        position += size;
    }
}

fn grant_stable_space(byte_size: u64) {
    debug_assert!(byte_size < u64::MAX - PAGE_SIZE - 1);
    let required_pages = (byte_size + PAGE_SIZE - 1) / PAGE_SIZE;
    let available_pages = stable_memory_physical_size();
    if required_pages > available_pages {
        let additional_pages = required_pages - available_pages;
        debug_assert_ne!(additional_pages, u64::MAX);
        let result = stable_memory_physical_grow(additional_pages);
        if result == u64::MAX {
            rts_trap_with("Insufficient stable memory");
        }
    }
}

fn stable_memory_physical_size() -> u64 {
    unsafe { stable_mem::ic0_stable64_size() }
}

fn stable_memory_physical_grow(additional_pages: u64) -> u64 {
    unsafe { stable_mem::ic0_stable64_grow(additional_pages) }
}
