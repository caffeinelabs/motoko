//! A leaf's value in one row wherever it lies: a buffered row image, a group image's cell, a patch entry, a
//! value chain, a long value or the schema's default. A value is named by its place, so nothing borrows
//! table memory across host calls; readers copy it in pieces or compare it.
//!
//! Nat and Int values are normalized: one that fits i128 is a number (`Val::U`, `Val::I` or `Val::Wide`),
//! whatever its stored form; only a value beyond i128 stays bytes (its canonical LEB128).

use core::cmp::Ordering;

use crate::consts::PAGE;
use crate::fault::{rt, Corruption, Fault, ImageId};
use crate::image::{Image, ImageReader};
use crate::mem::{copy_bytes, rd_u8, PageHost, Region};

use super::buffer::RowLoc;
use super::cell::{self, Val};
use super::group::{form, GCell, GroupView, LeafDir};
use super::layout::{LeafInfo, Ty};
use super::long;
use super::patch::{self, Entry};
use super::row::{Item, RowRef, Stored};

/// Where a value's bytes lie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    /// Row image bytes at offset `off`.
    Row { loc: RowLoc, off: u64 },
    /// Group image bytes at offset `off`.
    Image { img: Image, g: u64, off: u64 },
    /// A long value's content from its start.
    Long { img: Image },
    /// Bytes of a patch block.
    Patch { block: u64, off: u64 },
    /// A patch value chain from its start.
    Chain { head: u64 },
}

/// The one referent of a huge value, which a rewrite that adopts the value must clear.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    /// A buffered row image's long item; its reference lies at row offset `at`.
    Item { loc: RowLoc, at: u64 },
    /// Entry `index` of a group's long-value list.
    List { page: u64, index: u64 },
    /// A patch entry's reference, at `data` of patch block `block`.
    Patch { block: u64, data: u64 },
}

/// A stored value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SVal {
    Null,
    Unit,
    /// A value read whole: Bool, numbers (Nat and Int up to i128), Char, floats' bits.
    V(Val<'static>),
    /// `len` bytes: Text, Blob, a Principal's bytes, `[u8; N]`, or a Nat's or Int's LEB128 beyond i128.
    B {
        at: At,
        len: u64,
    },
    /// A huge value: a long value whose content is the value's bytes.
    Huge {
        img: Image,
        len: u64,
        owner: Owner,
    },
}

impl SVal {
    pub fn is_null(&self) -> bool {
        matches!(self, SVal::Null)
    }

    /// Bytes of a byte-string value; 0 otherwise.
    pub fn len(&self) -> u64 {
        match *self {
            SVal::B { len, .. } | SVal::Huge { len, .. } => len,
            _ => 0,
        }
    }
}

/// A fixed value that borrows nothing.
#[inline(always)]
pub fn own(v: Val<'_>) -> Val<'static> {
    match v {
        Val::Null => Val::Null,
        Val::Bool(b) => Val::Bool(b),
        Val::U(x) => Val::U(x),
        Val::I(x) => Val::I(x),
        Val::Wide(x) => Val::Wide(x),
        Val::U128(x) => Val::U128(x),
        Val::I128(x) => Val::I128(x),
        Val::F32(x) => Val::F32(x),
        Val::F64(x) => Val::F64(x),
        Val::Unit => Val::Unit,
        Val::Arg(n) => Val::Arg(n),
        Val::Bytes(_) | Val::Big(_) => Val::Unit,
    }
}

/// Copies bytes [at, at + out.len()) of a value's place.
pub fn read_at<H: PageHost + ?Sized>(h: &H, a: &At, at: u64, out: &mut [u8]) -> Result<(), Fault> {
    match *a {
        At::Row { loc, off } => {
            let from = off.checked_add(at).ok_or(rt(Corruption::Range, at))?;
            match loc {
                RowLoc::Inline { block, off: base, len } => {
                    if from.checked_add(out.len() as u64).is_none_or(|e| e > len) {
                        return Err(rt(Corruption::Bounds, from));
                    }
                    let s = h.get(Region::Block(block))?;
                    copy_bytes(out, crate::mem::sub(s, base.wrapping_add(from), out.len() as u64)?);
                    Ok(())
                }
                RowLoc::Long(img) => long::read(h, img, from, out),
            }
        }
        At::Image { img, g, off } => {
            let r = ImageReader::new(h, ImageId::Group(g), img);
            long::read_image(&r, off.checked_add(at).ok_or(rt(Corruption::Range, at))?, out)
        }
        At::Long { img } => long::read(h, img, at, out),
        At::Patch { block, off } => {
            let s = h.get(Region::Block(block))?;
            copy_bytes(
                out,
                crate::mem::sub(s, off.checked_add(at).ok_or(rt(Corruption::Range, at))?, out.len() as u64)?,
            );
            Ok(())
        }
        At::Chain { head } => patch::chain_read(h, head, at, out),
    }
}

/// Copies bytes [at, at + out.len()) of a byte-string value.
pub fn read<H: PageHost + ?Sized>(h: &H, s: &SVal, at: u64, out: &mut [u8]) -> Result<(), Fault> {
    let end = at.checked_add(out.len() as u64).ok_or(rt(Corruption::Range, at))?;
    if end > s.len() {
        return Err(rt(Corruption::Bounds, at));
    }
    match s {
        SVal::B { at: a, .. } => read_at(h, a, at, out),
        SVal::Huge { img, .. } => long::read(h, *img, at, out),
        _ => Err(rt(Corruption::Encoding, at)),
    }
}

pub fn byte<H: PageHost + ?Sized>(h: &H, s: &SVal, j: u64) -> Result<u8, Fault> {
    let mut b = [0u8; 1];
    read(h, s, j, &mut b)?;
    Ok(b[0])
}

/// Whether leaf `l`'s Nat or Int value beyond i128, `s`, is negative: an Int whose LEB128 form's last byte
/// has the sign bit.
pub fn big_negative<H: PageHost + ?Sized>(h: &H, l: &LeafInfo, s: &SVal) -> Result<bool, Fault> {
    if l.ty != Ty::Int {
        return Ok(false);
    }
    let last = s.len().checked_sub(1).ok_or(rt(Corruption::Encoding, u64::from(l.ordinal)))?;
    Ok(byte(h, s, last)? & 0x40 != 0)
}

/// The order of two canonical LEB128 values beyond i128 of `la` and `lb` bytes, negative or not, byte j of
/// each read by `a` and `b`: by sign, then by length (a longer one is further from zero), then by their 7-bit
/// groups from the most significant.
pub fn big_order(
    (la, na): (u64, bool),
    (lb, nb): (u64, bool),
    mut a: impl FnMut(u64) -> Result<u8, Fault>,
    mut b: impl FnMut(u64) -> Result<u8, Fault>,
) -> Result<Ordering, Fault> {
    if na != nb {
        return Ok(if na { Ordering::Less } else { Ordering::Greater });
    }
    if la != lb {
        let o = la.cmp(&lb);
        return Ok(if na { o.reverse() } else { o });
    }
    let mut j = la;
    while j > 0 {
        j = j.wrapping_sub(1);
        let (x, y) = (a(j)? & 0x7F, b(j)? & 0x7F);
        if x != y {
            return Ok(x.cmp(&y));
        }
    }
    Ok(Ordering::Equal)
}

/// `piece` as one out-of-line copy, for the filter's per-row test, so that the row reads keep `piece`
/// inlined without a copy of it in every filter loop.
#[inline(never)]
pub fn piece_out<'h, H: PageHost + ?Sized>(h: &'h H, s: &SVal) -> Result<Option<&'h [u8]>, Fault> {
    piece(h, s)
}

/// A byte-string value as one slice of table memory, when it lies in one piece.
#[inline(always)]
pub fn piece<'h, H: PageHost + ?Sized>(h: &'h H, s: &SVal) -> Result<Option<&'h [u8]>, Fault> {
    let (a, len) = match *s {
        SVal::B { at, len } => (at, len),
        SVal::Huge { img, len, .. } => (At::Long { img }, len),
        _ => return Ok(None),
    };
    match a {
        At::Row { loc, off } => RowRef::new(h, loc)?.piece(off, len),
        At::Image { img, g, off } => {
            if (off % PAGE).saturating_add(len) > PAGE {
                return Ok(None);
            }
            Ok(Some(ImageReader::new(h, ImageId::Group(g), img).bytes(off, len)?))
        }
        At::Long { img } => {
            if long::HEAD.saturating_add(len) > PAGE {
                return Ok(None);
            }
            Ok(Some(ImageReader::new(h, ImageId::Runtime, img).bytes(long::HEAD, len)?))
        }
        At::Patch { block, off } => Ok(Some(crate::mem::sub(h.get(Region::Block(block))?, off, len)?)),
        At::Chain { head } => {
            if len > patch::CHAIN_ROOM {
                return Ok(None);
            }
            Ok(Some(crate::mem::sub(h.get(Region::Block(head))?, patch::CHAIN_DATA, len)?))
        }
    }
}

/// A Nat's or Int's LEB128 of `len` bytes at `a`: a number when it fits i128, else the bytes.
pub fn leb<H: PageHost + ?Sized>(h: &H, t: Ty, a: At, len: u64) -> Result<SVal, Fault> {
    let signed = matches!(t, Ty::Int);
    if len <= 19 {
        let mut b = [0u8; 19];
        let d = b.get_mut(..usize::try_from(len).unwrap_or(0)).unwrap_or(&mut []);
        read_at(h, &a, 0, d)?;
        if let Some(x) = cell::leb_value(d, signed) {
            return Ok(SVal::V(wide(t, x)));
        }
    }
    Ok(SVal::B { at: a, len })
}

/// A Nat or Int that fits i128 in its smallest cell.
pub fn wide(t: Ty, x: i128) -> Val<'static> {
    match t {
        Ty::Nat => u64::try_from(x).map_or(Val::Wide(x), Val::U),
        _ => i64::try_from(x).map_or(Val::Wide(x), Val::I),
    }
}

/// A buffered row's stored value of leaf `l` (whose item, for a variable leaf, is item `vi` of the row).
pub fn from_stored<H: PageHost + ?Sized>(
    row: &RowRef<'_, H>,
    shape: &super::layout::Shape,
    l: &LeafInfo,
    s: &Stored<'_>,
    vi: u64,
) -> Result<SVal, Fault> {
    if let Stored::Item(Item::Long { .. }) = *s {
        return from_stored_in(row, &row.head(shape)?, l, s, vi);
    }
    from_stored_in(row, &shape.head, l, s, vi)
}

/// `from_stored` in a row of layout `head` (only a long item reads it).
pub fn from_stored_in<H: PageHost + ?Sized>(
    row: &RowRef<'_, H>,
    head: &super::layout::RowHead,
    l: &LeafInfo,
    s: &Stored<'_>,
    vi: u64,
) -> Result<SVal, Fault> {
    Ok(match *s {
        Stored::Null => SVal::Null,
        Stored::Unit => SVal::Unit,
        Stored::Val(v) => SVal::V(own(v)),
        Stored::Bytes { at, len } => SVal::B { at: At::Row { loc: row.loc, off: at }, len },
        Stored::Item(Item::Inline { at, len }) => {
            let a = At::Row { loc: row.loc, off: at };
            if matches!(l.ty, Ty::Nat | Ty::Int) {
                leb(row.h, l.ty, a, len)?
            } else {
                SVal::B { at: a, len }
            }
        }
        Stored::Item(Item::Long { img, len }) => {
            let at = row.long_ref_in(head, vi)?.ok_or(rt(Corruption::Link, u64::from(l.ordinal)))?;
            SVal::Huge { img, len, owner: Owner::Item { loc: row.loc, at } }
        }
        Stored::Item(Item::None) => return Err(rt(Corruption::Link, u64::from(l.ordinal))),
    })
}

/// A group image's cell of leaf `l` (directory entry `ld`), at slot `s` of position `p`.
pub fn from_cell<H: PageHost + ?Sized>(
    gv: &GroupView<'_, H>,
    ld: &LeafDir,
    l: &LeafInfo,
    s: u64,
    p: u64,
) -> Result<SVal, Fault> {
    let c = gv.cell(ld, s, p)?;
    let signed = matches!(l.ty, Ty::I(_) | Ty::Int);
    Ok(match c {
        GCell::Null => SVal::Null,
        GCell::Unit => {
            if matches!(l.ty, Ty::Unit) {
                SVal::Unit
            } else {
                SVal::Null
            }
        }
        GCell::Bool(b) => SVal::V(Val::Bool(b)),
        GCell::U(x) if ld.form == form::SEQ && signed => SVal::V(Val::I(x as i64)),
        GCell::U(x) => SVal::V(Val::U(x)),
        GCell::I(x) => SVal::V(Val::I(x)),
        GCell::F32(b) => SVal::V(Val::F32(b)),
        GCell::F64(b) => SVal::V(Val::F64(b)),
        GCell::X16(b) => match l.ty {
            Ty::U128 => SVal::V(Val::U128(u128::from_le_bytes(b))),
            _ => SVal::V(Val::I128(i128::from_le_bytes(b))),
        },
        GCell::Bytes { off, len } => {
            let a = At::Image { img: gv.img, g: gv.g, off };
            match l.ty {
                Ty::Principal => {
                    let n = u64::from(rd_u8(gv.reader().bytes(off, 30)?, 29)?).min(29);
                    SVal::B { at: a, len: n }
                }
                Ty::Nat | Ty::Int => leb(gv.h, l.ty, a, len)?,
                _ => SVal::B { at: a, len },
            }
        }
        GCell::Huge { img, len, index } => SVal::Huge { img, len, owner: Owner::List { page: gv.page, index } },
    })
}

/// `from_cell`'s value of a cell held whole (`Val::Null` for a null); `None` for a byte string or a huge value.
pub fn whole(c: GCell, f: u8, ty: Ty) -> Option<Val<'static>> {
    let signed = matches!(ty, Ty::I(_) | Ty::Int);
    Some(match c {
        GCell::Null => Val::Null,
        GCell::Unit if matches!(ty, Ty::Unit) => Val::Unit,
        GCell::Unit => Val::Null,
        GCell::Bool(b) => Val::Bool(b),
        GCell::U(x) if f == form::SEQ && signed => Val::I(x as i64),
        GCell::U(x) => Val::U(x),
        GCell::I(x) => Val::I(x),
        GCell::F32(b) => Val::F32(b),
        GCell::F64(b) => Val::F64(b),
        GCell::X16(b) => match ty {
            Ty::U128 => Val::U128(u128::from_le_bytes(b)),
            _ => Val::I128(i128::from_le_bytes(b)),
        },
        GCell::Bytes { .. } | GCell::Huge { .. } => return None,
    })
}

/// `from_cell`'s value as a cell, read straight from the image: a byte string or a big Nat or Int copied into
/// `piece`.
pub fn cell_val<'a, H: PageHost + ?Sized>(
    gv: &GroupView<'_, H>,
    ld: &LeafDir,
    l: &LeafInfo,
    s: u64,
    p: u64,
    piece: &'a mut [u8],
) -> Result<Val<'a>, Fault> {
    let signed = matches!(l.ty, Ty::I(_) | Ty::Int);
    Ok(match gv.cell(ld, s, p)? {
        GCell::Null => Val::Null,
        GCell::Unit if matches!(l.ty, Ty::Unit) => Val::Unit,
        GCell::Unit => Val::Null,
        GCell::Bool(b) => Val::Bool(b),
        GCell::U(x) if ld.form == form::SEQ && signed => Val::I(x as i64),
        GCell::U(x) => Val::U(x),
        GCell::I(x) => Val::I(x),
        GCell::F32(b) => Val::F32(b),
        GCell::F64(b) => Val::F64(b),
        GCell::X16(b) => match l.ty {
            Ty::U128 => Val::U128(u128::from_le_bytes(b)),
            _ => Val::I128(i128::from_le_bytes(b)),
        },
        GCell::Bytes { off, len } => {
            let r = gv.reader();
            let n = match l.ty {
                Ty::Principal => u64::from(rd_u8(r.bytes(off, 30)?, 29)?).min(29),
                _ => len,
            };
            let d = piece.get_mut(..usize::try_from(n).unwrap_or(usize::MAX)).ok_or(Fault::Scratch { need: n })?;
            long::read_image(&r, off, d)?;
            match l.ty {
                Ty::Nat | Ty::Int => match cell::leb_value(d, matches!(l.ty, Ty::Int)).filter(|_| n <= 19) {
                    Some(x) => wide(l.ty, x),
                    None => Val::Big(d),
                },
                _ => Val::Bytes(d),
            }
        }
        GCell::Huge { img, len, .. } => {
            let d = piece.get_mut(..usize::try_from(len).unwrap_or(usize::MAX)).ok_or(Fault::Scratch { need: len })?;
            long::read(gv.h, img, 0, d)?;
            if matches!(l.ty, Ty::Nat | Ty::Int) {
                Val::Big(d)
            } else {
                Val::Bytes(d)
            }
        }
    })
}

/// A patch entry's value of leaf `l`.
pub fn from_entry<H: PageHost + ?Sized>(h: &H, e: &Entry, l: &LeafInfo) -> Result<SVal, Fault> {
    if e.flags & patch::F_NULL != 0 {
        return Ok(SVal::Null);
    }
    let s = h.get(Region::Block(e.block))?;
    if e.flags & patch::F_LONG != 0 {
        let img = Image::read_ref(s, e.data)?;
        let len = long::content_len(h, img)?;
        return Ok(SVal::Huge { img, len, owner: Owner::Patch { block: e.block, data: e.data } });
    }
    if e.flags & patch::F_CHAIN != 0 {
        let head = crate::mem::rd_u64(s, e.data)?;
        let len = u64::from(crate::mem::rd_u32(s, e.data.wrapping_add(8))?);
        let a = At::Chain { head };
        if matches!(l.ty, Ty::Nat | Ty::Int) {
            return leb(h, l.ty, a, len);
        }
        return Ok(SVal::B { at: a, len });
    }
    let d = crate::mem::sub(s, e.data, e.len)?;
    decode_ve(h, l, d, At::Patch { block: e.block, off: e.data })
}

/// A flat leaf's value from its VE `d`, which lies at `a`. A Bool has no VE: its updates are written in place.
pub fn decode_ve<H: PageHost + ?Sized>(h: &H, l: &LeafInfo, d: &[u8], a: At) -> Result<SVal, Fault> {
    let bad = || rt(Corruption::Encoding, u64::from(l.ordinal));
    let le = |n: usize| -> Result<u64, Fault> {
        let b = d.get(..n).filter(|b| b.len() <= 8).ok_or(bad())?;
        let mut x = [0u8; 8];
        copy_bytes(&mut x, b);
        Ok(u64::from_le_bytes(x))
    };
    let le16 = || -> Result<u128, Fault> {
        let b = d.get(..16).ok_or(bad())?;
        let mut x = [0u8; 16];
        copy_bytes(&mut x, b);
        Ok(u128::from_le_bytes(x))
    };
    let le4 = || -> Result<u32, Fault> { Ok(u32::from_le_bytes(crate::mem::rd_array::<4>(d, 0)?)) };
    let shift = |a: At, by: u64| match a {
        At::Patch { block, off } => At::Patch { block, off: off.wrapping_add(by) },
        At::Row { loc, off } => At::Row { loc, off: off.wrapping_add(by) },
        At::Image { img, g, off } => At::Image { img, g, off: off.wrapping_add(by) },
        other => other,
    };
    Ok(match l.ty {
        Ty::Unit => SVal::Unit,
        Ty::Bool => return Err(bad()),
        Ty::U(w) => SVal::V(Val::U(le(usize::from(w))?)),
        Ty::I(w) => {
            let raw = le(usize::from(w))?;
            let sh = 64u32.saturating_sub(u32::from(w).wrapping_mul(8));
            SVal::V(Val::I((raw.wrapping_shl(sh) as i64).wrapping_shr(sh)))
        }
        Ty::F32 => SVal::V(Val::F32(le4()?)),
        Ty::F64 => SVal::V(Val::F64(le(8)?)),
        Ty::Char => SVal::V(Val::U(u64::from(le4()?))),
        Ty::U128 => SVal::V(Val::U128(le16()?)),
        Ty::I128 => SVal::V(Val::I128(le16()? as i128)),
        Ty::Nat | Ty::Int => leb(h, l.ty, a, d.len() as u64)?,
        Ty::Text | Ty::Blob => {
            let (n, k) = crate::bytes::rd_uleb(d, 0, ImageId::Runtime)?;
            if k.saturating_add(n) != d.len() as u64 {
                return Err(bad());
            }
            SVal::B { at: shift(a, k), len: n }
        }
        Ty::Principal => {
            let n = u64::from(d.first().copied().ok_or(bad())?);
            if n > 29 || n.wrapping_add(1) != d.len() as u64 {
                return Err(bad());
            }
            SVal::B { at: shift(a, 1), len: n }
        }
        Ty::Fixed(n) => {
            if u64::from(n) != d.len() as u64 {
                return Err(bad());
            }
            SVal::B { at: a, len: u64::from(n) }
        }
    })
}

// ---------------------------------------------------------------- comparisons

/// A Principal's 30-byte cell (its key form) from its bytes.
pub fn principal_cell<H: PageHost + ?Sized>(h: &H, s: &SVal) -> Result<[u8; 30], Fault> {
    let mut c = [0u8; 30];
    let n = s.len().min(29);
    read(h, s, 0, c.get_mut(..usize::try_from(n).unwrap_or(0)).unwrap_or(&mut []))?;
    // n ≤ 29.
    #[allow(clippy::cast_possible_truncation)]
    {
        c[29] = n as u8;
    }
    Ok(c)
}

/// Whether a stored value equals a checked host value of the same leaf. A value still in the message's
/// argument is never equal: its bytes are not read before they are written.
pub fn same<H: PageHost + ?Sized>(h: &H, l: &LeafInfo, s: &SVal, v: &Val<'_>) -> Result<bool, Fault> {
    Ok(match (s, v) {
        (SVal::Null, Val::Null) => true,
        (SVal::Null, _) | (_, Val::Null) => false,
        (_, Val::Arg(_)) => false,
        (SVal::Unit, Val::Unit) => true,
        (SVal::V(a), b) => match (a, b) {
            (Val::F32(x), Val::F32(y)) => x == y,
            (Val::F64(x), Val::F64(y)) => x == y,
            // A Nat or Int compares by value, whatever cell it came in.
            (_, Val::U(_) | Val::I(_) | Val::Wide(_)) if matches!(l.ty, Ty::Nat | Ty::Int) => {
                as_i128(a) == as_i128(b) && as_i128(a).is_some()
            }
            _ => a == b,
        },
        (SVal::B { .. } | SVal::Huge { .. }, Val::Bytes(b) | Val::Big(b)) => {
            same_bytes(s.len(), b, |at, dst| read(h, s, at, dst))?
        }
        _ => false,
    })
}

fn as_i128(v: &Val<'_>) -> Option<i128> {
    match *v {
        Val::U(x) => Some(i128::from(x)),
        Val::I(x) => Some(i128::from(x)),
        Val::Wide(x) => Some(x),
        _ => None,
    }
}

/// Whether a stored byte string of `len` bytes, whose bytes `read(at, out)` copies, equals `b`.
pub fn same_bytes<F>(len: u64, b: &[u8], mut read: F) -> Result<bool, Fault>
where
    F: FnMut(u64, &mut [u8]) -> Result<(), Fault>,
{
    if len != b.len() as u64 {
        return Ok(false);
    }
    let mut piece = [0u8; 256];
    let mut at = 0u64;
    for chunk in b.chunks(piece.len()) {
        let dst = piece.get_mut(..chunk.len()).ok_or(rt(Corruption::Range, at))?;
        read(at, dst)?;
        if dst != chunk {
            return Ok(false);
        }
        at = at.wrapping_add(chunk.len() as u64);
    }
    Ok(true)
}

// ---------------------------------------------------------------- VE of host values

/// The VE of a host value whose VE is short (all but Text, Blob, `[u8; N]` and Nat and Int beyond i128, and
/// Bool, whose updates are written in place): its bytes and length. `None` for the others.
pub fn ve_short(l: &LeafInfo, v: &Val<'_>, out: &mut [u8; 32]) -> Option<usize> {
    let mut put = |b: &[u8]| {
        copy_bytes(out, b);
        b.len()
    };
    Some(match (l.ty, *v) {
        (_, Val::Null) => 0,
        (Ty::Unit, _) => 0,
        (Ty::U(w), Val::U(x)) => put(x.to_le_bytes().get(..usize::from(w))?),
        (Ty::I(w), Val::I(x)) => put(x.to_le_bytes().get(..usize::from(w))?),
        (Ty::F32, Val::F32(b)) => put(&b.to_le_bytes()),
        (Ty::F64, Val::F64(b)) => put(&b.to_le_bytes()),
        (Ty::Char, Val::U(x)) => put(&u32::try_from(x).ok()?.to_le_bytes()),
        (Ty::U128, Val::U128(x)) => put(&x.to_le_bytes()),
        (Ty::I128, Val::I128(x)) => put(&x.to_le_bytes()),
        (Ty::Nat | Ty::Int, Val::U(_) | Val::I(_) | Val::Wide(_)) => {
            let mut b = [0u8; 19];
            let n = cell::small_leb(v, matches!(l.ty, Ty::Int), &mut b);
            put(b.get(..n)?)
        }
        (Ty::Principal, Val::Bytes(b)) if b.len() <= 29 => {
            let n = b.len();
            // n ≤ 29.
            #[allow(clippy::cast_possible_truncation)]
            let head = n as u8;
            out[0] = head;
            copy_bytes(out.get_mut(1..)?, b);
            n.wrapping_add(1)
        }
        _ => return None,
    })
}

/// The VE header and payload length of a host value of a Text, Blob, `[u8; N]` or big Nat or Int: the header
/// (a ULEB128 length for Text and Blob, nothing for the others) and the payload's bytes.
pub fn ve_long_parts(l: &LeafInfo, v: &Val<'_>, head: &mut [u8; 11]) -> Option<(usize, u64)> {
    let n = cell::var_len(v, l.ty);
    match (l.ty, *v) {
        (Ty::Text | Ty::Blob, Val::Bytes(_) | Val::Arg(_)) => {
            let k = crate::bytes::put_uleb(head, 0, n).ok()?;
            Some((usize::try_from(k).ok()?, n))
        }
        (Ty::Fixed(_), Val::Bytes(b)) => Some((0, b.len() as u64)),
        (Ty::Nat | Ty::Int, Val::Big(b)) => Some((0, b.len() as u64)),
        _ => None,
    }
}
