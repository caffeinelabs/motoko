//! Writes of buffered rows: the insert into the open group's tail block (with the seal of a full group and new
//! tail blocks), updates (in place, moved to the tail block, or as a long row image), and deletes, each keeping
//! the integer leaves' totals and the group's extremes.

use crate::consts::{BLOCK, BUFFER_BLOCKS_MAX, GROUP_BYTES, GROUP_LARGE, GROUP_OOL_BYTES, GROUP_SLOTS, ROW_INLINE_MAX};
use crate::fault::{rt, Corruption, Fault};
use crate::mem::{fill, put, put_u16, put_u32, put_u64, rd_u16, rd_u32, rd_u64, PageHost, Region};

use super::buffer::{
    self, Located, RowLoc, Slot, B_ROWS_END, B_SLOTS, B_SLOTS_START, DESC_BIT, D_BLOCKS, D_RECORDS, H_LIVE,
    H_OPEN_INLINE, H_OPEN_LARGE, H_OPEN_MEDIUM, H_OPEN_POS, LONG_STUB, MOVED_STUB, SLOT, SLOT_DEAD, SLOT_LONG,
    SLOT_MOVED,
};
use super::cell::Val;
use super::cur;
use super::drain;
use super::group;
use super::hot::{RunCursor, Sign};
use super::layout::{LeafInfo, Shape};
use super::long::{self, LongOut, Meta};
use super::root;
use super::row::{self, Item, Plan, RowRef, StackOut, Stored, Walk};
use super::totals::{self, At, TSrc};
use super::RowSource;

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
        if run_at == At::None || !l.live() {
            walk.skip(&l);
            continue;
        }
        let vi = walk.var;
        let s = {
            let row = RowRef::new(&*h, loc)?;
            match walk.next(shape, &row, &l)? {
                Stored::Absent => super::defaults::of(&*h, shape, &l)?,
                st => cur::from_stored(&row, shape, &l, &st, vi)?,
            }
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
    let mut so = StackOut::new();
    let mut filled = filled;
    if stub.is_none() {
        filled = row::write(shape, h, tbl, uid, src, p, &mut so, None)?;
        filled = filled.saturating_add(so.filled);
    }
    let pos = buffer::u16_of(&*h, tail, H_OPEN_POS)?;
    let rid = g.checked_mul(GROUP_SLOTS).and_then(|x| x.checked_add(pos)).ok_or(rt(Corruption::Range, g))?;
    {
        let b = h.get_mut(Region::Block(tail))?;
        let end = u64::from(rd_u16(b, B_ROWS_END)?);
        let slots = u64::from(rd_u16(b, B_SLOTS)?);
        match stub {
            Some(s) => put(b, end, &s)?,
            None => {
                put(b, end, so.bytes())?;
                fill(b, end.wrapping_add(len), reserve(len).wrapping_sub(len), 0)?;
            }
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
            if !l.live() {
                continue;
            }
            let cur = match st {
                Stored::Absent => super::defaults::of(&*h, shape, &l)?,
                st => cur::from_stored(&row, shape, &l, &st, vi)?,
            };
            let v = row::cell_of(src, &l)?;
            if !cur::same(&*h, &l, &cur, &v)? {
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
            if run_at == At::None || !l.live() {
                walk.skip(&l);
                continue;
            }
            let vi = walk.var;
            let (olds, v) = {
                let row = RowRef::new(&*h, old)?;
                let st = walk.next(shape, &row, &l)?;
                let cur = match st {
                    Stored::Absent => super::defaults::of(&*h, shape, &l)?,
                    st => cur::from_stored(&row, shape, &l, &st, vi)?,
                };
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
        let mut so = StackOut::new();
        filled = row::write(shape, h, tbl, uid, src, &p, &mut so, Some(old))?.saturating_add(so.filled);
        released(h, tbl, shape, src, old)?;
        let (block, off, _) = old_inline.ok_or(rt(Corruption::Link, 0))?;
        let b = h.get_mut(Region::Block(block))?;
        put(b, off, so.bytes())?;
        let len16 = u16::try_from(p.len).map_err(|_| rt(Corruption::Range, p.len))?;
        if slot.flags & SLOT_MOVED != 0 {
            put_u16(h.get_mut(Region::Block(at.block))?, slot.off.wrapping_add(10), len16)?;
        } else {
            buffer::write_slot(h.get_mut(Region::Block(at.block))?, at.slot, &Slot { len: p.len, ..slot })?;
        }
        new_loc = RowLoc::Inline { block, off, len: p.len };
    } else if !long && room(&*h, tail)? >= reserve(p.len) {
        // Moved to the open group's tail block; the slot names it through a moved stub.
        let mut so = StackOut::new();
        filled = row::write(shape, h, tbl, uid, src, &p, &mut so, Some(old))?.saturating_add(so.filled);
        released(h, tbl, shape, src, old)?;
        let end = buffer::u16_of(&*h, tail, B_ROWS_END)?;
        {
            let b = h.get_mut(Region::Block(tail))?;
            put(b, end, so.bytes())?;
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
        let keep = l.live()
            && match row::cell_of(src, &l)? {
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
