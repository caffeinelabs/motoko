//! The one memory interface: main memory lent block by block or page by page, and the
//! checked reads and writes of lent slices.
//!
//! Every byte of every table lies in main memory. The crate reaches table bytes only through `PageHost`,
//! which lends one block (4,096 bytes) or one page (65,536 bytes) as a slice for the length of a borrow,
//! and reads and writes those slices only through the helpers below, none of which indexes out of bounds or
//! panics. A range outside the slice returns `Corrupt(Runtime, Bounds)`; the image reader replaces the
//! image id for bytes of an image.

use crate::consts::PAGE;
use crate::fault::{bounds, Fault};

/// A block (4,096 bytes) or a page (65,536 bytes), named by its host reference.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Region {
    Block(u64),
    Page(u64),
}

/// Main memory, lent. Motoko: store pages in store partitions; Rust EOP (a Rust library for upgrades that
/// keep the heap in place): pages of arenas.
///
/// A reference is a u64 the host defines; 0 means none, and bits 62 and 63 are never set, so formats may
/// use bit 63 of a stored word for their own flags. A page reference is the reference of its block 0.
pub trait PageHost {
    /// Reference of block i (0..16) of page p.
    fn block_of(&self, page: u64, i: u32) -> u64;
    /// The 4,096 bytes of a block or the 65,536 bytes of a page. The slice lives as long as the shared
    /// borrow of the host.
    fn get(&self, r: Region) -> Result<&[u8], Fault>;
    fn get_mut(&mut self, r: Region) -> Result<&mut [u8], Fault>;
    /// Two distinct regions, writable; `Err(BadRef)` when they overlap.
    fn pair_mut(&mut self, a: Region, b: Region) -> Result<(&mut [u8], &mut [u8]), Fault>;
    /// One region read and a different one written.
    fn read_write(&mut self, r: Region, wr: Region) -> Result<(&[u8], &mut [u8]), Fault>;
    /// A fresh page; its contents are unspecified (the crate writes before it reads).
    fn page_alloc(&mut self) -> Result<u64, Fault>;
    /// Returns page p; nothing references it any more. O(1). n consecutive frees change at most
    /// 1 + ⌈n/511⌉ 4 KiB blocks of the host's free-page stack plus one block of host state.
    fn page_free(&mut self, p: u64) -> Result<(), Fault>;
    /// The 4 KiB pages the next `page_alloc` may change besides the page it returns, in any memory: the
    /// host-state block for a pop or a bump; Motoko's records of a new store partition; Rust EOP's pages of a
    /// new arena. A maintenance unit ends before a `page_alloc` unless 1 plus this still fits its pages.
    fn alloc_changes(&self) -> u64;
    /// Test mode: the fill pattern P a test host poisons fresh pages with, or
    /// `None`. With `Some(p)`, `area::alloc_block` overwrites the stale free-stack link of every block it takes
    /// with p. Production hosts keep this default, which the optimizer folds away. Only test hosts
    /// override it.
    fn test_fill(&self) -> Option<u8> {
        None
    }
}

/// The slice range [off, off + len) of a slice of `len_of` bytes.
fn range(len_of: usize, off: u64, len: u64) -> Result<core::ops::Range<usize>, Fault> {
    let end = off.checked_add(len).ok_or(bounds(off))?;
    let s = usize::try_from(off).map_err(|_| bounds(off))?;
    let e = usize::try_from(end).map_err(|_| bounds(off))?;
    if e > len_of {
        return Err(bounds(off));
    }
    Ok(s..e)
}

#[inline]
pub fn sub(b: &[u8], off: u64, len: u64) -> Result<&[u8], Fault> {
    let r = range(b.len(), off, len)?;
    b.get(r).ok_or(bounds(off))
}

#[inline]
pub fn sub_mut(b: &mut [u8], off: u64, len: u64) -> Result<&mut [u8], Fault> {
    let r = range(b.len(), off, len)?;
    b.get_mut(r).ok_or(bounds(off))
}

/// The N bytes at `off`, as an array.
#[inline]
pub fn rd_array<const N: usize>(b: &[u8], off: u64) -> Result<[u8; N], Fault> {
    let s = usize::try_from(off).map_err(|_| bounds(off))?;
    let tail = b.get(s..).ok_or(bounds(off))?;
    let (head, _) = tail.split_first_chunk::<N>().ok_or(bounds(off))?;
    Ok(*head)
}

/// The little-endian u8 at `off`.
#[inline]
pub fn rd_u8(b: &[u8], off: u64) -> Result<u8, Fault> {
    let s = usize::try_from(off).map_err(|_| bounds(off))?;
    b.get(s).copied().ok_or(bounds(off))
}

/// The little-endian u16 at `off`.
#[inline]
pub fn rd_u16(b: &[u8], off: u64) -> Result<u16, Fault> {
    rd_array::<2>(b, off).map(u16::from_le_bytes)
}

/// The little-endian u32 at `off`.
#[inline]
pub fn rd_u32(b: &[u8], off: u64) -> Result<u32, Fault> {
    rd_array::<4>(b, off).map(u32::from_le_bytes)
}

/// The little-endian u64 at `off`.
#[inline]
pub fn rd_u64(b: &[u8], off: u64) -> Result<u64, Fault> {
    rd_array::<8>(b, off).map(u64::from_le_bytes)
}

#[inline]
pub fn put(b: &mut [u8], off: u64, src: &[u8]) -> Result<(), Fault> {
    let len = u64::try_from(src.len()).map_err(|_| bounds(off))?;
    let d = sub_mut(b, off, len)?;
    // The lengths are equal: `d` was cut to `src.len()` above.
    copy_bytes(d, src);
    Ok(())
}

/// Writes the u8 `v` at `off`.
#[inline]
pub fn put_u8(b: &mut [u8], off: u64, v: u8) -> Result<(), Fault> {
    let s = usize::try_from(off).map_err(|_| bounds(off))?;
    let slot = b.get_mut(s).ok_or(bounds(off))?;
    *slot = v;
    Ok(())
}

/// Writes the little-endian u16 `v` at `off`.
#[inline]
pub fn put_u16(b: &mut [u8], off: u64, v: u16) -> Result<(), Fault> {
    put(b, off, &v.to_le_bytes())
}

/// Writes the little-endian u32 `v` at `off`.
#[inline]
pub fn put_u32(b: &mut [u8], off: u64, v: u32) -> Result<(), Fault> {
    put(b, off, &v.to_le_bytes())
}

/// Writes the little-endian u64 `v` at `off`.
#[inline]
pub fn put_u64(b: &mut [u8], off: u64, v: u64) -> Result<(), Fault> {
    put(b, off, &v.to_le_bytes())
}

#[inline]
pub fn fill(b: &mut [u8], off: u64, len: u64, v: u8) -> Result<(), Fault> {
    sub_mut(b, off, len)?.fill(v);
    Ok(())
}

/// Copies min(dst.len(), src.len()) bytes; callers pass equal lengths. Both sides are first cut to the same
/// length n, so `copy_from_slice`'s length check compares n with n and the optimizer removes it, leaving one
/// bulk copy (`memory.copy` on wasm). A zipped byte loop was tried first: it is panic-free too, but on wasm64
/// it stays a byte loop, about twelve times the cost of a bulk copy for 64 KiB. The panic scan pins that no
/// panicking branch is left.
#[inline(always)]
pub fn copy_bytes(dst: &mut [u8], src: &[u8]) {
    let n = dst.len().min(src.len());
    if let (Some(d), Some(s)) = (dst.get_mut(..n), src.get(..n)) {
        d.copy_from_slice(s);
    }
}

/// The bytes one chunk of `shift` buffers.
const SHIFT_CHUNK: usize = 4_096;

/// memmove of `len` bytes from `from` to `to` inside one block or page: both ranges checked, then chunks of
/// at most 4,096 bytes copied through a stack buffer, front to back when moving down and back to front
/// when moving up, so a chunk never overwrites source bytes not yet read. Within one block that is one
/// chunk: two memcpys.
pub fn shift(b: &mut [u8], from: u64, to: u64, len: u64) -> Result<(), Fault> {
    let src = range(b.len(), from, len)?;
    let dst = range(b.len(), to, len)?;
    let n = src.len();
    let mut tmp = [0u8; SHIFT_CHUNK];
    let mut done = 0usize;
    while done < n {
        let k = (n.wrapping_sub(done)).min(SHIFT_CHUNK);
        // Moving up: take chunks from the end.
        let c = if to > from { n.wrapping_sub(done).wrapping_sub(k) } else { done };
        let s0 = src.start.wrapping_add(c);
        let d0 = dst.start.wrapping_add(c);
        let piece = tmp.get_mut(..k).ok_or(bounds(from))?;
        copy_bytes(piece, b.get(s0..s0.wrapping_add(k)).ok_or(bounds(from))?);
        copy_bytes(b.get_mut(d0..d0.wrapping_add(k)).ok_or(bounds(to))?, piece);
        done = done.wrapping_add(k);
    }
    Ok(())
}

/// memmove of `len` ≤ 65,536 bytes between two different regions.
pub fn copy<H: PageHost>(
    h: &mut H,
    from: Region,
    from_off: u64,
    to: Region,
    to_off: u64,
    len: u64,
) -> Result<(), Fault> {
    if len > PAGE {
        return Err(bounds(from_off));
    }
    let (r, wr) = h.read_write(from, to)?;
    let s = sub(r, from_off, len)?;
    put(wr, to_off, s)
}
