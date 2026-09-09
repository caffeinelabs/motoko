//! Mark bitmap in the incremental GC.
//!
//! A separate mark bitmap memory is associated for each non-free partition that does not host large objects.
//! The bitmap memory is allocated in one or multiple separate bitmap partitions, that are freed on completion
//! of a GC run.
//!
//! A bitmap is represented in raw memory (no object header, no blob) of the fixed size
//! `PARTITION_SIZE / WORD_SIZE / u8::BITS` bytes. For simplicity, the bitmaps is not shortened for partitions
//! that also accommodate static space besides dynamic space.
//!
//! To access a mark bit of an object, the corresponding bitmap and address offset inside the object's partition
//! needs to be first determined. The corresponding bit is then accessed at the byte with index
//! `offset / WORD_SIZE / u8::BITS` and at the bit index `(offset / WORD_SIZE) % u8::BITS`.
//!
//! If an object is marked, the corresponding bit is set to `1`. Otherwise, if the object is not marked or
//! the bit does not denote the start of an object, it is `0`.
//!
//! The mark bitmap memory must be entirely cleared (zeroed) when assigned at the beginning of each mark phase.
//!
//! The mark bitmap serves for fast traversal of marked objects in a partition with few marked objects
//! (and many garbage objects).

use core::ptr::null_mut;

use crate::{constants::WORD_SIZE, mem_utils::memzero, types::Bytes};

use super::partitioned_heap::PARTITION_SIZE;

const BITS_PER_BYTE: usize = u8::BITS as usize;
const BITMAP_FRACTION: usize = WORD_SIZE * BITS_PER_BYTE;

pub const BITMAP_SIZE: usize = PARTITION_SIZE / BITMAP_FRACTION;

/// Partition-associated mark bitmap.
/// Use a long-term representation by relying on C layout.
#[repr(C)]
pub struct MarkBitmap {
    pointer: *mut u8,
}

pub(crate) const DEFAULT_MARK_BITMAP: MarkBitmap = MarkBitmap {
    pointer: null_mut(),
};

impl MarkBitmap {
    /// Allocate new zero-sized bitmap.
    pub fn new() -> MarkBitmap {
        DEFAULT_MARK_BITMAP
    }

    /// Assign and initialize the bitmap memory at the defined address.
    pub unsafe fn assign(&mut self, bitmap_address: *mut u8) {
        debug_assert_eq!(bitmap_address as usize % WORD_SIZE, 0);
        memzero(bitmap_address as usize, Bytes(BITMAP_SIZE).to_words());
        debug_assert_eq!(self.pointer, null_mut());
        self.pointer = bitmap_address;
    }

    /// Release the bitmap memory after garbage collection.
    pub fn release(&mut self) {
        self.pointer = null_mut();
    }

    fn word_index(&self, offset_in_partition: usize) -> usize {
        debug_assert_eq!(offset_in_partition % WORD_SIZE, 0);
        debug_assert!(offset_in_partition < PARTITION_SIZE);
        offset_in_partition / WORD_SIZE
    }

    /// Check whether the object at defined address offset in the partition is marked.
    pub unsafe fn is_marked(&self, offset_in_partition: usize) -> bool {
        debug_assert_ne!(self.pointer, null_mut());
        let word_index = self.word_index(offset_in_partition);
        let byte_index = word_index / BITS_PER_BYTE;
        let bit_index = word_index % BITS_PER_BYTE;
        let byte = self.pointer.add(byte_index);
        (*byte >> bit_index) & 0b1 != 0
    }

    /// Mark an object at the defined address offset in the partition.
    pub unsafe fn mark(&mut self, offset_in_partition: usize) {
        debug_assert_ne!(self.pointer, null_mut());
        let word_index = self.word_index(offset_in_partition);
        let byte_index = word_index / BITS_PER_BYTE;
        let bit_index = word_index % BITS_PER_BYTE;
        let byte = self.pointer.add(byte_index);
        *byte |= 0b1 << bit_index;
    }

    /// Obtain a new iterator for the bitmap.
    pub fn iterate(&self) -> BitmapIterator {
        BitmapIterator::new(self.pointer)
    }
}

/// The iterator separates advancing `next()` from inspection `current_marked_offset()`
/// to better support the incremental evacuation and update GC increments.
#[repr(C)]
pub struct BitmapIterator {
    /// Start address of the mark bitmap.
    bitmap_pointer: *mut u8,
    /// Index of next bit to continue iteration in the bitmap.
    /// Invariant during (initialized and unfinished):
    /// `lsb(current_word) == bitmap.bit(next_bit_index - 1)`.
    /// with `lsb` meaning "least significant bit" and `bitmap.bit()`
    /// reading the corresponding bit in the bitmap.
    /// Sentinel: `BITMAP_ITERATION_END` if the iteration is finished.
    next_bit_index: usize,
    /// Current word in the bitmap that we are iterating.
    current_word: usize,
    /// Number of leading bits that are initially zeroed in `current_word`.
    leading_zeros: usize,
}

/// End-of-iteration sentinel, faster than using an `Option` with `None`.
/// `usize::MAX` is not word-aligned and thus not a valid object address.
pub const BITMAP_ITERATION_END: usize = usize::MAX;

/// Last possible valid value of `next_bit_index`.
const BIT_INDEX_END: usize = BITMAP_SIZE * BITS_PER_BYTE;

const _: () = assert!(BIT_INDEX_END < BITMAP_ITERATION_END);

impl BitmapIterator {
    fn new(bitmap_pointer: *mut u8) -> BitmapIterator {
        debug_assert_ne!(bitmap_pointer, null_mut());
        debug_assert_eq!(PARTITION_SIZE % WORD_SIZE, 0);
        debug_assert_eq!(bitmap_pointer as usize % WORD_SIZE, 0);
        let mut iterator = BitmapIterator {
            bitmap_pointer,
            next_bit_index: 0,
            current_word: 0,
            leading_zeros: 0,
        };
        // Move to first marked bit or to the bitmap end if no bit is set.
        iterator.next();
        iterator
    }

    /// Returns the next marked address offset in the partition,
    /// or `BITMAP_ITERATION_END` if there are no more bits set.
    pub fn current_marked_offset(&self) -> usize {
        debug_assert!(self.next_bit_index > 0);
        if self.next_bit_index == BITMAP_ITERATION_END {
            return BITMAP_ITERATION_END;
        } else {
            (self.next_bit_index - 1) * WORD_SIZE
        }
    }

    /// Advance the iterator to the next marked offset.
    pub fn next(&mut self) {
        debug_assert!(self.next_bit_index <= BIT_INDEX_END);
        // Outer loop iterates the words.
        while self.next_bit_index < BIT_INDEX_END {
            // Examine the least significant bit(s) in the current word.
            if self.current_word != 0 {
                let shift = self.current_word.trailing_zeros() as usize;
                // Two shifts to avoid an overflow in the case of `shift == usize::BITS - 1`.
                self.current_word >>= shift;
                self.current_word >>= 1;
                self.next_bit_index += shift + 1;
                // Next set bit found. This could technically even be the
                // very last bit in the bitmap (although objects are not that small).
                return;
            }

            // Move on to next word.
            self.next_bit_index += self.leading_zeros;
            if self.next_bit_index < BIT_INDEX_END {
                debug_assert_eq!(self.next_bit_index % BITS_PER_BYTE, 0);
                let word_index = self.next_bit_index / BITS_PER_BYTE;
                self.current_word =
                    unsafe { *(self.bitmap_pointer.add(word_index) as *const usize) };
                self.leading_zeros = self.current_word.leading_zeros() as usize;
            }
        }
        // Finish the iteration.
        self.next_bit_index = BITMAP_ITERATION_END;
    }
}
