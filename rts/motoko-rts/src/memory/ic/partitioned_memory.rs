use super::{IcMemory, Memory};
use crate::types::*;

#[unsafe(no_mangle)]
unsafe extern "C" fn get_reclaimed() -> Bytes<u64> {
    crate::gc::incremental::get_partitioned_heap().reclaimed_size()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_total_allocations() -> Bytes<u64> {
    crate::gc::incremental::get_partitioned_heap().total_allocated_size()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_max_live_size() -> Bytes<usize> {
    crate::gc::incremental::get_max_live_size()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_heap_size() -> Bytes<usize> {
    crate::gc::incremental::get_partitioned_heap().occupied_size()
}

impl Memory for IcMemory {
    #[inline]
    unsafe fn alloc_words(&mut self, n: Words<usize>) -> Value {
        crate::gc::incremental::get_partitioned_heap().allocate(self, n)
    }

    #[inline(never)]
    unsafe fn grow_memory(&mut self, ptr: usize) {
        let memory_reserve = crate::gc::incremental::memory_reserve();
        super::grow_memory(ptr as u64, memory_reserve);
    }
}
