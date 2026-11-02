use std::mem::size_of;

use byteorder::{LE, ReadBytesExt, WriteBytesExt};

/// A unique object index, used in heap descriptions.
///
/// These are written as scalar values in object payloads, so they can be at most `usize::BITS - 1` bits.
/// Larger values will cause test failure in `make_scalar` below.
pub type ObjectIdx = usize;

/// Same as RTS `WORD_SIZE`, but `usize`
pub const WORD_SIZE: usize = motoko_rts::constants::WORD_SIZE as usize;

/// Enum for the GC implementations. GC functions are generic so we can't put them into arrays or
/// other data types, we use this type instead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GC {
    Incremental,
}

pub static GC_IMPLS: [GC; 1] = [GC::Incremental];

/// Read a little-endian (Wasm) word from given offset
pub fn read_word(heap: &[u8], offset: usize) -> usize {
    assert_eq!(size_of::<usize>(), size_of::<u64>());
    (&heap[offset..]).read_u64::<LE>().unwrap() as usize
}

/// Write a little-endian (Wasm) word to given offset
pub fn write_word(heap: &mut [u8], offset: usize, word: usize) {
    assert_eq!(size_of::<usize>(), size_of::<u64>());
    (&mut heap[offset..]).write_u64::<LE>(word as u64).unwrap()
}

/// Make a scalar value to be used in heap object payload
pub fn make_scalar(value: usize) -> usize {
    // Scalar values can be at most 31 bits
    assert_eq!(value >> 31, 0);
    value << 1
}

/// Inverse of `make_scalar`
pub fn get_scalar_value(scalar: usize) -> usize {
    assert_eq!(scalar & 0b1, 0);
    scalar >> 1
}

/// Make a pointer value to be used in heap object payload
pub fn make_pointer(addr: usize) -> usize {
    addr.wrapping_sub(1)
}

/// Inverse of `make_pointer`
pub fn unskew_pointer(skewed_ptr: usize) -> usize {
    skewed_ptr.wrapping_add(1)
}
