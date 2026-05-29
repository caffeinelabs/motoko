#[cfg(feature = "ic")]
use motoko_rts_macros::enhanced_orthogonal_persistence;

pub const PAGE_SIZE: u64 = 64 * 1024;

unsafe extern "C" {
    // physical ic0_stable64 operations re-exported by moc
    pub fn ic0_stable64_write(offset: u64, src: u64, size: u64);
    pub fn ic0_stable64_read(dst: u64, offset: u64, size: u64);
    /// Physical memory size.
    pub fn ic0_stable64_size() -> u64;
    /// Grow the physiscal memory by ignoring the compiler-specified stable memory limit.
    pub fn ic0_stable64_grow(additional_pages: u64) -> u64;

    // (virtual) stable_mem operations implemented by moc
    #[cfg(feature = "ic")]
    pub fn moc_stable_mem_get_version() -> usize;
    #[cfg(feature = "ic")]
    pub fn moc_stable_mem_set_version(version: usize);
    /// Virtual memory size.
    #[cfg(feature = "ic")]
    pub fn moc_stable_mem_get_size() -> u64;
    /// Initialize the virtual memory size.
    #[cfg(feature = "ic")]
    pub fn moc_stable_mem_set_size(pages: u64);
    /// Grow the virtual memory by respecting the compiler-specified virtual memory limit.
    #[cfg(feature = "ic")]
    pub fn moc_stable_mem_grow(additional_pages: u64) -> u64;
}

#[cfg(feature = "ic")]
pub fn get_version() -> usize {
    unsafe { moc_stable_mem_get_version() }
}

#[cfg(feature = "ic")]
pub fn set_version(version: usize) {
    unsafe { moc_stable_mem_set_version(version) }
}

/// Virtual memory size.
#[cfg(feature = "ic")]
pub fn size() -> u64 {
    // SAFETY: This is safe because of the ic0 api guarantees.
    unsafe { moc_stable_mem_get_size() }
}

/// Grow the virtual memory by respecting the compiler-specified stable memory limit.
#[cfg(feature = "ic")]
pub fn grow(pages: u64) -> u64 {
    // SAFETY: This is safe because of the ic0 api guarantees.
    unsafe { moc_stable_mem_grow(pages) }
}

#[cfg(feature = "ic")]
pub fn read(offset: u64, dst: &mut [u8]) {
    // SAFETY: This is safe because of the ic0 api guarantees.
    unsafe { ic0_stable64_read(dst.as_ptr() as u64, offset, dst.len() as u64) }
}

#[cfg(feature = "ic")]
pub fn write(offset: u64, src: &[u8]) {
    // SAFETY: This is safe because of the ic0 api guarantees.
    unsafe { ic0_stable64_write(offset, src.as_ptr() as u64, src.len() as u64) }
}

// Little endian.
#[cfg(feature = "ic")]
pub fn read_u8(offset: u64) -> u8 {
    let mut res: [u8; 1] = [0; 1];
    read(offset, &mut res);
    core::primitive::u8::from_le_bytes(res)
}

// Little endian.
#[cfg(feature = "ic")]
pub fn write_u8(offset: u64, value: u8) {
    write(offset, &core::primitive::u8::to_le_bytes(value));
}

// Little endian.
#[cfg(feature = "ic")]
pub fn read_u16(offset: u64) -> u16 {
    let mut res: [u8; 2] = [0; 2];
    read(offset, &mut res);
    core::primitive::u16::from_le_bytes(res)
}

// Little endian.
#[cfg(feature = "ic")]
pub fn write_u16(offset: u64, value: u16) {
    write(offset, &core::primitive::u16::to_le_bytes(value));
}

// Little endian.
#[cfg(feature = "ic")]
pub fn read_u32(offset: u64) -> u32 {
    let mut res: [u8; 4] = [0; 4];
    read(offset, &mut res);
    core::primitive::u32::from_le_bytes(res)
}

// Little endian.
#[cfg(feature = "ic")]
pub fn write_u32(offset: u64, value: u32) {
    write(offset, &core::primitive::u32::to_le_bytes(value));
}

// Little endian.
#[cfg(feature = "ic")]
pub fn read_u64(offset: u64) -> u64 {
    let mut res: [u8; 8] = [0; 8];
    read(offset, &mut res);
    core::primitive::u64::from_le_bytes(res)
}

// Little endian.
#[cfg(feature = "ic")]
pub fn write_u64(offset: u64, n: u64) {
    write(offset, &core::primitive::u64::to_le_bytes(n));
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
#[enhanced_orthogonal_persistence]
pub extern "C" fn read_persistence_version() -> usize {
    use crate::region::{LEGACY_VERSION_NO_STABLE_MEMORY, VERSION_STABLE_HEAP_NO_REGIONS};

    let physical_pages = unsafe { ic0_stable64_size() };
    if physical_pages == 0 {
        // No stable memory -> Use the new default: Enhanced orthogonal persistence.
        return VERSION_STABLE_HEAP_NO_REGIONS;
    }
    if read_u32(0) != 0 {
        // Old stabilization with no experimental stable memory and no regions.
        // It stores non-zero marker at address 0 -> Legacy version 0.
        return LEGACY_VERSION_NO_STABLE_MEMORY;
    }
    // Note: Do not use `types::size_of()` as it rounds to 64-bit words.
    let address = physical_pages * PAGE_SIZE - core::mem::size_of::<u32>() as u64;
    let version = read_u32(address);
    version as usize
}
