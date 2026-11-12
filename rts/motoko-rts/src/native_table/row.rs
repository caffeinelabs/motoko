//! Rows as Motoko values: the key and the row of a write, read in place, and the row or key of a
//! read, built as values of the key and row types.
//!
//! The row is a scalar or a record. A record's fields are found by label hash in the record's
//! own hash list, which may name more fields than the row type, since a record type is a
//! subtype of a record type with fewer fields. Every value's tag is checked against its
//! column's type before anything else is read from it, so a value of another type traps
//! instead of being read as one of the column's type.
//!
//! Values have the forms compiled code gives them. Nat and Int are compact numbers or BigInts;
//! Nat64 and Int64 are tagged numbers when they fit, else boxed 64-bit words; Nat8 to Nat32,
//! Int8 to Int32 and Char keep their payload in the top bits of the word and their type's tag
//! in the bits below it; Float is boxed; Bool is 0 or 1; Text, Blob and Principal are blobs,
//! and a Text may be a concatenation. An option of a scalar is null or the scalar itself.
//!
//! A field may also be an immutable array, or an option of one, of scalars, options of scalars or
//! records of them. The table holds an array as one byte string, its elements in the crate's value
//! encoding (`array_cell`), and a read builds the array again (`array_of_ve`).
//!
//! The key may also be a tuple of scalars. The table holds each part as a leaf of the key's column,
//! part j as leaf j, and every path for such a key starts on a failure edge of a scalar key's path:
//! a write's cells (`tuple_cells`), a read's tuple (`RowOut::tuple`, `RowOut::part`).

use core::ops::Bound;

use ::native_table::bytes::rd_uleb;
use ::native_table::key::{
    SliceSink, dec_bool, dec_bytes, dec_char, dec_principal, dec_u64, int_header,
};
use ::native_table::table::cell::{leb_value, put_sleb128, put_uleb128};
use ::native_table::table::filter::Cond;
use ::native_table::table::{CellIn, CellOut, RowSink, RowSource, Table};
use ::native_table::value::ty;
use ::native_table::{Fault, ImageId, PageHost};

use super::layout::{
    Entries, Entry, Layout, MAX_KEY_PARTS, Picks, array_type, element_parts, key_parts, merge,
    placed, scratch,
};
use crate::barriers::allocation_barrier;
use crate::bigint::{
    bigint_leb128_decode, bigint_leb128_encode, bigint_leb128_size, bigint_of_word64,
    bigint_sleb128_decode, bigint_sleb128_encode, bigint_sleb128_size, int_from_isize, mp_isneg,
};
use crate::buf::Buf;
use crate::constants::WORD_SIZE;
use crate::mem_utils::memcpy_bytes;
use crate::memory::{Memory, alloc_array, alloc_blob};
use crate::rts_trap_with;
use crate::text::blob_of_text;
use crate::tommath_bindings::{mp_count_bits, mp_get_i64};
use crate::types::{
    self, Bits64, Bytes, NULL_POINTER, Object, TAG_ARRAY_I, TAG_ARRAY_T, TAG_BIGINT, TAG_BITS64_F,
    TAG_BITS64_S, TAG_BITS64_U, TAG_BLOB_B, TAG_BLOB_P, TAG_BLOB_T, TAG_CONCAT, TAG_OBJECT,
    TAG_VARIANT, TRUE_VALUE, Tag, Value, Variant, Words,
};
use crate::utf8::utf8_valid;

/// The low bits of a compact Nat or Int.
const COMPACT_TAG: usize = 0b10;
const COMPACT_MASK: usize = 0b11;
const COMPACT_SHIFT: u32 = 2;

/// The low bits of a Nat64 or Int64 in tagged form, and the payload's shift: 60 bits of payload.
const NAT64_TAG: usize = 0b0100;
const INT64_TAG: usize = 0b1100;
const WORD64_MASK: usize = 0b1111;
const WORD64_SHIFT: u32 = 4;

/// The cells of one row, one per column, the key first: every column is one leaf, the first of
/// its column. A key that is a tuple has its first part's cell in column 0, and the cells of its
/// other parts after the last column's, part j the j-th from the end (`tuple_cells`).
#[derive(Clone, Copy)]
pub struct Cells<'a> {
    cells: &'a [CellIn<'a>],
}

impl<'a> Cells<'a> {
    pub fn new(cells: &'a [CellIn<'a>]) -> Self {
        Cells { cells }
    }

    /// These cells, with the cell of field leaf `leaf` the bytes `b`, which a `BlobScratch` gave
    /// and so live until the next collection, in a Blob that lives as long.
    pub unsafe fn with<M: Memory>(self, mem: &mut M, leaf: u32, b: &[u8]) -> Cells<'a> {
        let n = self.cells.len();
        let out = scratch::<CellIn, M>(mem, n);
        for (i, &c) in self.cells.iter().enumerate() {
            out.add(i).write(c);
        }
        let b = core::slice::from_raw_parts(b.as_ptr(), b.len());
        out.add((leaf >> 16) as usize).write(CellIn::Bytes(b));
        Cells::new(core::slice::from_raw_parts(out, n))
    }
}

/// The field position the map gives a dropped column.
const MAP_DROPPED: u16 = 0xFFFF;

/// The record position of a dropped column's field, past every field's.
pub const DROPPED: usize = usize::MAX;

/// The columns of a table whose row type has changed, read from its map page: the field count n
/// (u32), the column count m (u32), the column of each field position (u16 × n), then the field
/// position of each column (u16 × m, 0xFFFF for a dropped column). Position 0 is the key, in
/// column 0. A table whose row type never changed has no map: field position i is column i.
#[derive(Clone, Copy)]
pub struct Map {
    page: *const u8,
    n: usize,
    m: usize,
}

impl Map {
    /// The map at the start of `page`; `None` when it is not a map whose entries lie in range.
    pub fn of(page: &[u8]) -> Option<Map> {
        let word = |at: usize| Some(u32::from_le_bytes(page.get(at..at + 4)?.try_into().ok()?));
        let half = |at: usize| u16::from_le_bytes([page[at], page[at + 1]]);
        let (n, m) = (word(0)? as usize, word(4)? as usize);
        if 8 + 2 * (n + m) > page.len() {
            return None;
        }
        let columns_ok = (0..n).all(|p| usize::from(half(8 + 2 * p)) < m);
        let positions_ok = (0..m).all(|c| {
            let p = half(8 + 2 * (n + c));
            p == MAP_DROPPED || usize::from(p) < n
        });
        (columns_ok && positions_ok).then_some(Map {
            page: page.as_ptr(),
            n,
            m,
        })
    }

    /// The map bytes of `columns`, the column of each field position, for a table of `m`
    /// columns, written into `out`, which holds 8 + 2 · (columns.len() + m) bytes.
    pub fn write(columns: &[u16], m: usize, out: &mut [u8]) {
        let n = columns.len();
        out[..4].copy_from_slice(&(n as u32).to_le_bytes());
        out[4..8].copy_from_slice(&(m as u32).to_le_bytes());
        out[8..].fill(0xFF);
        for (p, &c) in columns.iter().enumerate() {
            out[8 + 2 * p..10 + 2 * p].copy_from_slice(&c.to_le_bytes());
            let at = 8 + 2 * (n + usize::from(c));
            out[at..at + 2].copy_from_slice(&(p as u16).to_le_bytes());
        }
    }

    /// The number of columns, dropped ones included.
    pub fn columns(self) -> usize {
        self.m
    }

    /// The column of the field at position `at`; traps when the map has no field there.
    pub fn column(self, at: usize) -> usize {
        if at >= self.n {
            rts_trap_with(Fault::BadRef.text());
        }
        unsafe { usize::from((self.page.add(8 + 2 * at) as *const u16).read_unaligned()) }
    }

    /// The leaf of the column of the field whose position `leaf` names as a leaf: position << 16.
    /// Out of line, so a caller keeps only the leaf of a field it finds by position.
    #[inline(never)]
    pub fn leaf(self, leaf: u32) -> u32 {
        (self.column((leaf >> 16) as usize) as u32) << 16
    }

    /// The field position of `column`, `DROPPED` for a dropped column or one past the last.
    pub fn position(self, column: usize) -> usize {
        if column >= self.m {
            return DROPPED;
        }
        let at =
            unsafe { (self.page.add(8 + 2 * (self.n + column)) as *const u16).read_unaligned() };
        if at == MAP_DROPPED {
            DROPPED
        } else {
            usize::from(at)
        }
    }

    /// `layout`, the layout of the table's row type, in column order (`Layout::by_column`).
    pub unsafe fn layout<'l, M: Memory>(self, mem: &mut M, layout: &Layout<'l>) -> Layout<'l> {
        layout.by_column(mem, self.m, |c| self.position(c))
    }

    /// The cells of a row by column, from `cells`, the row's cells by field position: null in
    /// dropped columns. In a Blob that lives until the next collection.
    pub unsafe fn cells<'a, M: Memory>(self, mem: &mut M, cells: Cells<'a>) -> Cells<'a> {
        // The cells of a key's parts after the first follow the columns.
        let parts = cells.cells.len() - self.n;
        let out = scratch::<CellIn, M>(mem, self.m + parts);
        for c in 0..self.m {
            out.add(c).write(CellIn::Null);
        }
        for (at, &cell) in cells.cells.iter().enumerate() {
            let c = if at < self.n {
                self.column(at)
            } else {
                self.m + at - self.n
            };
            out.add(c).write(cell);
        }
        Cells {
            cells: core::slice::from_raw_parts(out, self.m + parts),
        }
    }
}

impl RowSource for Cells<'_> {
    fn cell(&mut self, leaf: u32) -> Result<CellIn<'_>, Fault> {
        // Leaf c << 16 | j, rotated: column c when j is 0, else past every column (a table has
        // fewer than 65,536); leaf j > 0 of a key that is a tuple is its part j, the j-th cell from
        // the end.
        match self.cells.get(leaf.rotate_left(16) as usize) {
            Some(c) => Ok(*c),
            None => match self.cells.get(self.cells.len().wrapping_sub(leaf as usize)) {
                Some(c) => Ok(*c),
                None => Err(Fault::BadValue { leaf }),
            },
        }
    }

    fn fill(&mut self, _leaf: u32, _at: u64, _dst: &mut [u8]) -> Result<(), Fault> {
        // No cell is given as `CellIn::Arg`.
        Err(Fault::BadRef)
    }
}

fn mismatch() -> ! {
    rts_trap_with("native table: value does not match the table's key or row type")
}

/// The heap tag of `v`, or `None` when `v` is not a heap object.
unsafe fn tag_of(v: Value) -> Option<Tag> {
    v.is_non_null_ptr().then(|| v.forward_if_possible().tag())
}

/// The scalar type code of a column and whether the column is optional. Traps for an array's
/// column (`no_scalar`).
pub(super) fn column_type(codes: &[u8]) -> (u8, bool) {
    match codes {
        [ty::OPTION, code] => (*code, true),
        [code] => (*code, false),
        _ => no_scalar(codes),
    }
}

/// Traps for a column whose type codes are not a scalar's: an array's has no index key, filter
/// or total, and a checked layout has no other column types.
#[cold]
#[inline(never)]
fn no_scalar(codes: &[u8]) -> ! {
    if array_type(codes).is_some() {
        rts_trap_with("native table: a field of an array type has no index, filter or total");
    }
    if key_parts(codes).is_some() {
        rts_trap_with("native table: a key that is a tuple is not one cell");
    }
    rts_trap_with("native table: malformed layout")
}

/// The form of a Nat8 to Nat32, Int8 to Int32 or Char: the shift of its payload, the tag in the
/// bits below the payload, and whether the payload is signed.
fn small_form(code: u8) -> (u32, usize, bool) {
    match code {
        ty::U8 => (56, 0b01 << 54, false),
        ty::I8 => (56, 0b11 << 54, true),
        ty::U16 => (48, 0b01 << 46, false),
        ty::I16 => (48, 0b11 << 46, true),
        ty::U32 => (32, 0b01 << 30, false),
        ty::I32 => (32, 0b11 << 30, true),
        _ => (43, 0b01 << 41, false),
    }
}

/// The payload of a small scalar of type `code`, sign-extended when the type is signed.
fn small(code: u8, v: Value) -> u64 {
    let (shift, tag, signed) = small_form(code);
    let raw = v.get_raw();
    if raw & ((1 << shift) - 1) != tag {
        mismatch();
    }
    if signed {
        (raw as i64 >> shift) as u64
    } else {
        (raw >> shift) as u64
    }
}

/// The 64 bits of a boxed Nat64, Int64 or Float with heap tag `tag`.
unsafe fn bits64(v: Value, tag: Tag) -> u64 {
    if tag_of(v) != Some(tag) {
        mismatch();
    }
    (*(v.as_obj() as *const Bits64)).bits
}

/// A Nat64 or Int64: tagged with `word_tag` when it fits, else boxed with heap tag `tag`.
unsafe fn word64(v: Value, word_tag: usize, tag: Tag) -> u64 {
    let raw = v.get_raw();
    if raw & WORD64_MASK == word_tag {
        if word_tag == INT64_TAG {
            (raw as i64 >> WORD64_SHIFT) as u64
        } else {
            (raw >> WORD64_SHIFT) as u64
        }
    } else {
        bits64(v, tag)
    }
}

/// A Nat or Int that is compact, or `None`.
pub(super) fn compact(v: Value) -> Option<i64> {
    let raw = v.get_raw();
    (raw & COMPACT_MASK == COMPACT_TAG).then(|| raw as i64 >> COMPACT_SHIFT)
}

/// A Nat: a compact one as a number, a BigInt as its LEB128 bytes.
unsafe fn nat<'a, M: Memory>(mem: &mut M, v: Value) -> CellIn<'a> {
    if let Some(n) = compact(v) {
        if n < 0 {
            mismatch();
        }
        return CellIn::U64(n as u64);
    }
    if tag_of(v) != Some(TAG_BIGINT) || mp_isneg(v.as_bigint().mp_int_ptr()) {
        mismatch();
    }
    let n = bigint_leb128_size(v);
    let bytes = scratch::<u8, M>(mem, n);
    bigint_leb128_encode(v, bytes);
    CellIn::Nat(core::slice::from_raw_parts(bytes, n))
}

/// An Int: a compact one as a number, a BigInt as its signed LEB128 bytes.
unsafe fn int<'a, M: Memory>(mem: &mut M, v: Value) -> CellIn<'a> {
    if let Some(n) = compact(v) {
        return CellIn::I64(n);
    }
    if tag_of(v) != Some(TAG_BIGINT) {
        mismatch();
    }
    let n = bigint_sleb128_size(v);
    let bytes = scratch::<u8, M>(mem, n);
    bigint_sleb128_encode(v, bytes);
    CellIn::Int(core::slice::from_raw_parts(bytes, n))
}

/// The bytes of a blob with heap tag `tag`, in place.
unsafe fn bytes<'a>(v: Value, tag: Tag) -> CellIn<'a> {
    if tag_of(v) != Some(tag) {
        mismatch();
    }
    let blob = v.as_blob();
    CellIn::Bytes(core::slice::from_raw_parts(
        blob.payload_const(),
        blob.len().as_usize(),
    ))
}

/// A Text as its bytes, in place; a concatenation is flattened first.
unsafe fn text<'a, M: Memory>(mem: &mut M, v: Value) -> CellIn<'a> {
    if !matches!(tag_of(v), Some(TAG_BLOB_T | TAG_CONCAT)) {
        mismatch();
    }
    bytes(blob_of_text(mem, v), TAG_BLOB_T)
}

/// The cell of value `v` of scalar type `code`.
pub(super) unsafe fn scalar<'a, M: Memory>(mem: &mut M, code: u8, v: Value) -> CellIn<'a> {
    match code {
        ty::BOOL => match v.get_raw() {
            0 => CellIn::Bool(false),
            TRUE_VALUE => CellIn::Bool(true),
            _ => mismatch(),
        },
        ty::U8 | ty::U16 | ty::U32 | ty::CHAR => CellIn::U64(small(code, v)),
        ty::I8 | ty::I16 | ty::I32 => CellIn::I64(small(code, v) as i64),
        ty::U64 => CellIn::U64(word64(v, NAT64_TAG, TAG_BITS64_U)),
        ty::I64 => CellIn::I64(word64(v, INT64_TAG, TAG_BITS64_S) as i64),
        ty::NAT => nat(mem, v),
        ty::INT => int(mem, v),
        ty::F64 => CellIn::F64(f64::from_bits(bits64(v, TAG_BITS64_F))),
        ty::TEXT => text(mem, v),
        // `encodeUtf8` hands back the Text's own blob, which keeps the Text tag.
        ty::BLOB if tag_of(v) == Some(TAG_BLOB_T) => bytes(v, TAG_BLOB_T),
        ty::BLOB => bytes(v, TAG_BLOB_B),
        ty::PRINCIPAL => bytes(v, TAG_BLOB_P),
        // A checked layout has no other scalar types.
        _ => rts_trap_with("native table: malformed layout"),
    }
}

/// The cell of value `v` in a column of type `codes`.
unsafe fn cell<'a, M: Memory>(mem: &mut M, codes: &[u8], v: Value) -> CellIn<'a> {
    let (code, optional) = match codes {
        [ty::OPTION, code] => (*code, true),
        [code] => (*code, false),
        _ => return array_cell(mem, codes, v),
    };
    if optional && v == NULL_POINTER {
        CellIn::Null
    } else {
        scalar(mem, code, v)
    }
}

/// The cell of value `v` in a column of an array type `codes`: null for an option's null, else the
/// bytes of the array in the crate's value encoding, in a Blob that lives until the next
/// collection: the element count, then each element's parts (`element_parts`), each one as a row's
/// leaf of its type holds it, an option's flag byte first.
#[cold]
#[inline(never)]
unsafe fn array_cell<'a, M: Memory>(mem: &mut M, codes: &[u8], v: Value) -> CellIn<'a> {
    let Some((optional, element)) = array_type(codes) else {
        return *tuple_cells(mem, codes, v, 1).0;
    };
    if optional && v == NULL_POINTER {
        return CellIn::Null;
    }
    if tag_of(v) != Some(TAG_ARRAY_I) {
        mismatch();
    }
    let array = v.as_array();
    let (record, parts) = element_parts(element);
    let n = array.len();
    // The cells of every element's parts, in order, and the bytes they take.
    let cells = scratch::<CellIn, M>(mem, n * parts.clone().count());
    let mut len = put_uleb128(&mut [], n as u128);
    let mut k = 0;
    for i in 0..n {
        let x = array.get(i);
        let mut fields = record.then(|| Fields::of(x));
        for e in parts.clone() {
            let v = match &mut fields {
                Some(fields) => fields.field(e.hash),
                None => x,
            };
            let c = cell(mem, e.codes, v);
            put_cell(&mut [], &mut len, e.codes, c);
            cells.add(k).write(c);
            k += 1;
        }
    }
    let out = core::slice::from_raw_parts_mut(scratch::<u8, M>(mem, len), len);
    let mut at = put_uleb128(out, n as u128);
    let mut k = 0;
    for _ in 0..n {
        for e in parts.clone() {
            put_cell(out, &mut at, e.codes, *cells.add(k));
            k += 1;
        }
    }
    CellIn::Bytes(out)
}

/// Room for the cells of a row of `n` columns whose key `v` is a tuple of parts of the type codes
/// `codes` (`key_parts`), in a Blob that lives until the next collection: the cell of the key's
/// first part in column 0, and those of its other parts after the last column, part j the j-th from
/// the end (`Cells`); and the number of cells. Traps when `codes` are not a key's tuple's.
#[cold]
#[inline(never)]
unsafe fn tuple_cells<'a, M: Memory>(
    mem: &mut M,
    codes: &[u8],
    v: Value,
    n: usize,
) -> (*mut CellIn<'a>, usize) {
    let Some(parts) = key_parts(codes).filter(|p| (1..=MAX_KEY_PARTS).contains(&p.len())) else {
        rts_trap_with("native table: malformed layout")
    };
    if tag_of(v) != Some(TAG_ARRAY_T) || v.as_array().len() != parts.len() {
        mismatch();
    }
    let len = n + parts.len() - 1;
    let cells = scratch::<CellIn, M>(mem, len);
    for (j, &code) in parts.iter().enumerate() {
        let at = if j == 0 { 0 } else { len - j };
        cells.add(at).write(scalar(mem, code, v.as_array().get(j)));
    }
    (cells, len)
}

/// The bytes of a fixed-size value of scalar type `code`.
fn width(code: u8) -> usize {
    match code {
        ty::BOOL => 1,
        ty::U8..=ty::U64 => 1 << (code - ty::U8),
        ty::I8..=ty::I64 => 1 << (code - ty::I8),
        ty::CHAR => 4,
        _ => 8,
    }
}

/// Writes cell `c` of a part of type `codes`, a scalar's or an option of one's, in the crate's
/// value encoding at `out[*at..]`, an option's flag byte first, and moves `at` past it: a value of
/// a fixed size little-endian at its width, a Nat or Int as LEB128 or signed LEB128, a Text or
/// Blob as its length and bytes, a Principal as its length in one byte and its bytes. Writes
/// nothing past the end of `out`, so with no room it only measures.
fn put_cell(out: &mut [u8], at: &mut usize, codes: &[u8], c: CellIn) {
    let (code, optional) = column_type(codes);
    let mut put = |b: &[u8]| {
        if let Some(d) = out.get_mut(*at..*at + b.len()) {
            d.copy_from_slice(b);
        }
        *at += b.len();
    };
    if optional {
        put(&[u8::from(c != CellIn::Null)]);
    }
    let mut n = [0u8; 19];
    match c {
        CellIn::Null => {}
        CellIn::U64(x) if code == ty::NAT => {
            let k = put_uleb128(&mut n, x.into());
            put(&n[..k])
        }
        CellIn::I64(x) if code == ty::INT => {
            let k = put_sleb128(&mut n, x.into());
            put(&n[..k])
        }
        CellIn::Bool(b) => put(&[u8::from(b)]),
        CellIn::U64(x) => put(&x.to_le_bytes()[..width(code)]),
        CellIn::I64(x) => put(&x.to_le_bytes()[..width(code)]),
        CellIn::F64(f) => put(&f.to_bits().to_le_bytes()),
        CellIn::Nat(b) | CellIn::Int(b) => put(b),
        CellIn::Bytes(b) if code == ty::PRINCIPAL => {
            put(&[b.len() as u8]);
            put(b)
        }
        CellIn::Bytes(b) => {
            let k = put_uleb128(&mut n, b.len() as u128);
            put(&n[..k]);
            put(b)
        }
        _ => rts_trap_with("native table: malformed layout"),
    }
}

/// The fields of a record, found by label hash in ascending order.
struct Fields {
    record: *mut Object,
    hashes: *const u64,
    count: usize,
    /// The first field not found yet.
    next: usize,
}

impl Fields {
    unsafe fn of(r: Value) -> Fields {
        if tag_of(r) != Some(TAG_OBJECT) {
            mismatch();
        }
        let record = r.as_object();
        let hashes = (*record).hash_blob.as_blob();
        Fields {
            record,
            hashes: hashes.payload_const() as *const u64,
            count: hashes.len().as_usize() / size_of::<u64>(),
            next: 0,
        }
    }

    /// The field with label hash `hash`, larger than the hash of the field found before. The
    /// record's hash list ascends too, and may name fields the row type has not.
    unsafe fn field(&mut self, hash: u32) -> Value {
        let hash = u64::from(hash);
        while self.next < self.count && *self.hashes.add(self.next) < hash {
            self.next += 1;
        }
        if self.next == self.count || *self.hashes.add(self.next) != hash {
            mismatch();
        }
        self.next += 1;
        self.record.get(self.next - 1)
    }
}

/// The cells of key `k` and row `r` in the columns of `layout`, in a Blob that lives until the
/// next collection. Traps when the layout has fewer column entries than columns.
pub unsafe fn cells<'a, M: Memory>(mem: &mut M, layout: &Layout, k: Value, r: Value) -> Cells<'a> {
    let mut fields = layout.record_row().then(|| Fields::of(r));
    let mut entries = layout.entries();
    let (out, len) = row_of(mem, entries.next(), layout.columns(), k);
    for c in 1..layout.columns() {
        let Some(e) = entries.next() else {
            rts_trap_with("native table: malformed layout")
        };
        let v = match &mut fields {
            Some(fields) => fields.field(e.hash),
            None => r,
        };
        out.add(c).write(cell(mem, e.codes, v));
    }
    Cells {
        cells: core::slice::from_raw_parts(out, len),
    }
}

/// Room for the cells of a row of `n` columns whose key `k` has the column entry `key`, in a Blob
/// that lives until the next collection, with the key's cell in column 0, of its first part when the
/// key is a tuple (`tuple_cells`); and the number of cells.
#[inline(always)]
unsafe fn row_of<'a, M: Memory>(
    mem: &mut M,
    key: Option<Entry>,
    n: usize,
    k: Value,
) -> (*mut CellIn<'a>, usize) {
    let Some(e) = key else {
        rts_trap_with("native table: malformed layout")
    };
    if key_parts(e.codes).is_some() {
        return tuple_cells(mem, e.codes, k, n);
    }
    let out = scratch::<CellIn, M>(mem, n);
    out.write(cell(mem, e.codes, k));
    (out, n)
}

/// The cells of the fields of `r`, a value of the row type of `named`, at their positions in
/// `layout`, those of key `k` at the key's (`row_of`), null at the other fields', in a Blob that
/// lives until the next collection; the leaf of each field of `named`, its position << 16, written
/// at `leaves`. Each field is found by `merge`, which takes a Nat for an Int, or at its place in
/// `places` (`placed`), and its cell is made for the table's type. Traps when `named` has fewer
/// column entries than columns. Inlined, so that `nt_update` and `nt_upsert` make the cells in
/// their own code.
#[inline(always)]
pub unsafe fn update_cells<'a, M: Memory>(
    mem: &mut M,
    layout: &Layout,
    named: &Layout,
    places: Option<&[u8]>,
    k: Value,
    r: Value,
    leaves: *mut u32,
) -> Cells<'a> {
    let mut fields = named.record_row().then(|| Fields::of(r));
    let n = layout.columns();
    let (out, len) = row_of(mem, layout.entries().next(), n, k);
    for i in 1..n {
        out.add(i).write(CellIn::Null);
    }
    let mut i = 0;
    let mut by_name = merge(layout, named, true);
    let mut by_place = places.map(|places| placed(layout, named, places));
    loop {
        let (p, codes, f) = match &mut by_place {
            Some(by_place) => match by_place.next() {
                Some((p, f)) => (p, f.codes, f),
                None => break,
            },
            None => match by_name.next() {
                Some((p, e, f)) => (p, e.codes, f),
                None => break,
            },
        };
        let v = match &mut fields {
            Some(fields) => fields.field(f.hash),
            None => r,
        };
        out.add(p).write(cell(mem, codes, v));
        leaves.add(i).write((p as u32) << 16);
        i += 1;
    }
    if i + 1 != named.columns() {
        rts_trap_with("native table: malformed layout");
    }
    Cells {
        cells: core::slice::from_raw_parts(out, len),
    }
}

/// The cells of key `$k`, the first column of `$layout`, as a row's: the key's cell, put in `$one`,
/// a `[CellIn; 1]` its caller declares, or the cells of any other key (`other_key`). A macro, so a
/// scalar's cell is made in the caller's place, not copied there.
macro_rules! key {
    ($mem:expr, $layout:expr, $k:expr, $one:ident) => {
        match $layout.entries().next() {
            Some(e) if e.codes.len() == 1 => {
                $one = [$crate::native_table::row::scalar($mem, e.codes[0], $k)];
                $crate::native_table::row::Cells::new(&$one)
            }
            key => $crate::native_table::row::other_key($mem, key, $k),
        }
    };
}
pub(super) use key;

/// The cells of key `k`, whose column entry `key` is not of a scalar type, as a row's (`row_of`):
/// of its parts when it is a tuple.
#[cold]
#[inline(never)]
pub(super) unsafe fn other_key<'a, M: Memory>(
    mem: &mut M,
    key: Option<Entry>,
    k: Value,
) -> Cells<'a> {
    let (out, len) = row_of(mem, key, 1, k);
    Cells {
        cells: core::slice::from_raw_parts(out, len),
    }
}

/// The bytes of Text `v`, in place; a concatenation is flattened first.
pub unsafe fn text_bytes<'a, M: Memory>(mem: &mut M, v: Value) -> &'a [u8] {
    match text(mem, v) {
        CellIn::Bytes(b) => b,
        _ => mismatch(),
    }
}

/// The hash of a label, as the compiler computes it.
const fn label_hash(label: &[u8]) -> usize {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i < label.len() {
        sum = sum.wrapping_mul(223).wrapping_add(label[i] as u32);
        i += 1;
    }
    (sum & 0x7FFF_FFFF) as usize
}

/// The cases of an index key cell.
const CELL_NULL: usize = label_hash(b"null_");
const CELL_BOOL: usize = label_hash(b"bool");
const CELL_NAT: usize = label_hash(b"nat");
const CELL_INT: usize = label_hash(b"int");
const CELL_FLOAT: usize = label_hash(b"float");
const CELL_TEXT: usize = label_hash(b"text");
const CELL_BLOB: usize = label_hash(b"blob");
const CELL_PRINCIPAL: usize = label_hash(b"principal");

fn key_mismatch() -> ! {
    rts_trap_with("native table: index key does not match the index's fields")
}

fn out_of_range() -> ! {
    rts_trap_with("native table: index key value out of range")
}

/// A Nat or Int as an i128, or `None` when its magnitude takes more than 64 bits.
unsafe fn int_value(v: Value) -> Option<i128> {
    if let Some(n) = compact(v) {
        return Some(n.into());
    }
    if tag_of(v) != Some(TAG_BIGINT) {
        key_mismatch();
    }
    let p = v.as_bigint().mp_int_ptr();
    if mp_count_bits(p) > 64 {
        return None;
    }
    // The low 64 bits, negated when the number is negative.
    let low = mp_get_i64(p) as u64;
    Some(if mp_isneg(p) {
        -i128::from(low.wrapping_neg())
    } else {
        i128::from(low)
    })
}

/// The number of `v`, a Nat or Int, in `lo..=hi`; traps when it lies outside.
unsafe fn in_range(v: Value, lo: i128, hi: i128) -> i128 {
    match int_value(v) {
        Some(n) if lo <= n && n <= hi => n,
        _ => out_of_range(),
    }
}

/// The cell of a number, a Nat or Int, in a column of scalar type `code`. Inlined into `key_cell`,
/// as is `key_cell` into the read of an index key: as calls, they cost each count or read by an
/// indexed value tens of instructions.
#[inline(always)]
unsafe fn number<'a, M: Memory>(mem: &mut M, code: u8, v: Value) -> CellIn<'a> {
    let unsigned = |bits: u32| in_range(v, 0, (1i128 << bits) - 1) as u64;
    let signed = |bits: u32| in_range(v, -(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1) as i64;
    match code {
        ty::U8 => CellIn::U64(unsigned(8)),
        ty::U16 => CellIn::U64(unsigned(16)),
        ty::U32 => CellIn::U64(unsigned(32)),
        ty::U64 => CellIn::U64(unsigned(64)),
        ty::I8 => CellIn::I64(signed(8)),
        ty::I16 => CellIn::I64(signed(16)),
        ty::I32 => CellIn::I64(signed(32)),
        ty::I64 => CellIn::I64(signed(64)),
        ty::CHAR => match in_range(v, 0, 0x10_FFFF) {
            0xD800..=0xDFFF => out_of_range(),
            c => CellIn::U64(c as u64),
        },
        ty::NAT => {
            let negative = match compact(v) {
                Some(n) => n < 0,
                None => tag_of(v) != Some(TAG_BIGINT) || mp_isneg(v.as_bigint().mp_int_ptr()),
            };
            if negative {
                out_of_range();
            }
            nat(mem, v)
        }
        ty::INT => int(mem, v),
        _ => key_mismatch(),
    }
}

/// The cell of index key cell `v` in a column of type `codes`: null for an optional column, a
/// Nat or Int for a column of an integer type or Char when it lies in the type's range, and every
/// other case for a column of its own type. Traps otherwise.
#[inline(always)]
pub unsafe fn key_cell<'a, M: Memory>(mem: &mut M, codes: &[u8], v: Value) -> CellIn<'a> {
    if tag_of(v) != Some(TAG_VARIANT) {
        key_mismatch();
    }
    let variant = v.as_obj() as *mut Variant;
    let payload = (*variant).field;
    let (code, optional) = column_type(codes);
    match ((*variant).tag, code) {
        (CELL_NULL, _) if optional => CellIn::Null,
        (CELL_BOOL, ty::BOOL) => scalar(mem, code, payload),
        (CELL_NAT | CELL_INT, _) => number(mem, code, payload),
        (CELL_FLOAT, ty::F64) => scalar(mem, code, payload),
        (CELL_TEXT, ty::TEXT) => scalar(mem, code, payload),
        (CELL_BLOB, ty::BLOB) => scalar(mem, code, payload),
        (CELL_PRINCIPAL, ty::PRINCIPAL) => scalar(mem, code, payload),
        _ => key_mismatch(),
    }
}

/// `key_cell` out of line, for the cells of a filter: a filter is read once per call to the table,
/// and inlined there `key_cell` makes that read dearer.
#[inline(never)]
unsafe fn filter_cell<'a, M: Memory>(mem: &mut M, codes: &[u8], v: Value) -> CellIn<'a> {
    key_cell(mem, codes, v)
}

/// The cases of a filter and of the ends of its range, and the fields of a range.
const FILTER_EQ: usize = label_hash(b"eq");
const FILTER_RANGE: usize = label_hash(b"range");
const END_INCL: usize = label_hash(b"incl");
const END_EXCL: usize = label_hash(b"excl");
const RANGE_LOW: u32 = label_hash(b"low") as u32;
const RANGE_HIGH: u32 = label_hash(b"high") as u32;
// A record's fields are found in ascending label hash order.
const _: () = assert!(RANGE_LOW < RANGE_HIGH);

/// The case and the payload of the variant `v`.
unsafe fn variant_parts(v: Value) -> (usize, Value) {
    if tag_of(v) != Some(TAG_VARIANT) {
        key_mismatch();
    }
    let variant = v.as_obj() as *mut Variant;
    ((*variant).tag, (*variant).field)
}

/// One end of a filter's range, `v` of type `?{ #incl : NtCell; #excl : NtCell }`, in a column of
/// type `codes`: open when null. Its cell is read as `key_cell` reads one, and `#null_` traps as a
/// cell of the wrong kind.
unsafe fn range_end<'a, M: Memory>(mem: &mut M, codes: &[u8], v: Value) -> Bound<CellIn<'a>> {
    if v == NULL_POINTER {
        return Bound::Unbounded;
    }
    let (case, payload) = variant_parts(v);
    let cell = match filter_cell(mem, codes, payload) {
        CellIn::Null => key_mismatch(),
        cell => cell,
    };
    match case {
        END_INCL => Bound::Included(cell),
        END_EXCL => Bound::Excluded(cell),
        _ => key_mismatch(),
    }
}

/// What the filter `v`, of type `{ #eq : NtCell; #range : { low : ?…; high : ?… } }`, asks of the
/// value of a column of type `codes`: to equal a cell, null for `#null_`, or to lie within the
/// range's ends. Its cells are read as `key_cell` reads an index key's, and trap as those do.
pub unsafe fn filter_cond<'a, M: Memory>(mem: &mut M, codes: &[u8], v: Value) -> Cond<'a> {
    let (case, payload) = variant_parts(v);
    match case {
        FILTER_EQ => Cond::Eq(filter_cell(mem, codes, payload)),
        FILTER_RANGE => {
            let mut fields = Fields::of(payload);
            let low = range_end(mem, codes, fields.field(RANGE_LOW));
            let high = range_end(mem, codes, fields.field(RANGE_HIGH));
            Cond::Range(low, high)
        }
        _ => key_mismatch(),
    }
}

/// The unit value, as compiled code gives it: the payload of `#null_`.
const UNIT: Value = Value::from_raw(0x4000_0000_0000_0000);

/// A new variant `#case(payload)`.
unsafe fn variant_of<M: Memory>(mem: &mut M, case: usize, payload: Value) -> Value {
    let v = mem.alloc_words(types::size_of::<Variant>());
    let variant = v.get_ptr() as *mut Variant;
    (*variant).header.tag = TAG_VARIANT;
    (*variant).header.init_forward(v);
    (*variant).tag = case;
    (*variant).field = payload;
    allocation_barrier(v)
}

/// A Nat or Int of the big-endian magnitude `m`, each byte complemented and the number negative
/// when `negative`, as an index key holds one.
unsafe fn int_of_magnitude<M: Memory>(mem: &mut M, negative: bool, m: &[u8]) -> Value {
    let byte = |i: usize| if negative { !m[i] } else { m[i] };
    if m.len() < 16 {
        let x = (0..m.len()).fold(0i128, |x, i| x << 8 | i128::from(byte(i)));
        return int_of_i128(mem, if negative { -x } else { x });
    }
    // Its two's complement, little-endian, with a byte for the sign.
    let n = m.len() + 1;
    let b = scratch::<u8, M>(mem, n);
    let mut carry = negative as u16;
    for i in 0..n {
        let x = if i < m.len() {
            byte(m.len() - 1 - i)
        } else {
            0
        };
        *b.add(i) = if negative {
            let t = u16::from(!x) + carry;
            carry = t >> 8;
            t as u8
        } else {
            x
        };
    }
    int_of_twos(mem, core::slice::from_raw_parts(b, n))
}

/// The cells of the index key `key`, as `NtCell` values in a new array, decoded part by part: the
/// parts are of the scalar types `parts`, each with whether it can be null. `bytes` is scratch for
/// a Text or Blob part, at least as long as the key.
pub unsafe fn key_cells<M: Memory>(
    mem: &mut M,
    parts: &[(u8, bool)],
    key: &[u8],
    bytes: &mut [u8],
) -> Value {
    let image = ImageId::Runtime;
    let mut cells = [UNIT; 7];
    let mut pos = 0;
    for (cell, &(code, nullable)) in cells.iter_mut().zip(parts) {
        if nullable {
            let (some, at) = or_trap(dec_bool(key, pos, image));
            pos = at;
            if !some {
                *cell = variant_of(mem, CELL_NULL, UNIT);
                continue;
            }
        }
        let (case, payload) = match code {
            ty::BOOL => {
                let (b, at) = or_trap(dec_bool(key, pos, image));
                pos = at;
                (CELL_BOOL, Value::from_raw(b as usize))
            }
            ty::U8 | ty::U16 | ty::U32 | ty::U64 => {
                let width = 1 << (code - ty::U8);
                let (x, at) = or_trap(dec_u64(key, pos, width, image));
                pos = at;
                (CELL_NAT, nat_of_u64(x))
            }
            ty::I8 | ty::I16 | ty::I32 | ty::I64 => {
                // Two's complement with the top bit flipped.
                let width = 1 << (code - ty::I8);
                let (x, at) = or_trap(dec_u64(key, pos, width, image));
                pos = at;
                let shift = 64 - 8 * u32::from(width);
                let x = ((x ^ 1 << (63 - shift)) << shift) as i64 >> shift;
                (CELL_INT, int_from_isize(x as isize))
            }
            ty::CHAR => {
                let (c, at) = or_trap(dec_char(key, pos, image));
                pos = at;
                (CELL_NAT, nat_of_u64(u64::from(c)))
            }
            ty::NAT | ty::INT => {
                let (negative, n, at) = or_trap(int_header(key, pos, image));
                let end = at.saturating_add(n);
                let Some(m) = key.get(at as usize..end as usize) else {
                    rts_trap_with("native table: malformed index key")
                };
                pos = end;
                let case = if code == ty::NAT { CELL_NAT } else { CELL_INT };
                (case, int_of_magnitude(mem, negative, m))
            }
            ty::F64 => {
                // The bits with the sign bit set when the sign is 0, else their complement.
                let (x, at) = or_trap(dec_u64(key, pos, 8, image));
                pos = at;
                let bits = if x >> 63 != 0 { x ^ 1 << 63 } else { !x };
                (CELL_FLOAT, box64(mem, TAG_BITS64_F, bits))
            }
            ty::TEXT | ty::BLOB => {
                let mut out = SliceSink::new(&mut *bytes);
                pos = or_trap(dec_bytes(key, pos, &mut out, image));
                let b = out.written();
                if code == ty::TEXT {
                    check_utf8(b);
                    (CELL_TEXT, blob_of(mem, TAG_BLOB_T, b))
                } else {
                    (CELL_BLOB, blob_of(mem, TAG_BLOB_B, b))
                }
            }
            ty::PRINCIPAL => {
                let (b, n, at) = or_trap(dec_principal(key, pos, image));
                pos = at;
                (CELL_PRINCIPAL, blob_of(mem, TAG_BLOB_P, &b[..n as usize]))
            }
            _ => rts_trap_with("native table: malformed layout"),
        };
        *cell = variant_of(mem, case, payload);
    }
    if pos != key.len() as u64 {
        rts_trap_with("native table: malformed index key");
    }
    let array = alloc_array(mem, TAG_ARRAY_I, parts.len());
    for (i, &cell) in cells.iter().take(parts.len()).enumerate() {
        array.as_array().initialize(i, cell, mem);
    }
    allocation_barrier(array)
}

/// The value of `r`; traps with the fault's text on a fault.
pub(super) fn or_trap<T>(r: Result<T, Fault>) -> T {
    r.unwrap_or_else(|fault| rts_trap_with(fault.text()))
}

/// A small scalar of type `code` with payload `x`.
fn small_of(code: u8, x: u64) -> Value {
    let (shift, tag, _) = small_form(code);
    Value::from_raw((x as usize) << shift | tag)
}

/// A boxed 64-bit word with heap tag `tag`.
unsafe fn box64<M: Memory>(mem: &mut M, tag: Tag, bits: u64) -> Value {
    let v = mem.alloc_words(types::size_of::<Bits64>());
    let b = v.get_ptr() as *mut Bits64;
    (*b).header.tag = tag;
    (*b).header.init_forward(v);
    (*b).bits = bits;
    allocation_barrier(v)
}

/// A Nat64: tagged when it fits in 60 bits, else boxed.
pub(super) unsafe fn nat64_of<M: Memory>(mem: &mut M, x: u64) -> Value {
    if x >> (64 - WORD64_SHIFT) == 0 {
        Value::from_raw((x as usize) << WORD64_SHIFT | NAT64_TAG)
    } else {
        box64(mem, TAG_BITS64_U, x)
    }
}

/// An Int64: tagged when it fits in 60 bits, else boxed.
unsafe fn int64_of<M: Memory>(mem: &mut M, x: i64) -> Value {
    if (x << WORD64_SHIFT) >> WORD64_SHIFT == x {
        Value::from_raw((x as usize) << WORD64_SHIFT | INT64_TAG)
    } else {
        box64(mem, TAG_BITS64_S, x as u64)
    }
}

/// A Nat of at most 64 bits: compact when it fits.
pub(super) unsafe fn nat_of_u64(n: u64) -> Value {
    match isize::try_from(n) {
        Ok(n) => int_from_isize(n),
        Err(_) => bigint_of_word64(n),
    }
}

/// A buffer over `b` for the runtime's LEB128 decoders.
fn buf_of(b: &[u8]) -> Buf {
    Buf {
        ptr: b.as_ptr() as *mut u8,
        end: b.as_ptr().wrapping_add(b.len()) as *mut u8,
    }
}

/// A Nat of its LEB128 bytes, past 64 bits.
unsafe fn nat_of_leb(b: &[u8]) -> Value {
    bigint_leb128_decode(&mut buf_of(b))
}

/// An Int of its signed LEB128 bytes, past 64 bits.
unsafe fn int_of_sleb(b: &[u8]) -> Value {
    bigint_sleb128_decode(&mut buf_of(b))
}

/// An Int of its two's-complement bytes, little-endian, past 64 bits. Inlined, as `int_of_i128`
/// is, into each primitive that reads an integer total: as a call shared by two of them, it
/// costs a read of a sum tens of instructions.
#[inline(always)]
pub(super) unsafe fn int_of_twos<M: Memory>(mem: &mut M, b: &[u8]) -> Value {
    // Signed LEB128: 7 bits per byte, low bits first; the last byte's top bit holds the sign,
    // since it lies at or past the top bit of `b`.
    let n = (b.len() * 8).div_ceil(7);
    let leb = scratch::<u8, M>(mem, n);
    let ext = if b.last().is_some_and(|&x| x & 0x80 != 0) {
        0xFF
    } else {
        0
    };
    for i in 0..n {
        let (at, shift) = (7 * i / 8, 7 * i % 8);
        let lo = u16::from(b.get(at).copied().unwrap_or(ext));
        let hi = u16::from(b.get(at + 1).copied().unwrap_or(ext));
        let bits = ((hi << 8 | lo) >> shift) as u8 & 0x7F;
        *leb.add(i) = if i + 1 < n { bits | 0x80 } else { bits };
    }
    int_of_sleb(core::slice::from_raw_parts(leb, n))
}

/// An Int of an i128: compact when it fits.
#[inline(always)]
pub(super) unsafe fn int_of_i128<M: Memory>(mem: &mut M, x: i128) -> Value {
    match i64::try_from(x) {
        Ok(x) => int_from_isize(x as isize),
        Err(_) => int_of_twos(mem, &x.to_le_bytes()),
    }
}

/// A new blob with heap tag `tag` that holds `b`.
unsafe fn blob_of<M: Memory>(mem: &mut M, tag: Tag, b: &[u8]) -> Value {
    let v = alloc_blob(mem, tag, Bytes(b.len()));
    memcpy_bytes(
        v.as_blob_mut().payload_addr() as usize,
        b.as_ptr() as usize,
        Bytes(b.len()),
    );
    allocation_barrier(v)
}

/// The pair (a, b), a tuple as compiled code builds it.
pub(super) unsafe fn pair<M: Memory>(mem: &mut M, a: Value, b: Value) -> Value {
    let v = alloc_array(mem, TAG_ARRAY_T, 2);
    let array = v.as_array();
    array.initialize(0, a, mem);
    array.initialize(1, b, mem);
    allocation_barrier(v)
}

/// Checks that the bytes of a Text read from a table are UTF-8.
unsafe fn check_utf8(b: &[u8]) {
    if !utf8_valid(b.as_ptr() as *const _, b.len()) {
        rts_trap_with("native table: stored text is not UTF-8");
    }
}

/// The `NtCell` of cell `v` of a column of scalar type `code`, an option when `optional`:
/// `#null_` for null, `#nat` for a value of a Nat type or Char, `#int` for one of an Int type, and
/// the case of its own type for any other value; `None` for a cell that is not of the column's
/// type, and for one that does not lie in one piece.
unsafe fn cell_value<M: Memory>(
    mem: &mut M,
    code: u8,
    optional: bool,
    v: CellOut,
) -> Option<Value> {
    let (case, payload) = match (code, v) {
        (_, CellOut::Null) if optional => (CELL_NULL, UNIT),
        (ty::BOOL, CellOut::Bool(b)) => (CELL_BOOL, Value::from_raw(b as usize)),
        (ty::U8 | ty::U16 | ty::U32 | ty::U64 | ty::CHAR | ty::NAT, CellOut::U64(x)) => {
            (CELL_NAT, nat_of_u64(x))
        }
        (ty::NAT, CellOut::Nat(b)) => (CELL_NAT, nat_of_leb(b)),
        (ty::I8 | ty::I16 | ty::I32 | ty::I64 | ty::INT, CellOut::I64(x)) => {
            (CELL_INT, int_from_isize(x as isize))
        }
        (ty::INT, CellOut::Int(b)) => (CELL_INT, int_of_sleb(b)),
        (ty::F64, CellOut::F64(f)) => (CELL_FLOAT, box64(mem, TAG_BITS64_F, f.to_bits())),
        (ty::TEXT, CellOut::Bytes(b)) => {
            check_utf8(b);
            (CELL_TEXT, blob_of(mem, TAG_BLOB_T, b))
        }
        (ty::BLOB, CellOut::Bytes(b)) => (CELL_BLOB, blob_of(mem, TAG_BLOB_B, b)),
        (ty::PRINCIPAL, CellOut::Bytes(b)) => (CELL_PRINCIPAL, blob_of(mem, TAG_BLOB_P, b)),
        _ => return None,
    };
    Some(variant_of(mem, case, payload))
}

/// The value of cell `v` of scalar type `code`, an option of one when `optional`, built as compiled
/// code builds one; `None` when the cell is not of that type or does not lie in one piece.
#[inline(always)]
unsafe fn value_of<M: Memory>(mem: &mut M, code: u8, optional: bool, v: CellOut) -> Option<Value> {
    Some(match (code, v) {
        (_, CellOut::Null) if optional => NULL_POINTER,
        (ty::BOOL, CellOut::Bool(b)) => Value::from_raw(b as usize),
        (ty::U8 | ty::U16 | ty::U32 | ty::CHAR, CellOut::U64(x)) => small_of(code, x),
        (ty::I8 | ty::I16 | ty::I32, CellOut::I64(x)) => small_of(code, x as u64),
        (ty::U64, CellOut::U64(x)) => nat64_of(mem, x),
        (ty::I64, CellOut::I64(x)) => int64_of(mem, x),
        (ty::NAT, CellOut::U64(n)) => nat_of_u64(n),
        (ty::NAT, CellOut::Nat(b)) => nat_of_leb(b),
        (ty::INT, CellOut::I64(n)) => int_from_isize(n as isize),
        (ty::INT, CellOut::Int(b)) => int_of_sleb(b),
        (ty::F64, CellOut::F64(f)) => box64(mem, TAG_BITS64_F, f.to_bits()),
        (ty::TEXT, CellOut::Bytes(b)) => {
            check_utf8(b);
            blob_of(mem, TAG_BLOB_T, b)
        }
        (ty::BLOB, CellOut::Bytes(b)) => blob_of(mem, TAG_BLOB_B, b),
        (ty::PRINCIPAL, CellOut::Bytes(b)) => blob_of(mem, TAG_BLOB_P, b),
        _ => return None,
    })
}

fn malformed_array() -> ! {
    rts_trap_with("native table: malformed array")
}

/// The array whose bytes `b` are as `array_cell` writes them, of elements of the type codes
/// `element`, built as compiled code builds one; a record element's hash list is made once, here.
/// Traps when `b` does not hold one.
unsafe fn array_of_ve<M: Memory>(mem: &mut M, element: &[u8], b: &[u8]) -> Value {
    let (n, at) = rd_uleb(b, 0, ImageId::Runtime).unwrap_or_else(|_| malformed_array());
    // Every element takes a byte at least.
    if n > b.len() as u64 {
        malformed_array();
    }
    let (n, mut at) = (n as usize, at as usize);
    let (record, parts) = element_parts(element);
    let labels = if record {
        let fields = parts.clone().count();
        let v = alloc_blob(mem, TAG_BLOB_B, Bytes(fields * size_of::<u64>()));
        let hashes = v.as_blob_mut().payload_addr() as *mut u64;
        for (i, e) in parts.clone().enumerate() {
            hashes.add(i).write_unaligned(u64::from(e.hash));
        }
        allocation_barrier(v)
    } else {
        NULL_POINTER
    };
    let array = alloc_array(mem, TAG_ARRAY_I, n);
    for i in 0..n {
        let x = if record {
            let v = new_object(mem, labels);
            let fields = (v.get_ptr() as *mut Object).payload_addr();
            for (j, e) in parts.clone().enumerate() {
                *fields.add(j) = part_of_ve(mem, e.codes, b, &mut at);
            }
            allocation_barrier(v)
        } else {
            part_of_ve(mem, element, b, &mut at)
        };
        array.as_array().initialize(i, x, mem);
    }
    if at != b.len() {
        malformed_array();
    }
    allocation_barrier(array)
}

/// The value of a part of the type codes `codes` (`element_parts`) at `b[*at..]`, as `put_cell`
/// writes it, and `at` moved past it. Traps when `b` does not hold one there.
unsafe fn part_of_ve<M: Memory>(mem: &mut M, codes: &[u8], b: &[u8], at: &mut usize) -> Value {
    let (code, optional) = column_type(codes);
    if optional {
        let some = b.get(*at).copied();
        *at += 1;
        match some {
            Some(0) => return NULL_POINTER,
            Some(1) => {}
            _ => malformed_array(),
        }
    }
    let rest = b.get(*at..).unwrap_or(&[]);
    let take = |n: usize| rest.get(..n).unwrap_or_else(|| malformed_array());
    let (c, n) = match code {
        ty::NAT | ty::INT => {
            let n = rest
                .iter()
                .position(|&x| x < 0x80)
                .map_or(usize::MAX, |i| i + 1);
            let leb = take(n);
            let c = match (code, leb_value(leb, code == ty::INT)) {
                (ty::NAT, Some(x)) if x <= u64::MAX as i128 => CellOut::U64(x as u64),
                (ty::INT, Some(x)) if i64::try_from(x).is_ok() => CellOut::I64(x as i64),
                (ty::NAT, _) => CellOut::Nat(leb),
                _ => CellOut::Int(leb),
            };
            (c, n)
        }
        ty::TEXT | ty::BLOB => {
            let (len, k) = rd_uleb(rest, 0, ImageId::Runtime).unwrap_or_else(|_| malformed_array());
            let end = (k as usize).saturating_add(len as usize);
            (CellOut::Bytes(&take(end)[k as usize..]), end)
        }
        ty::PRINCIPAL => {
            let len = usize::from(take(1)[0]);
            // A principal has at most 29 bytes.
            if len > 29 {
                malformed_array();
            }
            (CellOut::Bytes(&take(1 + len)[1..]), 1 + len)
        }
        _ => {
            let w = width(code);
            let mut x = [0u8; 8];
            x[..w].copy_from_slice(take(w));
            let x = u64::from_le_bytes(x);
            let c = match code {
                ty::BOOL if x > 1 => malformed_array(),
                ty::BOOL => CellOut::Bool(x == 1),
                // A small scalar's value keeps only its low bits.
                ty::I8..=ty::I64 => CellOut::I64(x as i64),
                ty::F64 => CellOut::F64(f64::from_bits(x)),
                ty::CHAR if char::from_u32(x as u32).is_none() => malformed_array(),
                _ => CellOut::U64(x),
            };
            (c, w)
        }
    };
    *at += n;
    value_of(mem, code, false, c).unwrap_or_else(|| malformed_array())
}

/// A new record whose hash list is `labels`, its fields not written yet.
unsafe fn new_object<M: Memory>(mem: &mut M, labels: Value) -> Value {
    let labels = labels.forward_if_possible();
    let fields = labels.as_blob().len().as_usize() / WORD_SIZE;
    let v = mem.alloc_words(types::size_of::<Object>() + Words(fields));
    let record = v.get_ptr() as *mut Object;
    (*record).header.tag = TAG_OBJECT;
    (*record).header.init_forward(v);
    (*record).hash_blob = labels;
    v
}

/// The bytes of the value of `leaf` in row `rid` of `table`, read into `v`, the Blob made for them
/// when the value's cell came.
#[inline(always)]
unsafe fn read_later<'b, H: PageHost>(
    table: &Table<'_, H>,
    rid: u64,
    leaf: u32,
    v: Value,
) -> Result<&'b mut [u8], Fault> {
    let blob = v.as_blob_mut();
    let len = blob.len().as_usize();
    let bytes = core::slice::from_raw_parts_mut(blob.payload_addr(), len);
    if table.read_large(rid, leaf, 0, bytes)? != len {
        return Err(Fault::BadValue { leaf });
    }
    Ok(bytes)
}

/// Rows of a cell scan, each built as an array of `NtCell` values, one per cell the scan hands
/// out: the cells of the columns asked for, the key's among them when asked for, in the order
/// asked. No row record is built. A value that does not lie in one piece of table memory gets its Blob when its cell
/// comes, and its bytes and its cell once the row id is known. The rows built go to the start of
/// `rows`.
pub struct CellRows<'t, 'h, M: Memory, H: PageHost> {
    mem: *mut M,
    table: &'t Table<'h, H>,
    /// The leaf of each cell of a row, and the scalar type code of its column and whether the
    /// column is an option.
    leaves: &'t [u32],
    types: &'t [(u8, bool)],
    /// The cells of the row so far, and their number.
    cells: *mut Value,
    at: usize,
    /// The cells whose bytes `row_end` reads: their places in the row.
    pending: *mut usize,
    pending_len: usize,
    rows: *mut Value,
    n: usize,
}

impl<'t, 'h, M: Memory, H: PageHost> CellRows<'t, 'h, M, H> {
    /// Rows of the cells of `leaves`, of the types `types`, read from `table`, at most as many as
    /// `rows` holds.
    pub unsafe fn new(
        mem: *mut M,
        table: &'t Table<'h, H>,
        leaves: &'t [u32],
        types: &'t [(u8, bool)],
        rows: *mut Value,
    ) -> Self {
        let width = leaves.len();
        CellRows {
            mem,
            table,
            leaves,
            types,
            cells: scratch::<Value, M>(&mut *mem, width),
            at: 0,
            pending: scratch::<usize, M>(&mut *mem, width),
            pending_len: 0,
            rows,
            n: 0,
        }
    }

    /// The number of rows built.
    pub fn len(&self) -> usize {
        self.n
    }
}

impl<M: Memory, H: PageHost> RowSink for CellRows<'_, '_, M, H> {
    fn cell(&mut self, leaf: u32, v: CellOut<'_>) -> Result<(), Fault> {
        let at = self.at;
        if self.leaves.get(at) != Some(&leaf) {
            return Err(Fault::BadValue { leaf });
        }
        let (code, optional) = self.types[at];
        unsafe {
            let mem = &mut *self.mem;
            let x = match v {
                CellOut::Arg { len } if matches!(code, ty::NAT | ty::INT | ty::TEXT | ty::BLOB) => {
                    *self.pending.add(self.pending_len) = at;
                    self.pending_len += 1;
                    let tag = if code == ty::TEXT {
                        TAG_BLOB_T
                    } else {
                        TAG_BLOB_B
                    };
                    allocation_barrier(alloc_blob(mem, tag, Bytes(len as usize)))
                }
                _ => match cell_value(mem, code, optional, v) {
                    Some(x) => x,
                    None => return Err(Fault::BadValue { leaf }),
                },
            };
            *self.cells.add(at) = x;
        }
        self.at += 1;
        Ok(())
    }

    fn row_end(&mut self, rid: u64) -> Result<(), Fault> {
        let width = self.leaves.len();
        if self.at != width {
            let leaf = self.leaves[self.at];
            return Err(Fault::BadValue { leaf });
        }
        unsafe {
            let mem = &mut *self.mem;
            for i in 0..self.pending_len {
                let at = *self.pending.add(i);
                let slot = self.cells.add(at);
                let bytes = read_later(self.table, rid, self.leaves[at], *slot)?;
                let (case, payload) = match self.types[at].0 {
                    ty::NAT => (CELL_NAT, nat_of_leb(bytes)),
                    ty::INT => (CELL_INT, int_of_sleb(bytes)),
                    ty::TEXT => {
                        check_utf8(bytes);
                        (CELL_TEXT, *slot)
                    }
                    _ => (CELL_BLOB, *slot),
                };
                *slot = variant_of(mem, case, payload);
            }
            let array = alloc_array(mem, TAG_ARRAY_I, width);
            for i in 0..width {
                array.as_array().initialize(i, *self.cells.add(i), mem);
            }
            self.rows.add(self.n).write(allocation_barrier(array));
        }
        self.n += 1;
        self.at = 0;
        self.pending_len = 0;
        Ok(())
    }
}

/// What a read builds of each row it reads.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Out {
    /// The row.
    Row,
    /// The key.
    Key,
    /// The pair of the key and the row.
    Entry,
}

/// How a row builder finds the field of each cell the table hands over.
pub trait Places<'a> {
    /// The field position and the type codes of the field of `column`, whose cell comes after the
    /// `at` cells of the row handed over already; `None` when that cell is not one of `column`.
    fn next(&mut self, at: usize, column: usize) -> Option<(usize, &'a [u8])>;
    /// Whether the cell of the field at position `at` is built: a dropped field's is thrown away.
    fn kept(&self, at: usize) -> bool;
    /// The field position of `column`, one handed over already or the one `next` was given.
    fn position(&self, column: usize) -> usize;
    /// The next row's cells start from the first column again.
    fn reset(&mut self);
    /// The type codes of the field of `column`, in full: `next` gives a view's array field only
    /// the first two.
    fn codes(&self, column: usize) -> &'a [u8];
    /// The leaves the table hands over, when not every column.
    #[inline(always)]
    fn leaves(&self) -> Option<&'a [u32]> {
        None
    }
    /// The type codes of the key.
    fn key_codes(&self) -> &'a [u8] {
        self.codes(0)
    }
}

/// The fields of a table whose row type never changed: the table hands every column over in
/// order, and column i holds the field at position i, read from the layout in step with the cells.
pub struct InOrder<'a> {
    /// Every column of the layout.
    all: Entries<'a>,
    /// The columns whose cells have not come yet.
    entries: Entries<'a>,
}

impl<'a> Places<'a> for InOrder<'a> {
    #[inline(always)]
    fn next(&mut self, at: usize, column: usize) -> Option<(usize, &'a [u8])> {
        if column != at {
            return None;
        }
        self.entries.next().map(|e| (column, e.codes))
    }

    #[inline(always)]
    fn kept(&self, _: usize) -> bool {
        true
    }

    #[inline(always)]
    fn position(&self, column: usize) -> usize {
        column
    }

    #[inline(always)]
    fn reset(&mut self) {
        self.entries = self.all.clone();
    }

    fn codes(&self, column: usize) -> &'a [u8] {
        self.all.clone().nth(column).map_or(&[], |e| e.codes)
    }
}

/// The fields of a table whose row type has changed: the table hands every column over in order,
/// and the field position and the type codes of each column's field are found, by column, through
/// the table's map once per call; position `DROPPED` and no codes for a dropped column.
pub struct ByColumn<'a> {
    fields: &'a [(usize, &'a [u8])],
}

impl<'a> Places<'a> for ByColumn<'a> {
    fn next(&mut self, at: usize, column: usize) -> Option<(usize, &'a [u8])> {
        if column != at {
            return None;
        }
        self.fields.get(column).copied()
    }

    fn kept(&self, at: usize) -> bool {
        at != DROPPED
    }

    fn position(&self, column: usize) -> usize {
        self.fields[column].0
    }

    fn reset(&mut self) {}

    fn codes(&self, column: usize) -> &'a [u8] {
        self.fields.get(column).map_or(&[], |f| f.1)
    }
}

/// The fields of a view: the table hands over the cells of `leaves`, in order, the key's first
/// when the key is built. Each other cell is the view's next field, in the order of the view's
/// row type, and the cells of the parts after the first of a key that is a tuple come last.
pub struct Picked<'a> {
    leaves: &'a [u32],
    picks: Picks<'a>,
    /// The key's type codes, when the key's cell comes first.
    key: Option<&'a [u8]>,
    /// The columns of the table's layout, where the picks' positions are.
    table: Entries<'a>,
}

impl<'a> Places<'a> for Picked<'a> {
    fn next(&mut self, at: usize, column: usize) -> Option<(usize, &'a [u8])> {
        if (*self.leaves.get(at)? >> 16) as usize != column {
            return None;
        }
        match self.key {
            Some(codes) if at == 0 => Some((0, codes)),
            Some(_) => Some((at, self.picks.get(at - 1)?.1)),
            None => Some((at + 1, self.picks.get(at)?.1)),
        }
    }

    fn kept(&self, _: usize) -> bool {
        true
    }

    fn position(&self, column: usize) -> usize {
        let at = self
            .leaves
            .iter()
            .position(|&l| (l >> 16) as usize == column);
        at.map_or(DROPPED, |at| at + usize::from(self.key.is_none()))
    }

    fn reset(&mut self) {}

    fn codes(&self, column: usize) -> &'a [u8] {
        let pick = self.picks.get(self.position(column).wrapping_sub(1));
        let entry = pick.and_then(|(p, _)| self.table.clone().nth(p));
        entry.map_or(&[], |e| e.codes)
    }

    fn leaves(&self) -> Option<&'a [u32]> {
        Some(self.leaves)
    }

    fn key_codes(&self) -> &'a [u8] {
        self.key.unwrap_or(&[])
    }
}

/// Rows read from a table, one after the other, each built as Motoko values: the key as a value
/// of the key type, the row as a record whose hash list is the codec's labels, or as the one
/// scalar. Every value is built as compiled code builds it, and every new object goes through
/// the allocation barrier. A value that does not lie in one piece of table memory gets its Blob
/// when its cell comes, and its bytes in `finish`, once the row id is known. `F` finds the field
/// of each column: `InOrder` for a table whose row type never changed, `ByColumn` for one whose
/// row type has changed, `Picked` for a view.
pub struct RowOut<'a, M: Memory, F: Places<'a> = InOrder<'a>> {
    mem: *mut M,
    out: Out,
    fields: F,
    /// The number of cells of the row handed over so far, and of all of them.
    next: usize,
    columns: usize,
    /// The record's hash list, `None` when the row is a scalar or is not built.
    labels: Option<Value>,
    /// The record, null until its first field comes.
    record: *mut Object,
    /// The row: the record or the scalar.
    value: Value,
    key: Value,
    rid: u64,
    /// Leaves whose bytes `finish` reads, each as leaf << 8 | its type code; in a Blob made on the
    /// first.
    pending: *mut usize,
    pending_len: usize,
    _layout: core::marker::PhantomData<&'a ()>,
}

impl<'a, M: Memory> RowOut<'a, M> {
    /// Rows of `layout`, a record of it with `labels` as its hash list, of which `out` is built.
    /// Traps when a record is built and `labels` is not the hash list of the row type, so every
    /// record built has the fields its hash list names.
    pub unsafe fn new(mem: *mut M, layout: &Layout<'a>, labels: Value, out: Out) -> Self {
        let record = layout.record_row() && out != Out::Key;
        if record {
            if tag_of(labels) != Some(TAG_BLOB_B) {
                mismatch();
            }
            let blob = labels.as_blob();
            let bytes = core::slice::from_raw_parts(blob.payload_const(), blob.len().as_usize());
            if !layout.is_hash_list(bytes) {
                mismatch();
            }
        }
        RowOut {
            mem,
            out,
            fields: InOrder {
                all: layout.entries(),
                entries: layout.entries(),
            },
            next: 0,
            columns: layout.columns(),
            labels: record.then_some(labels),
            record: core::ptr::null_mut(),
            value: Value::from_scalar(0),
            key: Value::from_scalar(0),
            rid: 0,
            pending: core::ptr::null_mut(),
            pending_len: 0,
            _layout: core::marker::PhantomData,
        }
    }

    /// The same rows for a table whose row type has changed, whose fields `map` finds: the cell of
    /// each column goes to its field's place, and a dropped column's cell is thrown away.
    pub unsafe fn mapped(&self, map: Map) -> RowOut<'a, M, ByColumn<'a>> {
        let m = map.columns();
        let fields = scratch::<(usize, &'a [u8]), M>(&mut *self.mem, m);
        for c in 0..m {
            fields.add(c).write((DROPPED, &[]));
        }
        for (at, e) in self.fields.all.clone().enumerate() {
            fields.add(map.column(at)).write((at, e.codes));
        }
        RowOut {
            mem: self.mem,
            out: self.out,
            fields: ByColumn {
                fields: core::slice::from_raw_parts(fields, m),
            },
            next: 0,
            columns: m,
            labels: self.labels,
            record: core::ptr::null_mut(),
            value: Value::from_scalar(0),
            key: Value::from_scalar(0),
            rid: 0,
            pending: core::ptr::null_mut(),
            pending_len: 0,
            _layout: core::marker::PhantomData,
        }
    }

    /// Rows of a view of a table of `layout` with the fields `picks` (`Layout::view`), the row a
    /// record with `labels` as its hash list, or the one scalar, of which `out` is built; `map`
    /// is the table's map when its row type has changed. The leaves the table hands over
    /// (`leaves`) are the key's first when the key is built, then one per field of the view, then
    /// those of the parts after the first of a key that is a tuple, in a Blob that lives until the
    /// next collection. Traps when `labels` does not have one label per field of the view, or none
    /// for a view of a scalar.
    pub unsafe fn picked(
        mem: *mut M,
        layout: &Layout<'a>,
        picks: Picks<'a>,
        map: Option<Map>,
        labels: Value,
        out: Out,
    ) -> RowOut<'a, M, Picked<'a>> {
        let n = picks.len();
        if tag_of(labels) != Some(TAG_BLOB_B) {
            mismatch();
        }
        let record = labels.as_blob().len().as_usize() == n * size_of::<u64>();
        if !record && (labels.as_blob().len().as_usize() != 0 || n != 1) {
            mismatch();
        }
        let key = match (out, layout.entries().next()) {
            (Out::Row, _) => None,
            (_, Some(e)) => Some(e.codes),
            (_, None) => rts_trap_with("native table: malformed layout"),
        };
        let first = usize::from(key.is_some());
        let width = first + if out == Out::Key { 0 } else { n };
        // Room for the leaves of the parts after the first of a key that is a tuple, after the
        // view's fields.
        let leaves = scratch::<u32, M>(&mut *mem, width + MAX_KEY_PARTS - 1);
        let mut all = width;
        if let Some(codes) = key {
            leaves.write(0);
            for j in 1..key_parts(codes).map_or(0, <[u8]>::len) {
                leaves.add(all).write(j as u32);
                all += 1;
            }
        }
        for i in first..width {
            let p = picks.get(i - first).map_or(0, |(p, _)| p);
            let column = map.map_or(p, |map| map.column(p));
            leaves.add(i).write((column as u32) << 16);
        }
        let leaves = core::slice::from_raw_parts(leaves, all);
        RowOut {
            mem,
            out,
            fields: Picked {
                leaves,
                picks,
                key,
                table: layout.entries(),
            },
            next: 0,
            columns: width,
            labels: (record && out != Out::Key).then_some(labels),
            record: core::ptr::null_mut(),
            value: Value::from_scalar(0),
            key: Value::from_scalar(0),
            rid: 0,
            pending: core::ptr::null_mut(),
            pending_len: 0,
            _layout: core::marker::PhantomData,
        }
    }
}

impl<'a, M: Memory, F: Places<'a>> RowOut<'a, M, F> {
    /// The leaves of each row the table hands over, in order, when not every column: a view's.
    #[inline(always)]
    pub fn leaves(&self) -> Option<&'a [u32]> {
        self.fields.leaves()
    }

    /// The record, made on the first call.
    unsafe fn record(&mut self, labels: Value) -> *mut Object {
        if self.record.is_null() {
            let v = new_object(&mut *self.mem, labels);
            self.record = v.get_ptr() as *mut Object;
            self.value = v;
        }
        self.record
    }

    /// Where the value of the field at position `at` goes.
    unsafe fn slot(&mut self, at: usize) -> *mut Value {
        match (at, self.labels) {
            (0, _) => &mut self.key,
            (_, Some(labels)) => self.record(labels).payload_addr().add(at - 1),
            (_, None) => &mut self.value,
        }
    }

    /// A Blob of `len` bytes for `leaf` of scalar type `code`, filled by `finish`.
    unsafe fn later(&mut self, leaf: u32, code: u8, len: u64) -> Value {
        let mem = &mut *self.mem;
        if self.pending.is_null() {
            self.pending = scratch::<usize, M>(mem, self.columns + MAX_KEY_PARTS);
        }
        *self.pending.add(self.pending_len) = (leaf as usize) << 8 | code as usize;
        self.pending_len += 1;
        let tag = if code == ty::TEXT {
            TAG_BLOB_T
        } else {
            TAG_BLOB_B
        };
        allocation_barrier(alloc_blob(mem, tag, Bytes(len as usize)))
    }

    /// The value of cell `v` of `column` when the column is of an array type, its type codes
    /// `codes`, or else the ones `Places::codes` finds; `None` when it is not, or the cell is not
    /// an array's. An array that does not lie in one piece gets its Blob, and is built in `finish`.
    #[cold]
    #[inline(never)]
    unsafe fn array(&mut self, column: usize, codes: &[u8], v: CellOut) -> Option<Value> {
        let (optional, element) = match array_type(codes) {
            Some(t) => t,
            None => match array_type(self.fields.codes(column)) {
                Some(t) => t,
                None => return self.tuple(codes, v),
            },
        };
        match v {
            CellOut::Null if optional => Some(NULL_POINTER),
            CellOut::Bytes(b) => Some(array_of_ve(&mut *self.mem, element, b)),
            CellOut::Arg { len } => Some(self.later((column as u32) << 16, ty::LIST, len)),
            _ => None,
        }
    }

    /// The key, when `codes` are those of a key that is a tuple and `v` is the cell of its first
    /// part: a new tuple with that part's value, its other parts set as their cells come (`part`);
    /// `None` when they are not, or `v` is not of the part's type.
    #[cold]
    #[inline(never)]
    unsafe fn tuple(&mut self, codes: &[u8], v: CellOut) -> Option<Value> {
        let parts = key_parts(codes).filter(|p| (1..=MAX_KEY_PARTS).contains(&p.len()))?;
        let first = self.part_value(0, parts[0], v)?;
        let mem = &mut *self.mem;
        let tuple = alloc_array(mem, TAG_ARRAY_T, parts.len());
        tuple.as_array().initialize(0, first, mem);
        for j in 1..parts.len() {
            tuple.as_array().initialize(j, Value::from_scalar(0), mem);
        }
        Some(allocation_barrier(tuple))
    }

    /// The value of cell `v` of part `leaf` of a key that is a tuple, of scalar type `code`; a value
    /// that does not lie in one piece gets its Blob, filled by `finish`.
    unsafe fn part_value(&mut self, leaf: u32, code: u8, v: CellOut) -> Option<Value> {
        match v {
            CellOut::Arg { len } if matches!(code, ty::NAT | ty::INT | ty::TEXT | ty::BLOB) => {
                Some(self.later(leaf, code, len))
            }
            _ => value_of(&mut *self.mem, code, false, v),
        }
    }

    /// Part j > 0 of a key that is a tuple, the table's leaf j: its value goes into the key's tuple
    /// (`tuple`) when the key is built, and is passed over when it is not. Any other leaf is not one
    /// a row builder takes.
    #[cold]
    #[inline(never)]
    fn part(&mut self, leaf: u32, v: CellOut) -> Result<(), Fault> {
        let bad = Err(Fault::BadValue { leaf });
        if leaf == 0 || leaf >> 16 != 0 {
            return bad;
        }
        if self.out == Out::Row {
            return Ok(());
        }
        let j = leaf as usize;
        let Some(&code) = key_parts(self.fields.key_codes()).and_then(|parts| parts.get(j)) else {
            return bad;
        };
        unsafe {
            if tag_of(self.key) != Some(TAG_ARRAY_T) {
                return bad;
            }
            let Some(x) = self.part_value(leaf, code, v) else {
                return bad;
            };
            self.key.as_array().set_raw(j, x);
        }
        Ok(())
    }

    /// Where the value of `leaf` goes: its field's place (`slot`), or its part's place in the key
    /// when the key is a tuple.
    #[inline(never)]
    unsafe fn slot_of(&mut self, leaf: u32) -> *mut Value {
        if leaf >> 16 == 0 && tag_of(self.key) == Some(TAG_ARRAY_T) {
            return self.key.as_array().payload_addr().add(leaf as usize);
        }
        self.slot(self.fields.position((leaf >> 16) as usize))
    }

    /// The array of `column`, of an array type, whose bytes are `bytes`.
    #[cold]
    #[inline(never)]
    unsafe fn long_array(&self, column: usize, bytes: &[u8]) -> Value {
        match array_type(self.fields.codes(column)) {
            Some((_, element)) => array_of_ve(&mut *self.mem, element, bytes),
            None => rts_trap_with("native table: malformed layout"),
        }
    }

    /// What is built of the row, once the table has handed the row over: the bytes of long values
    /// are read, and the record goes through the allocation barrier. The next row can come then.
    pub unsafe fn finish<H: PageHost>(&mut self, table: &Table<'_, H>) -> Value {
        if self.next != self.columns {
            let leaf = (self.next as u32) << 16;
            rts_trap_with(Fault::BadValue { leaf }.text());
        }
        for i in 0..self.pending_len {
            let p = *self.pending.add(i);
            let (leaf, code) = ((p >> 8) as u32, p as u8);
            let slot = self.slot_of(leaf);
            let bytes = or_trap(read_later(table, self.rid, leaf, *slot));
            match code {
                ty::NAT => *slot = nat_of_leb(bytes),
                ty::INT => *slot = int_of_sleb(bytes),
                ty::TEXT => check_utf8(bytes),
                ty::LIST => *slot = self.long_array((leaf >> 16) as usize, bytes),
                _ => {}
            }
        }
        if let Some(labels) = self.labels {
            // A row type with no fields has had no field to make the record.
            self.record(labels);
            allocation_barrier(self.value);
        }
        let built = match self.out {
            Out::Row => self.value,
            Out::Key => self.key,
            Out::Entry => pair(&mut *self.mem, self.key, self.value),
        };
        self.fields.reset();
        self.next = 0;
        self.record = core::ptr::null_mut();
        self.pending_len = 0;
        built
    }
}

impl<'a, M: Memory, F: Places<'a>> RowSink for RowOut<'a, M, F> {
    fn cell(&mut self, leaf: u32, v: CellOut<'_>) -> Result<(), Fault> {
        // Leaf c << 16 | j, rotated: column c when j is 0, else past every column.
        let column = leaf.rotate_left(16) as usize;
        let Some((at, codes)) = self.fields.next(self.next, column) else {
            return self.part(leaf, v);
        };
        self.next += 1;
        let built = match self.out {
            Out::Row => at > 0,
            Out::Key => at == 0,
            Out::Entry => true,
        };
        if !built || !self.fields.kept(at) {
            return Ok(());
        }
        // An array's codes are none of a scalar's.
        let (code, optional) = match codes {
            [ty::OPTION, code] => (*code, true),
            [code] => (*code, false),
            _ => (ty::LIST, false),
        };
        unsafe {
            let mem = &mut *self.mem;
            let x = match (code, v) {
                (_, CellOut::Null) if optional => NULL_POINTER,
                (ty::BOOL, CellOut::Bool(b)) => Value::from_raw(b as usize),
                (ty::U8 | ty::U16 | ty::U32 | ty::CHAR, CellOut::U64(x)) => small_of(code, x),
                (ty::I8 | ty::I16 | ty::I32, CellOut::I64(x)) => small_of(code, x as u64),
                (ty::U64, CellOut::U64(x)) => nat64_of(mem, x),
                (ty::I64, CellOut::I64(x)) => int64_of(mem, x),
                (ty::NAT, CellOut::U64(n)) => nat_of_u64(n),
                (ty::NAT, CellOut::Nat(b)) => nat_of_leb(b),
                (ty::INT, CellOut::I64(n)) => int_from_isize(n as isize),
                (ty::INT, CellOut::Int(b)) => int_of_sleb(b),
                (ty::F64, CellOut::F64(f)) => box64(mem, TAG_BITS64_F, f.to_bits()),
                (ty::TEXT, CellOut::Bytes(b)) => {
                    check_utf8(b);
                    blob_of(mem, TAG_BLOB_T, b)
                }
                (ty::BLOB, CellOut::Bytes(b)) => blob_of(mem, TAG_BLOB_B, b),
                (ty::PRINCIPAL, CellOut::Bytes(b)) => blob_of(mem, TAG_BLOB_P, b),
                (ty::NAT | ty::INT | ty::TEXT | ty::BLOB, CellOut::Arg { len }) => {
                    self.later(leaf, code, len)
                }
                _ => match self.array(column, codes, v) {
                    Some(x) => x,
                    None => return Err(Fault::BadValue { leaf }),
                },
            };
            *self.slot(at) = x;
        }
        Ok(())
    }

    fn row_end(&mut self, rid: u64) -> Result<(), Fault> {
        self.rid = rid;
        Ok(())
    }
}
