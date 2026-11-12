//! The layout of a table: the Blob the compiler builds from the table's key and row types, and the
//! schema image made from it.
//!
//! A layout, numbers little-endian: the magic "NTCD", the u32 version 1, the u64 codec hash, the
//! u32 column count, then per column, the key first and then the fields of the row: the crate's
//! type code of a scalar, or the crate's tuple type of a key of several parts (`ty::TUPLE`, the u16
//! number of parts, then each part's code), or the crate's list type of an array (`ty::LIST`, then
//! the element's codes: a scalar's, an option of one's, or a record's, each field an entry as a
//! column's), after the option code when the column is optional; the u32 label hash; the u16 length
//! of the column's name and the name (the key has none).
//!
//! The layout of a view (`view_layout`) is the layout of the table it reads, then the byte `VIEW`,
//! which no column entry starts with, then 4 bytes per field of the view's row: the u16 position
//! of the field in the table's layout, then its first two type codes, padded with 0; then the u32
//! offset of the `VIEW` byte. So a call made with a view's codec finds the table's layout where a
//! table's codec has it: a call that checks only the layout's header reads the table's, and a call
//! that checks every column entry fails at the `VIEW` byte.

use ::native_table::Fault;
use ::native_table::fault::bounds;
use ::native_table::mem::{rd_u8, rd_u16, rd_u32, rd_u64};
use ::native_table::schema::{
    self, Column, ColumnType, DECL_BYTES, IndexDecl, LEAF_NULLABLE, Leaf, LeafSink, SchemaParts,
    SchemaView, flatten, list_end, node, tuple_end, write_schema,
};
use ::native_table::value::ty;

use crate::barriers::allocation_barrier;
use crate::memory::{Memory, alloc_blob};
use crate::rts_trap_with;
use crate::types::{Bytes, TAG_BLOB_B, Value};

const MAGIC: &[u8] = b"NTCD";
const VERSION: u32 = 1;
/// The byte after the table's layout in a view's layout.
const VIEW: u8 = 0xFF;
/// The bytes of a field of a view.
const PICK: usize = 4;
/// Bytes before the first column.
const HEADER: usize = 20;
/// The schema version of a new table, which added every column.
pub const SCHEMA_VERSION: u32 = 1;

/// A checked layout.
#[derive(Clone, Copy)]
pub struct Layout<'a> {
    pub hash: u64,
    columns: usize,
    /// The column entries.
    entries: &'a [u8],
}

/// A column of a layout: its type codes, its label hash and its name, empty for none.
#[derive(Clone, Copy)]
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

/// Whether each type code is that of a scalar the codec accepts: `ty::BOOL` to `ty::INT` and
/// `ty::F64` to `ty::PRINCIPAL`.
static SCALARS: [bool; 256] = {
    let mut s = [false; 256];
    let mut code = ty::BOOL;
    while code <= ty::PRINCIPAL {
        s[code as usize] = code != ty::F32;
        code += 1;
    }
    s
};

/// Whether `code` is the type code of a scalar the codec accepts.
fn scalar(code: u8) -> bool {
    SCALARS[code as usize]
}

/// The number of type codes at the start of `b` when they are an array's (the crate's list type,
/// after the option code when the array is an option) or a key's tuple's, else `usize::MAX`.
#[cold]
#[inline(never)]
fn array_codes(b: &[u8]) -> usize {
    let at = u32::from(b.first() == Some(&ty::OPTION));
    match list_end(b, at) {
        Ok(end) => end as usize,
        Err(_) if b.first() == Some(&ty::TUPLE) => {
            tuple_end(b, 0).map_or(usize::MAX, |end| end as usize)
        }
        Err(_) => usize::MAX,
    }
}

/// The type code of each part of a key of the type codes `codes` when it is a tuple, else `None`.
pub fn key_parts(codes: &[u8]) -> Option<&[u8]> {
    (codes.first() == Some(&ty::TUPLE)).then(|| codes.get(3..).unwrap_or(&[]))
}

/// The most parts of a key that is a tuple.
pub const MAX_KEY_PARTS: usize = 7;

/// Whether the type codes `codes` are an array's, an option of one when the first is the option
/// code, and the codes of its element; `None` when they are not an array's.
pub fn array_type(codes: &[u8]) -> Option<(bool, &[u8])> {
    let optional = codes.first() == Some(&ty::OPTION);
    let at = usize::from(optional);
    let end = list_end(codes, at as u32).ok()? as usize;
    (end == codes.len()).then(|| (optional, &codes[at + 1..]))
}

/// The parts of an array's element of the type codes `element`, each with its type codes: the
/// fields of a record, as column entries, else the element itself; with whether it is a record.
pub fn element_parts(element: &[u8]) -> (bool, impl Iterator<Item = Entry<'_>> + Clone) {
    let record = element.first() == Some(&ty::RECORD);
    let one = (!record).then_some(Entry {
        codes: element,
        hash: 0,
        name: &[],
    });
    let rest = if record {
        element.get(3..).unwrap_or(&[])
    } else {
        &[]
    };
    (record, one.into_iter().chain(Entries { rest }))
}

/// The column entry at the start of `b`, and the bytes after it.
fn entry(b: &[u8]) -> Option<(Entry<'_>, &[u8])> {
    let n = if *b.first()? == ty::OPTION { 2 } else { 1 };
    let codes = b.get(..n)?;
    if scalar(codes[n - 1]) {
        named(b, codes)
    } else {
        named(b, b.get(..array_codes(b))?)
    }
}

/// The column entry at the start of `b` whose type codes are `codes`, and the bytes after it.
#[inline(always)]
fn named<'a>(b: &'a [u8], codes: &'a [u8]) -> Option<(Entry<'a>, &'a [u8])> {
    let n = codes.len();
    let hash = u32_at(b, n)?;
    let name_at = n + 6;
    let name_end = name_at + u16_at(b, n + 4)? as usize;
    let name = b.get(name_at..name_end)?;
    Some((Entry { codes, hash, name }, &b[name_end..]))
}

impl<'a> Layout<'a> {
    /// The layout in `bytes`, or `None` when they do not hold one.
    #[inline(always)]
    pub fn parse(bytes: &'a [u8]) -> Option<Layout<'a>> {
        Self::header(bytes)?.checked()
    }

    /// The layout when every column entry is well formed and they are as many as it says.
    #[inline(always)]
    fn checked(self) -> Option<Layout<'a>> {
        let mut rest = self.entries;
        let mut columns = 0;
        while !rest.is_empty() {
            rest = entry(rest)?.1;
            columns += 1;
        }
        (columns == self.columns && columns > 0).then_some(self)
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

    /// The layout of the table a view's layout `bytes` reads, its header alone checked (as
    /// `header` checks one), and the view's fields (`Picks`); `None` when `bytes` is not a view's
    /// layout.
    pub fn view(bytes: &'a [u8]) -> Option<(Layout<'a>, Picks<'a>)> {
        let table = Self::view_table(bytes)?;
        let picks = &bytes[table.len() + 1..bytes.len() - 4];
        let layout = Self::header(table)?;
        (picks.len() % PICK == 0).then_some((layout, Picks { picks }))
    }

    /// The bytes of the table's layout in a view's layout `bytes`, or `None` when `bytes` is not a
    /// view's layout.
    fn view_table(bytes: &'a [u8]) -> Option<&'a [u8]> {
        let start = u32_at(bytes, bytes.len().checked_sub(4)?)? as usize;
        (*bytes.get(..bytes.len() - 4)?.get(start)? == VIEW).then(|| &bytes[..start])
    }

    /// The layout of the table the codec layout `bytes` is of, a table's or a view's, every column
    /// checked as `parse` checks them, and its bytes; `None` when `bytes` is neither.
    fn table(bytes: &'a [u8]) -> Option<(Layout<'a>, &'a [u8])> {
        let table = match Self::parse(bytes) {
            Some(layout) => return Some((layout, bytes)),
            None => Self::view_table(bytes)?,
        };
        Some((Self::parse(table)?, table))
    }

    /// The number of columns, the key's included.
    pub fn columns(&self) -> usize {
        self.columns
    }

    /// Whether the row is a record. A row that is a scalar is the one column after the key, and
    /// it has no name, while every field of a record has one.
    #[inline(always)]
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

    /// The layout of a table of `m` columns whose column c holds the field at position
    /// `position(c)` of this layout, in column order: column c has its field's entry, and a column
    /// whose position is past the last field's, a dropped one, an entry no field name finds. In a
    /// Blob that lives until the next collection.
    pub unsafe fn by_column<M: Memory>(
        &self,
        mem: &mut M,
        m: usize,
        position: impl Fn(usize) -> usize,
    ) -> Layout<'a> {
        const NONE: &[u8] = &[ty::BOOL, 0, 0, 0, 0, 0, 0];
        let fields = scratch::<&[u8], M>(mem, self.columns);
        let mut rest = self.entries;
        for p in 0..self.columns {
            let (_, after) =
                entry(rest).unwrap_or_else(|| rts_trap_with("native table: malformed layout"));
            fields.add(p).write(&rest[..rest.len() - after.len()]);
            rest = after;
        }
        let out = scratch::<u8, M>(mem, self.entries.len() + m * NONE.len());
        let mut end = 0;
        for c in 0..m {
            let p = position(c);
            let e = if p < self.columns {
                *fields.add(p)
            } else {
                NONE
            };
            core::ptr::copy_nonoverlapping(e.as_ptr(), out.add(end), e.len());
            end += e.len();
        }
        Layout {
            hash: self.hash,
            columns: m,
            entries: core::slice::from_raw_parts(out, end),
        }
    }
}

/// The fields of a view (`Layout::view`), in the order of the view's row type.
#[derive(Clone, Copy)]
pub struct Picks<'a> {
    picks: &'a [u8],
}

impl<'a> Picks<'a> {
    pub fn len(&self) -> usize {
        self.picks.len() / PICK
    }

    /// The position in the table's layout and the type codes of field `i` of the view; none for an
    /// `i` past the last field, `usize::MAX` included.
    pub fn get(&self, i: usize) -> Option<(usize, &'a [u8])> {
        let p = self
            .picks
            .get(PICK.wrapping_mul(i)..PICK.wrapping_mul(i.wrapping_add(1)))?;
        let n = if p[2] == ty::OPTION { 2 } else { 1 };
        Some((usize::from(u16::from_le_bytes([p[0], p[1]])), &p[2..2 + n]))
    }
}

/// Each field of the row of `named` found among the fields of the row of `layout`, a table's, by
/// label hash and name, both in label-hash order: in `named`'s order, the field's position in
/// `layout`, its entry there and its entry in `named`. Traps when the table's row has no field of a
/// name, or has it with other type codes; with `widen`, a field of `named` may also be a Nat where
/// the table's is an Int, or an option of a Nat where the table's is an option of an Int, since a
/// Nat value is an Int value.
pub fn merge<'l, 'n>(
    layout: &Layout<'l>,
    named: &Layout<'n>,
    widen: bool,
) -> impl Iterator<Item = (usize, Entry<'l>, Entry<'n>)> {
    let mut fields = layout.entries().enumerate().skip(1);
    named.entries().skip(1).map(move |f| {
        let (p, e) = loop {
            match fields.next() {
                Some((_, e)) if e.hash < f.hash => {}
                Some((p, e)) if e.hash == f.hash && e.name == f.name => break (p, e),
                _ => rts_trap_with("native table: the row type has no field of that name"),
            }
        };
        let wide = widen
            && matches!(
                (e.codes, f.codes),
                ([ty::INT], [ty::NAT]) | ([ty::OPTION, ty::INT], [ty::OPTION, ty::NAT])
            );
        if e.codes != f.codes && !wide {
            rts_trap_with("native table: a field has another type than the table's");
        }
        (p, e, f)
    })
}

/// The layout of a view of the table whose codec layout is `codec`, a table's or a view's, with
/// the row type of `view`, of the same key type, in a new Blob: each field of the view's row is
/// found by `merge`, of the same type codes.
pub unsafe fn view_layout<M: Memory>(mem: &mut M, codec: &[u8], view: &Layout) -> Value {
    let Some((layout, table)) = Layout::table(codec) else {
        rts_trap_with("native table: malformed layout")
    };
    let start = table.len();
    let size = start + 1 + PICK * (view.columns - 1) + 4;
    let blob = alloc_blob(mem, TAG_BLOB_B, Bytes(size));
    let out = core::slice::from_raw_parts_mut(blob.as_blob_mut().payload_addr(), size);
    out[..start].copy_from_slice(table);
    out[start..].fill(0);
    out[start] = VIEW;
    for (i, (p, e, _)) in merge(&layout, view, false).enumerate() {
        let p = u16::try_from(p).unwrap_or_else(|_| rts_trap_with(bounds(p as u64).text()));
        let at = start + 1 + PICK * i;
        out[at..at + 2].copy_from_slice(&p.to_le_bytes());
        let codes = &e.codes[..e.codes.len().min(2)];
        out[at + 2..at + 2 + codes.len()].copy_from_slice(codes);
    }
    out[size - 4..].copy_from_slice(&(start as u32).to_le_bytes());
    allocation_barrier(blob)
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
/// the key, flagged as such, a leaf per part when the key is a tuple, and every field of the row is
/// one more column whose name is in the name pool. Every column was added by the first schema
/// version.
pub unsafe fn schema_image<'b, M: Memory>(
    mem: &mut M,
    layout: &Layout,
    uid: u64,
    version: u32,
    indexes: &[IndexDecl],
) -> Result<&'b [u8], Fault> {
    let n = layout.columns;
    let (mut type_bytes, mut name_bytes, mut leaf_count) = (0, 0, 0);
    for e in layout.entries() {
        type_bytes += e.codes.len();
        leaf_count += key_parts(e.codes).map_or(1, <[u8]>::len);
        if !e.name.is_empty() {
            name_bytes += 2 + e.name.len();
        }
    }
    let columns = scratch::<Column, M>(mem, n);
    let types = core::slice::from_raw_parts_mut(scratch::<u8, M>(mem, type_bytes), type_bytes);
    let names = core::slice::from_raw_parts_mut(scratch::<u8, M>(mem, name_bytes), name_bytes);
    let mut leaves = Leaves {
        at: scratch::<Leaf, M>(mem, leaf_count),
        len: 0,
        cap: leaf_count,
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

/// The column flag of a dropped column.
const COLUMN_DROPPED: u8 = 1;

/// The bytes of the name the name-pool offset `at` names in `names`: its u16 length comes just
/// before them, and an offset below 2 names none.
fn name_at(names: &[u8], at: u32) -> &[u8] {
    let at = at as usize;
    match at.checked_sub(2).and_then(|len_at| u16_at(names, len_at)) {
        Some(len) => names.get(at..at + len as usize).unwrap_or(&[]),
        None => &[],
    }
}

/// The type codes at `at` in the type pool `pool`: a scalar's or an array's, after the option code
/// when the type is an option.
fn codes_at(pool: &[u8], at: u32) -> &[u8] {
    let b = pool.get(at as usize..).unwrap_or(&[]);
    let n = match array_codes(b) {
        usize::MAX if b.first() == Some(&ty::OPTION) => 2,
        usize::MAX => 1,
        n => n,
    };
    b.get(..n).unwrap_or(&[])
}

/// The schema image of the next schema version of the table whose schema image is `stored`, for
/// the row type of `next`, a record, in a Blob that lives until the next collection; with the
/// column of each field position of `next` (the key's, 0, first) and the number of columns, the
/// table's map. Each field is found by name among the stored columns not dropped: one of the same
/// type codes keeps its column, one of other codes traps. A field no stored column names is a new
/// column after the last one, of a new leaf after the last leaf, added by the new version and
/// nullable; it traps when its type is not an option. A stored column no field names is dropped:
/// its dropped flag set, and its leaf's kind `node::DROPPED` with the nullable flag. Every other
/// column, leaf, type and name stays as stored, at its place in the pools; the new columns' types
/// and names come after. The index declarations are the stored ones less those with a part on a
/// dropped leaf; the default pool is empty; the version is the stored one + 1; the uid is kept.
pub unsafe fn evolved_image<'b, M: Memory>(
    mem: &mut M,
    stored: &SchemaView,
    next: &Layout,
) -> Result<(&'b [u8], &'b [u16], usize), Fault> {
    let (old_columns, old_leaves) = (stored.columns as usize, stored.leaves as usize);
    let version = stored
        .version
        .checked_add(1)
        .ok_or(bounds(u64::from(stored.version)))?;
    let pool = stored.types();
    let names = stored
        .bytes
        .get(stored.layout.names as usize..stored.layout.total as usize)
        .unwrap_or(&[]);
    // The column of each field position: a stored one found by name, or a new one.
    let n = next.columns;
    let at = scratch::<u16, M>(mem, n);
    let (mut added, mut type_bytes, mut name_bytes) = (0, 0, 0);
    for (p, e) in next.entries().enumerate() {
        let column = if p == 0 {
            0
        } else {
            let mut found = None;
            for i in 0..stored.columns {
                let c = stored.column(i)?;
                if c.flags & COLUMN_DROPPED == 0 && name_at(names, c.name) == e.name {
                    found = Some(c);
                    break;
                }
            }
            match found {
                Some(c) if codes_at(pool, c.ty) == e.codes => c.id as usize,
                Some(_) => rts_trap_with("native table: a field changes its type"),
                None if e.codes[0] == ty::OPTION => {
                    added += 1;
                    type_bytes += e.codes.len();
                    name_bytes += 2 + e.name.len();
                    old_columns + added - 1
                }
                None => rts_trap_with("native table: a new field is not an option"),
            }
        };
        if column >= u16::MAX as usize {
            return Err(bounds(column as u64));
        }
        at.add(p).write(column as u16);
    }
    let at = core::slice::from_raw_parts(at, n);
    let m = old_columns + added;
    // Whether each stored column is named by a field; a stored column no field names is dropped.
    let named = scratch::<bool, M>(mem, old_columns);
    for i in 0..old_columns {
        named.add(i).write(false);
    }
    for &c in at {
        if (c as usize) < old_columns {
            named.add(c as usize).write(true);
        }
    }
    let dropped = |column: usize| column < old_columns && !*named.add(column);
    // The pools: the stored ones, then the new columns' types and names.
    let (old_types, old_names) = (pool.len(), names.len());
    let types = scratch::<u8, M>(mem, old_types + type_bytes);
    let types = core::slice::from_raw_parts_mut(types, old_types + type_bytes);
    types[..old_types].copy_from_slice(pool);
    let new_names = scratch::<u8, M>(mem, old_names + name_bytes);
    let new_names = core::slice::from_raw_parts_mut(new_names, old_names + name_bytes);
    new_names[..old_names].copy_from_slice(names);
    // The columns and leaves: the stored ones, the dropped ones marked, then the new ones.
    let columns = scratch::<Column, M>(mem, m);
    for i in 0..old_columns {
        let mut c = stored.column(i as u16)?;
        if dropped(c.id as usize) {
            c.flags |= COLUMN_DROPPED;
        }
        columns.add(i).write(c);
    }
    let mut leaves = Leaves {
        at: scratch::<Leaf, M>(mem, old_leaves + added),
        len: old_leaves,
        cap: old_leaves + added,
    };
    for o in 0..old_leaves {
        let mut l = stored.leaf(o as u16)?;
        if dropped((l.id >> 16) as usize) {
            l.kind = node::DROPPED;
            l.flags |= LEAF_NULLABLE;
        }
        leaves.at.add(o).write(l);
    }
    let (mut type_end, mut name_end) = (old_types, old_names);
    for (e, &c) in next.entries().zip(at) {
        let c = c as usize;
        if c < old_columns {
            continue;
        }
        types[type_end..type_end + e.codes.len()].copy_from_slice(e.codes);
        let name = name_end + 2;
        new_names[name_end..name].copy_from_slice(&(e.name.len() as u16).to_le_bytes());
        new_names[name..name + e.name.len()].copy_from_slice(e.name);
        name_end = name + e.name.len();
        let first = u16::try_from(leaves.len).map_err(|_| bounds(leaves.len as u64))?;
        let col = ColumnType {
            pool: types,
            column: c as u32,
            ty: type_end as u32,
            version,
            first_ordinal: first,
            key: false,
        };
        flatten(col, &mut leaves)?;
        columns.add(c).write(Column {
            id: c as u32,
            flags: 0,
            first_leaf: first,
            ty: type_end as u32,
            default_off: 0,
            default_len: 0,
            name: name as u32,
        });
        type_end += e.codes.len();
    }
    // The index declarations, less those with a part on a dropped leaf.
    let count = stored.indexes as usize;
    let decls = scratch::<IndexDecl, M>(mem, count);
    let mut kept = 0;
    for i in 0..count {
        let b = stored.bytes;
        let o = u64::from(stored.layout.indexes) + i as u64 * DECL_BYTES;
        let parts = rd_u8(b, o + 4)?;
        let nullable = rd_u8(b, o + 5)?;
        let mut decl = IndexDecl {
            id: rd_u8(b, o)?,
            flags: rd_u8(b, o + 1)?,
            w: rd_u16(b, o + 2)?,
            parts,
            part: [(0, false); 7],
        };
        let mut gone = false;
        for (k, part) in decl.part.iter_mut().take(parts as usize).enumerate() {
            let leaf = rd_u32(b, o + 8 + 4 * k as u64)?;
            *part = (leaf, nullable >> k & 1 != 0);
            gone |= dropped((leaf >> 16) as usize);
        }
        if !gone {
            decls.add(kept).write(decl);
            kept += 1;
        }
    }
    let parts = SchemaParts {
        version,
        columns: core::slice::from_raw_parts(columns, m),
        leaves: core::slice::from_raw_parts(leaves.at, leaves.len),
        indexes: core::slice::from_raw_parts(decls, kept),
        type_pool: types,
        default_pool: &[],
        name_pool: new_names,
    };
    let total = schema::layout(&parts)?.total as usize;
    let image = core::slice::from_raw_parts_mut(scratch::<u8, M>(mem, total), total);
    let (bytes, _) = write_schema(&parts, rd_u64(stored.bytes, 16)?, image)?;
    Ok((&image[..bytes as usize], at, m))
}
