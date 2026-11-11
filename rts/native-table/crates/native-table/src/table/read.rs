//! Reads of rows: a row's cells handed to a `RowSink` in leaf order, and long values read in pieces.

use crate::consts::{GROUP_SLOTS, PAGE};
use crate::fault::Fault;
use crate::mem::{rd_u8, PageHost};

use super::buffer::{self, DESC_BIT};
use super::cell::{self, Val};
use super::cur::{self, SVal};
use super::group::{GCell, GroupView, LeafDir};
use super::layout::{LeafInfo, Shape, Ty};
use super::overlay;
use super::row::{RowRef, Walk};
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
        emit_sval(h, out, &l, &v)?;
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
    for o in 0..shape.leaves {
        emit_leaf(gv, s, &shape.leaf(gv.h, o)?, &gv.leaf(o)?, slot, pos, out)?;
    }
    Ok(())
}

/// Hands leaf `l`'s current value at position `pos` of sealed group `s` (viewed by `gv`, the leaf's directory
/// entry `ld`, the position's stored slot `slot`) to `out`: straight from its image cell when the group has no
/// patches and the value lies in one piece of the image.
#[inline(always)]
pub fn emit_leaf<H: PageHost + ?Sized, S: RowSink>(
    gv: &GroupView<'_, H>,
    s: &overlay::Sealed,
    l: &LeafInfo,
    ld: &LeafDir,
    slot: Option<u64>,
    pos: u64,
    out: &mut S,
) -> Result<(), Fault> {
    if let (Some(slot), 0) = (slot, s.dir) {
        if emit_cell(gv, out, l, ld, slot, pos)? {
            return Ok(());
        }
    }
    // Read where the result holds it: moving the value out copies it.
    let v = overlay::leaf_value(gv, s, l, Some(ld), slot, pos);
    emit_sval(gv.h, out, l, v.as_ref().map_err(|e| *e)?)
}

/// Leaf `l`'s image cell (directory entry `ld`) at stored slot `slot` of position `pos` handed to `out` straight
/// from the image; `false`, with nothing handed out, for a value that does not lie in one piece of it.
#[inline(always)]
fn emit_cell<H: PageHost + ?Sized, S: RowSink>(
    gv: &GroupView<'_, H>,
    out: &mut S,
    l: &LeafInfo,
    ld: &LeafDir,
    slot: u64,
    pos: u64,
) -> Result<bool, Fault> {
    match gv.cell(ld, slot, pos)? {
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
