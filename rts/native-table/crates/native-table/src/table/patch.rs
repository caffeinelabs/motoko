//! Patch columns: the values of sealed rows that an update could not write in place, by leaf and position.
//!
//! **Patch directory** (area 0, zero-filled when allocated): {magic `NTPD` u32 @0; `FORMAT_TABLE` u16 @4;
//! units with patches u16 @6 (first block only); live patch bytes u32 @8 (first block only); zero u32 @12;
//! 254 unit records of 16 bytes from @16 {leaf ordinal u16, blocks u16, live entries u16, zero u16, first
//! patch block u64}; the link record @4,080 {`0xFFFF` u16, zero u16 × 3, next directory block u64}}. Each
//! directory block keeps its records sorted by leaf ordinal; a new unit goes into the last block, so every
//! block but the last holds 254 records, and a 255th unit takes a new block linked from the last.
//!
//! **Patch block** (area 0): {magic `NTPB` u32 @0; `FORMAT_TABLE` u16 @4; entries u16 @6; next block of the
//! chain u64 @8; data low u16 @16 (data grows down from 4,096); leaf ordinal u16 @18; zero @20..32; the
//! first block of a chain: the patched bitmap by position @32..544}, then entries of 8 bytes sorted by
//! position {position u16, flags u8 (bit 0 null, bit 1 long, bit 2 dead, bit 3 chain), zero u8, length u16,
//! data offset u16}.
//!
//! An entry's data is the value's VE when it fits `LONG_VALUE`; a longer value lies in its own chain of
//! area-0 blocks {next u64 @0; 4,088 bytes} (flag bit 3; data: {chain head u64, length u32}) or, past
//! `HUGE_VALUE`, in a long value of content 3 (flag bit 1; data: its 24-byte image reference). A chain or
//! long value holds the value's own bytes (Text and Blob bytes, a Nat's or Int's LEB128), so a rewrite can
//! adopt a long one as a huge value. A null is flag bit 0 with length 0.

use crate::consts::{magic, BLOCK, FORMAT_TABLE, PATCH_UNITS_PER_DIR};
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::image::{Image, REF_BYTES};
use crate::mem::{
    fill, put, put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u64, rd_u8, Owner, PageHost, Region,
};

use super::drain;

// ---- directory ----
pub const D_UNITS: u64 = 6;
pub const D_LIVE: u64 = 8;
pub const D_RECORDS: u64 = 16;
pub const D_LINK: u64 = 4_080;
pub const D_REC: u64 = 16;
const R_BLOCKS: u64 = 2;
const R_LIVE: u64 = 4;
const R_FIRST: u64 = 8;

// ---- patch block ----
pub const B_ENTRIES: u64 = 6;
pub const B_NEXT: u64 = 8;
pub const B_LOW: u64 = 16;
pub const B_LEAF: u64 = 18;
pub const B_BITMAP: u64 = 32;
/// Where the entries start: after the bitmap in a chain's first block, after the header in the others.
pub const FIRST_ENTRIES: u64 = 544;
pub const LATER_ENTRIES: u64 = 32;
pub const ENTRY: u64 = 8;

/// Entry flags.
pub const F_NULL: u8 = 1;
pub const F_LONG: u8 = 2;
pub const F_DEAD: u8 = 4;
pub const F_CHAIN: u8 = 8;

/// Value chain blocks: {next u64 @0; 4,088 data bytes}.
pub const CHAIN_DATA: u64 = 8;
pub const CHAIN_ROOM: u64 = BLOCK - CHAIN_DATA;
/// Bytes of a chain's stub and of a long value's reference in an entry's data.
pub const CHAIN_STUB: u64 = 12;

const _: () = assert!(D_RECORDS + PATCH_UNITS_PER_DIR * D_REC == D_LINK);

fn rt(kind: Corruption, at: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, at)
}

fn w16(x: u64) -> Result<u16, Fault> {
    u16::try_from(x).map_err(|_| rt(Corruption::Range, x))
}

/// One unit's record: where it lies and its fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unit {
    pub dir_block: u64,
    pub rec: u64,
    pub ordinal: u64,
    pub blocks: u64,
    pub live: u64,
    pub first: u64,
}

/// One entry of a patch block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub block: u64,
    pub at: u64,
    pub pos: u64,
    pub flags: u8,
    pub len: u64,
    pub data: u64,
}

impl Entry {
    fn read(b: &[u8], block: u64, at: u64) -> Result<Entry, Fault> {
        Ok(Entry {
            block,
            at,
            pos: u64::from(rd_u16(b, at)?),
            flags: rd_u8(b, at.wrapping_add(2))?,
            len: u64::from(rd_u16(b, at.wrapping_add(4))?),
            data: u64::from(rd_u16(b, at.wrapping_add(6))?),
        })
    }
}

/// Units of the directory whose first block is `dir`.
pub fn units<H: PageHost + ?Sized>(h: &H, dir: u64) -> Result<u64, Fault> {
    if dir == 0 {
        return Ok(0);
    }
    Ok(u64::from(rd_u16(h.get(Region::Block(dir))?, D_UNITS)?))
}

/// The directory's blocks: those `unit` searches at most.
pub fn dir_blocks<H: PageHost + ?Sized>(h: &H, dir: u64) -> Result<u64, Fault> {
    Ok(units(h, dir)?.div_ceil(PATCH_UNITS_PER_DIR))
}

/// Live patch bytes of the directory.
pub fn live_bytes<H: PageHost + ?Sized>(h: &H, dir: u64) -> Result<u64, Fault> {
    if dir == 0 {
        return Ok(0);
    }
    Ok(u64::from(rd_u32(h.get(Region::Block(dir))?, D_LIVE)?))
}

/// Records in directory block k of a directory of `total` units.
fn in_block(total: u64, k: u64) -> u64 {
    total.saturating_sub(k.saturating_mul(PATCH_UNITS_PER_DIR)).min(PATCH_UNITS_PER_DIR)
}

/// The record of leaf `ordinal`: a binary search per directory block of the chain.
pub fn unit<H: PageHost + ?Sized>(h: &H, dir: u64, ordinal: u64) -> Result<Option<Unit>, Fault> {
    if dir == 0 {
        return Ok(None);
    }
    let total = units(h, dir)?;
    let mut b = dir;
    let mut k = 0u64;
    while b != 0 {
        let s = h.get(Region::Block(b))?;
        if rd_u32(s, 0)? != magic::NTPD {
            return Err(rt(Corruption::Header, b));
        }
        let n = in_block(total, k);
        let (mut lo, mut hi) = (0u64, n);
        while lo < hi {
            let mid = lo.wrapping_add(hi.wrapping_sub(lo) / 2);
            let at = D_RECORDS.wrapping_add(mid.wrapping_mul(D_REC));
            let o = u64::from(rd_u16(s, at)?);
            match o.cmp(&ordinal) {
                core::cmp::Ordering::Equal => {
                    return Ok(Some(Unit {
                        dir_block: b,
                        rec: at,
                        ordinal,
                        blocks: u64::from(rd_u16(s, at.wrapping_add(R_BLOCKS))?),
                        live: u64::from(rd_u16(s, at.wrapping_add(R_LIVE))?),
                        first: rd_u64(s, at.wrapping_add(R_FIRST))?,
                    }))
                }
                core::cmp::Ordering::Less => lo = mid.wrapping_add(1),
                core::cmp::Ordering::Greater => hi = mid,
            }
        }
        b = rd_u64(s, D_LINK.wrapping_add(8))?;
        k = k.wrapping_add(1);
    }
    Ok(None)
}

/// Whether position `pos` has a patch entry in the unit (live or dead): its bit in the first block's bitmap.
pub fn marked<H: PageHost + ?Sized>(h: &H, u: &Unit, pos: u64) -> Result<bool, Fault> {
    if pos >= 4_096 {
        return Ok(false);
    }
    let s = h.get(Region::Block(u.first))?;
    Ok(rd_u8(s, B_BITMAP.wrapping_add(pos / 8))? >> (pos % 8) & 1 != 0)
}

fn entries_at(first: bool) -> u64 {
    if first {
        FIRST_ENTRIES
    } else {
        LATER_ENTRIES
    }
}

/// The live entry of position `pos` in block `b`, searching around a binary-search hit, since a dead and a
/// live entry of one position may share a block.
fn find_in_block<H: PageHost + ?Sized>(h: &H, b: u64, first: bool, pos: u64) -> Result<Option<Entry>, Fault> {
    let s = h.get(Region::Block(b))?;
    if rd_u32(s, 0)? != magic::NTPB {
        return Err(rt(Corruption::Header, b));
    }
    let n = u64::from(rd_u16(s, B_ENTRIES)?);
    let base = entries_at(first);
    let at = |i: u64| base.wrapping_add(i.wrapping_mul(ENTRY));
    // The first entry whose position is at least pos.
    let (mut lo, mut hi) = (0u64, n);
    while lo < hi {
        let mid = lo.wrapping_add(hi.wrapping_sub(lo) / 2);
        if u64::from(rd_u16(s, at(mid))?) < pos {
            lo = mid.wrapping_add(1);
        } else {
            hi = mid;
        }
    }
    let mut i = lo;
    while i < n {
        let e = Entry::read(s, b, at(i))?;
        if e.pos != pos {
            break;
        }
        if e.flags & F_DEAD == 0 {
            return Ok(Some(e));
        }
        i = i.wrapping_add(1);
    }
    Ok(None)
}

/// The live entry of position `pos` in a unit, or `None`.
pub fn lookup_in<H: PageHost + ?Sized>(h: &H, u: &Unit, pos: u64) -> Result<Option<Entry>, Fault> {
    if !marked(h, u, pos)? {
        return Ok(None);
    }
    let mut b = u.first;
    let mut first = true;
    let mut hops = 0u64;
    while b != 0 {
        if let Some(e) = find_in_block(h, b, first, pos)? {
            return Ok(Some(e));
        }
        b = rd_u64(h.get(Region::Block(b))?, B_NEXT)?;
        first = false;
        hops = hops.wrapping_add(1);
        if hops > u.blocks.saturating_add(1) {
            return Err(rt(Corruption::Link, u.first));
        }
    }
    Ok(None)
}

/// The live entry of leaf `ordinal` at position `pos`, or `None`.
pub fn lookup<H: PageHost + ?Sized>(h: &H, dir: u64, ordinal: u64, pos: u64) -> Result<Option<Entry>, Fault> {
    match unit(h, dir, ordinal)? {
        Some(u) => lookup_in(h, &u, pos),
        None => Ok(None),
    }
}

/// An entry's data as written: a VE value, a chain's stub, or a long value's reference.
#[derive(Clone, Copy, Debug)]
pub enum Data<'a> {
    Null,
    Ve(&'a [u8]),
    Chain { head: u64, len: u64 },
    Long { img: Image, len: u64 },
}

impl Data<'_> {
    fn flags(&self) -> u8 {
        match self {
            Data::Null => F_NULL,
            Data::Ve(_) => 0,
            Data::Chain { .. } => F_CHAIN,
            Data::Long { .. } => F_LONG,
        }
    }

    fn bytes(&self) -> u64 {
        match self {
            Data::Null => 0,
            Data::Ve(b) => b.len() as u64,
            Data::Chain { .. } => CHAIN_STUB,
            Data::Long { .. } => REF_BYTES,
        }
    }

    /// The live patch bytes the value counts for.
    fn live(&self) -> u64 {
        match self {
            Data::Null => 0,
            Data::Ve(b) => b.len() as u64,
            Data::Chain { len, .. } | Data::Long { len, .. } => *len,
        }
    }

    fn write(&self, b: &mut [u8], at: u64) -> Result<(), Fault> {
        match self {
            Data::Null => Ok(()),
            Data::Ve(v) => put(b, at, v),
            Data::Chain { head, len } => {
                put_u64(b, at, *head)?;
                put_u32(b, at.wrapping_add(8), u32::try_from(*len).map_err(|_| rt(Corruption::Range, *len))?)
            }
            Data::Long { img, .. } => img.put_ref(b, at),
        }
    }
}

/// What an entry held before an upsert replaced it, for the caller to release: a chain or a long value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Old {
    None,
    Chain { head: u64 },
    Long { img: Image },
}

/// The value an entry names out of its block, to release.
pub fn old_of<H: PageHost + ?Sized>(h: &H, e: &Entry) -> Result<Old, Fault> {
    let s = h.get(Region::Block(e.block))?;
    if e.flags & F_CHAIN != 0 {
        return Ok(Old::Chain { head: rd_u64(s, e.data)? });
    }
    if e.flags & F_LONG != 0 {
        let raw = crate::mem::sub(s, e.data, REF_BYTES)?;
        if raw.iter().all(|&x| x == 0) {
            return Ok(Old::None);
        }
        return Ok(Old::Long { img: Image::read_ref(s, e.data)? });
    }
    Ok(Old::None)
}

/// Hands a released value to the drain.
pub fn release<H: PageHost>(h: &mut H, tbl: u64, old: Old) -> Result<(), Fault> {
    match old {
        Old::None => Ok(()),
        Old::Chain { head } => drain::push(h, tbl, &drain::Rec::new(drain::K_CHAIN, head, 0)),
        Old::Long { img } => drain::push_image(h, tbl, &img),
    }
}

/// The result of an upsert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Upserted {
    /// The value the replaced entry named, which the caller releases.
    pub old: Old,
    /// The live patch bytes the replaced entry counted.
    pub old_live: u64,
    /// A unit record was added; and a directory block with it.
    pub new_unit: bool,
    pub new_dir_block: bool,
    /// The unit's chain length, the directory's units and live bytes afterwards.
    pub blocks: u64,
    pub units: u64,
    pub live_bytes: u64,
}

/// A new directory block, zero-filled, with its header.
fn new_dir<H: PageHost>(h: &mut H, tbl: u64) -> Result<u64, Fault> {
    let d = crate::area::alloc_zeroed(h, tbl, 0)?;
    let b = h.get_mut(Region::Block(d), Owner(0))?;
    put_u32(b, 0, magic::NTPD)?;
    put_u16(b, 4, FORMAT_TABLE)?;
    put_u16(b, D_LINK, 0xFFFF)?;
    Ok(d)
}

/// A new patch block of leaf `ordinal`: the first of a chain carries the zeroed bitmap.
fn new_block<H: PageHost>(h: &mut H, tbl: u64, ordinal: u64, first: bool) -> Result<u64, Fault> {
    let p = crate::area::alloc_block(h, tbl, 0)?;
    let b = h.get_mut(Region::Block(p), Owner(0))?;
    put_u32(b, 0, magic::NTPB)?;
    put_u16(b, 4, FORMAT_TABLE)?;
    put_u16(b, B_ENTRIES, 0)?;
    put_u64(b, B_NEXT, 0)?;
    put_u16(b, B_LOW, w16(BLOCK)?)?;
    put_u16(b, B_LEAF, w16(ordinal)?)?;
    fill(b, 20, 12, 0)?;
    if first {
        fill(b, B_BITMAP, FIRST_ENTRIES.wrapping_sub(B_BITMAP), 0)?;
    }
    Ok(p)
}

/// Adds a unit record for leaf `ordinal` with a new first patch block; returns it and whether a directory
/// block was added.
fn add_unit<H: PageHost>(h: &mut H, tbl: u64, dir: u64, ordinal: u64) -> Result<(Unit, bool), Fault> {
    let total = units(h, dir)?;
    // The last directory block.
    let mut last = dir;
    let mut k = 0u64;
    loop {
        let next = rd_u64(h.get(Region::Block(last))?, D_LINK.wrapping_add(8))?;
        if next == 0 {
            break;
        }
        last = next;
        k = k.wrapping_add(1);
    }
    let mut n = in_block(total, k);
    let mut new_block_made = false;
    if n >= PATCH_UNITS_PER_DIR {
        let d = new_dir(h, tbl)?;
        put_u64(h.get_mut(Region::Block(last), Owner(0))?, D_LINK.wrapping_add(8), d)?;
        last = d;
        n = 0;
        new_block_made = true;
    }
    let first = new_block(h, tbl, ordinal, true)?;
    let b = h.get_mut(Region::Block(last), Owner(0))?;
    // The insertion point in this block's sorted records.
    let (mut lo, mut hi) = (0u64, n);
    while lo < hi {
        let mid = lo.wrapping_add(hi.wrapping_sub(lo) / 2);
        if u64::from(rd_u16(b, D_RECORDS.wrapping_add(mid.wrapping_mul(D_REC)))?) < ordinal {
            lo = mid.wrapping_add(1);
        } else {
            hi = mid;
        }
    }
    let at = D_RECORDS.wrapping_add(lo.wrapping_mul(D_REC));
    crate::mem::shift(b, at, at.wrapping_add(D_REC), n.wrapping_sub(lo).wrapping_mul(D_REC))?;
    fill(b, at, D_REC, 0)?;
    put_u16(b, at, w16(ordinal)?)?;
    put_u16(b, at.wrapping_add(R_BLOCKS), 1)?;
    put_u64(b, at.wrapping_add(R_FIRST), first)?;
    put_u16(h.get_mut(Region::Block(dir), Owner(0))?, D_UNITS, w16(total.wrapping_add(1))?)?;
    Ok((Unit { dir_block: last, rec: at, ordinal, blocks: 1, live: 0, first }, new_block_made))
}

/// Upserts (leaf `ordinal`, position `pos`) → `d` into the directory named by the u64 at `holder_off` of
/// block `holder` (made when 0): a live entry of exactly the new data's length is overwritten in place;
/// otherwise it is marked dead and a new entry goes into the chain's newest block when it has room, else into
/// a new block appended to the chain. The bitmap bit is set and the counters updated.
pub fn upsert<H: PageHost>(
    h: &mut H,
    tbl: u64,
    holder: u64,
    holder_off: u64,
    ordinal: u64,
    pos: u64,
    d: &Data<'_>,
) -> Result<Upserted, Fault> {
    if pos >= 4_096 {
        return Err(rt(Corruption::Range, pos));
    }
    let mut dir = rd_u64(h.get(Region::Block(holder))?, holder_off)?;
    if dir == 0 {
        dir = new_dir(h, tbl)?;
        put_u64(h.get_mut(Region::Block(holder), Owner(0))?, holder_off, dir)?;
    }
    let (u, new_unit, new_dir_block) = match unit(h, dir, ordinal)? {
        Some(u) => (u, false, false),
        None => {
            let (u, nb) = add_unit(h, tbl, dir, ordinal)?;
            (u, true, nb)
        }
    };
    let bytes = d.bytes();
    let old_entry = lookup_in(h, &u, pos)?;
    let mut out = Upserted {
        old: Old::None,
        old_live: 0,
        new_unit,
        new_dir_block,
        blocks: u.blocks,
        units: units(h, dir)?,
        live_bytes: 0,
    };
    let mut live_delta: i64 = i64::try_from(d.live()).map_err(|_| rt(Corruption::Range, d.live()))?;
    let mut entries_delta: i64 = 1;
    if let Some(e) = old_entry {
        out.old = old_of(&*h, &e)?;
        out.old_live = entry_live(&*h, &e)?;
        live_delta = live_delta.wrapping_sub(i64::try_from(out.old_live).map_err(|_| rt(Corruption::Range, 0))?);
        entries_delta = 0;
        if e.len == bytes {
            // Overwritten in place.
            let b = h.get_mut(Region::Block(e.block), Owner(0))?;
            d.write(b, e.data)?;
            put_u8(b, e.at.wrapping_add(2), d.flags())?;
            finish_counters(h, dir, &u, live_delta, entries_delta, 0)?;
            out.live_bytes = live_bytes(&*h, dir)?;
            return Ok(out);
        }
        let b = h.get_mut(Region::Block(e.block), Owner(0))?;
        put_u8(b, e.at.wrapping_add(2), e.flags | F_DEAD)?;
    }
    // The chain's newest block.
    let mut last = u.first;
    let mut first = true;
    loop {
        let next = rd_u64(h.get(Region::Block(last))?, B_NEXT)?;
        if next == 0 {
            break;
        }
        last = next;
        first = false;
    }
    let (n, low) = {
        let s = h.get(Region::Block(last))?;
        (u64::from(rd_u16(s, B_ENTRIES)?), u64::from(rd_u16(s, B_LOW)?))
    };
    let end = entries_at(first).wrapping_add(n.wrapping_add(1).wrapping_mul(ENTRY));
    let mut added_blocks = 0u64;
    let (blk, first_blk, n, low) = if end.saturating_add(bytes) <= low {
        (last, first, n, low)
    } else {
        let nb = new_block(h, tbl, ordinal, false)?;
        put_u64(h.get_mut(Region::Block(last), Owner(0))?, B_NEXT, nb)?;
        added_blocks = 1;
        (nb, false, 0, BLOCK)
    };
    let data_at = low.wrapping_sub(bytes);
    let b = h.get_mut(Region::Block(blk), Owner(0))?;
    let base = entries_at(first_blk);
    // The insertion point: after every entry of a smaller or equal position.
    let (mut lo, mut hi) = (0u64, n);
    while lo < hi {
        let mid = lo.wrapping_add(hi.wrapping_sub(lo) / 2);
        if u64::from(rd_u16(b, base.wrapping_add(mid.wrapping_mul(ENTRY)))?) <= pos {
            lo = mid.wrapping_add(1);
        } else {
            hi = mid;
        }
    }
    let at = base.wrapping_add(lo.wrapping_mul(ENTRY));
    crate::mem::shift(b, at, at.wrapping_add(ENTRY), n.wrapping_sub(lo).wrapping_mul(ENTRY))?;
    put_u16(b, at, w16(pos)?)?;
    put_u8(b, at.wrapping_add(2), d.flags())?;
    put_u8(b, at.wrapping_add(3), 0)?;
    put_u16(b, at.wrapping_add(4), w16(bytes)?)?;
    put_u16(b, at.wrapping_add(6), w16(data_at)?)?;
    d.write(b, data_at)?;
    put_u16(b, B_ENTRIES, w16(n.wrapping_add(1))?)?;
    put_u16(b, B_LOW, w16(data_at)?)?;
    // The bitmap bit, in the chain's first block.
    let fb = h.get_mut(Region::Block(u.first), Owner(0))?;
    let bit_at = B_BITMAP.wrapping_add(pos / 8);
    let v = rd_u8(fb, bit_at)?;
    put_u8(fb, bit_at, v | (1u8 << (pos % 8)))?;
    finish_counters(h, dir, &u, live_delta, entries_delta, added_blocks)?;
    out.blocks = u.blocks.wrapping_add(added_blocks);
    out.units = units(&*h, dir)?;
    out.live_bytes = live_bytes(&*h, dir)?;
    Ok(out)
}

/// The live patch bytes an entry counts.
fn entry_live<H: PageHost + ?Sized>(h: &H, e: &Entry) -> Result<u64, Fault> {
    let s = h.get(Region::Block(e.block))?;
    if e.flags & F_CHAIN != 0 {
        return Ok(u64::from(rd_u32(s, e.data.wrapping_add(8))?));
    }
    if e.flags & F_LONG != 0 {
        if crate::mem::sub(s, e.data, REF_BYTES)?.iter().all(|&x| x == 0) {
            return Ok(0);
        }
        let img = Image::read_ref(s, e.data)?;
        return super::long::content_len(h, img);
    }
    Ok(e.len)
}

/// Writes the unit's and the directory's counters.
fn finish_counters<H: PageHost>(
    h: &mut H,
    dir: u64,
    u: &Unit,
    live_delta: i64,
    entries_delta: i64,
    added_blocks: u64,
) -> Result<(), Fault> {
    if entries_delta != 0 || added_blocks != 0 {
        let b = h.get_mut(Region::Block(u.dir_block), Owner(0))?;
        let live = u64::from(rd_u16(b, u.rec.wrapping_add(R_LIVE))?).wrapping_add_signed(entries_delta);
        put_u16(b, u.rec.wrapping_add(R_LIVE), u16::try_from(live).unwrap_or(u16::MAX))?;
        let blocks = u64::from(rd_u16(b, u.rec.wrapping_add(R_BLOCKS))?).saturating_add(added_blocks);
        put_u16(b, u.rec.wrapping_add(R_BLOCKS), u16::try_from(blocks).unwrap_or(u16::MAX))?;
    }
    if live_delta != 0 {
        let b = h.get_mut(Region::Block(dir), Owner(0))?;
        let v = u64::from(rd_u32(b, D_LIVE)?).saturating_add_signed(live_delta);
        put_u32(b, D_LIVE, u32::try_from(v).unwrap_or(u32::MAX))?;
    }
    Ok(())
}

/// Copies bytes [at, at + out.len()) of a chain's value.
pub fn chain_read<H: PageHost + ?Sized>(h: &H, head: u64, at: u64, out: &mut [u8]) -> Result<(), Fault> {
    let mut b = head;
    let mut skip = at / CHAIN_ROOM;
    while skip > 0 {
        b = rd_u64(h.get(Region::Block(b))?, 0)?;
        if b == 0 {
            return Err(rt(Corruption::Link, head));
        }
        skip = skip.wrapping_sub(1);
    }
    let mut o = at % CHAIN_ROOM;
    let mut rest = out;
    while !rest.is_empty() {
        if b == 0 {
            return Err(rt(Corruption::Link, head));
        }
        let s = h.get(Region::Block(b))?;
        let n = usize::try_from(CHAIN_ROOM.wrapping_sub(o)).unwrap_or(usize::MAX).min(rest.len());
        let (now, later) = rest.split_at_mut_checked(n).ok_or(rt(Corruption::Range, at))?;
        crate::mem::copy_bytes(now, crate::mem::sub(s, CHAIN_DATA.wrapping_add(o), n as u64)?);
        rest = later;
        o = 0;
        b = rd_u64(s, 0)?;
    }
    Ok(())
}

/// A value chain being written, block by block.
pub struct ChainOut {
    pub head: u64,
    tail: u64,
    used: u64,
    pub len: u64,
}

impl ChainOut {
    pub fn new() -> Self {
        ChainOut { head: 0, tail: 0, used: CHAIN_ROOM, len: 0 }
    }

    /// The next piece to fill: at most the rest of the current block (a new block when it is full).
    pub fn piece<'h, H: PageHost>(&mut self, h: &'h mut H, tbl: u64, want: u64) -> Result<&'h mut [u8], Fault> {
        if self.used >= CHAIN_ROOM {
            let b = crate::area::alloc_block(h, tbl, 0)?;
            put_u64(h.get_mut(Region::Block(b), Owner(0))?, 0, 0)?;
            if self.tail != 0 {
                put_u64(h.get_mut(Region::Block(self.tail), Owner(0))?, 0, b)?;
            } else {
                self.head = b;
            }
            self.tail = b;
            self.used = 0;
        }
        let n = want.min(CHAIN_ROOM.wrapping_sub(self.used));
        let at = CHAIN_DATA.wrapping_add(self.used);
        self.used = self.used.wrapping_add(n);
        self.len = self.len.wrapping_add(n);
        crate::mem::sub_mut(h.get_mut(Region::Block(self.tail), Owner(0))?, at, n)
    }

    /// Appends bytes.
    pub fn put<H: PageHost>(&mut self, h: &mut H, tbl: u64, b: &[u8]) -> Result<(), Fault> {
        let mut rest = b;
        while !rest.is_empty() {
            let dst = self.piece(h, tbl, rest.len() as u64)?;
            let n = dst.len();
            let (now, later) = rest.split_at_checked(n).ok_or(rt(Corruption::Range, 0))?;
            crate::mem::copy_bytes(dst, now);
            rest = later;
        }
        Ok(())
    }
}

impl Default for ChainOut {
    fn default() -> Self {
        Self::new()
    }
}

/// Every unit of a directory in record order: (unit). Used by rewrites and checks.
pub fn for_each_unit<H: PageHost + ?Sized, F>(h: &H, dir: u64, mut f: F) -> Result<(), Fault>
where
    F: FnMut(&Unit) -> Result<(), Fault>,
{
    if dir == 0 {
        return Ok(());
    }
    let total = units(h, dir)?;
    let mut b = dir;
    let mut k = 0u64;
    while b != 0 {
        let s = h.get(Region::Block(b))?;
        for i in 0..in_block(total, k) {
            let at = D_RECORDS.wrapping_add(i.wrapping_mul(D_REC));
            let u = Unit {
                dir_block: b,
                rec: at,
                ordinal: u64::from(rd_u16(s, at)?),
                blocks: u64::from(rd_u16(s, at.wrapping_add(R_BLOCKS))?),
                live: u64::from(rd_u16(s, at.wrapping_add(R_LIVE))?),
                first: rd_u64(s, at.wrapping_add(R_FIRST))?,
            };
            f(&u)?;
        }
        b = rd_u64(s, D_LINK.wrapping_add(8))?;
        k = k.wrapping_add(1);
    }
    Ok(())
}

/// Clears an entry's long-value reference after a rewrite adopted the value, so the drain of the old patch
/// directory does not free it.
pub fn forget_long<H: PageHost>(h: &mut H, block: u64, data: u64) -> Result<(), Fault> {
    fill(h.get_mut(Region::Block(block), Owner(0))?, data, REF_BYTES, 0)
}

/// The number of blocks of a patch block chain and of directory blocks are bounded by the u16 counters.
pub const MAX_CHAIN: u64 = u16::MAX as u64;
