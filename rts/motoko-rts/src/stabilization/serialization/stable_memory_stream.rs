//! Streamed read/write access to stable memory.
//! Supporting Cheney's to-space in stable memory.

use core::mem::{MaybeUninit, size_of};

use crate::{
    stabilization::{
        deserialization::stable_memory_access::StableMemoryAccess, grant_stable_space,
    },
    stable_mem::{ic0_stable64_read, ic0_stable64_write},
};

/// Streamed reader/writer on stable memory.
/// Used for the to-space during stabilization.
///
/// The memory supports two location-independent streams:
/// * Streamed reading and updating, used for scanning and patching pointers.
/// * Streamed writing, used for allocating new objects.
pub struct StableMemoryStream {
    /// The pointers used in the serialized stable memory layout are
    /// relative to this start address of the to-space.
    base_address: u64,
    /// Used for reading and updating.
    scan_address: u64,
    /// Used for writing.
    free_address: u64,
}

pub trait ScanStream {
    // Determines whether the stream has reached the end.
    fn scan_completed(&self) -> bool;
    // Read a value from the stream.
    fn read<T>(&mut self) -> T;
    // Read raw data from the stream.
    fn raw_read(&mut self, data_address: usize, length: usize);
    // Skip data in the stream.
    fn skip(&mut self, length: usize);
    // Overwrite the value right before the stream position.
    fn update<T>(&mut self, value: &T);
    // Overwrite raw data right before the stream position.
    fn raw_update(&mut self, data_address: usize, length: usize);
}

pub trait WriteStream {
    // Append a value at the stream end.
    fn write<T>(&mut self, value: &T);
    // Append raw data at the stream end.
    fn raw_write(&mut self, data_address: usize, length: usize);
}

impl StableMemoryStream {
    pub fn open(start_address: u64) -> StableMemoryStream {
        StableMemoryStream {
            base_address: start_address,
            scan_address: start_address,
            free_address: start_address,
        }
    }

    pub fn close(&mut self) {
        debug_assert!(self.scan_address <= self.free_address);
    }

    /// Start address of the serialized data in stable memory.
    pub fn base_address(&self) -> u64 {
        self.base_address
    }

    /// Size of this memory space. Also serves to determine the addresses of a subsequently
    /// written object in the serialized stable memory format.
    pub fn written_length(&self) -> u64 {
        debug_assert!(self.base_address <= self.free_address);
        self.free_address - self.base_address
    }

    pub fn scanned_length(&self) -> u64 {
        debug_assert!(self.base_address <= self.scan_address);
        self.scan_address - self.base_address
    }

    pub fn read_preceding<T>(&self, offset: u64) -> T {
        let length = self.free_address - self.base_address;
        let access = StableMemoryAccess::open(self.base_address, length);
        access.read::<T>(offset)
    }
}

impl ScanStream for StableMemoryStream {
    fn scan_completed(&self) -> bool {
        debug_assert!(self.scan_address <= self.free_address);
        self.scan_address == self.free_address
    }

    fn read<T>(&mut self) -> T {
        let length = size_of::<T>();
        let mut value = unsafe { MaybeUninit::<T>::uninit().assume_init() };
        let value_address = &mut value as *mut T as usize;
        self.raw_read(value_address, length);
        value
    }

    fn raw_read(&mut self, data_address: usize, length: usize) {
        debug_assert!(self.scan_address + length as u64 <= self.free_address);
        unsafe {
            ic0_stable64_read(data_address as u64, self.scan_address, length as u64);
        }
        self.scan_address += length as u64;
    }

    fn skip(&mut self, length: usize) {
        debug_assert!(self.scan_address + length as u64 <= self.free_address);
        self.scan_address += length as u64;
    }

    fn update<T>(&mut self, value: &T) {
        let length = size_of::<T>();
        let value_address = value as *const T as usize;
        self.raw_update(value_address, length);
    }

    fn raw_update(&mut self, data_address: usize, length: usize) {
        debug_assert!(length as u64 <= self.scan_address);
        unsafe {
            ic0_stable64_write(
                self.scan_address - length as u64,
                data_address as u64,
                length as u64,
            );
        }
    }
}

impl WriteStream for StableMemoryStream {
    fn write<T>(&mut self, value: &T) {
        let length = size_of::<T>();
        let value_address = value as *const T as usize;
        self.raw_write(value_address, length);
    }

    fn raw_write(&mut self, data_address: usize, length: usize) {
        unsafe {
            grant_stable_space(self.free_address + length as u64);
            ic0_stable64_write(self.free_address, data_address as u64, length as u64);
        }
        self.free_address += length as u64;
    }
}
