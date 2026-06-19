use crate::gc::mark_compact::TAG_BLOB_B;
use crate::mem_utils::memzero;
use crate::memory::{Memory, alloc_blob};
use crate::types::{Blob, Bytes, Obj, size_of};

/* How the Wasm-heap maps to the bitmap

  +---- RTS stack ----+---- Motoko statics ----+---- Dynamic heap ------+ Heap limit
                  (prefix words)                   bitmap lives here -> | BM |
                                                                       /      \
                                                                      /        \
                                       each bit represents a word -> /...bits...\
                                          in the dynamic heap

## Marking with absolute addresses

When marking, we need to map an address to a bit in the bitmap. Internally the
bitmap is kept in a (non-moving) blob after the dynamic heap (DH). To
efficiently mark the right bit in the bitmap, we maintain a pointer that points
_before the start of the bitmap_ such that `address / 8` and `address % 8`
will address the right bit:


       +---- BITMAP_FORBIDDEN_PTR         +---- BITMAP_PTR
       v                                  v
       ,          (forbidden)             ,...................... bitmap ..........~~....,
       |   heap_prefix_words / 8 bytes    |               heap_size / 32 bytes           |
       |   get_bitmap_forbidden_size()    |                   BITMAP_SIZE                |
       ^                                  ^                                              ^
      /           ^^^^^^^^^^^              \                                             |
     /            corresponds               \                 ^^^^^^^^^^^               /
    /                                        \                corresponds              /
   /                                          \                                       /
  +---- Rts stack ----+---- Motoko statics ----+--------- Dynamic heap ----------~~--+
                                               !
                                               ! 32-byte aligned


Debug assertions guard the forbidden bytes from access, as this area potentially
overlaps with the Motoko dynamic heap.

## The alignment caveat

For this scheme to work, it is essential that the start of the DH is an address that
is divisible by 32 (`heap_prefix_words % 8 == 0`). Otherwise the `address / 8`
and `address % 8` operations on the DH's starting address will not yield the
least significant bit in the BM.

## Example calculation

Assume the DH is at 0x80000. Assume heap limit being at 0xB0000. Then the BM thus
could be placed at 0xB0008. Since the heap_prefix_words is 0x20000,
BITMAP_FORBIDDEN_PTR = 0xB0008 - 0x20000 / 8 = 0xAC008.

Now let's mark the address 0x80548 in the DH. Its absolute word number is 0x20152.
The `(0x20152 / 8, 0x20152 % 8)`-rule gives a bit position 2 with byte offset 0x402A,
thus we mark bit 2 in byte 0xAC004 + 0x402A = 0xB002E, which is in the BM.

 */

// Only designed for 32-bit.
const _: () = assert!(core::mem::size_of::<usize>() == core::mem::size_of::<u32>());

/// Current bitmap
static mut BITMAP_FORBIDDEN_PTR: *mut u8 = core::ptr::null_mut();
static mut BITMAP_PTR: *mut u8 = core::ptr::null_mut();
static mut BITMAP_SIZE: usize = 0;

unsafe fn get_bitmap_forbidden_size() -> usize {
    BITMAP_PTR as usize - BITMAP_FORBIDDEN_PTR as usize
}

pub unsafe fn alloc_bitmap<M: Memory>(
    mem: &mut M,
    heap_size: Bytes<usize>,
    heap_prefix_words: usize,
) {
    // See Note "How the Wasm-heap maps to the bitmap" above
    debug_assert_eq!(heap_prefix_words % 8, 0);
    // We will have at most this many objects in the heap, each requiring a bit
    let n_bits = heap_size.to_words().as_usize();
    // Each byte will hold 8 bits.
    BITMAP_SIZE = (n_bits + 7) / 8;
    // Also round allocation up to 8-bytes to make iteration efficient. We want to be able to read
    // 64 bits in a single read and check as many bits as possible with a single `word != 0`.
    let bitmap_bytes = Bytes(((BITMAP_SIZE + 7) / 8) * 8);
    // Allocating an actual object here as otherwise dump_heap gets confused
    // No post allocation barrier as this RTS-internal blob will be collected by the GC.
    let blob = alloc_blob(mem, TAG_BLOB_B, bitmap_bytes).get_ptr() as *mut Blob;
    memzero(blob.payload_addr() as usize, bitmap_bytes.to_words());

    BITMAP_PTR = blob.payload_addr();
    BITMAP_FORBIDDEN_PTR = BITMAP_PTR.sub(heap_prefix_words / 8)
}

pub unsafe fn free_bitmap() {
    BITMAP_PTR = core::ptr::null_mut();
    BITMAP_FORBIDDEN_PTR = core::ptr::null_mut();
    BITMAP_SIZE = 0;
}

pub unsafe fn get_bit(idx: usize) -> bool {
    let (byte_idx, bit_idx) = (idx / 8, idx % 8);
    debug_assert!(byte_idx >= get_bitmap_forbidden_size());
    debug_assert!(get_bitmap_forbidden_size() + BITMAP_SIZE > byte_idx);
    let byte = *BITMAP_FORBIDDEN_PTR.add(byte_idx);
    (byte >> bit_idx) & 0b1 != 0
}

pub unsafe fn set_bit(idx: usize) {
    let (byte_idx, bit_idx) = (idx / 8, idx % 8);
    debug_assert!(byte_idx >= get_bitmap_forbidden_size());
    debug_assert!(get_bitmap_forbidden_size() + BITMAP_SIZE > byte_idx);
    let byte = *BITMAP_FORBIDDEN_PTR.add(byte_idx);
    let new_byte = byte | (0b1 << bit_idx);
    *BITMAP_FORBIDDEN_PTR.add(byte_idx) = new_byte;
}

pub struct BitmapIter {
    /// Size of the bitmap, in bits. Does not change after initialization.
    size: u32,
    /// Current bit index
    current_bit_idx: u32,
    /// Current 64-bit word in the bitmap that we're iterating. We read in 64-bit chunks to be able
    /// to check as many bits as possible with a single `word != 0`.
    current_word: u64,
    /// How many leading bits are initially zeroed in the current_word
    leading_zeros: u32,
}

pub unsafe fn iter_bits() -> BitmapIter {
    let blob_len_bytes = (BITMAP_PTR.sub(size_of::<Blob>().to_bytes().as_usize()) as *mut Obj)
        .as_blob()
        .len()
        .as_usize() as u32;

    debug_assert_eq!(blob_len_bytes % 8, 0);

    let blob_len_64bit_words = blob_len_bytes / 8;

    let current_word = if blob_len_64bit_words == 0 {
        0
    } else {
        let bitmap_ptr64 = BITMAP_PTR as *const u64;
        bitmap_ptr64.read_unaligned()
    };

    debug_assert!(BITMAP_PTR as usize >= BITMAP_FORBIDDEN_PTR as usize);
    let forbidden_bits = get_bitmap_forbidden_size() as u32 * 8;

    BitmapIter {
        size: blob_len_bytes * 8 + forbidden_bits,
        current_bit_idx: forbidden_bits,
        current_word,
        leading_zeros: current_word.leading_zeros(),
    }
}

/// This value marks the end-of-stream in `BitmapIter`. Using this value instead of `None` for
/// end-of-stream reduces Wasm instructions executed by ~2.7% in some cases.
//
// Heap is 4GiB and each 32-bit word gets a bit, so this is one larger than the bit for the last
// word in heap.
//
// (We actually need less bits than that as when the heap is full we can't allocate bitmap and mark
// stack and can't do GC)
pub const BITMAP_ITER_END: usize = usize::MAX;

impl BitmapIter {
    /// Returns the next bit, or `BITMAP_ITER_END` if there are no more bits set.
    pub fn next(&mut self) -> usize {
        debug_assert!(self.current_bit_idx <= self.size);

        if self.current_bit_idx == self.size {
            return BITMAP_ITER_END;
        }

        // Outer loop iterates 64-bit words
        loop {
            // Examine the least significant bit(s) in the current word
            if self.current_word != 0 {
                let shift_amt = self.current_word.trailing_zeros();
                self.current_word >>= shift_amt;
                self.current_word >>= 1;
                let bit_idx = self.current_bit_idx + shift_amt;
                self.current_bit_idx = bit_idx + 1;
                return bit_idx as usize;
            }

            // Move on to next word (always 64-bit boundary)
            self.current_bit_idx += self.leading_zeros;
            unsafe {
                debug_assert_eq!(
                    (self.current_bit_idx - get_bitmap_forbidden_size() as u32 * 8) % 64,
                    0
                )
            }
            if self.current_bit_idx == self.size {
                return BITMAP_ITER_END;
            }
            self.current_word = unsafe {
                let ptr64 =
                    BITMAP_FORBIDDEN_PTR.add(self.current_bit_idx as usize / 8) as *const u64;
                ptr64.read_unaligned()
            };
            self.leading_zeros = self.current_word.leading_zeros();
        }
    }
}
