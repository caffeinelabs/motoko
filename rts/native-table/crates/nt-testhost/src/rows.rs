//! Rows for table tests: schemas built from type expressions, owned cell values, a `RowSource` over them
//! (argument values included, with a count of the bytes the table asked the host to copy), and a `RowSink`
//! that collects what a read hands out.

use native_table::schema::{
    flatten, node, write_schema, Column, ColumnType, IndexDecl, Leaf, LeafSink, SchemaParts, LEAF_NULLABLE,
};
use native_table::table::{CellIn, CellOut, RowSink, RowSource, Table};
use native_table::value::ty;
use native_table::{Fault, PageHost};

/// The Candid idl hash of a field name: h = h · 223 + b mod 2^32 over its UTF-8 bytes.
pub fn idl_hash(name: &[u8]) -> u32 {
    name.iter().fold(0u32, |h, &b| h.wrapping_mul(223).wrapping_add(u32::from(b)))
}

/// An owned cell value.
#[derive(Clone, Debug, PartialEq)]
pub enum V {
    Null,
    Bool(bool),
    U(u64),
    I(i64),
    U128(u128),
    I128(i128),
    F32(u32),
    F64(u64),
    Bytes(Vec<u8>),
    Nat(Vec<u8>),
    Int(Vec<u8>),
    Unit,
    /// In: a value the table copies from the message's argument. Out: a value read with `read_large`.
    Arg(Vec<u8>),
}

impl V {
    pub fn text(s: &str) -> V {
        V::Bytes(s.as_bytes().to_vec())
    }
}

/// Type expressions.
pub mod t {
    use super::*;
    pub fn scalar(code: u8) -> Vec<u8> {
        vec![code]
    }
    pub fn bytes(n: u16) -> Vec<u8> {
        let mut v = vec![ty::BYTES];
        v.extend(n.to_le_bytes());
        v
    }
    pub fn opt(inner: Vec<u8>) -> Vec<u8> {
        let mut v = vec![ty::OPTION];
        v.extend(inner);
        v
    }
}

struct Leaves(Vec<Leaf>);

impl LeafSink for Leaves {
    fn leaf(&mut self, l: &Leaf) -> Result<(), Fault> {
        self.0.push(*l);
        Ok(())
    }
}

/// A schema image of columns given as type expressions, column 0 the key; returns the image and its leaves.
pub fn schema(cols: &[Vec<u8>]) -> (Vec<u8>, Vec<Leaf>) {
    schema_indexed(cols, &[])
}

/// `schema` with these index declarations.
pub fn schema_indexed(cols: &[Vec<u8>], indexes: &[IndexDecl]) -> (Vec<u8>, Vec<Leaf>) {
    let cols: Vec<VCol> = cols.iter().map(|c| VCol { ty: c.clone(), added: 1, dropped: 0, default: None }).collect();
    schema_versioned(1, &cols, indexes)
}

/// A column of a schema version: its type expression, the version that added it, the version that dropped
/// it (0 for none) and its declared default (its VE). A dropped column is written as a schema change writes
/// it: the column flagged dropped, each of its leaves of the dropped kind with the nullable flag set.
#[derive(Clone)]
pub struct VCol {
    pub ty: Vec<u8>,
    pub added: u32,
    pub dropped: u32,
    pub default: Option<Vec<u8>>,
}

/// The schema image of version `version` with these columns, column 0 the key, and index declarations;
/// returns the image and its leaves.
pub fn schema_versioned(version: u32, cols: &[VCol], indexes: &[IndexDecl]) -> (Vec<u8>, Vec<Leaf>) {
    let mut pool = Vec::new();
    let mut at = Vec::new();
    for c in cols {
        at.push(pool.len() as u32);
        pool.extend(&c.ty);
    }
    let mut out = Leaves(Vec::new());
    let mut columns = Vec::new();
    let mut defaults = Vec::new();
    for (i, (&tpos, c)) in at.iter().zip(cols).enumerate() {
        let first = out.0.len() as u16;
        let ct =
            ColumnType { pool: &pool, column: i as u32, ty: tpos, version: c.added, first_ordinal: first, key: i == 0 };
        flatten(ct, &mut out).unwrap();
        if c.dropped != 0 {
            for l in out.0.iter_mut().skip(usize::from(first)) {
                l.kind = node::DROPPED;
                l.flags |= LEAF_NULLABLE;
            }
        }
        let (default_off, default_len) = match &c.default {
            Some(d) => {
                defaults.extend(d);
                ((defaults.len() - d.len()) as u32, d.len() as u32)
            }
            None => (0, 0),
        };
        columns.push(Column {
            id: i as u32,
            flags: u8::from(c.dropped != 0),
            first_leaf: first,
            ty: tpos,
            default_off,
            default_len,
            name: 0,
        });
    }
    let parts = SchemaParts {
        version,
        columns: &columns,
        leaves: &out.0,
        indexes,
        type_pool: &pool,
        default_pool: &defaults,
        name_pool: &[],
    };
    let mut img = vec![0u8; 65_536];
    let (bytes, _) = write_schema(&parts, 1, &mut img).unwrap();
    img.truncate(bytes as usize);
    (img, out.0)
}

/// A row (or a key) as cells by leaf id.
pub struct Src<'a> {
    pub cells: &'a [(u32, V)],
    /// Bytes the table asked the host to copy from the argument.
    pub filled: u64,
    /// `fill` calls.
    pub fills: u64,
}

impl<'a> Src<'a> {
    pub fn new(cells: &'a [(u32, V)]) -> Self {
        Src { cells, filled: 0, fills: 0 }
    }
}

impl RowSource for Src<'_> {
    fn cell(&mut self, leaf: u32) -> Result<CellIn<'_>, Fault> {
        let v = self.cells.iter().find(|(l, _)| *l == leaf).map(|(_, v)| v).ok_or(Fault::BadValue { leaf })?;
        Ok(match v {
            V::Null => CellIn::Null,
            V::Bool(b) => CellIn::Bool(*b),
            V::U(x) => CellIn::U64(*x),
            V::I(x) => CellIn::I64(*x),
            V::U128(x) => CellIn::U128(*x),
            V::I128(x) => CellIn::I128(*x),
            V::F32(b) => CellIn::F32(f32::from_bits(*b)),
            V::F64(b) => CellIn::F64(f64::from_bits(*b)),
            V::Bytes(b) => CellIn::Bytes(b),
            V::Nat(b) => CellIn::Nat(b),
            V::Int(b) => CellIn::Int(b),
            V::Unit => CellIn::Unit,
            V::Arg(b) => CellIn::Arg { len: b.len() as u64 },
        })
    }

    fn fill(&mut self, leaf: u32, at: u64, dst: &mut [u8]) -> Result<(), Fault> {
        let Some((_, V::Arg(b))) = self.cells.iter().find(|(l, _)| *l == leaf) else { return Err(Fault::BadRef) };
        let s = at as usize;
        dst.copy_from_slice(&b[s..s + dst.len()]);
        self.filled += dst.len() as u64;
        self.fills += 1;
        Ok(())
    }
}

/// Collects a row's cells.
#[derive(Default, Debug)]
pub struct Sink {
    pub cells: Vec<(u32, V)>,
    pub rid: Option<u64>,
}

impl RowSink for Sink {
    fn cell(&mut self, leaf: u32, v: CellOut<'_>) -> Result<(), Fault> {
        let o = match v {
            CellOut::Null => V::Null,
            CellOut::Bool(b) => V::Bool(b),
            CellOut::U64(x) => V::U(x),
            CellOut::I64(x) => V::I(x),
            CellOut::U128(x) => V::U128(x),
            CellOut::I128(x) => V::I128(x),
            CellOut::F32(f) => V::F32(f.to_bits()),
            CellOut::F64(f) => V::F64(f.to_bits()),
            CellOut::Bytes(b) => V::Bytes(b.to_vec()),
            CellOut::Nat(b) => V::Nat(b.to_vec()),
            CellOut::Int(b) => V::Int(b.to_vec()),
            CellOut::Unit => V::Unit,
            CellOut::Arg { len } => V::Arg(vec![0; len as usize]),
        };
        self.cells.push((leaf, o));
        Ok(())
    }

    fn row_end(&mut self, rid: u64) -> Result<(), Fault> {
        self.rid = Some(rid);
        Ok(())
    }
}

/// The rid of the row with these key cells, read with `get`.
pub fn rid_of<H: PageHost>(tb: &Table<'_, H>, key: &[(u32, V)]) -> Option<u64> {
    let mut s = Sink::default();
    let found = tb.get(&mut crate::VecScratch::new(crate::FILLS[0]), &mut Src::new(key), &mut s).unwrap();
    found.then(|| s.rid.unwrap())
}

/// Unsigned LEB128.
pub fn uleb(mut x: u128) -> Vec<u8> {
    let mut v = Vec::new();
    loop {
        let b = (x & 0x7F) as u8;
        x >>= 7;
        if x == 0 {
            v.push(b);
            return v;
        }
        v.push(b | 0x80);
    }
}
