use motoko_rts::{mem_utils::memcpy_bytes, types::Bytes};
use std::cell::RefCell;

const PAGE_SIZE: u64 = 64 * 1024;

thread_local! {
    static STABLE_MEMORY: RefCell<Vec<u8>> = RefCell::new(vec![]);
}

pub fn clear_stable_memory() {
    STABLE_MEMORY.with(|memory| {
        memory.borrow_mut().clear();
    })
}

#[unsafe(no_mangle)]
pub fn ic0_stable64_write(offset: u64, source: u64, size: u64) {
    STABLE_MEMORY.with(|memory| {
        assert!(offset + size <= memory.borrow().len() as u64);
        let destination = memory.borrow_mut().as_mut_ptr() as u64 + offset;
        unsafe {
            memcpy_bytes(destination as usize, source as usize, Bytes(size as usize));
        }
    });
}

#[unsafe(no_mangle)]
pub fn ic0_stable64_read(destination: u64, offset: u64, size: u64) {
    STABLE_MEMORY.with(|memory| {
        assert!(offset + size <= memory.borrow().len() as u64);
        let source = memory.borrow_mut().as_mut_ptr() as u64 + offset;
        unsafe {
            memcpy_bytes(destination as usize, source as usize, Bytes(size as usize));
        }
    });
}

#[unsafe(no_mangle)]
pub fn ic0_stable64_size() -> u64 {
    STABLE_MEMORY.with(|memory| memory.borrow().len()) as u64 / PAGE_SIZE
}

#[unsafe(no_mangle)]
pub fn ic0_stable64_grow(additional_pages: u64) -> u64 {
    for _ in 0..additional_pages * PAGE_SIZE {
        STABLE_MEMORY.with(|memory| memory.borrow_mut().push(0));
    }
    additional_pages
}
