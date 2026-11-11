//! The shared schema: a column's type as its leaf, the 28-byte leaf records, and the schema image.
//!
//! Types are expressions in a type pool (`value::ty`): every column is a scalar, `()`, a list, or an option of a
//! scalar or of a list, one leaf; a record outside a list, a tuple, an option of anything else, and any other
//! type code are refused as corrupt. Leaf ordinals are table-wide and append-only.

use crate::consts::{magic, FORMAT_IMAGE, IMAGE_V1};
use crate::fault::{bounds, corrupt, Corruption, Fault, ImageId};
use crate::hash::Hasher64;
use crate::mem::{put, put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u8};
use crate::value::ty;

/// Node kinds of leaf records.
pub mod node {
    pub const SCALAR: u8 = 1;
    pub const UNIT: u8 = 6;
    /// A leaf whose field a schema change dropped: its record keeps its id, code and width, with the
    /// nullable flag set.
    pub const DROPPED: u8 = 8;
}

/// Leaf record flags.
pub const LEAF_NULLABLE: u8 = 1;
pub const LEAF_KEY: u8 = 2;

/// Bytes of a leaf record: {id u32 @0; kind u8 @4; code u8 @5; flags u8 @6; zero u8 @7; width u16 @8; zero
/// u16 @10; path u32 @12; added u32 @16; dropped u32 @20; name u32 @24}.
pub const LEAF_BYTES: u64 = 28;

/// A leaf record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Leaf {
    /// Column id · 65,536 + the leaf's ordinal within its column.
    pub id: u32,
    pub kind: u8,
    /// The type code of a Scalar leaf, else 0.
    pub code: u8,
    pub flags: u8,
    /// N of a `[u8; N]`, else 0.
    pub width: u16,
    /// The leaf's type expression in the pool.
    pub path: u32,
    pub added: u32,
    pub dropped: u32,
    /// The record field's name in the name pool, or 0.
    pub name: u32,
}

impl Leaf {
    pub fn write(&self, b: &mut [u8], off: u64) -> Result<(), Fault> {
        put_u32(b, off, self.id)?;
        put_u8(b, off.wrapping_add(4), self.kind)?;
        put_u8(b, off.wrapping_add(5), self.code)?;
        put_u8(b, off.wrapping_add(6), self.flags)?;
        put_u8(b, off.wrapping_add(7), 0)?;
        put_u16(b, off.wrapping_add(8), self.width)?;
        put_u16(b, off.wrapping_add(10), 0)?;
        put_u32(b, off.wrapping_add(12), self.path)?;
        put_u32(b, off.wrapping_add(16), self.added)?;
        put_u32(b, off.wrapping_add(20), self.dropped)?;
        put_u32(b, off.wrapping_add(24), self.name)
    }

    pub fn read(b: &[u8], off: u64) -> Result<Leaf, Fault> {
        Ok(Leaf {
            id: rd_u32(b, off)?,
            kind: rd_u8(b, off.wrapping_add(4))?,
            code: rd_u8(b, off.wrapping_add(5))?,
            flags: rd_u8(b, off.wrapping_add(6))?,
            width: rd_u16(b, off.wrapping_add(8))?,
            path: rd_u32(b, off.wrapping_add(12))?,
            added: rd_u32(b, off.wrapping_add(16))?,
            dropped: rd_u32(b, off.wrapping_add(20))?,
            name: rd_u32(b, off.wrapping_add(24))?,
        })
    }
}

/// Where flattened leaves go.
pub trait LeafSink {
    fn leaf(&mut self, l: &Leaf) -> Result<(), Fault>;
}

fn rt(at: u32) -> Fault {
    corrupt(ImageId::Runtime, Corruption::Encoding, u64::from(at))
}

/// One column to flatten: its id, its type expression in `pool`, the schema version adding it, the
/// table-wide ordinal of its first leaf, and whether it is the key column.
#[derive(Clone, Copy, Debug)]
pub struct ColumnType<'p> {
    pub pool: &'p [u8],
    pub column: u32,
    pub ty: u32,
    pub version: u32,
    pub first_ordinal: u16,
    pub key: bool,
}

/// The offset just past the list type at `at` in `pool`: `ty::LIST`, then its element, a scalar, an option of
/// one, or a record (`ty::RECORD`) whose fields are each a scalar or an option of one.
pub fn list_end(pool: &[u8], at: u32) -> Result<u32, Fault> {
    // The offset past the scalar or the option of a scalar at `p`.
    let scalar = |p: u32| {
        let s = if rd_u8(pool, u64::from(p))? == ty::OPTION { p.checked_add(1).ok_or(rt(p))? } else { p };
        match rd_u8(pool, u64::from(s))? {
            ty::BOOL..=ty::I128 => s.checked_add(1).ok_or(rt(s)),
            _ => Err(rt(s)),
        }
    };
    if rd_u8(pool, u64::from(at))? != ty::LIST {
        return Err(rt(at));
    }
    let e = at.checked_add(1).ok_or(rt(at))?;
    if rd_u8(pool, u64::from(e))? != ty::RECORD {
        return scalar(e);
    }
    let mut p = e.checked_add(3).ok_or(rt(e))?;
    for _ in 0..rd_u16(pool, u64::from(e).wrapping_add(1))? {
        p = scalar(p)?;
        let name = rd_u16(pool, u64::from(p).wrapping_add(4))?;
        p = p.checked_add(6).and_then(|x| x.checked_add(u32::from(name))).ok_or(rt(p))?;
    }
    if u64::from(p) > pool.len() as u64 {
        return Err(rt(p));
    }
    Ok(p)
}

/// Flattens a column's type into its leaf, appended to `out` with the column's first ordinal; returns the
/// number of leaves, 1. A list's leaf is a byte string (`ty::BLOB`).
pub fn flatten<S: LeafSink>(col: ColumnType<'_>, out: &mut S) -> Result<u16, Fault> {
    let ColumnType { pool, column, ty: t, version, first_ordinal: _, key } = col;
    let mut at = t;
    let mut code = rd_u8(pool, u64::from(at))?;
    // An option of a scalar: the scalar's leaf, nullable.
    let nullable = code == ty::OPTION;
    if nullable {
        at = at.checked_add(1).ok_or(rt(at))?;
        code = rd_u8(pool, u64::from(at))?;
    }
    let (kind, code, width) = match code {
        ty::BOOL..=ty::I128 => (node::SCALAR, code, 0),
        ty::BYTES => (node::SCALAR, code, rd_u16(pool, u64::from(at).wrapping_add(1))?),
        ty::UNIT if !nullable => (node::UNIT, 0, 0),
        ty::LIST => {
            list_end(pool, at)?;
            (node::SCALAR, ty::BLOB, 0)
        }
        _ => return Err(rt(at)),
    };
    let id = column.checked_mul(65_536).ok_or(rt(at))?;
    let mut flags = 0u8;
    if nullable {
        flags |= LEAF_NULLABLE;
    }
    if key {
        flags |= LEAF_KEY;
    }
    out.leaf(&Leaf { id, kind, code, flags, width, path: at, added: version, dropped: 0, name: 0 })?;
    Ok(1)
}

// ---------------------------------------------------------------- the schema image

/// Image kinds.
pub const KIND_SCHEMA: u8 = 3;

/// Writes the common image header at offset 0.
pub fn put_common_header(
    b: &mut [u8],
    kind: u8,
    flags: u8,
    bytes: u64,
    uid: u64,
    schema_hash: u64,
) -> Result<(), Fault> {
    put_u32(b, 0, magic::NTIM)?;
    put_u16(b, 4, IMAGE_V1)?;
    put_u8(b, 6, kind)?;
    put_u8(b, 7, flags)?;
    put_u64(b, 8, bytes)?;
    put_u64(b, 16, uid)?;
    put_u64(b, 24, schema_hash)
}

/// A column of a schema: its id, flags (bit 0 dropped), first leaf, its type and default in the pools,
/// its name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Column {
    pub id: u32,
    pub flags: u8,
    pub first_leaf: u16,
    pub ty: u32,
    pub default_off: u32,
    pub default_len: u32,
    pub name: u32,
}

/// An index declaration: id, flags (bit 0 primary, bit 1 `eq_only`, bit 2 key runs allowed), W, and up to 7
/// parts {leaf id, nullable}. In the image, `DECL_BYTES` bytes: {id u8 @0; flags u8 @1; W u16 @2; parts u8
/// @4; nullable parts u8 @5 (bit k: part k); zero u16 @6; part k's leaf id u32 @8 + 4k; zero u32 @36}.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IndexDecl {
    pub id: u8,
    pub flags: u8,
    pub w: u16,
    pub parts: u8,
    pub part: [(u32, bool); 7],
}

/// Bytes of an index declaration in a schema image.
pub const DECL_BYTES: u64 = 40;

/// The parts of a schema image to write.
pub struct SchemaParts<'a> {
    pub version: u32,
    pub columns: &'a [Column],
    pub leaves: &'a [Leaf],
    pub indexes: &'a [IndexDecl],
    pub type_pool: &'a [u8],
    pub default_pool: &'a [u8],
    pub name_pool: &'a [u8],
}

/// Section offsets of a schema image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SchemaLayout {
    pub columns: u32,
    pub leaves: u32,
    pub indexes: u32,
    pub types: u32,
    pub defaults: u32,
    pub names: u32,
    pub total: u32,
}

fn align8(x: u64) -> u64 {
    x.wrapping_add(7) & !7
}

/// The layout of a schema image of these parts.
pub fn layout(p: &SchemaParts<'_>) -> Result<SchemaLayout, Fault> {
    let tooky = |x: u64| u32::try_from(x).map_err(|_| bounds(x));
    let cols = 76u64;
    let leaves = cols.checked_add((p.columns.len() as u64).checked_mul(24).ok_or(bounds(0))?).ok_or(bounds(0))?;
    let indexes =
        leaves.checked_add((p.leaves.len() as u64).checked_mul(LEAF_BYTES).ok_or(bounds(0))?).ok_or(bounds(0))?;
    let types =
        indexes.checked_add((p.indexes.len() as u64).checked_mul(DECL_BYTES).ok_or(bounds(0))?).ok_or(bounds(0))?;
    let defaults = align8(types.checked_add(p.type_pool.len() as u64).ok_or(bounds(0))?);
    let names = align8(defaults.checked_add(p.default_pool.len() as u64).ok_or(bounds(0))?);
    let total = align8(names.checked_add(p.name_pool.len() as u64).ok_or(bounds(0))?);
    Ok(SchemaLayout {
        columns: tooky(cols)?,
        leaves: tooky(leaves)?,
        indexes: tooky(indexes)?,
        types: tooky(types)?,
        defaults: tooky(defaults)?,
        names: tooky(names)?,
        total: tooky(total)?,
    })
}

/// Writes a schema image (kind 3) into `out`, every byte defined; returns (bytes, schema hash). The hash is
/// the canonical schema hash (`SchemaView::canonical_hash`) and is also recorded in the common header.
pub fn write_schema(p: &SchemaParts<'_>, uid: u64, out: &mut [u8]) -> Result<(u64, u64), Fault> {
    let l = layout(p)?;
    let total = u64::from(l.total);
    let body =
        out.get_mut(..usize::try_from(total).map_err(|_| bounds(total))?).ok_or(Fault::Scratch { need: total })?;
    body.fill(0);
    let cn = u16::try_from(p.columns.len()).map_err(|_| bounds(0))?;
    let ln = u16::try_from(p.leaves.len()).map_err(|_| bounds(0))?;
    let inn = u16::try_from(p.indexes.len()).map_err(|_| bounds(0))?;
    put_u32(body, 32, p.version)?;
    put_u16(body, 36, cn)?;
    put_u16(body, 38, ln)?;
    put_u16(body, 40, inn)?;
    for (k, v) in [l.columns, l.leaves, l.indexes, l.types, l.defaults, l.names, l.total].into_iter().enumerate() {
        put_u32(body, 48u64.wrapping_add((k as u64).wrapping_mul(4)), v)?;
    }
    for (i, c) in p.columns.iter().enumerate() {
        let o = u64::from(l.columns).wrapping_add((i as u64).wrapping_mul(24));
        put_u32(body, o, c.id)?;
        put_u8(body, o.wrapping_add(4), c.flags)?;
        put_u16(body, o.wrapping_add(6), c.first_leaf)?;
        put_u32(body, o.wrapping_add(8), c.ty)?;
        put_u32(body, o.wrapping_add(12), c.default_off)?;
        put_u32(body, o.wrapping_add(16), c.default_len)?;
        put_u32(body, o.wrapping_add(20), c.name)?;
    }
    for (i, lf) in p.leaves.iter().enumerate() {
        lf.write(body, u64::from(l.leaves).wrapping_add((i as u64).wrapping_mul(LEAF_BYTES)))?;
    }
    for (i, d) in p.indexes.iter().enumerate() {
        let o = u64::from(l.indexes).wrapping_add((i as u64).wrapping_mul(DECL_BYTES));
        put_u8(body, o, d.id)?;
        put_u8(body, o.wrapping_add(1), d.flags)?;
        put_u16(body, o.wrapping_add(2), d.w)?;
        put_u8(body, o.wrapping_add(4), d.parts)?;
        let mut nullable_parts = 0u8;
        for (k, (leaf, nullable)) in d.part.iter().enumerate().take(usize::from(d.parts.min(7))) {
            put_u32(body, o.wrapping_add(8).wrapping_add((k as u64).wrapping_mul(4)), *leaf)?;
            nullable_parts |= u8::from(*nullable) << k;
        }
        put_u8(body, o.wrapping_add(5), nullable_parts)?;
    }
    put(body, u64::from(l.types), p.type_pool)?;
    put(body, u64::from(l.defaults), p.default_pool)?;
    put(body, u64::from(l.names), p.name_pool)?;
    put_common_header(body, KIND_SCHEMA, 0, total, uid, 0)?;
    let hash = SchemaView::open_checked(body, false)?.canonical_hash()?;
    put_u64(body, 24, hash)?;
    Ok((total, hash))
}

/// The canonical form's writer over a type pool and a name pool.
struct Canon<'a, 'o> {
    pool: &'a [u8],
    names: &'a [u8],
    out: &'o mut dyn FnMut(&[u8]),
}

impl Canon<'_, '_> {
    fn rt(at: u32) -> Fault {
        corrupt(ImageId::Runtime, Corruption::Encoding, u64::from(at))
    }

    /// A name reference as u16 length ‖ bytes: a name's bytes begin at its offset in the name pool, its u16
    /// length just before them; an offset below 2 names none.
    fn name(&mut self, off: u32) -> Result<(), Fault> {
        let o = u64::from(off);
        let n = if o < 2 { 0 } else { rd_u16(self.names, o.wrapping_sub(2))? };
        (self.out)(&n.to_le_bytes());
        if n > 0 {
            (self.out)(crate::mem::sub(self.names, o, u64::from(n))?);
        }
        Ok(())
    }

    /// The canonical form of the type at `at`, a scalar, `()`, a list, or an option of a scalar or of a list.
    fn ty(&mut self, at: u32) -> Result<(), Fault> {
        let code = rd_u8(self.pool, u64::from(at))?;
        (self.out)(&[code]);
        let p = at.checked_add(1).ok_or(Self::rt(at))?;
        match code {
            ty::BOOL..=ty::I128 | ty::UNIT => Ok(()),
            ty::BYTES => {
                (self.out)(&rd_u16(self.pool, u64::from(p))?.to_le_bytes());
                Ok(())
            }
            // A list names nothing in another pool: its operand is its bytes.
            ty::LIST => {
                let end = list_end(self.pool, at)?;
                (self.out)(crate::mem::sub(self.pool, u64::from(p), u64::from(end.wrapping_sub(p)))?);
                Ok(())
            }
            ty::OPTION if rd_u8(self.pool, u64::from(p))? != ty::OPTION => self.ty(p),
            _ => Err(Self::rt(at)),
        }
    }
}

/// A parsed view of a schema image.
#[derive(Clone, Copy, Debug)]
pub struct SchemaView<'a> {
    pub bytes: &'a [u8],
    pub version: u32,
    pub columns: u16,
    pub leaves: u16,
    pub indexes: u16,
    pub layout: SchemaLayout,
    pub hash: u64,
}

impl<'a> SchemaView<'a> {
    /// Reads a schema image written by the runtime; a magic, kind or canonical hash that does not match is
    /// `Corrupt(Runtime, Header)`.
    pub fn open(bytes: &'a [u8]) -> Result<Self, Fault> {
        Self::open_checked(bytes, true)
    }

    fn open_checked(bytes: &'a [u8], check: bool) -> Result<Self, Fault> {
        let hdr = |at: u64| corrupt(ImageId::Runtime, Corruption::Header, at);
        if rd_u32(bytes, 0)? != magic::NTIM || rd_u8(bytes, 6)? != KIND_SCHEMA || rd_u16(bytes, 4)? > FORMAT_IMAGE {
            return Err(hdr(0));
        }
        let total = crate::mem::rd_u64(bytes, 8)?;
        let body = bytes.get(..usize::try_from(total).map_err(|_| hdr(8))?).ok_or(hdr(8))?;
        let hash = crate::mem::rd_u64(bytes, 24)?;
        let rd = |k: u64| rd_u32(body, 48u64.wrapping_add(k.wrapping_mul(4)));
        let layout = SchemaLayout {
            columns: rd(0)?,
            leaves: rd(1)?,
            indexes: rd(2)?,
            types: rd(3)?,
            defaults: rd(4)?,
            names: rd(5)?,
            total: rd(6)?,
        };
        let v = SchemaView {
            bytes: body,
            version: rd_u32(body, 32)?,
            columns: rd_u16(body, 36)?,
            leaves: rd_u16(body, 38)?,
            indexes: rd_u16(body, 40)?,
            layout,
            hash,
        };
        // An image whose canonical form cannot be read cannot match its hash either.
        if check && v.canonical_hash().map_err(|_| hdr(24))? != hash {
            return Err(hdr(24));
        }
        Ok(v)
    }

    /// The schema hash: `nt_hash64` of the image's canonical row-type form, which depends on the row type
    /// alone and never on the table's history (its schema versions, leaf ordinals, the order fields were
    /// added in, or pool offsets). The form is, per column in column-id order: {u32 column id; u8 flags (bit 0
    /// dropped); u16 name length ‖ the name's bytes; the column's type in canonical form; u32 default length ‖
    /// the default's bytes}. A type's canonical form is its codes and operands as in the type pool; a name
    /// offset becomes u16 length ‖ the name's bytes (an offset below 2 names none). O(image bytes), twice: once
    /// to count the form's bytes, once to hash them.
    pub fn canonical_hash(&self) -> Result<u64, Fault> {
        let mut n = 0u64;
        self.canonical(&mut |b: &[u8]| n = n.saturating_add(b.len() as u64))?;
        let mut hs = Hasher64::new(n);
        self.canonical(&mut |b: &[u8]| hs.update(b))?;
        Ok(hs.finish())
    }

    fn canonical(&self, out: &mut dyn FnMut(&[u8])) -> Result<(), Fault> {
        let pool = self.types();
        let defaults = self.bytes.get(self.layout.defaults as usize..self.layout.names as usize).unwrap_or(&[]);
        let names = self.bytes.get(self.layout.names as usize..self.layout.total as usize).unwrap_or(&[]);
        let mut c = Canon { pool, names, out };
        let mut last: Option<u32> = None;
        for i in 0..self.columns {
            let col = self.column(i)?;
            if last.is_some_and(|x| x >= col.id) {
                return Err(corrupt(ImageId::Runtime, Corruption::Order, u64::from(col.id)));
            }
            last = Some(col.id);
            (c.out)(&col.id.to_le_bytes());
            (c.out)(&[col.flags & 1]);
            c.name(col.name)?;
            c.ty(col.ty)?;
            (c.out)(&col.default_len.to_le_bytes());
            (c.out)(crate::mem::sub(defaults, u64::from(col.default_off), u64::from(col.default_len))?);
        }
        Ok(())
    }

    pub fn leaf(&self, ordinal: u16) -> Result<Leaf, Fault> {
        if ordinal >= self.leaves {
            return Err(bounds(u64::from(ordinal)));
        }
        Leaf::read(self.bytes, u64::from(self.layout.leaves).wrapping_add(u64::from(ordinal).wrapping_mul(LEAF_BYTES)))
    }

    pub fn column(&self, i: u16) -> Result<Column, Fault> {
        if i >= self.columns {
            return Err(bounds(u64::from(i)));
        }
        let o = u64::from(self.layout.columns).wrapping_add(u64::from(i).wrapping_mul(24));
        let b = self.bytes;
        Ok(Column {
            id: rd_u32(b, o)?,
            flags: rd_u8(b, o.wrapping_add(4))?,
            first_leaf: rd_u16(b, o.wrapping_add(6))?,
            ty: rd_u32(b, o.wrapping_add(8))?,
            default_off: rd_u32(b, o.wrapping_add(12))?,
            default_len: rd_u32(b, o.wrapping_add(16))?,
            name: rd_u32(b, o.wrapping_add(20))?,
        })
    }

    pub fn types(&self) -> &'a [u8] {
        self.bytes.get(self.layout.types as usize..self.layout.defaults as usize).unwrap_or(&[])
    }
}
