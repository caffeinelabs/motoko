//! The write buffer: buffer blocks with their slots, the tail block's hot state, and buffer descriptors.
//!
//! **Buffer block** (area 0): {magic `NTBF` u32 @0; `FORMAT_TABLE` u16 @4; slots u16 @6; group number u64
//! @8; position of slot 0 u16 @16; rows end u16 @18; slot array start u16 @20; zero u16 @22; buffer
//! descriptor u64 @24; the hot state [32, H) in the tail block; row images from H, each 8-aligned; slots from
//! 4,096 down}. Slot s at 4,096 − 8(s + 1): {offset u16, length u16, flags u8 (bit 0
//! dead, bit 1 long, bit 2 moved), zero u8, zero u16}. A long slot's bytes are a 32-byte long stub {flags u32
//! (bit 0 row image), zero u32, image reference}; a moved slot's a 16-byte stub {buffer block u64, offset
//! u16, length u16, zero u32}.
//!
//! **Buffer descriptor** (area 0): {magic `NTBD` u32 @0; `FORMAT_TABLE` u16 @4; blocks u16 @6; group u64 @8;
//! positions used u16 @16; flags u16 @18 (bit 0 frozen; bit 3 a row holds a long value); inline bytes u32 @20; next frozen descriptor u64
//! @24; large values u16 @32; medium bytes u32 @36; zero u32 @40; extremes block u64 @48 (0: none); pending
//! group page u64 @56; 240 block records of 16 bytes from @64 {block u64, first position u16, slots u16, zero
//! u32}; bytes 3,904–4,095 zero until the drain's walk of the descriptor uses them}.

use crate::consts::{magic, BLOCK, BUFFER_BLOCKS_MAX, FORMAT_TABLE, GROUP_SLOTS};
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::image::Image;
use crate::mem::{fill, put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u64, rd_u8, Owner, PageHost, Region};

use super::layout::Shape;
use super::root;

// ---- buffer block ----
pub const B_SLOTS: u64 = 6;
pub const B_GROUP: u64 = 8;
pub const B_FIRST: u64 = 16;
pub const B_ROWS_END: u64 = 18;
pub const B_SLOTS_START: u64 = 20;
const B_ZERO: u64 = 22;
pub const B_DESC: u64 = 24;
/// Slot flags.
pub const SLOT_DEAD: u8 = 1;
pub const SLOT_LONG: u8 = 2;
pub const SLOT_MOVED: u8 = 4;
/// Bytes of a slot, a long stub and a moved stub.
pub const SLOT: u64 = 8;
pub const LONG_STUB: u64 = 32;
pub const MOVED_STUB: u64 = 16;

// ---- hot state, in the tail block ----
pub const H_LIVE: u64 = 32;
pub const H_APPENDED: u64 = 40;
pub const H_OPEN_POS: u64 = 48;
pub const H_H: u64 = 50;
pub const H_OPEN_INLINE: u64 = 52;
pub const H_OPEN_MEDIUM: u64 = 56;
pub const H_FLAGS: u64 = 64;
/// The totals space's block list: its radix root (0: no space) and height (`totals`).
pub const H_TOTALS: u64 = 72;
pub const H_TOTALS_HEIGHT: u64 = 80;
pub const H_INDEX_RISE: u64 = 88;
pub const H_RELEASE: u64 = 96;
pub const H_RUN_VERSION: u64 = 176;
pub const H_OPEN_LARGE: u64 = 180;
pub const H_RUN_R: u64 = 182;
/// The buffered live rows: the live rows of the open and frozen groups, raised by inserts, lowered by deletes
/// of buffered rows and, at a seal's flip, by the sealed group's live rows. A new leaf's buffered running
/// record follows from it and the leaf's default.
pub const H_BUFFERED: u64 = 184;
/// Hot flags bit 1: a row of the open group holds a long value or is a long row image, or did; its freeze
/// sets its descriptor's `LONG_ROWS` from it.
pub const OPEN_LONG: u64 = 2;

// ---- descriptor ----
pub const D_BLOCKS: u64 = 6;
pub const D_GROUP: u64 = 8;
pub const D_POSITIONS: u64 = 16;
pub const D_FLAGS: u64 = 18;
pub const D_INLINE: u64 = 20;
pub const D_NEXT: u64 = 24;
pub const D_LARGE: u64 = 32;
pub const D_MEDIUM: u64 = 36;
/// The group's extremes block (`extremes`), 0 when it has none.
pub const D_EXT: u64 = 48;
pub const D_PENDING: u64 = 56;
pub const D_RECORDS: u64 = 64;
/// The descriptor's bytes past its block records, which the drain's walk of it uses.
pub const D_SPARE: u64 = 3_904;
/// Descriptor flags bit 0: frozen.
pub const FROZEN: u16 = 1;
/// Descriptor flags bit 1: a seal holds a snapshot of the group; its pending group page is at @56.
pub const SNAPSHOT: u16 = 2;
/// Descriptor flags bit 2: a seal reads the group's liveness into its pending group page (at @56) before its
/// snapshot; a delete meanwhile clears its row's bit there.
pub const LIVENESS: u16 = 4;
/// Descriptor flags bit 3, kept from the freeze on: a row of the frozen group holds a long value (a long item)
/// or is a long row image, or did; without it, its walk on the drain has no long value to hand on and reads
/// no row.
pub const LONG_ROWS: u16 = 8;
/// Directory entries naming a buffer descriptor carry bit 63.
pub const DESC_BIT: u64 = 1 << 63;

fn rt(kind: Corruption, at: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, at)
}

/// The offset of slot s.
#[inline]
pub fn slot_at(s: u64) -> u64 {
    // s < 4,096 / 8.
    BLOCK.wrapping_sub(SLOT.wrapping_mul(s.wrapping_add(1)))
}

/// One slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub off: u64,
    pub len: u64,
    pub flags: u8,
}

pub fn read_slot(b: &[u8], s: u64) -> Result<Slot, Fault> {
    let at = slot_at(s);
    Ok(Slot {
        off: u64::from(rd_u16(b, at)?),
        len: u64::from(rd_u16(b, at.wrapping_add(2))?),
        flags: rd_u8(b, at.wrapping_add(4))?,
    })
}

pub fn write_slot(b: &mut [u8], s: u64, slot: &Slot) -> Result<(), Fault> {
    let at = slot_at(s);
    let off = u16::try_from(slot.off).map_err(|_| rt(Corruption::Range, slot.off))?;
    let len = u16::try_from(slot.len).map_err(|_| rt(Corruption::Range, slot.len))?;
    put_u16(b, at, off)?;
    put_u16(b, at.wrapping_add(2), len)?;
    put_u8(b, at.wrapping_add(4), slot.flags)?;
    put_u8(b, at.wrapping_add(5), 0)?;
    put_u16(b, at.wrapping_add(6), 0)
}

/// Where a row image lies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowLoc {
    /// `len` bytes at `off` of buffer block `block`.
    Inline { block: u64, off: u64, len: u64 },
    /// A long row image.
    Long(Image),
}

/// A slot's row: its image's place, following a moved or long stub; `None` for a dead slot.
pub fn slot_row(h: &(impl PageHost + ?Sized), block: u64, s: u64) -> Result<Option<RowLoc>, Fault> {
    let b = h.get(Region::Block(block))?;
    let slot = read_slot(b, s)?;
    if slot.flags & SLOT_DEAD != 0 {
        return Ok(None);
    }
    stub_row(b, block, &slot).map(Some)
}

/// The row a live slot names.
pub fn stub_row(b: &[u8], block: u64, slot: &Slot) -> Result<RowLoc, Fault> {
    if slot.flags & SLOT_LONG != 0 {
        let img = Image::read_ref(b, slot.off.wrapping_add(8))?;
        return Ok(RowLoc::Long(img));
    }
    if slot.flags & SLOT_MOVED != 0 {
        let to = rd_u64(b, slot.off)?;
        let off = u64::from(rd_u16(b, slot.off.wrapping_add(8))?);
        let len = u64::from(rd_u16(b, slot.off.wrapping_add(10))?);
        return Ok(RowLoc::Inline { block: to, off, len });
    }
    Ok(RowLoc::Inline { block, off: slot.off, len: slot.len })
}

/// A row located by rid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Located {
    pub group: u64,
    pub pos: u64,
    pub desc: u64,
    pub frozen: bool,
    pub block: u64,
    pub slot: u64,
}

/// A buffered row by its rid: directory → buffer descriptor → the block record holding the position → the
/// slot. `None` when the group is not buffered or the position is past its slots.
pub fn locate<H: PageHost + ?Sized>(h: &H, tbl: u64, rid: u64) -> Result<Option<Located>, Fault> {
    let group = rid / GROUP_SLOTS;
    let pos = rid % GROUP_SLOTS;
    let entry = root::group(h, tbl, group)?;
    if entry & DESC_BIT == 0 || entry == 0 {
        return Ok(None);
    }
    let desc = entry & !DESC_BIT;
    let d = h.get(Region::Block(desc))?;
    let blocks = u64::from(rd_u16(d, D_BLOCKS)?);
    if blocks == 0 || blocks > BUFFER_BLOCKS_MAX {
        return Err(rt(Corruption::Count, desc));
    }
    let frozen = rd_u16(d, D_FLAGS)? & FROZEN != 0;
    // The last record whose first position is at or below pos.
    let (mut lo, mut hi) = (0u64, blocks);
    while hi.wrapping_sub(lo) > 1 {
        let mid = lo.wrapping_add(hi.wrapping_sub(lo) / 2);
        let first = u64::from(rd_u16(d, D_RECORDS.wrapping_add(mid.wrapping_mul(16)).wrapping_add(8))?);
        if first <= pos {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let rec = D_RECORDS.wrapping_add(lo.wrapping_mul(16));
    let block = rd_u64(d, rec)?;
    let b = h.get(Region::Block(block))?;
    let first = u64::from(rd_u16(b, B_FIRST)?);
    let slots = u64::from(rd_u16(b, B_SLOTS)?);
    let Some(slot) = pos.checked_sub(first) else { return Ok(None) };
    if slot >= slots {
        return Ok(None);
    }
    Ok(Some(Located { group, pos, desc, frozen, block, slot }))
}

/// The block and slot of position `pos` of the buffered group whose descriptor is `desc`; `None` past its
/// slots.
pub fn locate_in<H: PageHost + ?Sized>(h: &H, desc: u64, pos: u64) -> Result<Option<(u64, u64)>, Fault> {
    let d = h.get(Region::Block(desc))?;
    let blocks = u64::from(rd_u16(d, D_BLOCKS)?);
    if blocks == 0 || blocks > BUFFER_BLOCKS_MAX {
        return Err(rt(Corruption::Count, desc));
    }
    let (mut lo, mut hi) = (0u64, blocks);
    while hi.wrapping_sub(lo) > 1 {
        let mid = lo.wrapping_add(hi.wrapping_sub(lo) / 2);
        let first = u64::from(rd_u16(d, D_RECORDS.wrapping_add(mid.wrapping_mul(16)).wrapping_add(8))?);
        if first <= pos {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let block = rd_u64(d, D_RECORDS.wrapping_add(lo.wrapping_mul(16)))?;
    let b = h.get(Region::Block(block))?;
    let first = u64::from(rd_u16(b, B_FIRST)?);
    let slots = u64::from(rd_u16(b, B_SLOTS)?);
    let Some(slot) = pos.checked_sub(first) else { return Ok(None) };
    if slot >= slots {
        return Ok(None);
    }
    Ok(Some((block, slot)))
}

/// The open group's descriptor and tail block.
pub fn open_group<H: PageHost + ?Sized>(h: &H, tbl: u64) -> Result<(u64, u64, u64), Fault> {
    let g = root::u64_at(h, tbl, root::OPEN_GROUP)?;
    let entry = root::group(h, tbl, g)?;
    if entry & DESC_BIT == 0 {
        return Err(rt(Corruption::Link, g));
    }
    Ok((g, entry & !DESC_BIT, root::u64_at(h, tbl, root::TAIL)?))
}

/// A new buffer descriptor for group g whose first block is `first`; the fields a reader may read are
/// written, the block records past the first are not.
pub fn new_descriptor<H: PageHost>(h: &mut H, tbl: u64, g: u64, first: u64) -> Result<u64, Fault> {
    let d = crate::area::alloc_block(h, tbl, 0)?;
    let b = h.get_mut(Region::Block(d), Owner(0))?;
    fill(b, 0, D_RECORDS.wrapping_add(16), 0)?;
    put_u32(b, 0, magic::NTBD)?;
    put_u16(b, 4, FORMAT_TABLE)?;
    put_u16(b, D_BLOCKS, 1)?;
    put_u64(b, D_GROUP, g)?;
    put_u64(b, D_RECORDS, first)?;
    fill(b, D_SPARE, BLOCK.wrapping_sub(D_SPARE), 0)?;
    Ok(d)
}

/// A new buffer block of group g whose slot 0 is position `first`, for descriptor `desc`: its header and no
/// slot. With `hot_from`, the hot state [32, H) is copied from that block (the previous tail).
pub fn new_block<H: PageHost>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    g: u64,
    first: u64,
    desc: u64,
    hot_from: Option<u64>,
) -> Result<u64, Fault> {
    let hh = match hot_from {
        Some(old) => u16_of(&*h, old, H_H)?,
        None => shape.hot_h,
    };
    new_block_at(h, tbl, g, first, desc, hot_from, hh)
}

/// `new_block` whose first row byte is `hh`.
pub fn new_block_at<H: PageHost>(
    h: &mut H,
    tbl: u64,
    g: u64,
    first: u64,
    desc: u64,
    hot_from: Option<u64>,
    hh: u64,
) -> Result<u64, Fault> {
    let nb = crate::area::alloc_block(h, tbl, 0)?;
    if let Some(old) = hot_from {
        crate::mem::copy(h, Region::Block(old), 32, Region::Block(nb), 32, hh.wrapping_sub(32), Owner(0))?;
    }
    let b = h.get_mut(Region::Block(nb), Owner(0))?;
    put_u32(b, 0, magic::NTBF)?;
    put_u16(b, 4, FORMAT_TABLE)?;
    put_u16(b, B_SLOTS, 0)?;
    put_u64(b, B_GROUP, g)?;
    let w16 = |v: u64| u16::try_from(v).map_err(|_| rt(Corruption::Range, v));
    put_u16(b, B_FIRST, w16(first)?)?;
    put_u16(b, B_ROWS_END, w16(hh)?)?;
    put_u16(b, B_SLOTS_START, w16(BLOCK)?)?;
    put_u16(b, B_ZERO, 0)?;
    put_u64(b, B_DESC, desc)?;
    Ok(nb)
}

/// Zeroes the hot state of block `b` whose H is `hh`, except its running-record layout, which the caller
/// writes.
pub fn zero_hot(b: &mut [u8], shape: &Shape, hh: u64) -> Result<(), Fault> {
    fill(b, 32, hh.wrapping_sub(32), 0)?;
    let hh = u16::try_from(hh).map_err(|_| rt(Corruption::Range, hh))?;
    put_u16(b, H_H, hh)?;
    // Bit i for secondary index i; a table without secondary indexes has none set.
    put_u64(b, H_INDEX_RISE, 0)?;
    put_u32(b, H_RUN_VERSION, shape.version)?;
    let r = u16::try_from(shape.run_r).map_err(|_| rt(Corruption::Range, shape.run_r))?;
    put_u16(b, H_RUN_R, r)
}

/// A u16 field of a block.
pub fn u16_of<H: PageHost + ?Sized>(h: &H, b: u64, off: u64) -> Result<u64, Fault> {
    Ok(u64::from(rd_u16(h.get(Region::Block(b))?, off)?))
}

pub fn u32_of<H: PageHost + ?Sized>(h: &H, b: u64, off: u64) -> Result<u64, Fault> {
    Ok(u64::from(rd_u32(h.get(Region::Block(b))?, off)?))
}

pub fn u64_of<H: PageHost + ?Sized>(h: &H, b: u64, off: u64) -> Result<u64, Fault> {
    rd_u64(h.get(Region::Block(b))?, off)
}

/// Every slot of a buffered group in position order: (position, block, slot).
pub fn for_each_slot<H: PageHost + ?Sized, F>(h: &H, desc: u64, mut f: F) -> Result<(), Fault>
where
    F: FnMut(u64, u64, u64) -> Result<(), Fault>,
{
    let blocks = u16_of(h, desc, D_BLOCKS)?;
    for k in 0..blocks.min(BUFFER_BLOCKS_MAX) {
        let block = u64_of(h, desc, D_RECORDS.wrapping_add(k.wrapping_mul(16)))?;
        let first = u16_of(h, block, B_FIRST)?;
        let slots = u16_of(h, block, B_SLOTS)?;
        for s in 0..slots {
            f(first.wrapping_add(s), block, s)?;
        }
    }
    Ok(())
}
