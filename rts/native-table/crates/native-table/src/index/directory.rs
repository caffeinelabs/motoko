//! The index directory: up to three area-0 blocks named by the table root at 928, 936 and 944, made when
//! first needed: block 0 for index ids 0–30, block 1 for 31–61, block 2 for 62–63. Each is a 64-byte header
//! {magic `NTIX` u32 @0; format u16 @4; first index id u8 @6; descriptors u8 @7} and descriptors of 128 bytes
//! from offset 64.
//!
//! **Index descriptor** (128 bytes): kind u8 @0 (0 none, 1 primary, 2 secondary); its tree's key-area width W
//! u16 @2; flags u16 @4 (bit 2 keys unique); key spec hash u64 @8; the tree's root u64 @16, nodes u32 @24,
//! key-log blocks u32 @28, key-log radix root u64 @32 and key-log radix height u8 @40; a secondary index's
//! build, the rid it takes next u64 @48 and the rid it ends before u64 @56 (0 and 0: none); the run list block
//! u64 @80 (0: none); zero elsewhere.

use crate::area::alloc_zeroed;
use crate::consts::{magic, FORMAT_TABLE};
use crate::fault::{rt, Corruption, Fault};
use crate::mem::{put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u64, rd_u8, PageHost, Region};

/// Table root offset of the directory block references.
pub const DIRECTORY_REFS: u64 = 928;
/// Bytes of a directory block header.
pub const DIR_HEAD: u64 = 64;
/// Bytes of an index descriptor.
pub const INDEX_DESC: u64 = 128;
/// Table root offset of `run_seq`.
pub const RUN_SEQ: u64 = 96;

/// The directory block number, its first index id and its descriptor count for index `id`.
fn place(id: u8) -> Result<(u64, u8, u8), Fault> {
    match id {
        0..=30 => Ok((0, 0, 31)),
        31..=61 => Ok((1, 31, 31)),
        62..=63 => Ok((2, 62, 2)),
        _ => Err(rt(Corruption::Range, u64::from(id))),
    }
}

/// The block and offset of index `id`'s descriptor, or `None` when its directory block is not made yet.
/// Always inlined: out of line, each of its callers (a count's fixed part among them) pays about 40
/// instructions more, and the compiler's choice of how a caller's host reaches it varies with unrelated code.
#[inline(always)]
pub fn locate<H: PageHost + ?Sized>(h: &H, tbl: u64, id: u8) -> Result<Option<(u64, u64)>, Fault> {
    let (k, first, _) = place(id)?;
    // k ≤ 2.
    let b = rd_u64(h.get(Region::Block(tbl))?, DIRECTORY_REFS.wrapping_add(k.wrapping_mul(8)))?;
    if b == 0 {
        return Ok(None);
    }
    let blk = h.get(Region::Block(b))?;
    if rd_u32(blk, 0)? != magic::NTIX || rd_u16(blk, 4)? != FORMAT_TABLE || rd_u8(blk, 6)? != first {
        return Err(rt(Corruption::Header, b));
    }
    // id − first ≤ 30.
    let off = DIR_HEAD.wrapping_add(u64::from(id.wrapping_sub(first)).wrapping_mul(INDEX_DESC));
    Ok(Some((b, off)))
}

/// The block and offset of index `id`'s descriptor, making its directory block (zero-filled, with its
/// header) when needed.
pub fn ensure<H: PageHost>(h: &mut H, tbl: u64, id: u8) -> Result<(u64, u64), Fault> {
    if let Some(x) = locate(h, tbl, id)? {
        return Ok(x);
    }
    let (k, first, count) = place(id)?;
    let b = alloc_zeroed(h, tbl, 0)?;
    {
        let s = h.get_mut(Region::Block(b))?;
        put_u32(s, 0, magic::NTIX)?;
        put_u16(s, 4, FORMAT_TABLE)?;
        put_u8(s, 6, first)?;
        put_u8(s, 7, count)?;
    }
    put_u64(h.get_mut(Region::Block(tbl))?, DIRECTORY_REFS.wrapping_add(k.wrapping_mul(8)), b)?;
    locate(h, tbl, id)?.ok_or(rt(Corruption::Link, b))
}

/// Takes the table's next run sequence: returns `run_seq` and raises it by one.
pub fn take_run_seq<H: PageHost>(h: &mut H, tbl: u64) -> Result<u64, Fault> {
    let s = h.get_mut(Region::Block(tbl))?;
    let seq = rd_u64(s, RUN_SEQ)?;
    put_u64(s, RUN_SEQ, seq.checked_add(1).ok_or(rt(Corruption::Count, RUN_SEQ))?)?;
    Ok(seq)
}
