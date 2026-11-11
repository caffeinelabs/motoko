//! Writes of buffered rows: the insert into the open group's tail block (with the seal of a full group and new
//! tail blocks), updates (in place, moved to the tail block, or as a long row image), and deletes, each keeping
//! the integer leaves' totals and the group's extremes.

use crate::consts::{BLOCK, BUFFER_BLOCKS_MAX, GROUP_BYTES, GROUP_LARGE, GROUP_OOL_BYTES, GROUP_SLOTS, ROW_INLINE_MAX};
use crate::fault::{rt, Corruption, Fault};
use crate::mem::{
    copy_bytes, fill, put, put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u64, rd_u8, sub, sub_mut, PageHost,
    Region,
};

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
use super::RowSource;

/// Bytes a stored inline row image reserves in its block: 8-aligned and at least a long stub, so that its
/// place can always hold the stub a later update leaves there.
pub fn reserve(len: u64) -> u64 {
    group::align8(len).max(LONG_STUB)
}

/// Whether a row image of `len` bytes is stored as a long row image: longer than `ROW_INLINE_MAX`, or too long for
/// an empty buffer block of this schema.
pub fn long_row(shape: &Shape, len: u64) -> bool {
    len > ROW_INLINE_MAX || reserve(len).wrapping_add(SLOT) > BLOCK.wrapping_sub(shape.hot_h)
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
/// extremes of its group (block `ext`) with `sign`; the other leaves are passed without being read. Kept out of
/// line: inlined into a delete, it makes the delete dearer.
#[inline(never)]
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
/// new tail block, when needed), its image built from the source's cells, which the caller has checked, on the
/// stack, or in `big` when longer than an inline row image; returns its rid and the bytes the host copied from
/// the argument.
pub fn insert<H: PageHost, R: RowSource + ?Sized>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    src: &mut R,
    p: &Plan,
    big: &mut [u8],
) -> Result<(u64, u64), Fault> {
    let long = long_row(shape, p.len);
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
    // The row image, then its place: the tail block's free space, which holds `need` bytes, or a long row image.
    let (end, ext) = (buffer::u16_of(&*h, tail, B_ROWS_END)?, super::extremes::of_desc(&*h, desc)?);
    let (stub, filled) = row::staged(p.len, big, |buf| {
        let (mut filled, totals, longs) = build(&*h, shape, src, None, None, buf)?;
        if totals || longs {
            filled = filled.saturating_add(after(h, tbl, shape, tail, ext, src, None, None, buf, longs)?);
        }
        if !long {
            put(h.get_mut(Region::Block(tail))?, end, buf)?;
            return Ok((None, filled));
        }
        let mut lo = long_out(h, tbl, shape, tail, p.len)?;
        lo.put(h, buf)?;
        let img = lo.finish(h)?;
        let mut s = [0u8; 32];
        put_u32(&mut s, 0, 1)?;
        img.put_ref(&mut s, 8)?;
        Ok((Some(s), filled))
    })?;
    let (len, flags) = if long { (LONG_STUB, SLOT_LONG) } else { (p.len, 0) };
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
    }
    Ok((rid, filled))
}

// ---------------------------------------------------------------- writes of a buffered row

/// Where an updated buffered row's new image of `len` bytes goes when it is inline: the old image's place when
/// the old image is inline and its room holds the new one, else the end of the open group's tail block `tail`
/// when it has room (block, offset, whether moved there). `None`: a long row image.
#[inline(always)]
fn inline_to<H: PageHost + ?Sized>(
    h: &H,
    shape: &Shape,
    tail: u64,
    old: RowLoc,
    len: u64,
) -> Result<Option<(u64, u64, bool)>, Fault> {
    if long_row(shape, len) {
        return Ok(None);
    }
    if let RowLoc::Inline { block, off, len: was } = old {
        if reserve(len) <= reserve(was) {
            return Ok(Some((block, off, false)));
        }
    }
    Ok(if room(h, tail)? >= reserve(len) { Some((tail, buffer::u16_of(h, tail, B_ROWS_END)?, true)) } else { None })
}

/// Writes an updated buffered row's new inline image `img` where `inline_to` placed it, and points the row's slot
/// `at` at it: in its old place, the image's new length; moved to the tail block, a moved stub.
#[inline(always)]
fn put_inline<H: PageHost>(h: &mut H, at: &Located, to: (u64, u64, bool), img: &[u8]) -> Result<(), Fault> {
    let (block, off, moved) = to;
    let len = img.len() as u64;
    let w16 = |x: u64| u16::try_from(x).map_err(|_| rt(Corruption::Range, x));
    let slot = buffer::read_slot(h.get(Region::Block(at.block))?, at.slot)?;
    let b = h.get_mut(Region::Block(block))?;
    put(b, off, img)?;
    if !moved {
        let b = h.get_mut(Region::Block(at.block))?;
        return if slot.flags & SLOT_MOVED != 0 {
            put_u16(b, slot.off.wrapping_add(10), w16(len)?)
        } else {
            buffer::write_slot(b, at.slot, &Slot { len, ..slot })
        };
    }
    fill(b, off.wrapping_add(len), reserve(len).wrapping_sub(len), 0)?;
    put_u16(b, B_ROWS_END, w16(off.wrapping_add(reserve(len)))?)?;
    let mut stub = [0u8; 16];
    put_u64(&mut stub, 0, block)?;
    put_u16(&mut stub, 8, w16(off)?)?;
    put_u16(&mut stub, 10, w16(len)?)?;
    let b = h.get_mut(Region::Block(at.block))?;
    put(b, slot.off, &stub)?;
    buffer::write_slot(b, at.slot, &Slot { off: slot.off, len: MOVED_STUB, flags: SLOT_MOVED })
}

/// A long row image of `len` content bytes being written for a buffered row of the open group (tail block
/// `tail`), whose flag it sets.
fn long_out<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, tail: u64, len: u64) -> Result<LongOut, Fault> {
    mark_long(h, tail)?;
    let uid = root::u64_at(&*h, tbl, root::UID)?;
    let meta = Meta { uid, schema_hash: shape.hash, content: long::CONTENT_ROW, leaf: 0 };
    LongOut::begin(h, tbl, &meta, len)
}

/// Ends an updated buffered row's long row image and points the row's slot `at` at it through a long stub.
fn put_long<H: PageHost>(h: &mut H, at: &Located, lo: LongOut) -> Result<(), Fault> {
    let img = lo.finish(h)?;
    let mut stub = [0u8; 32];
    put_u32(&mut stub, 0, 1)?;
    img.put_ref(&mut stub, 8)?;
    let b = h.get_mut(Region::Block(at.block))?;
    let slot = buffer::read_slot(b, at.slot)?;
    put(b, slot.off, &stub)?;
    buffer::write_slot(b, at.slot, &Slot { off: slot.off, len: LONG_STUB, flags: SLOT_LONG })
}

/// Whether a write of a stored row takes leaf `l` from its source: a leaf `leaves` names (every one when `None`),
/// but never a key leaf, which keeps its value.
#[inline(always)]
fn named(leaves: Option<&[u32]>, l: &LeafInfo) -> bool {
    !l.key && leaves.is_none_or(|ids| ids.contains(&l.id))
}

/// The bytes of the new image of the buffered row `at` with the leaves `leaves` names set to the source's cells,
/// which the caller has checked.
pub fn spliced<H: PageHost + ?Sized, R: RowSource + ?Sized>(
    h: &H,
    shape: &Shape,
    src: &mut R,
    at: &Located,
    leaves: &[u32],
) -> Result<u64, Fault> {
    let row = RowRef::new(h, buffer::slot_row(h, at.block, at.slot)?.ok_or(rt(Corruption::Link, at.block))?)?;
    let mut walk = Walk::of(shape, &row)?;
    let mut len = row.len;
    for o in 0..shape.leaves {
        let l = shape.leaf(h, o)?;
        let vi = walk.var;
        walk.skip(&l);
        if named(Some(leaves), &l) && l.ty.variable() {
            let (_, was, _) = row.item_span(&walk.head, vi)?;
            let v = row::checked_cell(src, &l)?;
            let new = row::has_item(&l, &v)
                .then(|| cell::var_len(&v, l.ty))
                .map_or(0, |n| row::item_bytes(n, row::is_long(n)));
            len = len.checked_sub(was).and_then(|x| x.checked_add(new)).ok_or(rt(Corruption::Range, len))?;
        }
    }
    Ok(len)
}

/// Writes the buffered row `at` again with the leaves `leaves` names (every leaf but the key's when `None`) set to
/// the source's cells, which the caller has checked; `len` is the new image's bytes. The new row image is the
/// stored one with those leaves' bits, cells and items replaced and every other item copied as it is, built on
/// the stack, or in the scratch when longer than an inline row image. Returns the bytes the host copied from the
/// argument.
#[allow(clippy::too_many_arguments)]
pub fn splice<H: PageHost, R: RowSource + ?Sized, S: crate::host::Scratch>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    src: &mut R,
    at: &Located,
    leaves: Option<&[u32]>,
    len: u64,
    scratch: &mut S,
) -> Result<u64, Fault> {
    let tail = root::u64_at(&*h, tbl, root::TAIL)?;
    let old = buffer::slot_row(&*h, at.block, at.slot)?.ok_or(rt(Corruption::Link, at.block))?;
    let to = inline_to(&*h, shape, tail, old, len)?;
    let big = if len > row::STAGE as u64 { scratch.get(usize::try_from(len).unwrap_or(usize::MAX))? } else { &mut [] };
    row::staged(len, big, |buf| {
        let (mut filled, totals, longs) = build(&*h, shape, src, Some(old), leaves, buf)?;
        if totals || longs {
            let ext = super::extremes::of_desc(&*h, at.desc)?;
            filled = filled.saturating_add(after(h, tbl, shape, tail, ext, src, Some(old), leaves, buf, longs)?);
        }
        if let RowLoc::Long(img) = old {
            drain::free_image(h, tbl, &img)?;
        }
        match to {
            Some(to) => put_inline(h, at, to, buf)?,
            None => {
                let mut lo = long_out(h, tbl, shape, tail, len)?;
                lo.put(h, buf)?;
                put_long(h, at, lo)?;
            }
        }
        Ok(filled)
    })
}

/// Builds a row image in `buf`, its bytes: of a new row (`old` none), every leaf's from the source's cells; of the
/// stored row `old`, its image's header, bitmaps and fixed region with the bits and cells of the leaves the write
/// takes (`named`) set, then the item table and the items, a taken leaf's from its cell, every other one copied.
/// The reference of a new long value is left zero. Returns the bytes the host copied from the argument, whether
/// a taken leaf has a total, and whether either image holds a long value: the work `after` has.
#[inline(always)]
fn build<H: PageHost + ?Sized, R: RowSource + ?Sized>(
    h: &H,
    shape: &Shape,
    src: &mut R,
    old: Option<RowLoc>,
    leaves: Option<&[u32]>,
    buf: &mut [u8],
) -> Result<(u64, bool, bool), Fault> {
    let row = old.map(|loc| RowRef::new(h, loc)).transpose()?;
    let mut walk = match &row {
        Some(row) => Walk::of(shape, row)?,
        None => Walk { fixed: shape.head.fixed_at, var: 0, bit: 0, head: shape.head },
    };
    let head = walk.head;
    let table = head.items_table_at();
    match &row {
        Some(row) => row.read(0, sub_mut(buf, 0, table)?)?,
        None => {
            fill(buf, 0, table, 0)?;
            put_u32(buf, 0, shape.version)?;
        }
    }
    let at32 = |x: u64| u32::try_from(x).map_err(|_| rt(Corruption::Range, x));
    let (mut end, mut long, mut filled, mut totals) = (head.items_at(), false, 0u64, false);
    let was_long = rd_u16(buf, 4)? & row::ROW_LONG != 0;
    for o in 0..shape.leaves {
        let l = shape.leaf(h, o)?;
        let (fixed, vi, bit) = (walk.fixed, walk.var, walk.bit);
        walk.skip(&l);
        let entry = table.wrapping_add(vi.wrapping_mul(4));
        if let Some(row) = row.as_ref().filter(|_| !named(leaves, &l)) {
            // A whole row's key leaf keeps its value: another one that encodes the same (−0.0 for 0.0) is refused.
            if leaves.is_none() {
                let cur = cur::from_stored(row, shape, &l, &row::stored_at(&head, row, &l, fixed, vi, bit)?, vi)?;
                if !cur::same(h, &l, &cur, &row::checked_cell(src, &l)?)? {
                    return Err(Fault::BadValue { leaf: l.id });
                }
            }
            if l.ty.variable() {
                let (a, n, lg) = row.item_span(&head, vi)?;
                row.read(a, sub_mut(buf, end, n)?)?;
                put_u32(buf, entry, if a == 0 { 0 } else { at32(end)? })?;
                long |= lg;
                end = end.wrapping_add(n);
            }
            continue;
        }
        totals |= l.ty.running().is_some();
        let v = row::checked_cell(src, &l)?;
        put_bit(buf, 8, bit, v.is_null())?;
        put_bit(buf, 8u64.wrapping_add(head.bitmap), bit, cell::is_big(&v))?;
        let w = l.ty.fixed_width();
        if w > 0 {
            let d = sub_mut(buf, fixed, w)?;
            match v {
                Val::Bytes(b) if w > 30 => copy_bytes(d, b),
                _ if w > 30 => d.fill(0),
                _ => {
                    let mut c = [0u8; 30];
                    let k = cell::fixed_cell(&v, l.ty, &mut c);
                    copy_bytes(d, c.get(..usize::try_from(k).unwrap_or(0)).unwrap_or(&[]));
                }
            }
        }
        if !l.ty.variable() {
            continue;
        }
        let Some(n) = row::has_item(&l, &v).then(|| cell::var_len(&v, l.ty)) else {
            put_u32(buf, entry, 0)?;
            continue;
        };
        put_u32(buf, entry, at32(end)?)?;
        if row::is_long(n) {
            // Its reference is written once the long value is made.
            fill(buf, end, row::LONG_ITEM, 0)?;
            put_u8(buf, end, row::ITEM_LONG)?;
            long = true;
            end = end.wrapping_add(row::LONG_ITEM);
            continue;
        }
        put_u8(buf, end, row::ITEM_INLINE)?;
        let k = crate::bytes::put_uleb(buf, end.wrapping_add(1), n)?;
        let d = sub_mut(buf, end.wrapping_add(1).wrapping_add(k), n)?;
        match v {
            Val::Bytes(b) | Val::Big(b) => copy_bytes(d, b),
            Val::Arg(_) => {
                for (i, piece) in d.chunks_mut(usize::try_from(BLOCK).unwrap_or(4_096)).enumerate() {
                    src.fill(l.id, (i as u64).wrapping_mul(BLOCK), piece)?;
                }
                if matches!(l.ty, Ty::Text) && core::str::from_utf8(d).is_err() {
                    return Err(Fault::BadValue { leaf: l.id });
                }
                filled = filled.saturating_add(n);
            }
            _ => {
                let mut leb = [0u8; 19];
                let k = cell::small_leb(&v, matches!(l.ty, Ty::Int), &mut leb);
                copy_bytes(d, leb.get(..k).unwrap_or(&[]));
            }
        }
        end = end.wrapping_add(1).wrapping_add(k).wrapping_add(n);
    }
    let big = sub(buf, 8u64.wrapping_add(head.bitmap), head.bitmap)?.iter().any(|&b| b != 0);
    put_u16(buf, 4, (if big { row::ROW_BIG } else { 0 }) | (if long { row::ROW_LONG } else { 0 }))?;
    if end != buf.len() as u64 {
        return Err(rt(Corruption::Count, end));
    }
    Ok((filled, totals, long || was_long))
}

/// Sets bit `bit` of the bitmap at `off` of `b` to `on`.
fn put_bit(b: &mut [u8], off: u64, bit: u64, on: bool) -> Result<(), Fault> {
    let at = off.wrapping_add(bit / 8);
    let m = 1u8 << (bit % 8);
    let x = rd_u8(b, at)?;
    put_u8(b, at, if on { x | m } else { x & !m })
}

/// After `build`, while the stored image `old` (when there is one) is as it was: each integer leaf the write
/// takes moves the totals and the group's extremes (block `ext`) from its stored value (none for a new row) to
/// its new one when they differ; when an image holds a long value (`longs`), a taken leaf's new long value is
/// made and its reference written into `buf`, unless its stored long value holds the same bytes and is kept, and
/// a stored long value not kept is freed. Returns the bytes the host copied from the argument.
#[allow(clippy::too_many_arguments)]
fn after<H: PageHost, R: RowSource + ?Sized>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    tail: u64,
    ext: u64,
    src: &mut R,
    old: Option<RowLoc>,
    leaves: Option<&[u32]>,
    buf: &mut [u8],
    longs: bool,
) -> Result<u64, Fault> {
    let uid = root::u64_at(&*h, tbl, root::UID)?;
    let mut rc = RunCursor::open(&*h, tail)?;
    let mut walk = Walk { fixed: shape.head.fixed_at, var: 0, bit: 0, head: shape.head };
    let mut filled = 0u64;
    for o in 0..shape.leaves {
        let l = shape.leaf(&*h, o)?;
        let run_at = rc.place(h.get(Region::Block(tail))?, &l)?;
        let (fixed, vi, bit) = (walk.fixed, walk.var, walk.bit);
        walk.skip(&l);
        if old.is_some() && !named(leaves, &l) {
            continue;
        }
        if run_at != At::None {
            let v = row::checked_cell(src, &l)?;
            match old {
                Some(loc) => {
                    let cur = {
                        let row = RowRef::new(&*h, loc)?;
                        cur::from_stored(&row, shape, &l, &row::stored_at(&walk.head, &row, &l, fixed, vi, bit)?, vi)?
                    };
                    if !cur::same(&*h, &l, &cur, &v)? {
                        total_leaf(h, tbl, tail, run_at, &l, TSrc::Cur(&cur), Sign::Remove, ext)?;
                        total_leaf(h, tbl, tail, run_at, &l, TSrc::Val(&v), Sign::Add, ext)?;
                    }
                }
                None => total_leaf(h, tbl, tail, run_at, &l, TSrc::Val(&v), Sign::Add, ext)?,
            }
        }
        if !longs || !l.ty.variable() {
            continue;
        }
        let was = match old {
            Some(loc) => match row::stored_at(&walk.head, &RowRef::new(&*h, loc)?, &l, fixed, vi, bit)? {
                Stored::Item(Item::Long { img, len }) => Some((img, len)),
                _ => None,
            },
            None => None,
        };
        let (n, keep) = {
            let v = row::checked_cell(src, &l)?;
            let n = row::has_item(&l, &v).then(|| cell::var_len(&v, l.ty)).filter(|&n| row::is_long(n));
            let keep = match (n, was, v) {
                (Some(_), Some((img, len)), Val::Bytes(b) | Val::Big(b)) => {
                    cur::same_bytes(len, b, |at, dst| long::read(&*h, img, at, dst))?
                }
                _ => false,
            };
            (n, keep)
        };
        if let Some(n) = n {
            let img = match was {
                Some((img, _)) if keep => img,
                _ => {
                    mark_long(h, tail)?;
                    let (img, f) = row::long_value(shape, h, tbl, uid, src, &l, n)?;
                    filled = filled.saturating_add(f);
                    img
                }
            };
            let a = rd_u32(buf, shape.head.items_table_at().wrapping_add(vi.wrapping_mul(4)))?;
            img.put_ref(buf, u64::from(a).wrapping_add(8))?;
        }
        if let Some((img, _)) = was.filter(|_| !keep) {
            drain::free_image(h, tbl, &img)?;
        }
    }
    Ok(filled)
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

/// Frees a buffered row's long values: its long items, and a long row image itself.
pub fn free_longs<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, loc: RowLoc) -> Result<(), Fault> {
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
    Ok(())
}

/// Deletes a buffered row: its slot's dead flag, the totals and the live count; its long values are freed.
pub fn delete_buffered<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, at: &Located) -> Result<(), Fault> {
    let tail = root::u64_at(&*h, tbl, root::TAIL)?;
    let loc = buffer::slot_row(&*h, at.block, at.slot)?.ok_or(rt(Corruption::Link, at.block))?;
    let ext = super::extremes::of_desc(&*h, at.desc)?;
    totals_row(h, tbl, shape, tail, loc, Sign::Remove, ext)?;
    free_longs(h, tbl, shape, loc)?;
    let b = h.get_mut(Region::Block(at.block))?;
    let slot = buffer::read_slot(b, at.slot)?;
    buffer::write_slot(b, at.slot, &Slot { flags: slot.flags | SLOT_DEAD, ..slot })?;
    let t = h.get_mut(Region::Block(tail))?;
    let live = rd_u64(t, H_LIVE)?;
    put_u64(t, H_LIVE, live.wrapping_sub(1))?;
    let buffered = rd_u64(t, buffer::H_BUFFERED)?;
    put_u64(t, buffer::H_BUFFERED, buffered.wrapping_sub(1))
}
