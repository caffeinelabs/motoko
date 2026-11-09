//! The layout of a table: the Blob the compiler builds from the table's key and row types, and the
//! schema image made from it.
//!
//! A layout, numbers little-endian: the magic "NTCD", the u32 version 1, the u64 codec hash, the
//! u32 column count, then per column, the key first and then the fields of the row: the crate's
//! type code of a scalar, after the option code when the column is optional; the u32 label hash;
//! the u16 length of the column's name and the name (the key has none).

use ::native_table::Fault;
use ::native_table::fault::bounds;
use ::native_table::schema::{
    self, Column, ColumnType, IndexDecl, Leaf, LeafSink, SchemaParts, flatten, write_schema,
};
use ::native_table::value::ty;

use crate::memory::{Memory, alloc_blob};
use crate::types::{Bytes, TAG_BLOB_B};

const MAGIC: &[u8] = b"NTCD";
const VERSION: u32 = 1;
/// Bytes before the first column.
const HEADER: usize = 20;
/// The schema version of a new table, which added every column.
pub const SCHEMA_VERSION: u32 = 1;

/// A checked layout.
pub struct Layout<'a> {
    pub hash: u64,
    columns: usize,
    /// The column entries.
    entries: &'a [u8],
}

/// A column of a layout: its type codes, its label hash and its name, empty for none.
pub struct Entry<'a> {
    pub codes: &'a [u8],
    pub hash: u32,
    pub name: &'a [u8],
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// Whether `code` is the type code of a scalar the codec accepts.
fn scalar(code: u8) -> bool {
    matches!(code, ty::BOOL..=ty::INT | ty::F64..=ty::PRINCIPAL)
}

/// The column entry at the start of `b`, and the bytes after it.
fn entry(b: &[u8]) -> Option<(Entry<'_>, &[u8])> {
    let n = if *b.first()? == ty::OPTION { 2 } else { 1 };
    let codes = b.get(..n)?;
    if !scalar(codes[n - 1]) {
        return None;
    }
    let hash = u32_at(b, n)?;
    let name_at = n + 6;
    let name_end = name_at + u16_at(b, n + 4)? as usize;
    let name = b.get(name_at..name_end)?;
    Some((Entry { codes, hash, name }, &b[name_end..]))
}

impl<'a> Layout<'a> {
    /// The layout in `bytes`, or `None` when they do not hold one.
    pub fn parse(bytes: &'a [u8]) -> Option<Layout<'a>> {
        let layout = Self::header(bytes)?;
        let mut rest = layout.entries;
        let mut columns = 0;
        while !rest.is_empty() {
            rest = entry(rest)?.1;
            columns += 1;
        }
        (columns == layout.columns && columns > 0).then_some(layout)
    }

    /// The layout in `bytes` with only its header checked, or `None` when they do not start as a
    /// layout does. Its columns are checked only as `entries` reads them, and the column count is
    /// not: for a primitive that needs the codec hash, or one column found by name, and not every
    /// column.
    pub fn header(bytes: &'a [u8]) -> Option<Layout<'a>> {
        if bytes.get(..4)? != MAGIC || u32_at(bytes, 4)? != VERSION {
            return None;
        }
        Some(Layout {
            hash: u64_at(bytes, 8)?,
            columns: u32_at(bytes, 16)? as usize,
            entries: bytes.get(HEADER..)?,
        })
    }

    /// The number of columns, the key's included.
    pub fn columns(&self) -> usize {
        self.columns
    }

    /// Whether the row is a record. A row that is a scalar is the one column after the key, and
    /// it has no name, while every field of a record has one.
    pub fn record_row(&self) -> bool {
        !(self.columns == 2 && self.entries().nth(1).is_some_and(|e| e.name.is_empty()))
    }

    /// Whether `labels` is the hash list of a record of the row type: the label hashes of the
    /// row's fields, in the layout's order, as u64 words.
    pub fn is_hash_list(&self, labels: &[u8]) -> bool {
        labels.len() == (self.columns - 1) * size_of::<u64>()
            && self
                .entries()
                .skip(1)
                .zip(labels.chunks_exact(size_of::<u64>()))
                .all(|(e, w)| u64_at(w, 0) == Some(u64::from(e.hash)))
    }

    pub fn entries(&self) -> Entries<'a> {
        Entries { rest: self.entries }
    }
}

/// The column entries of a layout, in order.
#[derive(Clone)]
pub struct Entries<'a> {
    rest: &'a [u8],
}

impl<'a> Iterator for Entries<'a> {
    type Item = Entry<'a>;

    fn next(&mut self) -> Option<Entry<'a>> {
        let (e, rest) = entry(self.rest)?;
        self.rest = rest;
        Some(e)
    }
}

/// Leaves written into scratch.
struct Leaves {
    at: *mut Leaf,
    len: usize,
    cap: usize,
}

impl LeafSink for Leaves {
    fn leaf(&mut self, l: &Leaf) -> Result<(), Fault> {
        if self.len == self.cap {
            return Err(bounds(self.len as u64));
        }
        unsafe { self.at.add(self.len).write(*l) };
        self.len += 1;
        Ok(())
    }
}

/// Room for `n` values of `T` in a Blob that lives until the next collection.
pub(super) unsafe fn scratch<T, M: Memory>(mem: &mut M, n: usize) -> *mut T {
    let align = align_of::<T>();
    let blob = alloc_blob(mem, TAG_BLOB_B, Bytes(n * size_of::<T>() + align - 1));
    (blob.as_blob_mut().payload_addr() as usize).next_multiple_of(align) as *mut T
}

/// The schema image of `layout` for the table with uid `uid`, of schema version `version` and with
/// the index declarations `indexes`, in a Blob that lives until the next collection: column 0 is
/// the key, flagged as such, and every field of the row is one more column whose name is in the
/// name pool. Every column was added by the first schema version.
pub unsafe fn schema_image<'b, M: Memory>(
    mem: &mut M,
    layout: &Layout,
    uid: u64,
    version: u32,
    indexes: &[IndexDecl],
) -> Result<&'b [u8], Fault> {
    let n = layout.columns;
    let (mut type_bytes, mut name_bytes) = (0, 0);
    for e in layout.entries() {
        type_bytes += e.codes.len();
        if !e.name.is_empty() {
            name_bytes += 2 + e.name.len();
        }
    }
    let columns = scratch::<Column, M>(mem, n);
    let types = core::slice::from_raw_parts_mut(scratch::<u8, M>(mem, type_bytes), type_bytes);
    let names = core::slice::from_raw_parts_mut(scratch::<u8, M>(mem, name_bytes), name_bytes);
    let mut leaves = Leaves {
        at: scratch::<Leaf, M>(mem, n),
        len: 0,
        cap: n,
    };
    // The pools: a name's u16 length comes just before its bytes, and a column names the bytes.
    let (mut type_at, mut name_at) = (0, 0);
    for (i, e) in layout.entries().enumerate() {
        types[type_at..type_at + e.codes.len()].copy_from_slice(e.codes);
        let name = if e.name.is_empty() {
            0
        } else {
            let at = name_at + 2;
            names[name_at..at].copy_from_slice(&(e.name.len() as u16).to_le_bytes());
            names[at..at + e.name.len()].copy_from_slice(e.name);
            name_at = at + e.name.len();
            at as u32
        };
        columns.add(i).write(Column {
            id: i as u32,
            flags: 0,
            first_leaf: 0,
            ty: type_at as u32,
            default_off: 0,
            default_len: 0,
            name,
        });
        type_at += e.codes.len();
    }
    let columns = core::slice::from_raw_parts_mut(columns, n);
    for column in columns.iter_mut() {
        let first = u16::try_from(leaves.len).map_err(|_| bounds(leaves.len as u64))?;
        let col = ColumnType {
            pool: types,
            column: column.id,
            ty: column.ty,
            version: SCHEMA_VERSION,
            first_ordinal: first,
            key: column.id == 0,
        };
        flatten(col, &mut leaves)?;
        column.first_leaf = first;
    }
    let parts = SchemaParts {
        version,
        columns,
        leaves: core::slice::from_raw_parts(leaves.at, leaves.len),
        indexes,
        type_pool: types,
        default_pool: &[],
        name_pool: names,
    };
    let total = schema::layout(&parts)?.total as usize;
    let image = core::slice::from_raw_parts_mut(scratch::<u8, M>(mem, total), total);
    let (bytes, _) = write_schema(&parts, uid, image)?;
    Ok(&image[..bytes as usize])
}
