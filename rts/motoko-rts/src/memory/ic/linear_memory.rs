use super::{IcMemory, Memory, get_aligned_heap_base};
use crate::{memory::GENERAL_MEMORY_RESERVE, types::*};

/// Amount of garbage collected so far.
pub(crate) static mut RECLAIMED: Bytes<u64> = Bytes(0);

/// Maximum live data retained in a GC.
pub(crate) static mut MAX_LIVE: Bytes<usize> = Bytes(0);

// Heap pointer (skewed)
unsafe extern "C" {
    fn setHP(new_hp: usize);
    fn getHP() -> usize;
}

pub(crate) unsafe fn set_hp_unskewed(new_hp: usize) {
    setHP(skew(new_hp))
}
pub(crate) unsafe fn get_hp_unskewed() -> usize {
    unskew(getHP())
}

/// Heap pointer after last GC
pub(crate) static mut LAST_HP: usize = 0;

pub(crate) unsafe fn initialize() {
    LAST_HP = get_aligned_heap_base();
    set_hp_unskewed(LAST_HP);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn get_reclaimed() -> Bytes<u64> {
    RECLAIMED
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_total_allocations() -> Bytes<u64> {
    Bytes(get_heap_size().as_usize() as u64) + get_reclaimed()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_heap_size() -> Bytes<usize> {
    Bytes(get_hp_unskewed() - get_aligned_heap_base())
}

#[unsafe(no_mangle)]
unsafe extern "C" fn get_max_live_size() -> Bytes<usize> {
    MAX_LIVE
}

impl Memory for IcMemory {
    #[inline]
    unsafe fn alloc_words(&mut self, n: Words<usize>) -> Value {
        let bytes = n.to_bytes();
        let delta = bytes.as_usize() as u64;

        // Update heap pointer
        let old_hp = getHP() as u64;
        let new_hp = old_hp + delta;

        // Grow memory if needed
        if new_hp > ((super::wasm_memory_size() as u64) << 16) {
            linear_grow_memory(new_hp);
        }

        debug_assert!(new_hp <= core::usize::MAX as u64);
        setHP(new_hp as usize);

        Value::from_raw(old_hp as usize)
    }

    #[inline(never)]
    unsafe fn grow_memory(&mut self, ptr: usize) {
        linear_grow_memory(ptr as u64);
    }
}

unsafe fn linear_grow_memory(ptr: u64) {
    super::grow_memory(ptr as u64, GENERAL_MEMORY_RESERVE);
}
