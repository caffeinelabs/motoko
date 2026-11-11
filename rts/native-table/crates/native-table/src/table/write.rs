//! Writes of buffered rows: the insert into the open group's tail block (with the seal of a full group and new
//! tail blocks), updates (in place, moved to the tail block, or as a long row image), and deletes, each keeping
//! the integer leaves' totals and the group's extremes.

use crate::consts::{BLOCK, BUFFER_BLOCKS_MAX, GROUP_BYTES, GROUP_LARGE, GROUP_OOL_BYTES, GROUP_SLOTS, ROW_INLINE_MAX};
use crate::fault::{rt, Corruption, Fault};
use crate::mem::{fill, put, put_u16, put_u32, put_u64, rd_u16, rd_u32, rd_u64, sub, sub_mut, PageHost, Region};

use super::buffer::{
    self, Located, RowLoc, Slot, B_ROWS_END, B_SLOTS, B_SLOTS_START, DESC_BIT, D_BLOCKS, D_RECORDS, H_LIVE,
    H_OPEN_INLINE, H_OPEN_LARGE, H_OPEN_MEDIUM, H_OPEN_POS, LONG_STUB, MOVED_STUB, SLOT, SLOT_DEAD, SLOT_LONG,
    SLOT_MOVED,
};
use super::cell::{self, Val};
use super::cur;
use super::drain;
use super::group;
use super::hot::{RunCursor, Sign};
use super::layout::{LeafInfo, Shape, Ty};
use super::long::{self, LongOut, Meta};
use super::root;
use super::row::{self, Item, Plan, RowRef, Stored, Walk};
use super::totals::{self, At, TSrc};
use super::{CellIn, RowSource};

/// Bytes a stored inline row image reserves in its block: 8-aligned and at least a long stub, so that its
/// place can always hold the stub a later update leaves there.
pub fn reserve(len: u64) -> u64 {
    group::align8(len).max(LONG_STUB)
}

/// Whether a planned row image is stored as a long row image: longer than `ROW_INLINE_MAX`, or too long for
/// an empty buffer block of this schema.
pub fn long_row(shape: &Shape, p: &Plan) -> bool {
    p.len > ROW_INLINE_MAX || reserve(p.len).wrapping_add(SLOT) > BLOCK.wrapping_sub(shape.hot_h)
}

// ---------------------------------------------------------------- totals

/// Applies one leaf's value of a buffered row (`tv`, where the total reads it) to the leaf's total, when it
/// has one (`run_at`), and to the extremes of the row's group (block `ext`).
#[allow(clippy::too_many_arguments)]
fn total_leaf<H: PageHost>(
    h: &mut H,
    tbl: u64,
    tail: u64,
    run_at: At,
    l: &LeafInfo,
    tv: TSrc<'_>,
    sign: Sign,
    ext: u64,
) -> Result<(), Fault> {
    match run_at {
        At::None => Ok(()),
        at => totals::apply(h, tbl, tail, at, l, tv, sign, ext),
    }
}

/// Applies a whole stored buffered row (every live integer leaf, under the overlay) to the totals and to the
/// extremes of its group (block `ext`) with `sign`; the other leaves are passed without being read.
fn totals_row<H: PageHost>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    tail: u64,
    loc: RowLoc,
    sign: Sign,
    ext: u64,
) -> Result<(), Fault> {
    let mut rc = RunCursor::open(&*h, tail)?;
    let mut walk = {
        let row = RowRef::new(&*h, loc)?;
        Walk::of(shape, &row)?
    };
    for o in 0..shape.leaves {
        let l = shape.leaf(&*h, o)?;
        let run_at = rc.place(h.get(Region::Block(tail))?, &l)?;
        if run_at == At::None {
            walk.skip(&l);
            continue;
        }
        let vi = walk.var;
        let s = {
            let row = RowRef::new(&*h, loc)?;
            let st = walk.next(shape, &row, &l)?;
            cur::from_stored(&row, shape, &l, &st, vi)?
        };
        total_leaf(h, tbl, tail, run_at, &l, TSrc::Cur(&s), sign, ext)?;
    }
    Ok(())
}

// ---------------------------------------------------------------- the seal of a full group and tail blocks

/// Closes the open group, which is full: its descriptor takes its counters, the next group number opens with a
/// new descriptor and tail block, and the full group is sealed (`seal::seal`).
pub fn freeze<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape) -> Result<(), Fault> {
    let (g, desc, tail) = buffer::open_group(&*h, tbl)?;
    let pos = buffer::u16_of(&*h, tail, H_OPEN_POS)?;
    let inline = buffer::u32_of(&*h, tail, H_OPEN_INLINE)?;
    let medium = buffer::u32_of(&*h, tail, H_OPEN_MEDIUM)?;
    let large = buffer::u16_of(&*h, tail, H_OPEN_LARGE)?;
    let blocks = buffer::u16_of(&*h, desc, D_BLOCKS)?;
    let tail_slots = buffer::u16_of(&*h, tail, B_SLOTS)?;
    let w16 = |x: u64| u16::try_from(x).map_err(|_| rt(Corruption::Range, x));
    let w32 = |x: u64| u32::try_from(x).map_err(|_| rt(Corruption::Range, x));
    // The descriptor's counters.
    let open_long = buffer::u64_of(&*h, tail, buffer::H_FLAGS)? & buffer::OPEN_LONG != 0;
    {
        let d = h.get_mut(Region::Block(desc))?;
        let flags = rd_u16(d, buffer::D_FLAGS)?;
        let long = if open_long { buffer::LONG_ROWS } else { 0 };
        put_u16(d, buffer::D_FLAGS, flags | long)?;
        put_u16(d, buffer::D_POSITIONS, w16(pos)?)?;
        put_u32(d, buffer::D_INLINE, w32(inline)?)?;
        put_u16(d, buffer::D_LARGE, w16(large)?)?;
        put_u32(d, buffer::D_MEDIUM, w32(medium)?)?;
        let last = D_RECORDS.wrapping_add(blocks.saturating_sub(1).wrapping_mul(16));
        put_u16(d, last.wrapping_add(10), w16(tail_slots)?)?;
    }
    // The next group.
    let g2 = root::u64_at(&*h, tbl, root::NEXT_GROUP)?;
    if g2 <= g || g2 > crate::consts::GROUP_MAX {
        return Err(rt(Corruption::Range, g2));
    }
    root::set_u64(h, tbl, root::NEXT_GROUP, g2.wrapping_add(1))?;
    root::set_u64(h, tbl, root::OPEN_GROUP, g2)?;
    let nb = buffer::new_block(h, tbl, shape, g2, 0, 0, Some(tail))?;
    let d2 = buffer::new_descriptor(h, tbl, g2, nb)?;
    let ext = super::extremes::create(h, tbl, shape, g2)?;
    put_u64(h.get_mut(Region::Block(d2))?, buffer::D_EXT, ext)?;
    {
        let b = h.get_mut(Region::Block(nb))?;
        put_u64(b, buffer::B_DESC, d2)?;
        put_u16(b, H_OPEN_POS, 0)?;
        put_u32(b, H_OPEN_INLINE, 0)?;
        put_u32(b, H_OPEN_MEDIUM, 0)?;
        put_u16(b, H_OPEN_LARGE, 0)?;
        let f = rd_u64(b, buffer::H_FLAGS)?;
        put_u64(b, buffer::H_FLAGS, f & !buffer::OPEN_LONG)?;
    }
    root::set_group(h, tbl, g2, DESC_BIT | d2)?;
    root::set_u64(h, tbl, root::TAIL, nb)?;
    super::seal::seal(h, tbl, shape, desc)
}

/// Opens a new tail block for the open group, the hot state copied forward.
fn new_tail<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape) -> Result<u64, Fault> {
    let (g, desc, tail) = buffer::open_group(&*h, tbl)?;
    let pos = buffer::u16_of(&*h, tail, H_OPEN_POS)?;
    let blocks = buffer::u16_of(&*h, desc, D_BLOCKS)?;
    if blocks >= BUFFER_BLOCKS_MAX {
        return Err(rt(Corruption::Count, desc));
    }
    let tail_slots = buffer::u16_of(&*h, tail, B_SLOTS)?;
    let nb = buffer::new_block(h, tbl, shape, g, pos, desc, Some(tail))?;
    {
        let d = h.get_mut(Region::Block(desc))?;
        let last = D_RECORDS.wrapping_add(blocks.saturating_sub(1).wrapping_mul(16));
        put_u16(d, last.wrapping_add(10), u16::try_from(tail_slots).map_err(|_| rt(Corruption::Range, tail_slots))?)?;
        let rec = D_RECORDS.wrapping_add(blocks.wrapping_mul(16));
        put_u64(d, rec, nb)?;
        put_u16(d, rec.wrapping_add(8), u16::try_from(pos).map_err(|_| rt(Corruption::Range, pos))?)?;
        put_u16(d, rec.wrapping_add(10), 0)?;
        put_u32(d, rec.wrapping_add(12), 0)?;
        put_u16(d, D_BLOCKS, u16::try_from(blocks.wrapping_add(1)).unwrap_or(u16::MAX))?;
    }
    root::set_u64(h, tbl, root::TAIL, nb)?;
    Ok(nb)
}

/// Free bytes of a buffer block between its rows and its slot array.
fn room<H: PageHost + ?Sized>(h: &H, b: u64) -> Result<u64, Fault> {
    let end = buffer::u16_of(h, b, B_ROWS_END)?;
    let start = buffer::u16_of(h, b, B_SLOTS_START)?;
    Ok(start.saturating_sub(end))
}

// ---------------------------------------------------------------- insert

/// Appends a new row to the open group (sealing it first when the row would pass what fills it, or opening a
/// new tail block, when needed); returns its rid and the bytes the host copied from the argument.
pub fn insert<H: PageHost, R: RowSource + ?Sized>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    src: &mut R,
    p: &Plan,
) -> Result<(u64, u64), Fault> {
    let long = long_row(shape, p);
    let place = if long { LONG_STUB } else { reserve(p.len) };
    let need = place.wrapping_add(SLOT);
    // What fills the group, tested before the row is placed.
    {
        let (_, desc, tail) = buffer::open_group(&*h, tbl)?;
        let pos = buffer::u16_of(&*h, tail, H_OPEN_POS)?;
        let inline = buffer::u32_of(&*h, tail, H_OPEN_INLINE)?;
        let medium = buffer::u32_of(&*h, tail, H_OPEN_MEDIUM)?;
        let large = buffer::u16_of(&*h, tail, H_OPEN_LARGE)?;
        let blocks = buffer::u16_of(&*h, desc, D_BLOCKS)?;
        let fits = room(&*h, tail)? >= need;
        let pass = pos.wrapping_add(1) > GROUP_SLOTS
            || inline.saturating_add(p.inline) > GROUP_BYTES
            || medium.saturating_add(p.medium) > GROUP_OOL_BYTES
            || large.saturating_add(p.large) > GROUP_LARGE
            || (!fits && blocks >= BUFFER_BLOCKS_MAX);
        if pos > 0 && pass {
            freeze(h, tbl, shape)?;
        }
    }
    let (g, desc, mut tail) = buffer::open_group(&*h, tbl)?;
    if room(&*h, tail)? < need {
        tail = new_tail(h, tbl, shape)?;
    }
    let uid = root::u64_at(&*h, tbl, root::UID)?;
    // The row image, then its place in the tail block.
    let (stub, len, flags, filled) = if long {
        let meta = Meta { uid, schema_hash: shape.hash, content: long::CONTENT_ROW, leaf: 0 };
        let mut lo = LongOut::begin(h, tbl, &meta, p.len)?;
        let f = row::write(shape, h, tbl, uid, src, p, &mut lo, None)?;
        let f = f.saturating_add(lo.filled);
        let img = lo.finish(h)?;
        let mut s = [0u8; 32];
        put_u32(&mut s, 0, 1)?;
        img.put_ref(&mut s, 8)?;
        (Some(s), LONG_STUB, SLOT_LONG, f)
    } else {
        (None, p.len, 0, 0)
    };
    let mut filled = filled;
    if stub.is_none() {
        // The image goes into the tail block's free space, which holds `need` bytes.
        let end = buffer::u16_of(&*h, tail, B_ROWS_END)?;
        filled = row::staged(p.len, |so| {
            let f = row::write(shape, h, tbl, uid, src, p, so, None)?;
            put(h.get_mut(Region::Block(tail))?, end, so.bytes())?;
            Ok::<_, Fault>(f.saturating_add(so.filled))
        })?;
    }
    let pos = buffer::u16_of(&*h, tail, H_OPEN_POS)?;
    let rid = g.checked_mul(GROUP_SLOTS).and_then(|x| x.checked_add(pos)).ok_or(rt(Corruption::Range, g))?;
    {
        let b = h.get_mut(Region::Block(tail))?;
        let end = u64::from(rd_u16(b, B_ROWS_END)?);
        let slots = u64::from(rd_u16(b, B_SLOTS)?);
        match stub {
            Some(s) => put(b, end, &s)?,
            None => fill(b, end.wrapping_add(len), reserve(len).wrapping_sub(len), 0)?,
        }
        let resv = if stub.is_some() { LONG_STUB } else { reserve(len) };
        buffer::write_slot(b, slots, &Slot { off: end, len, flags })?;
        let w16 = |x: u64| u16::try_from(x).map_err(|_| rt(Corruption::Range, x));
        put_u16(b, B_ROWS_END, w16(end.wrapping_add(resv))?)?;
        put_u16(b, B_SLOTS, w16(slots.wrapping_add(1))?)?;
        put_u16(b, B_SLOTS_START, w16(buffer::slot_at(slots))?)?;
        // The hot state's counters.
        put_u16(b, H_OPEN_POS, w16(pos.wrapping_add(1))?)?;
        let add32 = |b: &mut [u8], off: u64, v: u64| -> Result<(), Fault> {
            let x = u64::from(rd_u32(b, off)?).saturating_add(v);
            put_u32(b, off, u32::try_from(x).unwrap_or(u32::MAX))
        };
        add32(b, H_OPEN_INLINE, p.inline)?;
        add32(b, H_OPEN_MEDIUM, p.medium)?;
        let large = u64::from(rd_u16(b, H_OPEN_LARGE)?).saturating_add(p.large);
        put_u16(b, H_OPEN_LARGE, u16::try_from(large).unwrap_or(u16::MAX))?;
        let live = rd_u64(b, H_LIVE)?;
        put_u64(b, H_LIVE, live.wrapping_add(1))?;
        let buffered = rd_u64(b, buffer::H_BUFFERED)?;
        put_u64(b, buffer::H_BUFFERED, buffered.wrapping_add(1))?;
        let app = rd_u64(b, buffer::H_APPENDED)?;
        put_u64(b, buffer::H_APPENDED, app.wrapping_add(1))?;
        if long || p.long {
            let f = rd_u64(b, buffer::H_FLAGS)?;
            put_u64(b, buffer::H_FLAGS, f | buffer::OPEN_LONG)?;
        }
    }
    let loc = buffer::slot_row(&*h, tail, buffer::u16_of(&*h, tail, B_SLOTS)?.wrapping_sub(1))?
        .ok_or(rt(Corruption::Link, tail))?;
    let ext = super::extremes::of_desc(&*h, desc)?;
    totals_row(h, tbl, shape, tail, loc, Sign::Add, ext)?;
    Ok((rid, filled))
}

// ---------------------------------------------------------------- update of a buffered row

/// Updates a buffered row to the source's values: every live leaf whose value differs. Returns whether
/// anything changed and the bytes the host copied from the argument.
pub fn update<H: PageHost, R: RowSource + ?Sized>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    src: &mut R,
    at: &Located,
) -> Result<(bool, u64), Fault> {
    let tail = root::u64_at(&*h, tbl, root::TAIL)?;
    let old = buffer::slot_row(&*h, at.block, at.slot)?.ok_or(rt(Corruption::Link, at.block))?;
    let mut key = crate::key::LenSink(0);
    let p = row::plan(shape, &*h, src, &mut key)?;
    // Which leaves change; a key leaf never does: refused before anything is written.
    let mut changed = false;
    {
        let row = RowRef::new(&*h, old)?;
        let mut walk = Walk::of(shape, &row)?;
        for o in 0..shape.leaves {
            let l = shape.leaf(&*h, o)?;
            let vi = walk.var;
            let st = walk.next(shape, &row, &l)?;
            let cur = cur::from_stored(&row, shape, &l, &st, vi)?;
            let kept = R::KEEPS && src.kept(l.id);
            let v = row::cell_of(src, &l)?;
            if !kept && !cur::same(&*h, &l, &cur, &v)? {
                if l.key {
                    return Err(Fault::BadValue { leaf: l.id });
                }
                changed = true;
            }
        }
    }
    if !changed {
        return Ok((false, 0));
    }
    // The changed integer leaves' totals and the group's extremes move by difference, before the image moves.
    {
        let ext = super::extremes::of_desc(&*h, at.desc)?;
        let mut walk = {
            let row = RowRef::new(&*h, old)?;
            Walk::of(shape, &row)?
        };
        let mut rc = RunCursor::open(&*h, tail)?;
        for o in 0..shape.leaves {
            let l = shape.leaf(&*h, o)?;
            let run_at = rc.place(h.get(Region::Block(tail))?, &l)?;
            if run_at == At::None {
                walk.skip(&l);
                continue;
            }
            let vi = walk.var;
            let (olds, v) = {
                let row = RowRef::new(&*h, old)?;
                let st = walk.next(shape, &row, &l)?;
                let cur = cur::from_stored(&row, shape, &l, &st, vi)?;
                (cur, row::cell_of(src, &l)?)
            };
            if cur::same(&*h, &l, &olds, &v)? {
                continue;
            }
            total_leaf(h, tbl, tail, run_at, &l, TSrc::Cur(&olds), Sign::Remove, ext)?;
            total_leaf(h, tbl, tail, run_at, &l, TSrc::Val(&v), Sign::Add, ext)?;
        }
    }
    let uid = root::u64_at(&*h, tbl, root::UID)?;
    let long = long_row(shape, &p);
    if p.long {
        mark_long(h, tail)?;
    }
    let old_inline = match old {
        RowLoc::Inline { block, off, len } => Some((block, off, len)),
        RowLoc::Long(_) => None,
    };
    let filled;
    let slot = buffer::read_slot(h.get(Region::Block(at.block))?, at.slot)?;
    let new_loc;
    if !long && old_inline.is_some_and(|(_, _, len)| reserve(p.len) <= reserve(len)) {
        // In place.
        let (block, off, _) = old_inline.ok_or(rt(Corruption::Link, 0))?;
        filled = row::staged(p.len, |so| {
            let f = row::write(shape, h, tbl, uid, src, &p, so, Some(old))?.saturating_add(so.filled);
            released(h, tbl, shape, src, old)?;
            put(h.get_mut(Region::Block(block))?, off, so.bytes())?;
            Ok::<_, Fault>(f)
        })?;
        let len16 = u16::try_from(p.len).map_err(|_| rt(Corruption::Range, p.len))?;
        if slot.flags & SLOT_MOVED != 0 {
            put_u16(h.get_mut(Region::Block(at.block))?, slot.off.wrapping_add(10), len16)?;
        } else {
            buffer::write_slot(h.get_mut(Region::Block(at.block))?, at.slot, &Slot { len: p.len, ..slot })?;
        }
        new_loc = RowLoc::Inline { block, off, len: p.len };
    } else if !long && room(&*h, tail)? >= reserve(p.len) {
        // Moved to the open group's tail block; the slot names it through a moved stub.
        let end = buffer::u16_of(&*h, tail, B_ROWS_END)?;
        filled = row::staged(p.len, |so| {
            let f = row::write(shape, h, tbl, uid, src, &p, so, Some(old))?.saturating_add(so.filled);
            released(h, tbl, shape, src, old)?;
            put(h.get_mut(Region::Block(tail))?, end, so.bytes())?;
            Ok::<_, Fault>(f)
        })?;
        {
            let b = h.get_mut(Region::Block(tail))?;
            fill(b, end.wrapping_add(p.len), reserve(p.len).wrapping_sub(p.len), 0)?;
            put_u16(
                b,
                B_ROWS_END,
                u16::try_from(end.wrapping_add(reserve(p.len))).map_err(|_| rt(Corruption::Range, end))?,
            )?;
        }
        let mut stub = [0u8; 16];
        put_u64(&mut stub, 0, tail)?;
        put_u16(&mut stub, 8, u16::try_from(end).map_err(|_| rt(Corruption::Range, end))?)?;
        put_u16(&mut stub, 10, u16::try_from(p.len).map_err(|_| rt(Corruption::Range, p.len))?)?;
        let b = h.get_mut(Region::Block(at.block))?;
        put(b, slot.off, &stub)?;
        buffer::write_slot(b, at.slot, &Slot { off: slot.off, len: MOVED_STUB, flags: SLOT_MOVED })?;
        new_loc = RowLoc::Inline { block: tail, off: end, len: p.len };
    } else {
        // A long row image; the slot holds its long stub.
        mark_long(h, tail)?;
        let meta = Meta { uid, schema_hash: shape.hash, content: long::CONTENT_ROW, leaf: 0 };
        let mut lo = LongOut::begin(h, tbl, &meta, p.len)?;
        filled = row::write(shape, h, tbl, uid, src, &p, &mut lo, Some(old))?.saturating_add(lo.filled);
        released(h, tbl, shape, src, old)?;
        let img = lo.finish(h)?;
        let mut stub = [0u8; 32];
        put_u32(&mut stub, 0, 1)?;
        img.put_ref(&mut stub, 8)?;
        let b = h.get_mut(Region::Block(at.block))?;
        put(b, slot.off, &stub)?;
        buffer::write_slot(b, at.slot, &Slot { off: slot.off, len: LONG_STUB, flags: SLOT_LONG })?;
        new_loc = RowLoc::Long(img);
    }
    let _ = new_loc;
    Ok((true, filled))
}

/// After an update's new row image is written, before the old one is overwritten: the old image's long values
/// that the new one does not keep (`row::write` reuses a long value whose bytes it keeps) are freed, and the old
/// image itself when it is a long row image.
fn released<H: PageHost, R: RowSource + ?Sized>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    src: &mut R,
    old: RowLoc,
) -> Result<(), Fault> {
    let mut walk = {
        let row = RowRef::new(&*h, old)?;
        Walk::of(shape, &row)?
    };
    for o in 0..shape.leaves {
        let l = shape.leaf(&*h, o)?;
        let st = {
            let row = RowRef::new(&*h, old)?;
            match walk.next(shape, &row, &l)? {
                Stored::Item(Item::Long { img, len }) => Some((img, len)),
                _ => None,
            }
        };
        let Some((img, len)) = st else { continue };
        if R::KEEPS && src.kept(l.id) {
            continue;
        }
        let keep = match row::cell_of(src, &l)? {
            Val::Bytes(b) | Val::Big(b) => {
                cur::same_bytes(len, b, |at, dst| long::read(&*h, img, at, dst))? && row::is_long(len)
            }
            _ => false,
        };
        if !keep {
            drain::free_image(h, tbl, &img)?;
        }
    }
    if let RowLoc::Long(img) = old {
        drain::free_image(h, tbl, &img)?;
    }
    Ok(())
}

// ---------------------------------------------------------------- update of some leaves of a buffered row

/// Bytes of a record of `Merged`: the leaf id u32, its kind u32, then its value (16 bytes) or the place of its
/// bytes in the record buffer (offset u64, length u64).
const REC: u64 = 24;
const K_SOURCE: u32 = 0;
const K_NULL: u32 = 1;
const K_UNIT: u32 = 2;
const K_BOOL: u32 = 3;
const K_U64: u32 = 4;
const K_I64: u32 = 5;
const K_U128: u32 = 6;
const K_I128: u32 = 7;
const K_F32: u32 = 8;
const K_F64: u32 = 9;
const K_WIDE_NAT: u32 = 10;
const K_WIDE_INT: u32 = 11;
const K_BYTES: u32 = 12;
const K_NAT: u32 = 13;
const K_INT: u32 = 14;
const K_KEPT: u32 = 15;

/// The cells of an update of some leaves of a buffered row: the leaves it names from its source, every other
/// leaf as the row holds it, from a record per leaf (in ordinal order) made before the write (`merged`). A long
/// Text or Blob the update does not name is kept as it is, unless it is a key leaf, which the write encodes.
pub struct Merged<'a, R: ?Sized> {
    src: &'a mut R,
    buf: &'a [u8],
    n: u64,
    /// The record last asked for: a write asks for the leaves in ordinal order.
    at: u64,
    leb: [u8; 19],
}

impl<R: ?Sized> Merged<'_, R> {
    /// The offset of leaf `leaf`'s record.
    fn find(&mut self, leaf: u32) -> Result<u64, Fault> {
        for _ in 0..self.n {
            if rd_u32(self.buf, self.at.wrapping_mul(REC))? == leaf {
                return Ok(self.at.wrapping_mul(REC));
            }
            self.at = if self.at.wrapping_add(1) < self.n { self.at.wrapping_add(1) } else { 0 };
        }
        Err(Fault::BadRef)
    }
}

impl<R: RowSource + ?Sized> RowSource for Merged<'_, R> {
    fn cell(&mut self, leaf: u32) -> Result<CellIn<'_>, Fault> {
        let r = self.find(leaf)?;
        let b = self.buf;
        let (lo, hi) = (rd_u64(b, r.wrapping_add(8))?, rd_u64(b, r.wrapping_add(16))?);
        let x = u128::from(lo) | u128::from(hi) << 64;
        let bytes = || sub(b, lo, hi);
        Ok(match rd_u32(b, r.wrapping_add(4))? {
            K_SOURCE => return self.src.cell(leaf),
            K_NULL => CellIn::Null,
            K_UNIT => CellIn::Unit,
            K_BOOL => CellIn::Bool(lo != 0),
            K_U64 => CellIn::U64(lo),
            K_I64 => CellIn::I64(lo as i64),
            K_U128 => CellIn::U128(x),
            K_I128 => CellIn::I128(x as i128),
            K_F32 => CellIn::F32(f32::from_bits(u32::try_from(lo).unwrap_or(0))),
            K_F64 => CellIn::F64(f64::from_bits(lo)),
            K_WIDE_NAT => {
                let n = cell::put_uleb128(&mut self.leb, x);
                CellIn::Nat(self.leb.get(..n).unwrap_or(&[]))
            }
            K_WIDE_INT => {
                let n = cell::put_sleb128(&mut self.leb, x as i128);
                CellIn::Int(self.leb.get(..n).unwrap_or(&[]))
            }
            K_BYTES => CellIn::Bytes(bytes()?),
            K_NAT => CellIn::Nat(bytes()?),
            K_INT => CellIn::Int(bytes()?),
            K_KEPT => CellIn::Arg { len: lo },
            k => return Err(rt(Corruption::Encoding, u64::from(k))),
        })
    }

    fn fill(&mut self, leaf: u32, at: u64, dst: &mut [u8]) -> Result<(), Fault> {
        self.src.fill(leaf, at, dst)
    }

    const KEEPS: bool = true;

    fn kept(&mut self, leaf: u32) -> bool {
        self.find(leaf).and_then(|r| rd_u32(self.buf, r.wrapping_add(4))).is_ok_and(|k| k == K_KEPT)
    }
}

/// The scratch bytes `merged` takes for the buffered row `at`: a record per leaf, a copy of the row image, and
/// the bytes of the long Nat and Int values and long key values it holds.
pub fn merged_need<H: PageHost + ?Sized>(h: &H, shape: &Shape, at: &Located) -> Result<usize, Fault> {
    let row = RowRef::new(h, buffer::slot_row(h, at.block, at.slot)?.ok_or(rt(Corruption::Link, at.block))?)?;
    let mut n = shape.leaves.wrapping_mul(REC).wrapping_add(row.len);
    let mut f = [0u8; 2];
    row.read(4, &mut f)?;
    if u16::from_le_bytes(f) & row::ROW_LONG != 0 {
        let mut walk = Walk::of(shape, &row)?;
        for o in 0..shape.leaves {
            let l = shape.leaf(h, o)?;
            if let Stored::Item(Item::Long { len, .. }) = walk.next(shape, &row, &l)? {
                if l.key || matches!(l.ty, Ty::Nat | Ty::Int) {
                    n = n.saturating_add(len);
                }
            }
        }
    }
    usize::try_from(n).map_err(|_| Fault::Scratch { need: n })
}

/// The cells of an update of the leaves `leaves` names in the buffered row `at` to the cells of `src`
/// (`Merged`), its records made in `buf`, `merged_need` bytes, from the row as it is stored.
pub fn merged<'a, H: PageHost + ?Sized, R: RowSource + ?Sized>(
    h: &H,
    shape: &Shape,
    at: &Located,
    leaves: &[u32],
    src: &'a mut R,
    buf: &'a mut [u8],
) -> Result<Merged<'a, R>, Fault> {
    let row = RowRef::new(h, buffer::slot_row(h, at.block, at.slot)?.ok_or(rt(Corruption::Link, at.block))?)?;
    let img = shape.leaves.wrapping_mul(REC);
    row.read(0, sub_mut(buf, img, row.len)?)?;
    let mut more = img.wrapping_add(row.len);
    let mut walk = Walk::of(shape, &row)?;
    for o in 0..shape.leaves {
        let l = shape.leaf(h, o)?;
        let bytes = match l.ty {
            Ty::Nat => K_NAT,
            Ty::Int => K_INT,
            _ => K_BYTES,
        };
        let (k, x) = if leaves.contains(&l.id) {
            walk.skip(&l);
            (K_SOURCE, 0)
        } else {
            match walk.next(shape, &row, &l)? {
                Stored::Null => (K_NULL, 0),
                Stored::Unit => (K_UNIT, 0),
                Stored::Val(Val::Bool(v)) => (K_BOOL, u128::from(v)),
                Stored::Val(Val::U(v)) => (K_U64, u128::from(v)),
                Stored::Val(Val::I(v)) => (K_I64, u128::from(v as u64)),
                Stored::Val(Val::U128(v)) => (K_U128, v),
                Stored::Val(Val::I128(v)) => (K_I128, v as u128),
                Stored::Val(Val::F32(v)) => (K_F32, u128::from(v)),
                Stored::Val(Val::F64(v)) => (K_F64, u128::from(v)),
                Stored::Val(Val::Wide(v)) if bytes == K_NAT => (K_WIDE_NAT, v as u128),
                Stored::Val(Val::Wide(v)) => (K_WIDE_INT, v as u128),
                Stored::Bytes { at, len } | Stored::Item(Item::Inline { at, len }) => {
                    (bytes, u128::from(img.wrapping_add(at)) | u128::from(len) << 64)
                }
                Stored::Item(Item::Long { len, .. }) if bytes == K_BYTES && !l.key => (K_KEPT, u128::from(len)),
                Stored::Item(Item::Long { img: li, len }) => {
                    long::read(h, li, 0, sub_mut(buf, more, len)?)?;
                    more = more.wrapping_add(len);
                    (bytes, u128::from(more.wrapping_sub(len)) | u128::from(len) << 64)
                }
                _ => return Err(rt(Corruption::Encoding, o)),
            }
        };
        let r = o.wrapping_mul(REC);
        put_u32(buf, r, l.id)?;
        put_u32(buf, r.wrapping_add(4), k)?;
        put(buf, r.wrapping_add(8), &x.to_le_bytes())?;
    }
    Ok(Merged { src, buf, n: shape.leaves, at: 0, leb: [0; 19] })
}

/// A buffered row of the open group now holds a long value or is a long row image: the open group's flag in
/// the hot state, so that the free of its descriptor after its seal reads its rows.
fn mark_long<H: PageHost>(h: &mut H, tail: u64) -> Result<(), Fault> {
    let f = buffer::u64_of(&*h, tail, buffer::H_FLAGS)?;
    if f & buffer::OPEN_LONG == 0 {
        put_u64(h.get_mut(Region::Block(tail))?, buffer::H_FLAGS, f | buffer::OPEN_LONG)?;
    }
    Ok(())
}

// ---------------------------------------------------------------- deletes

/// Deletes a buffered row: its slot's dead flag, the totals and the live count; its long values are freed.
pub fn delete_buffered<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, at: &Located) -> Result<(), Fault> {
    let tail = root::u64_at(&*h, tbl, root::TAIL)?;
    let loc = buffer::slot_row(&*h, at.block, at.slot)?.ok_or(rt(Corruption::Link, at.block))?;
    let ext = super::extremes::of_desc(&*h, at.desc)?;
    totals_row(h, tbl, shape, tail, loc, Sign::Remove, ext)?;
    // Long values: the row's long items, and a long row image itself.
    let mut walk = {
        let row = RowRef::new(&*h, loc)?;
        Walk::of(shape, &row)?
    };
    for o in 0..shape.leaves {
        let l = shape.leaf(&*h, o)?;
        let st = {
            let row = RowRef::new(&*h, loc)?;
            match walk.next(shape, &row, &l)? {
                Stored::Item(Item::Long { img, .. }) => Some(img),
                _ => None,
            }
        };
        if let Some(img) = st {
            drain::free_image(h, tbl, &img)?;
        }
    }
    if let RowLoc::Long(img) = loc {
        drain::free_image(h, tbl, &img)?;
    }
    let b = h.get_mut(Region::Block(at.block))?;
    let slot = buffer::read_slot(b, at.slot)?;
    buffer::write_slot(b, at.slot, &Slot { flags: slot.flags | SLOT_DEAD, ..slot })?;
    let t = h.get_mut(Region::Block(tail))?;
    let live = rd_u64(t, H_LIVE)?;
    put_u64(t, H_LIVE, live.wrapping_sub(1))?;
    let buffered = rd_u64(t, buffer::H_BUFFERED)?;
    put_u64(t, buffer::H_BUFFERED, buffered.wrapping_sub(1))
}
