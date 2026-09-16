use motoko_rts_macros::classical_persistence;

use crate::types::Bytes;

/// Wasm word size. RTS only works correctly on platforms with this word size.
pub const WORD_SIZE: usize = core::mem::size_of::<usize>();

/// Wasm page size (64 KiB) in bytes
pub const WASM_PAGE_SIZE: Bytes<usize> = Bytes(64 * KB);

/// Byte constants
pub const KB: usize = 1024;
pub const MB: usize = 1024 * KB;
pub const GB: usize = 1024 * MB;

#[classical_persistence]
use crate::types::Words;

/// Wasm 32-bit heap size (4 GiB) in words.
/// Note that `to_bytes` on this value will overflow as 4 GiB in bytes is `u32::MAX + 1`.
#[classical_persistence]
pub const WASM32_HEAP_SIZE: Words<usize> = Words(1024 * 1024 * 1024);

// The optimized array iterator requires array lengths to fit in signed compact numbers.
// See `compile_enhanced.ml`, `GetPastArrayOffset`.
// Two bits reserved: Two for Int tag (0b10L) and one for the sign bit.
pub const MAX_ARRAY_LENGTH_FOR_ITERATOR: usize = 1 << (usize::BITS as usize - 3);

// Inclusive upper bound on the element count of an allocatable array. This is not the
// iterator bound above: an array of that many elements could not be allocated at all,
// and admitting it lets the byte size `(header + len) * WORD_SIZE` wrap, handing out a
// few bytes for an array whose header claims ~2^61 elements. 2^48 elements are 2 PiB of
// payload, far beyond any addressable memory, and leave the size computation 13 bits of
// headroom. The 32-bit bound is unchanged and cannot wrap.
pub const MAX_ARRAY_LENGTH: usize = if usize::BITS == 64 { 1 << 48 } else { 1 << 29 };
