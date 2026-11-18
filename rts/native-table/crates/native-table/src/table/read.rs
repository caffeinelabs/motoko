//! Reads of rows: a row's cells handed to a `RowSink` in leaf order, and long values read in pieces.

use crate::consts::{GROUP_SLOTS, PAGE};
use crate::fault::Fault;
use crate::fault::ImageId;
use crate::mem::{rd_u32, rd_u64, rd_u8, sub, PageHost, Region};
use crate::schema::{Leaf, LEAF_KEY, LEAF_NULLABLE};

use super::buffer::{self, DESC_BIT};
use super::cell::{self, Val};
use super::cur::{self, SVal};
use super::group::{self as grp, form, Cur, GCell, GroupView, LeafDir, Seg, Tables};
use super::layout::{LeafInfo, Shape, Ty};
use super::overlay;
use super::row::{self as rowimg, RowRef, Walk};
use super::{root, CellOut, RowSink};

/// A value of at most this many bytes that does not lie in one piece is copied and handed out whole.
const SHORT: u64 = 64;
const SHORT_BUF: usize = 64;

/// Where a live row lies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Buffered(buffer::Located),
    /// A sealed row: its group page and position.
    Sealed {
        page: u64,
        pos: u64,
    },
}

/// Leaf `l`'s current value `v` as a cell: a fixed value as it is, a byte string or a big Nat or Int copied
/// into `piece`.
#[inline(always)]
pub fn sval_val<'a, H: PageHost + ?Sized>(
    h: &H,
    l: &LeafInfo,
    v: &SVal,
    piece: &'a mut [u8],
) -> Result<Val<'a>, Fault> {
    Ok(match *v {
        SVal::Null => Val::Null,
        SVal::Unit => Val::Unit,
        SVal::V(x) => x,
        _ => {
            let n = v.len();
            let d = piece.get_mut(..usize::try_from(n).unwrap_or(usize::MAX)).ok_or(Fault::Scratch { need: n })?;
            cur::read(h, v, 0, d)?;
            if matches!(l.ty, Ty::Nat | Ty::Int) {
                Val::Big(d)
            } else {
                Val::Bytes(d)
            }
        }
    })
}

/// Leaf `l`'s current value at position `pos` of sealed group `s` (viewed by `gv`, the position's stored slot
/// `slot`) as a cell, a byte string or a big Nat or Int copied into `piece`: straight from its image cell
/// when the group has no patches, else through the overlay.
pub fn sealed_cell<'a, H: PageHost + ?Sized>(
    gv: &GroupView<'_, H>,
    s: &overlay::Sealed,
    l: &LeafInfo,
    slot: Option<u64>,
    pos: u64,
    piece: &'a mut [u8],
) -> Result<Val<'a>, Fault> {
    if let (Some(slot), 0) = (slot, s.dir) {
        return cur::cell_val(gv, &gv.leaf(u64::from(l.ordinal))?, l, slot, pos, piece);
    }
    patched_cell(gv, s, l, slot, pos, piece)
}

/// `sealed_cell` through the overlay: kept out of line, so the common read's stack stays small.
#[inline(never)]
fn patched_cell<'a, H: PageHost + ?Sized>(
    gv: &GroupView<'_, H>,
    s: &overlay::Sealed,
    l: &LeafInfo,
    slot: Option<u64>,
    pos: u64,
    piece: &'a mut [u8],
) -> Result<Val<'a>, Fault> {
    let ld = gv.leaf(u64::from(l.ordinal))?;
    let v = overlay::leaf_value(gv, s, l, Some(&ld), slot, pos)?;
    sval_val(gv.h, l, &v, piece)
}

/// A live row with its group opened once for all the reads of one operation: a buffered row's place, or a
/// sealed row's group state and position. Large by design: it lives on the stack and is passed by reference.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy, Debug)]
pub enum Live {
    Buffered(buffer::Located),
    Sealed(overlay::Sealed, u64),
}

impl Live {
    pub fn row(&self) -> Row {
        match *self {
            Live::Buffered(at) => Row::Buffered(at),
            Live::Sealed(ref s, pos) => Row::Sealed { page: s.page, pos },
        }
    }

    /// Row `r`, opened. Inlined into each caller, as `live` is.
    #[inline(always)]
    pub fn open<H: PageHost + ?Sized>(h: &H, r: Row) -> Result<Live, Fault> {
        Ok(match r {
            Row::Buffered(at) => Live::Buffered(at),
            Row::Sealed { page, pos } => Live::Sealed(overlay::sealed(h, page)?, pos),
        })
    }
}

/// The live row with rid `rid`, or `None`.
pub fn find<H: PageHost + ?Sized>(h: &H, tbl: u64, rid: u64) -> Result<Option<Row>, Fault> {
    Ok(live(h, tbl, rid)?.map(|x| x.row()))
}

/// `find`'s row, opened. Inlined into each caller, so the opened row is built where the caller keeps it and
/// never moved.
#[inline(always)]
pub fn live<H: PageHost + ?Sized>(h: &H, tbl: u64, rid: u64) -> Result<Option<Live>, Fault> {
    let g = rid / GROUP_SLOTS;
    let pos = rid % GROUP_SLOTS;
    let entry = root::group(h, tbl, g)?;
    if entry == 0 {
        return Ok(None);
    }
    if entry & DESC_BIT != 0 {
        let Some(at) = buffer::locate(h, tbl, rid)? else { return Ok(None) };
        if !overlay::buffered_live(h, &at)? {
            return Ok(None);
        }
        return Ok(Some(Live::Buffered(at)));
    }
    let s = overlay::sealed(h, entry)?;
    if !overlay::sealed_live(h, &s, pos)? {
        return Ok(None);
    }
    Ok(Some(Live::Sealed(s, pos)))
}

/// A Nat or Int value of i128 as its cell: U64 or I64 when it fits, else its LEB128 form.
fn emit_wide<S: RowSink>(out: &mut S, l: &LeafInfo, x: i128) -> Result<(), Fault> {
    let mut b = [0u8; 19];
    if matches!(l.ty, Ty::Int) {
        if let Ok(y) = i64::try_from(x) {
            return out.cell(l.id, CellOut::I64(y));
        }
        let n = cell::put_sleb128(&mut b, x);
        out.cell(l.id, CellOut::Int(b.get(..n).unwrap_or(&[])))
    } else {
        if let Ok(y) = u64::try_from(x) {
            return out.cell(l.id, CellOut::U64(y));
        }
        let n = cell::put_uleb128(&mut b, x as u128);
        out.cell(l.id, CellOut::Nat(b.get(..n).unwrap_or(&[])))
    }
}

/// A Nat or Int value as LEB128 bytes in one slice: U64 or I64 when it fits, else Nat or Int.
fn emit_leb<S: RowSink>(out: &mut S, l: &LeafInfo, b: &[u8]) -> Result<(), Fault> {
    let signed = matches!(l.ty, Ty::Int);
    match cell::leb_value(b, signed) {
        Some(x) => emit_wide(out, l, x),
        None if signed => out.cell(l.id, CellOut::Int(b)),
        None => out.cell(l.id, CellOut::Nat(b)),
    }
}

/// A byte-string or variable value in one slice.
fn emit_bytes<S: RowSink>(out: &mut S, l: &LeafInfo, b: &[u8]) -> Result<(), Fault> {
    match l.ty {
        Ty::Nat | Ty::Int => emit_leb(out, l, b),
        _ => out.cell(l.id, CellOut::Bytes(b)),
    }
}

/// One fixed-cell value.
fn emit_val<S: RowSink>(out: &mut S, l: &LeafInfo, v: Val<'_>) -> Result<(), Fault> {
    let c = match (v, l.ty) {
        (Val::Null, _) => CellOut::Null,
        (Val::Unit, _) => CellOut::Unit,
        (Val::Bool(b), _) => CellOut::Bool(b),
        (Val::U(x), Ty::Nat) => return emit_wide(out, l, i128::from(x)),
        (Val::I(x), Ty::Int) => return emit_wide(out, l, i128::from(x)),
        (Val::Wide(x), _) => return emit_wide(out, l, x),
        (Val::U(x), _) => CellOut::U64(x),
        (Val::I(x), _) => CellOut::I64(x),
        (Val::U128(x), _) => CellOut::U128(x),
        (Val::I128(x), _) => CellOut::I128(x),
        (Val::F32(b), _) => CellOut::F32(f32::from_bits(b)),
        (Val::F64(b), _) => CellOut::F64(f64::from_bits(b)),
        (Val::Bytes(b), _) => CellOut::Bytes(b),
        (Val::Big(b), _) => return emit_leb(out, l, b),
        (Val::Arg(n), _) => CellOut::Arg { len: n },
    };
    out.cell(l.id, c)
}

/// One value handed to `out`. A value that does not lie in one piece of table memory is handed as
/// `CellOut::Arg { len }`, to be read with `read_large`; a short one is copied and handed whole.
pub fn emit_sval<H: PageHost + ?Sized, S: RowSink>(h: &H, out: &mut S, l: &LeafInfo, v: &SVal) -> Result<(), Fault> {
    match *v {
        SVal::Null => out.cell(l.id, CellOut::Null),
        SVal::Unit => out.cell(l.id, CellOut::Unit),
        SVal::V(x) => emit_val(out, l, x),
        _ => {
            if let Some(b) = cur::piece(h, v)? {
                return emit_bytes(out, l, b);
            }
            if v.len() <= SHORT {
                let mut b = [0u8; SHORT_BUF];
                let d = b.get_mut(..usize::try_from(v.len()).unwrap_or(0)).unwrap_or(&mut []);
                cur::read(h, v, 0, d)?;
                return emit_bytes(out, l, d);
            }
            out.cell(l.id, CellOut::Arg { len: v.len() })
        }
    }
}

/// `emit_sval` for value `v` of the buffered row `row`: a byte string in the row's own bytes is handed from them.
pub fn emit_row_sval<H: PageHost + ?Sized, S: RowSink>(
    row: &RowRef<'_, H>,
    out: &mut S,
    l: &LeafInfo,
    v: &SVal,
) -> Result<(), Fault> {
    if let SVal::B { at: cur::At::Row { off, .. }, len } = *v {
        if let Some(b) = row.inline_bytes(off, len) {
            return emit_bytes(out, l, b);
        }
    }
    emit_sval(row.h, out, l, v)
}

/// A leaf a planned read hands out (`emit_planned`), with where its value lies in a row image of the schema
/// version the plan is made for: past `fixed` bytes of the fixed region, after `var` of the row's items. Its id,
/// ordinal, `[u8; N]` width, kind, type code and flags are those of its leaf record (`schema::Leaf`). Plain numbers
/// in C's layout, so a host can keep a plan in its own memory.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Planned {
    pub id: u32,
    pub ordinal: u16,
    pub width: u16,
    pub kind: u8,
    pub code: u8,
    pub flags: u8,
    pub fixed: u32,
    pub var: u32,
}

impl Planned {
    /// The leaf; `BadValue` for a kind or type code the table does not hold.
    #[inline(always)]
    pub fn leaf(&self) -> Result<LeafInfo, Fault> {
        let ty = Ty::of(&Leaf { kind: self.kind, code: self.code, width: self.width, ..Leaf::default() });
        Ok(LeafInfo {
            ordinal: self.ordinal,
            id: self.id,
            ty: ty.ok_or(Fault::BadValue { leaf: self.id })?,
            nullable: self.flags & LEAF_NULLABLE != 0,
            key: self.flags & LEAF_KEY != 0,
            path: 0,
        })
    }
}

/// Hands the leaves of `plan` of the live row `row` (rid `rid`) to `out`, in plan order, as `filter::row_cells`
/// hands a view's leaves, without looking any leaf up: a buffered row's from its place in the row image when the
/// row has the schema version the plan was made for, a sealed row's from the group's leaf directory, its image's
/// block list read once.
pub fn emit_planned<H: PageHost + ?Sized, S: RowSink>(
    h: &H,
    shape: &Shape,
    plan: &[Planned],
    row: &Live,
    rid: u64,
    out: &mut S,
) -> Result<(), Fault> {
    match *row {
        Live::Buffered(at) => planned_buffered(h, shape, &overlay::buffered(h, &at)?, plan, None, out)?,
        Live::Sealed(ref s, pos) => planned_sealed(&GroupView::from_meta(h, &s.meta), s, plan, pos, out)?,
    }
    out.row_end(rid)
}

/// Hands the leaves of `plan` of the row at position `pos` of the sealed group `s`, viewed by `gv`, to `out`, in
/// plan order.
fn planned_sealed<H: PageHost + ?Sized, S: RowSink>(
    gv: &GroupView<'_, H>,
    s: &overlay::Sealed,
    plan: &[Planned],
    pos: u64,
    out: &mut S,
) -> Result<(), Fault> {
    let (t, slot, mut m) = (gv.tables(), gv.slot(pos)?, Memo::of(gv));
    for p in plan {
        let l = p.leaf()?;
        if let (Some(slot), true) = (slot, s.clean(l.ordinal)) {
            if let Some(loc) = locate(gv, &t, &mut m, &l)? {
                if emit_loc(gv, &mut m, &loc, out, &l, slot)? {
                    continue;
                }
            }
        }
        emit_leaf(gv, Some(&t), s, &l, &gv.leaf_in(&t, u64::from(l.ordinal))?, None, slot, pos, out)?;
    }
    Ok(())
}

/// Hands the leaves of `plan` of the buffered row `b` to `out`, in plan order; `tested`, the value of one leaf a
/// filter has read already.
pub(crate) fn planned_buffered<H: PageHost + ?Sized, S: RowSink>(
    h: &H,
    shape: &Shape,
    b: &overlay::Buffered,
    plan: &[Planned],
    tested: Option<(u16, &SVal)>,
    out: &mut S,
) -> Result<(), Fault> {
    let r = RowRef::new(h, b.loc)?;
    let head = shape.head;
    let same = r.version()? == head.version;
    let img = r.inline_all().filter(|_| same);
    for p in plan {
        let l = p.leaf()?;
        if let Some((_, x)) = tested.filter(|(o, _)| *o == l.ordinal) {
            emit_row_sval(&r, out, &l, x)?;
            continue;
        }
        if let Some(img) = img {
            if buffered_lean(img, &head, p, &l, out)? {
                continue;
            }
        }
        let v = if same {
            let (fixed, var) = (head.fixed_at.wrapping_add(u64::from(p.fixed)), u64::from(p.var));
            let st = rowimg::stored_at(&head, &r, &l, fixed, var, u64::from(l.ordinal))?;
            cur::from_stored_in(&r, &head, &l, &st, var)?
        } else {
            overlay::buffered_leaf(h, shape, b, &l)?
        };
        emit_row_sval(&r, out, &l, &v)?;
    }
    Ok(())
}

/// Image blocks a planned read of a sealed group has read, kept for its next leaves and rows by block number
/// modulo 4, with the image's block list when it is one level (`image::radix_entries`), read once.
#[derive(Default)]
pub(crate) struct Memo<'h> {
    /// The block number + 1 and the page of each kept block; 0 for none.
    kept: [(u64, &'h [u8]); 4],
    entries: &'h [u8],
}

impl<'h> Memo<'h> {
    /// A memo for the group `gv` views.
    pub(crate) fn of<H: PageHost + ?Sized>(gv: &GroupView<'h, H>) -> Self {
        Memo { entries: crate::image::radix_entries(gv.h, &gv.img), ..Memo::default() }
    }

    /// Image bytes [off, off + len) when they lie in one image block, else `None`.
    #[inline(always)]
    fn slice<H: PageHost + ?Sized>(
        &mut self,
        gv: &GroupView<'h, H>,
        off: u64,
        len: u64,
    ) -> Result<Option<&'h [u8]>, Fault> {
        let (k, o) = (off / PAGE, off % PAGE);
        if o.saturating_add(len) > PAGE || off.saturating_add(len) > gv.img.length {
            return Ok(None);
        }
        let Some(kept) = self.kept.get_mut(usize::try_from(k % 4).unwrap_or(0)) else { return Ok(None) };
        if kept.0 != k.wrapping_add(1) {
            let Some(page) = crate::image::block(gv.h, &gv.img, self.entries, k)? else { return Ok(None) };
            *kept = (k.wrapping_add(1), page);
        }
        Ok(sub(kept.1, o, len).ok())
    }

    /// Entry `i` of `n` bytes of the group table at image offset `at`, from `held` when it holds the table.
    #[inline(always)]
    fn entry<H: PageHost + ?Sized>(
        &mut self,
        gv: &GroupView<'h, H>,
        held: &'h [u8],
        at: u64,
        i: u64,
        n: u64,
    ) -> Result<Option<&'h [u8]>, Fault> {
        let o = i.wrapping_mul(n);
        match sub(held, o, n) {
            Ok(b) => Ok(Some(b)),
            Err(_) => self.slice(gv, at.wrapping_add(o), n),
        }
    }
}

/// Where a leaf's cells lie in a sealed group, for its one cell segment of a fixed-width or a variable form
/// (`locate`): its form and width, the segment's cells, first slot and slots, where its cells start past its
/// validity bitmap, and the image offset and bytes of the leaf's run of values, with the run itself once `held`.
#[derive(Clone, Copy, Default)]
pub(crate) struct Loc<'h> {
    f: u8,
    width: u64,
    validity: bool,
    cells: &'h [u8],
    first: u64,
    n: u64,
    body: u64,
    run_off: u64,
    run_bytes: u64,
    run: &'h [u8],
}

impl<'h> Loc<'h> {
    /// The same, with the leaf's run of values read once, when it lies in one image block: for many rows.
    pub(crate) fn held<H: PageHost + ?Sized>(mut self, gv: &GroupView<'h, H>, m: &mut Memo<'h>) -> Result<Self, Fault> {
        self.run = m.slice(gv, self.run_off, self.run_bytes)?.unwrap_or(&[]);
        Ok(self)
    }
}

/// Where leaf `l`'s cells lie in the sealed group `gv` views (with its tables `t`), when it has one cell segment
/// of a fixed form of at most 8 bytes or of a variable form, and no out-of-line values; `None` for any other leaf,
/// which `emit_leaf` reads. Its directory entry and segment entries are read in place.
#[inline(always)]
pub(crate) fn locate<'h, H: PageHost + ?Sized>(
    gv: &GroupView<'h, H>,
    t: &Tables<'h>,
    m: &mut Memo<'h>,
    l: &LeafInfo,
) -> Result<Option<Loc<'h>>, Fault> {
    let o = u64::from(l.ordinal);
    let d = m.entry(gv, t.dir, grp::HEADER, o, grp::LD_BYTES)?.filter(|_| o < gv.l);
    let Some(&[i0, i1, i2, i3, _, _, f, flags, _, _, _, _, w0, w1, _, _, s0, s1, s2, s3, n0, n1, n2, n3, ..]) =
        d.and_then(|d| <&[u8; 32]>::try_from(d).ok())
    else {
        return Ok(None);
    };
    let (variable, width) = (grp::variable(f), u64::from(u16::from_le_bytes([w0, w1])));
    let fixed = matches!(f, form::BOOL | form::U8 | form::U16 | form::U32 | form::U64)
        || matches!(f, form::I8 | form::I16 | form::I32 | form::I64 | form::F64);
    let segs = u32::from_le_bytes([n0, n1, n2, n3]);
    if flags & grp::LF_OOL != 0 || !(variable || fixed && width <= 8) || segs != u32::from(variable).wrapping_add(1) {
        return Ok(None);
    }
    if u32::from_le_bytes([i0, i1, i2, i3]) != l.id {
        return Ok(None);
    }
    let first = u64::from(u32::from_le_bytes([s0, s1, s2, s3]));
    let seg = |m: &mut Memo<'h>, i: u64| -> Result<Option<Seg>, Fault> {
        let e = m.entry(gv, t.segs, gv.segs_at, i, grp::SEG_BYTES)?.and_then(|e| <&[u8; 32]>::try_from(e).ok());
        let Some(&[o0, o1, o2, o3, o4, o5, o6, o7, f0, f1, f2, f3, p0, p1, p2, p3, b0, b1, b2, b3, kind, ..]) = e
        else {
            return Ok(None);
        };
        Ok(Some(Seg {
            off: u64::from_le_bytes([o0, o1, o2, o3, o4, o5, o6, o7]),
            first: u32::from_le_bytes([f0, f1, f2, f3]),
            positions: u32::from_le_bytes([p0, p1, p2, p3]),
            bytes: u32::from_le_bytes([b0, b1, b2, b3]),
            kind,
        }))
    };
    let Some(sg) = seg(m, first)? else { return Ok(None) };
    let (off, from, n, bytes) = (sg.off, u64::from(sg.first), u64::from(sg.positions), u64::from(sg.bytes));
    let Some(cells) = m.slice(gv, off, bytes)? else { return Ok(None) };
    let validity = flags & grp::LF_VALIDITY != 0;
    let body = if validity { n.div_ceil(64).wrapping_mul(8) } else { 0 };
    let mut loc = Loc { f, width, validity, cells, first: from, n, body, ..Loc::default() };
    if variable {
        match seg(m, first.wrapping_add(1))? {
            Some(run) if run.kind == grp::seg::RUN => (loc.run_off, loc.run_bytes) = (run.off, u64::from(run.bytes)),
            _ => return Ok(None),
        }
    }
    Ok(Some(loc))
}

/// Leaf `l`'s cell at stored slot `slot`, where `loc` says its cells lie, handed to `out` straight from the image.
/// `false`, with nothing handed out, when the slot is not in the segment or the value does not lie in one image
/// block; `emit_leaf` reads it then.
#[inline(always)]
pub(crate) fn emit_loc<'h, H: PageHost + ?Sized, S: RowSink>(
    gv: &GroupView<'h, H>,
    m: &mut Memo<'h>,
    loc: &Loc<'h>,
    out: &mut S,
    l: &LeafInfo,
    slot: u64,
) -> Result<bool, Fault> {
    let Some(i) = slot.checked_sub(loc.first).filter(|&i| i < loc.n) else { return Ok(false) };
    let b = loc.cells;
    if loc.validity && rd_u64(b, (i / 64).wrapping_mul(8))? >> (i % 64) & 1 == 0 {
        out.cell(l.id, CellOut::Null)?;
        return Ok(true);
    }
    if grp::variable(loc.f) {
        let at = loc.body.wrapping_add(i.wrapping_mul(4));
        let (a, e) = (u64::from(rd_u32(b, at)?), u64::from(rd_u32(b, at.wrapping_add(4))?));
        let Some(len) = e.checked_sub(a).filter(|_| e <= loc.run_bytes) else { return Ok(false) };
        let v = match sub(loc.run, a, len) {
            Ok(v) => v,
            Err(_) => match m.slice(gv, loc.run_off.wrapping_add(a), len)? {
                Some(v) => v,
                None => return Ok(false),
            },
        };
        emit_bytes(out, l, v)?;
        return Ok(true);
    }
    let c = if loc.f == form::BOOL {
        GCell::Bool(rd_u64(b, loc.body.wrapping_add((i / 64).wrapping_mul(8)))? >> (i % 64) & 1 != 0)
    } else {
        let mut a = [0u8; 8];
        crate::mem::copy_bytes(&mut a, sub(b, loc.body.wrapping_add(i.wrapping_mul(loc.width)), loc.width)?);
        let raw = u64::from_le_bytes(a);
        match loc.f {
            form::F64 => GCell::F64(raw),
            form::I8 | form::I16 | form::I32 | form::I64 => {
                let shift = 64u64.saturating_sub(loc.width.wrapping_mul(8));
                // shift < 64.
                #[allow(clippy::cast_possible_truncation)]
                let x = ((raw << shift as u32) as i64) >> shift as u32;
                GCell::I(x)
            }
            _ => GCell::U(raw),
        }
    };
    emit_val(out, l, cur::whole(c, loc.f, l.ty).unwrap_or(Val::Null))?;
    Ok(true)
}

/// Leaf `l`'s value, planned as `p`, in the buffered row image `img` (whole, of the plan's schema version `head`),
/// handed to `out` straight from its place: a fixed-width value, or an item held in the row. `false`, with nothing
/// handed out, for any other value, which `rowimg::stored_at` finds.
#[inline(always)]
fn buffered_lean<S: RowSink>(
    img: &[u8],
    head: &super::layout::RowHead,
    p: &Planned,
    l: &LeafInfo,
    out: &mut S,
) -> Result<bool, Fault> {
    let o = u64::from(l.ordinal);
    let bit = |at: u64| -> Result<bool, Fault> { Ok(rd_u8(img, at.wrapping_add(o / 8))? >> (o % 8) & 1 != 0) };
    // The null bitmap, then the bitmap of the Nat and Int values held as items.
    if bit(8)? {
        out.cell(l.id, CellOut::Null)?;
        return Ok(true);
    }
    match l.ty {
        Ty::Text | Ty::Blob => {
            let at = u64::from(rd_u32(img, head.items_table_at().wrapping_add(u64::from(p.var).wrapping_mul(4)))?);
            if at == 0 || rd_u8(img, at)? != rowimg::ITEM_INLINE {
                return Ok(false);
            }
            let (len, k) = crate::bytes::rd_uleb(img, at.wrapping_add(1), ImageId::Runtime)?;
            out.cell(l.id, CellOut::Bytes(sub(img, at.wrapping_add(1).wrapping_add(k), len)?))?;
        }
        Ty::Nat | Ty::Int if bit(head.bitmap.wrapping_add(8))? => return Ok(false),
        Ty::Nat | Ty::Int | Ty::Bool | Ty::U(_) | Ty::I(_) | Ty::F64 | Ty::Char => {
            let c = sub(img, head.fixed_at.wrapping_add(u64::from(p.fixed)), l.ty.fixed_width())?;
            emit_val(out, l, cur::own(cell::from_fixed(l.ty, c)))?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Hands a buffered row's leaves to `out` in leaf order, under the overlay.
pub fn emit_buffered<H: PageHost + ?Sized, S: RowSink>(
    h: &H,
    shape: &Shape,
    b: &overlay::Buffered,
    out: &mut S,
) -> Result<(), Fault> {
    let row = RowRef::new(h, b.loc)?;
    let mut walk = Walk::of(shape, &row)?;
    for o in 0..shape.leaves {
        let l = shape.leaf(h, o)?;
        let v = overlay::buffered_value(shape, &row, &mut walk, &l)?;
        emit_row_sval(&row, out, &l, &v)?;
    }
    Ok(())
}

/// Hands the leaves of the row at position `pos` of sealed group `s` (viewed by `gv`, the position's stored slot
/// `slot`) to `out` in leaf order, under the overlay.
pub fn emit_sealed<H: PageHost + ?Sized, S: RowSink>(
    gv: &GroupView<'_, H>,
    shape: &Shape,
    s: &overlay::Sealed,
    pos: u64,
    slot: Option<u64>,
    out: &mut S,
) -> Result<(), Fault> {
    let sb = gv.h.get(Region::Page(shape.page))?;
    let t = gv.tables();
    for o in 0..shape.leaves {
        emit_leaf(gv, Some(&t), s, &shape.leaf_in(sb, o)?, &gv.leaf_in(&t, o)?, None, slot, pos, out)?;
    }
    Ok(())
}

/// Hands leaf `l`'s current value at position `pos` of sealed group `s` (viewed by `gv`, the leaf's directory
/// entry `ld`, its segments read through `c` when given, else found in the group's tables `t` when given, the
/// position's stored slot `slot`) to `out`: straight from its image cell when the group has no patches and the
/// value lies in one piece of the image.
#[allow(clippy::too_many_arguments)]
#[inline(always)]
pub fn emit_leaf<'h, H: PageHost + ?Sized, S: RowSink>(
    gv: &GroupView<'h, H>,
    t: Option<&Tables<'h>>,
    s: &overlay::Sealed,
    l: &LeafInfo,
    ld: &LeafDir,
    c: Option<&mut Cur<'h>>,
    slot: Option<u64>,
    pos: u64,
    out: &mut S,
) -> Result<(), Fault> {
    if let (Some(slot), true) = (slot, s.clean(l.ordinal)) {
        if emit_cell(gv, t, out, l, ld, c, slot, pos)? {
            return Ok(());
        }
    }
    // Read where the result holds it: moving the value out copies it.
    let v = overlay::leaf_value(gv, s, l, Some(ld), slot, pos);
    emit_sval(gv.h, out, l, v.as_ref().map_err(|e| *e)?)
}

/// Leaf `l`'s image cell (directory entry `ld`) at stored slot `slot` of position `pos` handed to `out` straight
/// from the image; `false`, with nothing handed out, for a value that does not lie in one piece of it.
#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn emit_cell<'h, H: PageHost + ?Sized, S: RowSink>(
    gv: &GroupView<'h, H>,
    t: Option<&Tables<'h>>,
    out: &mut S,
    l: &LeafInfo,
    ld: &LeafDir,
    c: Option<&mut Cur<'h>>,
    slot: u64,
    pos: u64,
) -> Result<bool, Fault> {
    let x = match (c, t) {
        (Some(c), _) => gv.cell_at(ld, c, slot, pos)?,
        (None, Some(t)) => gv.cell_in::<true>(t, ld, &mut Cur::default(), slot, pos)?,
        (None, None) => gv.cell(ld, slot, pos)?,
    };
    match x {
        GCell::Bytes { off, len } if (off % PAGE).saturating_add(len) <= PAGE => {
            let b = gv.reader().bytes(off, len)?;
            // A Principal's 30-byte cell holds its length in its last byte.
            let n = if l.ty == Ty::Principal { usize::from(rd_u8(b, 29)?.min(29)) } else { b.len() };
            emit_bytes(out, l, b.get(..n).unwrap_or(&[]))?;
        }
        GCell::Bytes { .. } | GCell::Huge { .. } => return Ok(false),
        c => emit_val(out, l, cur::whole(c, ld.form, l.ty).unwrap_or(Val::Null))?,
    }
    Ok(true)
}

/// Leaf `id`'s current value in row `rid`.
pub fn value_of<H: PageHost + ?Sized>(h: &H, tbl: u64, shape: &Shape, rid: u64, id: u32) -> Result<SVal, Fault> {
    let o = shape.ordinal_of(h, id)?.ok_or(Fault::BadValue { leaf: id })?;
    let l = shape.leaf(h, o)?;
    let row = find(h, tbl, rid)?.ok_or(Fault::BadRef)?;
    match row {
        Row::Buffered(at) => {
            let b = overlay::buffered(h, &at)?;
            overlay::buffered_leaf(h, shape, &b, &l)
        }
        Row::Sealed { page, pos } => {
            let s = overlay::sealed(h, page)?;
            overlay::sealed_value(h, &s, &l, pos)
        }
    }
}

/// Copies bytes [at, at + buf.len()) of leaf `id`'s value in row `rid` (a Text, Blob, `[u8; N]`, Principal, or
/// a Nat's or Int's LEB128 form) into `buf`; returns how many, fewer at the value's end.
pub fn read_large<H: PageHost + ?Sized>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    rid: u64,
    id: u32,
    at: u64,
    buf: &mut [u8],
) -> Result<usize, Fault> {
    let v = value_of(h, tbl, shape, rid, id)?;
    let len = v.len();
    let n = len.saturating_sub(at).min(buf.len() as u64);
    let nu = usize::try_from(n).unwrap_or(0);
    if n > 0 {
        cur::read(h, &v, at, buf.get_mut(..nu).unwrap_or(&mut []))?;
    }
    Ok(nu)
}
