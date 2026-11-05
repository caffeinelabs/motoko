//! Values of leaves a row or an image lacks: a leaf added by a later schema version reads its default in
//! the rows and groups written before it, and a row of an older version keeps the layout of its own
//! version.
//!
//! The default of an added column is its declared default (the column's default VE in the schema image),
//! else null for an option, else the zero of a zero-valid type; so is a field's added to a record a column
//! reaches, its declared default the field entry's (its default offset and length in the default pool).

use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::mem::{rd_u16, rd_u32, rd_u8, PageHost, Region};
use crate::schema::SchemaView;
use crate::value::{skip_ty, ty, FRAME};

use super::cell::Val;
use super::cur::{self, At, SVal};
use super::layout::{LeafInfo, Shape, Ty};

fn rt(kind: Corruption, at: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, at)
}

/// The default of leaf `l`.
pub fn of<H: PageHost + ?Sized>(h: &H, shape: &Shape, l: &LeafInfo) -> Result<SVal, Fault> {
    if let Some(v) = declared(h, shape, l)? {
        return Ok(v);
    }
    Ok(cur::zero_of(l, shape.page))
}

/// The deepest nesting of records a field's default is looked up through.
const FIELD_DEPTH: usize = 16;

/// The record field whose type holds type offset `path`, among the records column type `cty` nests (a
/// record's fields, an option's record, recursively): its default's (offset, length) in the default pool and
/// its type's offset. `None` when no record field holds it.
pub fn field_of(pool: &[u8], cty: u32, path: u32) -> Result<Option<(u32, u32, u32)>, Fault> {
    let mut scratch = [0u8; 8 * FRAME];
    let mut at = cty;
    for _ in 0..FIELD_DEPTH {
        let code = rd_u8(pool, u64::from(at))?;
        if code == ty::OPTION {
            at = at.checked_add(1).ok_or(rt(Corruption::Encoding, u64::from(at)))?;
            continue;
        }
        if code != ty::RECORD {
            return Ok(None);
        }
        let n = u32::from(rd_u16(pool, u64::from(at).wrapping_add(1))?);
        let a = u32::from(rd_u16(pool, u64::from(at).wrapping_add(3))?);
        let mut e = at.checked_add(5).ok_or(rt(Corruption::Encoding, u64::from(at)))?;
        let mut inner = None;
        for _ in 0..n.saturating_add(a) {
            let t = e.checked_add(ty::FIELD_HDR).ok_or(rt(Corruption::Encoding, u64::from(e)))?;
            let end = skip_ty(pool, t, &mut scratch)?;
            if path >= t && path < end {
                let off = rd_u32(pool, u64::from(e).wrapping_add(9))?;
                let len = rd_u32(pool, u64::from(e).wrapping_add(13))?;
                let fc = rd_u8(pool, u64::from(t))?;
                // A field that is itself a record (or an option of one) holds the leaf one level down.
                let nested =
                    fc == ty::RECORD || (fc == ty::OPTION && rd_u8(pool, u64::from(t).wrapping_add(1))? == ty::RECORD);
                if nested && path != t {
                    inner = Some(t);
                    break;
                }
                return Ok(Some((off, len, t)));
            }
            e = end;
        }
        match inner {
            Some(t) => at = t,
            None => return Ok(None),
        }
    }
    Ok(None)
}

/// A declared default: leaf `l` is a whole column whose default VE is in the schema image, or a record
/// field's whose entry names one.
fn declared<H: PageHost + ?Sized>(h: &H, shape: &Shape, l: &LeafInfo) -> Result<Option<SVal>, Fault> {
    let bytes = h.get(Region::Page(shape.page))?;
    let v = SchemaView::open_stored(bytes)?;
    let col = l.id >> 16;
    // Columns are numbered in their table's order: the column at index `col` is the one, and the walk over
    // the columns is for a table whose ids say otherwise.
    let (from, to) = match u16::try_from(col) {
        Ok(ci) if ci < v.columns && v.column(ci)?.id == col => (ci, ci.wrapping_add(1)),
        _ => (0, v.columns),
    };
    for i in from..to {
        let c = v.column(i)?;
        if c.id != col {
            continue;
        }
        let types = u64::from(v.layout.types);
        // The column is this one scalar leaf, its type the leaf's own (or an option of it); else the leaf is a
        // record field's, and the field's entry names its default.
        let (tpos, off, len) = if l.id & 0xFFFF == 0 && c.default_len > 0 {
            (c.ty, c.default_off, c.default_len)
        } else {
            match field_of(v.types(), c.ty, l.path)? {
                Some((off, len, t)) if len > 0 => (t, off, len),
                _ => return Ok(None),
            }
        };
        let tcode = rd_u8(bytes, types.wrapping_add(u64::from(tpos)))?;
        let at = u64::from(v.layout.defaults).wrapping_add(u64::from(off));
        let len = u64::from(len);
        let d = crate::mem::sub(bytes, at, len)?;
        if tcode == ty::OPTION {
            // An option's VE: 0, or 1 and the value.
            return match d.first() {
                Some(0) => Ok(Some(SVal::Null)),
                Some(1) => {
                    let rest = d.get(1..).ok_or(rt(Corruption::Encoding, at))?;
                    Ok(Some(cur::decode_ve(h, l, rest, At::Schema { page: shape.page, off: at.wrapping_add(1) })?))
                }
                _ => Err(rt(Corruption::Encoding, at)),
            };
        }
        if !matches!(l.ty, Ty::Unit) && tcode == ty::UNIT {
            return Ok(Some(SVal::Unit));
        }
        return Ok(Some(cur::decode_ve(h, l, d, At::Schema { page: shape.page, off: at })?));
    }
    Ok(None)
}

/// Whether a default is a value a running record counts as non-null, and its sum part.
pub fn small_of(v: &SVal) -> Option<i128> {
    match v {
        SVal::V(Val::U(x)) => Some(i128::from(*x)),
        SVal::V(Val::I(x)) => Some(i128::from(*x)),
        _ => None,
    }
}
