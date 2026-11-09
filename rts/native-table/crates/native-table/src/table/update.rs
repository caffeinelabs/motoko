//! Writes to sealed rows.
//!
//! An update of a sealed row writes each changed leaf in place when its cell can take the value (a fixed form
//! whose width holds it, `Bool`, a variable value of exactly its length that is not out of line, a null
//! transition where the segment has validity; never `Seq`) and no patch entry names the leaf at the row;
//! otherwise it upserts a patch. An integer leaf's total and the group's extremes move by difference.

use crate::consts::{HUGE_VALUE, LONG_VALUE};
use crate::fault::{corrupt, rt, Corruption, Fault, ImageId};
use crate::image::ImageReader;
use crate::mem::{fill, put_u16, put_u64, rd_u16, rd_u32, rd_u64, PageHost, Region};
use crate::value::Utf8Stream;

use super::buffer::{self, H_LIVE};
use super::cell::Val;
use super::cur::{self, SVal};
use super::drain;
use super::group::{self, form, seg, GCell, GroupView, LeafDir, LF_OOL, LF_VALIDITY};
use super::hot::{self, RunCursor, Sign};
use super::jobs;
use super::layout::{LeafInfo, Shape, Ty};
use super::long::{LongOut, Meta};
use super::overlay::{self, Sealed};
use super::page;
use super::patch::{self, ChainOut, Data};
use super::root;
use super::row;
use super::totals::{self, At, TSrc};
use super::RowSource;

/// Content kind of a long value holding a patch value.
pub const CONTENT_PATCH: u8 = 3;

// ---------------------------------------------------------------- patch values from host cells

/// A patch value being written: its VE when short, or its bytes in a chain or a long value.
#[allow(clippy::large_enum_variant)]
pub enum PVal {
    Null,
    Ve { buf: [u8; 2_064], len: usize },
    Chain { head: u64, len: u64 },
    Long { img: crate::image::Image, len: u64 },
}

impl PVal {
    pub fn data(&self) -> Data<'_> {
        match self {
            PVal::Null => Data::Null,
            PVal::Ve { buf, len } => Data::Ve(buf.get(..*len).unwrap_or(&[])),
            PVal::Chain { head, len } => Data::Chain { head: *head, len: *len },
            PVal::Long { img, len } => Data::Long { img: *img, len: *len },
        }
    }
}

/// The patch value of leaf `l` for the source's checked value, the bytes a value in the message's argument
/// copied by the host counted in the second result.
pub fn patch_value<H: PageHost, R: RowSource + ?Sized>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    l: &LeafInfo,
    src: &mut R,
) -> Result<(PVal, u64), Fault> {
    let n = {
        let v = row::cell_of(src, l)?;
        match v {
            Val::Arg(n) => n,
            v => return value_of(h, tbl, shape, l, &v, None::<&mut R>),
        }
    };
    value_of(h, tbl, shape, l, &Val::Arg(n), Some(src))
}

/// `patch_value` of a host value; `src` fills a value still in the message's argument.
fn value_of<H: PageHost, R: RowSource + ?Sized>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    l: &LeafInfo,
    v: &Val<'_>,
    mut src: Option<&mut R>,
) -> Result<(PVal, u64), Fault> {
    if v.is_null() {
        return Ok((PVal::Null, 0));
    }
    let mut short = [0u8; 32];
    if let Some(n) = cur::ve_short(l, v, &mut short) {
        let mut buf = [0u8; 2_064];
        crate::mem::copy_bytes(&mut buf, short.get(..n).unwrap_or(&[]));
        return Ok((PVal::Ve { buf, len: n }, 0));
    }
    let mut head = [0u8; 11];
    let (k, n) = cur::ve_long_parts(l, v, &mut head).ok_or(Fault::BadValue { leaf: l.id })?;
    let ve = (k as u64).checked_add(n).ok_or(rt(Corruption::Range, n))?;
    let text = matches!(l.ty, Ty::Text);
    let arg = matches!(v, Val::Arg(_));
    let bytes: &[u8] = match v {
        Val::Bytes(b) | Val::Big(b) => b,
        _ => &[],
    };
    if ve <= LONG_VALUE {
        let mut buf = [0u8; 2_064];
        crate::mem::copy_bytes(&mut buf, head.get(..k).unwrap_or(&[]));
        let end = usize::try_from(ve).map_err(|_| rt(Corruption::Range, ve))?;
        let dst = buf.get_mut(k..end).ok_or(rt(Corruption::Range, ve))?;
        let mut filled = 0u64;
        if let (true, Some(src)) = (arg, &mut src) {
            src.fill(l.id, 0, dst)?;
            if text && core::str::from_utf8(dst).is_err() {
                return Err(Fault::BadValue { leaf: l.id });
            }
            filled = n;
        } else {
            crate::mem::copy_bytes(dst, bytes);
        }
        return Ok((PVal::Ve { buf, len: end }, filled));
    }
    if ve <= HUGE_VALUE {
        // The value's own bytes in a chain of blocks, filled straight from the argument when it is there.
        let mut c = ChainOut::new();
        let mut utf = Utf8Stream::new();
        let mut at = 0u64;
        while at < n {
            let dst = c.piece(h, tbl, n.wrapping_sub(at))?;
            let k = dst.len() as u64;
            if let (true, Some(src)) = (arg, &mut src) {
                src.fill(l.id, at, dst)?;
                if text && !utf.feed(dst) {
                    return Err(Fault::BadValue { leaf: l.id });
                }
            } else {
                let s0 = usize::try_from(at).map_err(|_| rt(Corruption::Range, at))?;
                let piece = bytes.get(s0..s0.saturating_add(dst.len())).ok_or(rt(Corruption::Range, at))?;
                crate::mem::copy_bytes(dst, piece);
            }
            at = at.wrapping_add(k);
        }
        if arg && text && !utf.finish() {
            return Err(Fault::BadValue { leaf: l.id });
        }
        return Ok((PVal::Chain { head: c.head, len: n }, if arg { n } else { 0 }));
    }
    let uid = root::u64_at(&*h, tbl, root::UID)?;
    let meta = Meta { uid, schema_hash: shape.hash, content: CONTENT_PATCH, leaf: u32::from(l.ordinal) };
    let mut lo = LongOut::begin(h, tbl, &meta, n)?;
    if let (true, Some(src)) = (arg, &mut src) {
        lo.fill(h, &mut **src, l.id, n, text)?;
    } else {
        lo.put(h, bytes)?;
    }
    let filled = lo.filled;
    let img = lo.finish(h)?;
    Ok((PVal::Long { img, len: n }, filled))
}

// ---------------------------------------------------------------- in-place cell writes

/// Writes `src` at image offset `off` of a group image, one 4 KiB block at a time.
fn image_write<H: PageHost>(h: &mut H, img: &crate::image::Image, off: u64, src: &[u8]) -> Result<(), Fault> {
    let mut at = off;
    let mut rest = src;
    while !rest.is_empty() {
        let k = at / crate::consts::PAGE;
        let o = at % crate::consts::PAGE;
        let p = img.entry(&*h, k)?;
        if p == 0 {
            return Err(rt(Corruption::Link, k));
        }
        let room = crate::consts::BLOCK.wrapping_sub(o % crate::consts::BLOCK);
        let n = usize::try_from(room).unwrap_or(usize::MAX).min(rest.len());
        let (now, later) = rest.split_at_checked(n).ok_or(rt(Corruption::Range, at))?;
        // o < 65,536, so the block index is below 16.
        let j = u32::try_from(o / crate::consts::BLOCK).map_err(|_| rt(Corruption::Range, o))?;
        let b = h.block_of(p, j);
        crate::mem::put(h.get_mut(Region::Block(b))?, o % crate::consts::BLOCK, now)?;
        at = at.wrapping_add(n as u64);
        rest = later;
    }
    Ok(())
}

/// The cell segment of leaf `ld` holding slot s, and s's index in it.
fn cell_seg<H: PageHost + ?Sized>(gv: &GroupView<'_, H>, ld: &LeafDir, s: u64) -> Result<(group::Seg, u64), Fault> {
    let n = ld.cell_segs();
    for k in 0..n {
        let sg = gv.seg(u64::from(ld.first_seg).wrapping_add(k))?;
        let first = u64::from(sg.first);
        if s >= first && s < first.wrapping_add(u64::from(sg.positions)) {
            return Ok((sg, s.wrapping_sub(first)));
        }
    }
    Err(corrupt(ImageId::Group(gv.g), Corruption::Range, s))
}

/// Writes leaf `l`'s cell at position `pos` in place when its form can take `v`; `false` (nothing written)
/// when it cannot. The target region is checked to lie in [H, the segment table) first.
fn in_place<H: PageHost>(h: &mut H, s: &Sealed, l: &LeafInfo, pos: u64, v: &Val<'_>) -> Result<bool, Fault> {
    let (plan, img) = {
        let gv = GroupView::from_meta(&*h, &s.meta);
        let Some(ld) = gv.leaf_by_ordinal(u64::from(l.ordinal))? else { return Ok(false) };
        if ld.form == form::SEQ {
            return Ok(false);
        }
        let validity = ld.flags & LF_VALIDITY != 0;
        if !group::has_cells(ld.form, validity) {
            return Ok(false);
        }
        let slot = gv.slot(pos)?.ok_or(rt(Corruption::Link, pos))?;
        let (sg, i) = cell_seg(&gv, &ld, slot)?;
        let hd = gv.reader().bytes(0, group::HEADER)?;
        let hh = u64::from(rd_u32(hd, group::G_H)?);
        let seg_end = sg.off.checked_add(u64::from(sg.bytes)).ok_or(rt(Corruption::Range, sg.off))?;
        if sg.off < hh || seg_end > gv.segs_at {
            return Err(corrupt(ImageId::Group(gv.g), Corruption::Bounds, sg.off));
        }
        let words = u64::from(sg.positions).div_ceil(64).wrapping_mul(8);
        let body = if validity { sg.off.wrapping_add(words) } else { sg.off };
        let r = gv.reader();
        let was_null = validity && r.u64_at(sg.off.wrapping_add((i / 64).wrapping_mul(8)))? >> (i % 64) & 1 == 0;
        let null = v.is_null();
        if null && !validity {
            return Ok(false);
        }
        let width = u64::from(ld.width);
        // The cell's new bytes, or for a variable form the inline value's place.
        let mut cell = [0u8; 1_016];
        let mut cell_len = 0usize;
        let mut var_at: Option<(u64, u64)> = None;
        if !null {
            match (ld.form, v) {
                (form::BOOL, Val::Bool(_)) | (form::UNIT, Val::Unit) => {}
                (form::U8 | form::U16 | form::U32 | form::U64, Val::U(x)) => {
                    let bits = width.wrapping_mul(8);
                    if bits < 64 && x >> bits != 0 {
                        return Ok(false);
                    }
                    crate::mem::copy_bytes(&mut cell, &x.to_le_bytes());
                    cell_len = usize::try_from(width).unwrap_or(0);
                }
                (form::I8 | form::I16 | form::I32 | form::I64, Val::I(x)) => {
                    let bits = u32::try_from(width.wrapping_mul(8)).unwrap_or(64);
                    let sh = 64u32.saturating_sub(bits);
                    if bits < 64 && (x.wrapping_shl(sh)).wrapping_shr(sh) != *x {
                        return Ok(false);
                    }
                    crate::mem::copy_bytes(&mut cell, &x.to_le_bytes());
                    cell_len = usize::try_from(width).unwrap_or(0);
                }
                (form::F32, Val::F32(b)) => {
                    crate::mem::copy_bytes(&mut cell, &b.to_le_bytes());
                    cell_len = 4;
                }
                (form::F64, Val::F64(b)) => {
                    crate::mem::copy_bytes(&mut cell, &b.to_le_bytes());
                    cell_len = 8;
                }
                (form::X16, Val::U128(x)) => {
                    crate::mem::copy_bytes(&mut cell, &x.to_le_bytes());
                    cell_len = 16;
                }
                (form::X16, Val::I128(x)) => {
                    crate::mem::copy_bytes(&mut cell, &x.to_le_bytes());
                    cell_len = 16;
                }
                (form::FIXED, Val::Bytes(b)) => {
                    if matches!(l.ty, Ty::Principal) {
                        let mut c = [0u8; 30];
                        let n = super::cell::fixed_cell(v, l.ty, &mut c);
                        if n != width {
                            return Ok(false);
                        }
                        crate::mem::copy_bytes(&mut cell, &c);
                        cell_len = 30;
                    } else {
                        if b.len() as u64 != width {
                            return Ok(false);
                        }
                        crate::mem::copy_bytes(&mut cell, b);
                        cell_len = b.len();
                    }
                }
                (f, _) if group::variable(f) => {
                    if ld.flags & LF_OOL != 0 {
                        // An out-of-line value is not written in place.
                        let dir = (u64::from(ld.first_seg)..u64::from(ld.first_seg).wrapping_add(u64::from(ld.segs)))
                            .map(|k| gv.seg(k))
                            .find(|x| matches!(x, Ok(sg) if sg.kind == seg::OOL));
                        if let Some(Ok(d)) = dir {
                            let word = r.u64_at(d.off.wrapping_add((slot / 64).wrapping_mul(8)))?;
                            if word >> (slot % 64) & 1 != 0 {
                                return Ok(false);
                            }
                        }
                    }
                    let o = body.wrapping_add(i.wrapping_mul(4));
                    let a = u64::from(rd_u32(r.bytes(o, 4)?, 0)?);
                    let b2 = u64::from(rd_u32(r.bytes(o.wrapping_add(4), 4)?, 0)?);
                    let len = b2.checked_sub(a).ok_or(corrupt(ImageId::Group(gv.g), Corruption::Order, o))?;
                    let new_len = match (l.ty, v) {
                        (Ty::Nat | Ty::Int, Val::U(_) | Val::I(_) | Val::Wide(_)) => {
                            let mut lb = [0u8; 19];
                            super::cell::small_leb(v, matches!(l.ty, Ty::Int), &mut lb) as u64
                        }
                        (_, Val::Bytes(b) | Val::Big(b)) => b.len() as u64,
                        _ => return Ok(false),
                    };
                    if new_len != len {
                        return Ok(false);
                    }
                    let run = (u64::from(ld.first_seg)..u64::from(ld.first_seg).wrapping_add(u64::from(ld.segs)))
                        .map(|k| gv.seg(k))
                        .find(|x| matches!(x, Ok(sg) if sg.kind == seg::RUN))
                        .ok_or(rt(Corruption::Link, u64::from(ld.ordinal)))??;
                    let run_end = run.off.checked_add(u64::from(run.bytes)).ok_or(rt(Corruption::Range, run.off))?;
                    if b2 > u64::from(run.bytes) || run.off < hh || run_end > gv.segs_at {
                        return Err(corrupt(ImageId::Group(gv.g), Corruption::Bounds, run.off));
                    }
                    var_at = Some((run.off.wrapping_add(a), len));
                }
                _ => return Ok(false),
            }
        } else if group::variable(ld.form) {
            // A null takes an empty range: only an empty value becomes null in place.
            let o = body.wrapping_add(i.wrapping_mul(4));
            let a = u64::from(rd_u32(r.bytes(o, 4)?, 0)?);
            let b2 = u64::from(rd_u32(r.bytes(o.wrapping_add(4), 4)?, 0)?);
            if a != b2 {
                return Ok(false);
            }
        }
        if was_null && !null && group::variable(ld.form) && var_at.is_some_and(|(_, len)| len > 0) {
            return Ok(false);
        }
        (Some((ld.form, validity, sg.off, body, i, width, cell, cell_len, var_at, null)), gv.img)
    };
    let Some((f, validity, seg_off, body, i, width, cell, cell_len, var_at, null)) = plan else { return Ok(false) };
    // The validity bit.
    if validity {
        let w_at = seg_off.wrapping_add((i / 64).wrapping_mul(8));
        let r = ImageReader::new(&*h, ImageId::Group(s.meta.g), img);
        let w = r.u64_at(w_at)?;
        let nw = if null { w & !(1u64 << (i % 64)) } else { w | (1u64 << (i % 64)) };
        if nw != w {
            image_write(h, &img, w_at, &nw.to_le_bytes())?;
        }
    }
    match f {
        form::BOOL => {
            let w_at = body.wrapping_add((i / 64).wrapping_mul(8));
            let r = ImageReader::new(&*h, ImageId::Group(s.meta.g), img);
            let w = r.u64_at(w_at)?;
            let on = matches!(v, Val::Bool(true));
            let nw = if on { w | (1u64 << (i % 64)) } else { w & !(1u64 << (i % 64)) };
            if nw != w {
                image_write(h, &img, w_at, &nw.to_le_bytes())?;
            }
        }
        form::UNIT => {}
        f if group::variable(f) => {
            if let Some((at, len)) = var_at {
                let mut piece = [0u8; 256];
                let mut k = 0u64;
                let leb_bytes;
                let src: &[u8] = match v {
                    Val::Bytes(b) | Val::Big(b) => b,
                    _ => {
                        let mut lb = [0u8; 19];
                        let n = super::cell::small_leb(v, matches!(l.ty, Ty::Int), &mut lb);
                        leb_bytes = lb;
                        leb_bytes.get(..n).unwrap_or(&[])
                    }
                };
                while k < len {
                    let n = len.wrapping_sub(k).min(256);
                    let s0 = usize::try_from(k).unwrap_or(0);
                    let p = piece.get_mut(..usize::try_from(n).unwrap_or(0)).unwrap_or(&mut []);
                    crate::mem::copy_bytes(p, src.get(s0..s0.saturating_add(p.len())).unwrap_or(&[]));
                    image_write(h, &img, at.wrapping_add(k), p)?;
                    k = k.wrapping_add(n);
                }
            }
        }
        _ => {
            let at = body.wrapping_add(i.wrapping_mul(width));
            let bytes = if null { [0u8; 1_016] } else { cell };
            image_write(
                h,
                &img,
                at,
                bytes.get(..if null { usize::try_from(width).unwrap_or(0) } else { cell_len }).unwrap_or(&[]),
            )?;
        }
    }
    Ok(true)
}

// ---------------------------------------------------------------- updates of sealed rows

/// Frees a value a patch or a delete leaves behind: a huge value of the image's long-value list (its entry
/// cleared), or nothing for an image cell.
fn release_image_value<H: PageHost>(h: &mut H, tbl: u64, v: &SVal) -> Result<(), Fault> {
    if let SVal::Huge { img, owner: cur::Owner::List { page: pg, index }, .. } = *v {
        let (b, off) = page::lv_entry(&*h, pg, index)?;
        drain::free_image(h, tbl, &img)?;
        fill(h.get_mut(Region::Block(b))?, off, crate::image::REF_BYTES, 0)?;
    }
    Ok(())
}

/// Updates a sealed row (position `pos` of group page `pg`) to the source's values; returns whether anything
/// changed and the bytes the host copied from the argument. Every refusal comes before the first write.
pub fn update_sealed<H: PageHost, R: RowSource + ?Sized>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    src: &mut R,
    pg: u64,
    pos: u64,
) -> Result<(bool, u64), Fault> {
    let s = overlay::sealed(&*h, pg)?;
    // Which leaves change; a key leaf never does.
    let mut any = false;
    for o in 0..shape.leaves {
        let l = shape.leaf(&*h, o)?;
        if !l.live() {
            continue;
        }
        let cur = overlay::sealed_value(&*h, shape, &s, &l, pos)?;
        let v = row::cell_of(src, &l)?;
        if cur::same(&*h, &l, &cur, &v)? {
            continue;
        }
        if l.key {
            return Err(Fault::BadValue { leaf: l.id });
        }
        any = true;
    }
    if !any {
        return Ok((false, 0));
    }
    let mut filled = 0u64;
    for o in 0..shape.leaves {
        let l = shape.leaf(&*h, o)?;
        if !l.live() {
            continue;
        }
        let s = overlay::sealed(&*h, pg)?;
        let old = overlay::sealed_value(&*h, shape, &s, &l, pos)?;
        {
            let v = row::cell_of(src, &l)?;
            if cur::same(&*h, &l, &old, &v)? {
                continue;
            }
        }
        filled = filled.saturating_add(write_leaf(h, tbl, shape, src, &s, &l, pos, &old)?);
    }
    Ok((true, filled))
}

/// Writes one changed leaf of a sealed row.
#[allow(clippy::too_many_arguments)]
fn write_leaf<H: PageHost, R: RowSource + ?Sized>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    src: &mut R,
    s: &Sealed,
    l: &LeafInfo,
    pos: u64,
    old: &SVal,
) -> Result<u64, Fault> {
    let o = u64::from(l.ordinal);
    // An integer leaf's total and the group's extremes lose the old value here, and take the new one once it
    // is written.
    let tot = if l.ty.running().is_some() {
        let tail = root::u64_at(&*h, tbl, root::TAIL)?;
        let at = hot::place_of(&*h, shape, tail, l)?;
        let ext = super::extremes::of_page(&*h, s.page)?;
        hot::apply_at(h, tbl, tail, at, l, old, Sign::Remove, ext)?;
        Some((tail, at, ext))
    } else {
        None
    };
    let patched = s.dir != 0 && patch::lookup(&*h, s.dir, o, pos)?.is_some();
    let mut filled = 0u64;
    let wrote = if patched {
        false
    } else {
        let v = row::cell_of(src, l)?;
        if matches!(v, Val::Arg(_)) {
            false
        } else {
            in_place(h, s, l, pos, &v)?
        }
    };
    if wrote {
        if let Some((tail, at, ext)) = tot {
            let new = overlay::image_value(&*h, shape, &s.meta, l, pos)?;
            hot::apply_at(h, tbl, tail, at, l, &new, Sign::Add, ext)?;
        }
        return Ok(filled);
    }
    let (pv, f) = patch_value(h, tbl, shape, l, src)?;
    filled = f;
    let up = patch::upsert(h, tbl, s.page, page::P_PATCH_DIR, o, pos, &pv.data())?;
    patch::release(h, tbl, up.old)?;
    // A value the image held out of line is superseded for good: its long value is freed now.
    if !patched {
        release_image_value(h, tbl, old)?;
    }
    let flags = buffer::u16_of(&*h, s.page, page::P_FLAGS)?;
    if flags & u64::from(page::PF_PATCHES) == 0 {
        put_u16(
            h.get_mut(Region::Block(s.page))?,
            page::P_FLAGS,
            u16::try_from(flags).unwrap_or(0) | page::PF_PATCHES,
        )?;
    }
    jobs::after_patch(h, tbl, s.meta.g, s.page, &up)?;
    if let Some((tail, at, ext)) = tot {
        let new = new_value(&*h, s.page, page::P_PATCH_DIR, l, pos)?;
        hot::apply_at(h, tbl, tail, at, l, &new, Sign::Add, ext)?;
    }
    Ok(filled)
}

fn new_value<H: PageHost + ?Sized>(h: &H, holder: u64, off: u64, l: &LeafInfo, pos: u64) -> Result<SVal, Fault> {
    let dir = rd_u64(h.get(Region::Block(holder))?, off)?;
    let e = patch::lookup(h, dir, u64::from(l.ordinal), pos)?.ok_or(rt(Corruption::Link, pos))?;
    cur::from_entry(h, &e, l)
}

// ---------------------------------------------------------------- deletes of sealed rows

/// Deletes a sealed row: its live bit, its integer leaves' totals, its huge values freed with their list
/// entries cleared, and the live count.
pub fn delete_sealed<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, s: &Sealed, pos: u64) -> Result<(), Fault> {
    let pg = s.page;
    let ext = super::extremes::of_page(&*h, pg)?;
    let tail = root::u64_at(&*h, tbl, root::TAIL)?;
    let mut rc = RunCursor::open(&*h, tail)?;
    // A group whose image holds huge values frees the row's: every leaf is read then.
    let huge = buffer::u16_of(&*h, pg, page::P_FLAGS)? & u64::from(page::PF_HUGE) != 0;
    let slot = GroupView::from_meta(&*h, &s.meta).slot(pos)?;
    for o in 0..shape.leaves {
        let l = shape.leaf(&*h, o)?;
        let run = rc.place(h.get(Region::Block(tail))?, &l)?;
        if !l.live() || (run == At::None && !huge) {
            continue;
        }
        let gv = GroupView::from_meta(&*h, &s.meta);
        let ld = gv.leaf_by_ordinal(u64::from(l.ordinal))?;
        // In a group with no patches, the image cell itself: a total takes it as it is, unless it is a huge
        // value or a Nat's or Int's bytes.
        if let (Some(ld), Some(slot), 0) = (&ld, slot, s.dir) {
            let c = gv.cell(ld, slot, pos)?;
            if !matches!(c, GCell::Huge { .. }) {
                match (run, cur::whole(c, ld.form, l.ty)) {
                    (At::None, _) => continue,
                    (run, Some(w)) => {
                        totals::apply(h, tbl, tail, run, &l, TSrc::Val(&w), Sign::Remove, ext)?;
                        continue;
                    }
                    _ => {}
                }
            }
        }
        let v = match (&ld, slot, s.dir) {
            (Some(ld), Some(slot), 0) => cur::from_cell(&gv, ld, &l, slot, pos)?,
            _ => overlay::leaf_value(&gv, shape, s, &l, ld.as_ref(), slot, pos)?,
        };
        hot::apply_at(h, tbl, tail, run, &l, &v, Sign::Remove, ext)?;
        release_image_value(h, tbl, &v)?;
    }
    page::clear_live(h, pg, pos)?;
    {
        let b = h.get_mut(Region::Block(pg))?;
        let live = rd_u16(b, page::P_LIVE)?;
        put_u16(b, page::P_LIVE, live.saturating_sub(1))?;
        let dead = rd_u16(b, page::P_DEAD)?;
        put_u16(b, page::P_DEAD, dead.saturating_add(1))?;
    }
    let t = h.get_mut(Region::Block(tail))?;
    let live = rd_u64(t, H_LIVE)?;
    put_u64(t, H_LIVE, live.wrapping_sub(1))?;
    jobs::after_delete(h, tbl, s.meta.g, pg)
}
