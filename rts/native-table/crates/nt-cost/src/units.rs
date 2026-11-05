//! The table's maintenance units metered one at a time: `units_setup` builds a scenario's table and leaves
//! its maintenance pending (not metered), `units_step` runs one `Table::unit` (the runner meters this call),
//! `units_between` makes the scenario's writes between two units (not metered), and `units_info` reports
//! what the last unit ran and what it estimated it spent. With the `fuel-meter` feature and `units_meter(1)`,
//! the units read the runner's count of the replica's instructions as their host meter (`Budget::used`), as
//! they read `ic0.performance_counter` on the IC; else they have only their own estimates. With the feature,
//! every write the host lends (`get_mut`, `pair_mut`, `read_write`) is reported to the runner through
//! `env.nt_dirty`, which counts each unit's changed 4 KiB blocks. With `meter-notes` besides, the crate's
//! maintenance decisions tell the runner their look-ahead (`Budget::note`, through `env.nt_note`).
//!
//! Scenarios:
//! - 0: a group of 4,096 rows of eight columns (Text, Bool, i64, ?u8, u8, f64, Blob), every 97th Text past
//!   `LONG_VALUE` and every 500th past `HUGE_VALUE`, sealed; then 1,100 of its rows deleted and 600 updated
//!   (in place and as patches), compacted; then a second group whose rows are all deleted, dropped; the drain
//!   throughout.
//! - 1: rows of a 64 KiB-class Text until the group freezes at its out-of-line bytes, sealed; then every row
//!   updated to another 64 KiB-class value (patches), compacted.
//! - 2: a key and 620 u8 leaves: a group sealed, then 600 patch units, the urgent compaction that folds them.
//! - 3: values that share long prefixes: Text and Blob of 64 KiB and more (out of line and huge) equal but in
//!   their last bytes, and Nat values of 1,800 LEB128 bytes equal but in their lowest; sealed, then a third
//!   of the rows deleted and a third updated to other such values, compacted.
//! - 4: a key and 1,023 leaves of these flat kinds (u8, Text, Bool, f64, ?u8): a group sealed, then
//!   700 patch units and 300 deletes, the urgent compaction; then a second group whose rows are all deleted,
//!   dropped.
//! - 5: rows of three schema versions in one group: 200 rows of a key, 300 u8, 20 Text (one holding huge
//!   values, one medium ones) and a Nat; then 50 u8 and both long Text columns dropped and 300 u8 and two Text
//!   added; rows of the new version until the group freezes, sealed; then 8 more Text columns dropped (the
//!   column release rewrites the sealed group) and 300 cells patched.
//! - 6: the rows of scenario 0 with writes between units: while the first seal runs, an insert, an update and
//!   a delete after each of its first 1,500 units; then a compaction of 1,100 deletes, and a second group
//!   sealed.
//! - 7: a key and 170 Text columns, every 40th row a huge value in one of them: sealed; then a third of the rows deleted and the first quarter's others updated (patches),
//!   compacted; then 60 of the Text columns dropped (the column release rewrites the sealed group).
//! - 8: a key and 120 Bool and u8 columns: sealed; then a second group whose rows are all deleted, sealed
//!   and dropped, and a third of the first group's rows updated.
//! - 9: a key and a Text of 64 KiB values that share a 64 KiB prefix (each a huge value, differing only in its
//!   last 16 bytes): sealed; then a third of the rows deleted and a third updated to other such values,
//!   compacted.
//! - 10: the reference row (an Int timestamp, a Nat amount, a Nat8 status of three values, a Principal, a
//!   10–60-byte Text and a Bool, keyed by Nat): a group sealed.
//! - 11 on: the adversarial scenarios of `stress`.

use std::cell::{Cell, RefCell};

use native_table::consts::{BLOCK, HUGE_VALUE, LONG_VALUE, PAGE};
use native_table::host::Budget;
use native_table::schema::{flatten, write_schema, Column, ColumnType, IndexDecl, Leaf, LeafSink, SchemaParts};
use native_table::table::jobs::{Ran, Spent};
use native_table::table::layout::Ty;
use native_table::table::{seal, CellIn, CellOut, RowSink, RowSource, Table};
use native_table::value::{ty, FRAME};
use native_table::{Fault, Owner, PageHost, Region};

/// Scratch for keys longer than a point operation's stack: a vector grown on demand.
pub(crate) struct Sc(Vec<u8>);

impl native_table::Scratch for Sc {
    fn get(&mut self, need: usize) -> Result<&mut [u8], Fault> {
        if self.0.len() < need {
            self.0.resize(need, 0);
        }
        Ok(&mut self.0[..need])
    }
}

pub(crate) fn key_scratch() -> Sc {
    Sc(Vec::new())
}

#[cfg(all(target_family = "wasm", feature = "fuel-meter"))]
#[link(wasm_import_module = "env")]
extern "C" {
    /// The replica's instructions the runner counted so far.
    fn nt_fuel_used() -> u64;
    /// A write lent: a block's address, or a page's address plus 1 (its 16 blocks).
    fn nt_dirty(at: u64);
    /// The host's own work begins (1) or ends (0): the runner leaves the bulk memory instructions between
    /// out of the units' cost (a fresh page's fill and the vector's growth are this host's, not the table's).
    fn nt_host(on: u32);
}

#[cfg(all(target_family = "wasm", feature = "meter-notes"))]
#[link(wasm_import_module = "env")]
extern "C" {
    /// A maintenance decision's look-ahead, its pages and its source file (its bytes' address and length) and
    /// line (`Budget::note`).
    fn nt_note(ahead: u64, pages: u64, file: u64, len: u64, line: u32);
}

thread_local! {
    static METER: Cell<bool> = const { Cell::new(false) };
}

/// The host meter the units read: the runner's count when on, else none (0).
struct Fuel;

impl Budget for Fuel {
    fn used(&self) -> u64 {
        #[cfg(all(target_family = "wasm", feature = "fuel-meter"))]
        if METER.with(Cell::get) {
            // SAFETY: an import of the runner's that takes nothing and returns a number.
            return unsafe { nt_fuel_used() };
        }
        0
    }
    fn limit(&self) -> u64 {
        u64::MAX
    }
    #[cfg(all(target_family = "wasm", feature = "meter-notes"))]
    fn note(&self, ahead: u64, pages: u64, file: &'static str, line: u32) {
        if METER.with(Cell::get) {
            // SAFETY: an import of the runner's that takes numbers and returns nothing; it reads the file
            // name's bytes from the module's memory, where they stay.
            unsafe { nt_note(ahead, pages, file.as_ptr() as u64, file.len() as u64, line) };
        }
    }
}

/// Reports a written region to the runner.
#[inline]
fn dirty(r: Region) {
    #[cfg(all(target_family = "wasm", feature = "fuel-meter"))]
    {
        let at = match r {
            Region::Block(b) => b,
            Region::Page(p) => p | 1,
        };
        // SAFETY: an import of the runner's that takes a number and returns nothing.
        unsafe { nt_dirty(at) };
    }
    #[cfg(not(all(target_family = "wasm", feature = "fuel-meter")))]
    let _ = r;
}

/// Marks the host's own work for the runner.
#[inline]
fn host_work(on: bool) {
    #[cfg(all(target_family = "wasm", feature = "fuel-meter"))]
    // SAFETY: an import of the runner's that takes a number and returns nothing.
    unsafe {
        nt_host(u32::from(on))
    };
    #[cfg(not(all(target_family = "wasm", feature = "fuel-meter")))]
    let _ = on;
}

/// Turns the fuel meter on (1) or off (0).
#[no_mangle]
pub extern "C" fn units_meter(on: u32) {
    METER.with(|m| m.set(on != 0));
}

/// Where the allocator's own blocks would lie, as the runner counts changed blocks: its state block (the
/// free-page stack's count and the bump pointer) and its free-page stack, 511 page references a block. They
/// are outside every page the host lends.
const HOST_STATE: u64 = 1 << 61;
const FREE_STACK: u64 = HOST_STATE + BLOCK;

/// A host over one growing vector; freed pages are reused. Its allocator's blocks are counted among a unit's
/// changed blocks: a `page_alloc` changes its state block, a `page_free` its state block and the stack block
/// the entry lands in.
pub struct VecPages {
    pub(crate) mem: Vec<u8>,
    pub(crate) free: Vec<u64>,
}

impl VecPages {
    fn span(&self, r: Region) -> Result<std::ops::Range<usize>, Fault> {
        let (base, len) = match r {
            Region::Page(p) if p >= PAGE && p % PAGE == 0 => (p - PAGE, PAGE),
            Region::Block(b) if b >= PAGE && b % BLOCK == 0 => (b - PAGE, BLOCK),
            _ => return Err(Fault::BadRef),
        };
        let end = base + len;
        if end > self.mem.len() as u64 {
            return Err(Fault::BadRef);
        }
        Ok(base as usize..end as usize)
    }
}

impl PageHost for VecPages {
    fn block_of(&self, page: u64, i: u32) -> u64 {
        page + u64::from(i) * BLOCK
    }
    fn get(&self, r: Region) -> Result<&[u8], Fault> {
        let s = self.span(r)?;
        Ok(&self.mem[s])
    }
    fn get_mut(&mut self, r: Region, _w: Owner) -> Result<&mut [u8], Fault> {
        let s = self.span(r)?;
        dirty(r);
        Ok(&mut self.mem[s])
    }
    fn pair_mut(&mut self, a: Region, b: Region, _w: Owner) -> Result<(&mut [u8], &mut [u8]), Fault> {
        let (x, y) = (self.span(a)?, self.span(b)?);
        if x.start < y.end && y.start < x.end {
            return Err(Fault::BadRef);
        }
        dirty(a);
        dirty(b);
        if x.start < y.start {
            let (lo, hi) = self.mem.split_at_mut(y.start);
            Ok((&mut lo[x], &mut hi[..y.end - y.start]))
        } else {
            let (lo, hi) = self.mem.split_at_mut(x.start);
            Ok((&mut hi[..x.end - x.start], &mut lo[y]))
        }
    }
    fn read_write(&mut self, r: Region, wr: Region, w: Owner) -> Result<(&[u8], &mut [u8]), Fault> {
        let (x, y) = (self.span(r)?, self.span(wr)?);
        if x.start < y.end && y.start < x.end {
            return Err(Fault::BadRef);
        }
        let _ = w;
        dirty(wr);
        if x.start < y.start {
            let (lo, hi) = self.mem.split_at_mut(y.start);
            Ok((&lo[x], &mut hi[..y.end - y.start]))
        } else {
            let (lo, hi) = self.mem.split_at_mut(x.start);
            Ok((&hi[..x.end - x.start], &mut lo[y]))
        }
    }
    fn page_alloc(&mut self, _w: Owner) -> Result<u64, Fault> {
        // The host state block: the stack's count or the bump pointer.
        dirty(Region::Block(HOST_STATE));
        if let Some(p) = self.free.pop() {
            return Ok(p);
        }
        let k = self.mem.len() as u64 / PAGE + 1;
        host_work(true);
        self.mem.resize((k * PAGE) as usize, 0xA5);
        host_work(false);
        Ok(k * PAGE)
    }
    fn page_free(&mut self, p: u64, _w: Owner) -> Result<(), Fault> {
        self.span(Region::Page(p))?;
        // The free-page stack's block the entry lands in (511 entries a block), and the host state block.
        dirty(Region::Block(FREE_STACK + (self.free.len() as u64 / 511) * BLOCK));
        dirty(Region::Block(HOST_STATE));
        // The vector's growth is this host's own work: a replica's free-page stack takes its entry in place.
        if self.free.len() == self.free.capacity() {
            host_work(true);
            self.free.reserve(self.free.len().max(512));
            host_work(false);
        }
        self.free.push(p);
        Ok(())
    }
    fn alloc_changes(&self) -> u64 {
        1
    }
    fn test_fill(&self) -> Option<u8> {
        Some(0xA5)
    }
}

struct Leaves(Vec<Leaf>);

impl LeafSink for Leaves {
    fn leaf(&mut self, l: &Leaf) -> Result<(), Fault> {
        self.0.push(*l);
        Ok(())
    }
}

/// A schema of whole columns that grows by versions: columns added (their leaves after the last ordinal) and
/// dropped (their leaves and column marked with the version).
#[derive(Clone)]
pub(crate) struct Evo {
    pub(crate) version: u32,
    pub(crate) pool: Vec<u8>,
    pub(crate) cols: Vec<Column>,
    pub(crate) leaves: Vec<Leaf>,
    pub(crate) indexes: Vec<IndexDecl>,
}

impl Evo {
    pub(crate) fn new() -> Evo {
        Evo { version: 1, pool: Vec::new(), cols: Vec::new(), leaves: Vec::new(), indexes: Vec::new() }
    }

    /// Adds a column of type `t`; returns its id.
    pub(crate) fn add(&mut self, t: &[u8]) -> u32 {
        let id = self.cols.len() as u32;
        let tpos = self.pool.len() as u32;
        self.pool.extend(t);
        let first = self.leaves.len() as u16;
        let mut out = Leaves(Vec::new());
        let mut scratch = vec![0u8; 256 * FRAME];
        let ct = ColumnType {
            pool: &self.pool,
            column: id,
            ty: tpos,
            version: self.version,
            first_ordinal: first,
            key: id == 0,
        };
        flatten(ct, &mut out, &mut scratch).unwrap();
        self.leaves.extend(out.0);
        self.cols.push(Column { id, flags: 0, first_leaf: first, ty: tpos, default_off: 0, default_len: 0, name: 0 });
        id
    }

    pub(crate) fn drop_col(&mut self, col: u32) {
        self.cols[col as usize].flags |= 1;
        let v = self.version;
        for l in self.leaves.iter_mut().filter(|l| l.id >> 16 == col) {
            l.dropped = v;
        }
    }

    pub(crate) fn image(&self) -> Vec<u8> {
        let parts = SchemaParts {
            version: self.version,
            columns: &self.cols,
            leaves: &self.leaves,
            indexes: &self.indexes,
            type_pool: &self.pool,
            default_pool: &[],
            name_pool: &[],
        };
        let mut img = vec![0u8; 1 << 17];
        let (n, _) = write_schema(&parts, 1, &mut img).unwrap();
        img.truncate(n as usize);
        img
    }
}

fn schema(cols: &[Vec<u8>]) -> Vec<u8> {
    let mut e = Evo::new();
    for c in cols {
        e.add(c);
    }
    e.image()
}

/// An owned cell.
#[derive(Clone)]
pub(crate) enum C {
    Null,
    U(u64),
    I(i64),
    B(bool),
    F(f64),
    Bytes(Vec<u8>),
    Nat(Vec<u8>),
    Int(Vec<u8>),
    F32(f32),
    U128(u128),
    I128(i128),
    Unit,
}

impl C {
    pub(crate) fn cell(&self) -> CellIn<'_> {
        match self {
            C::Null => CellIn::Null,
            C::U(x) => CellIn::U64(*x),
            C::I(x) => CellIn::I64(*x),
            C::B(b) => CellIn::Bool(*b),
            C::F(f) => CellIn::F64(*f),
            C::Bytes(b) => CellIn::Bytes(b),
            C::Nat(b) => CellIn::Nat(b),
            C::Int(b) => CellIn::Int(b),
            C::F32(f) => CellIn::F32(*f),
            C::U128(x) => CellIn::U128(*x),
            C::I128(x) => CellIn::I128(*x),
            C::Unit => CellIn::Unit,
        }
    }
}

/// A row as cells by column (each column one leaf).
struct Row(Vec<C>);

impl RowSource for Row {
    fn cell(&mut self, leaf: u32) -> Result<CellIn<'_>, Fault> {
        Ok(self.0.get((leaf >> 16) as usize).ok_or(Fault::BadValue { leaf })?.cell())
    }
    fn fill(&mut self, _leaf: u32, _at: u64, _dst: &mut [u8]) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
}

/// A row read back with `get`, its cells by leaf id; the values handed out as long are read with `read_large`
/// after it.
#[derive(Default)]
struct Back {
    cells: Vec<(u32, C)>,
    large: Vec<u32>,
    rid: u64,
}

impl RowSink for Back {
    fn cell(&mut self, leaf: u32, v: CellOut<'_>) -> Result<(), Fault> {
        let c = match v {
            CellOut::Null => C::Null,
            CellOut::Bool(b) => C::B(b),
            CellOut::U64(x) => C::U(x),
            CellOut::I64(x) => C::I(x),
            CellOut::U128(x) => C::U128(x),
            CellOut::I128(x) => C::I128(x),
            CellOut::F32(x) => C::F32(x),
            CellOut::F64(x) => C::F(x),
            CellOut::Bytes(b) => C::Bytes(b.to_vec()),
            CellOut::Nat(b) => C::Nat(b.to_vec()),
            CellOut::Int(b) => C::Int(b.to_vec()),
            CellOut::Unit => C::Unit,
            CellOut::Arg { len } => {
                self.large.push(leaf);
                C::Bytes(vec![0; len as usize])
            }
        };
        self.cells.push((leaf, c));
        Ok(())
    }
    fn row_end(&mut self, rid: u64) -> Result<(), Fault> {
        self.rid = rid;
        Ok(())
    }
}

impl RowSource for Back {
    fn cell(&mut self, leaf: u32) -> Result<CellIn<'_>, Fault> {
        Ok(self.cells.iter().find(|x| x.0 == leaf).ok_or(Fault::BadValue { leaf })?.1.cell())
    }
    fn fill(&mut self, _leaf: u32, _at: u64, _dst: &mut [u8]) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
}

/// Sets leaf `leaf` of the row with key k to `v` through `upsert`: the row read back with `get`, its other
/// cells as they read.
pub(crate) fn set_leaf(t: &mut Table<'static, VecPages>, k: u64, leaf: u32, v: C) {
    let mut row = Back::default();
    assert!(t.get(&mut key_scratch(), &mut Key(k), &mut row).unwrap(), "key {k} present");
    for id in std::mem::take(&mut row.large) {
        let i = row.cells.iter().position(|x| x.0 == id).unwrap();
        let C::Bytes(mut b) = std::mem::replace(&mut row.cells[i].1, C::Null) else { unreachable!() };
        let mut at = 0;
        while at < b.len() {
            at += t.read_large(row.rid, id, at as u64, &mut b[at..]).unwrap();
        }
        let o = t.shape().ordinal_of(t.host(), id).unwrap().unwrap();
        row.cells[i].1 = match t.shape().leaf(t.host(), o).unwrap().ty {
            Ty::Nat => C::Nat(b),
            Ty::Int => C::Int(b),
            _ => C::Bytes(b),
        };
    }
    row.cells.iter_mut().find(|x| x.0 == leaf).unwrap().1 = v;
    t.upsert(&mut key_scratch(), &mut row).unwrap();
}

pub(crate) struct Key(pub(crate) u64);

impl RowSource for Key {
    fn cell(&mut self, _leaf: u32) -> Result<CellIn<'_>, Fault> {
        Ok(CellIn::U64(self.0))
    }
    fn fill(&mut self, _leaf: u32, _at: u64, _dst: &mut [u8]) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
}

pub(crate) struct Gen(pub(crate) u64);

impl Gen {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

pub(crate) struct State {
    pub(crate) table: Table<'static, VecPages>,
    pub(crate) scenario: u32,
    pub(crate) phase: u32,
    pub(crate) rows: u64,
    /// The scenario's next key.
    pub(crate) next_key: u64,
    /// The schema of scenarios 5 and 7 and of the scenarios from 11, as it evolves.
    pub(crate) evo: Option<Evo>,
    /// Units run since the scenario began, and the writes between them so far.
    pub(crate) units: u64,
    pub(crate) gen: Gen,
    pub(crate) last: (Ran, u8, Spent),
    /// The scenarios from 11: the writes a scenario makes once, between units, already made.
    pub(crate) done: u32,
}

thread_local! {
    pub(crate) static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Scenario 10's row: the reference row.
fn reference_row(g: &mut Gen, k: u64) -> Row {
    let x = g.next();
    let n = 10 + (x % 51) as usize;
    let text: Vec<u8> = (0..n).map(|i| b'a' + ((x >> (i % 60)) % 26) as u8).collect();
    let principal: Vec<u8> = (0..29u32).map(|i| (x.rotate_left(i) ^ k) as u8).collect();
    Row(vec![
        C::U(k),
        C::I(1_700_000_000_000 + k as i64 * 1_000),
        C::U(x % 1_000_000),
        C::U(x % 3),
        C::Bytes(principal),
        C::Bytes(text),
        C::B(x % 2 == 0),
    ])
}

fn mixed_row(g: &mut Gen, k: u64) -> Row {
    let x = g.next();
    let text = if k % 500 == 7 {
        vec![b'h'; HUGE_VALUE as usize + 100]
    } else if k % 97 == 3 {
        vec![b'm'; LONG_VALUE as usize + 1 + (k % 300) as usize]
    } else {
        format!("row {k} {}", x % 1_000).into_bytes()
    };
    Row(vec![
        C::U(k),
        C::Bytes(text),
        C::B(x % 2 == 0),
        C::I(x as i64 >> (x % 50)),
        if x % 4 == 0 { C::Null } else { C::U(x % 256) },
        C::U(x % 3),
        C::F((x % 2_000) as f64 / 7.0),
        C::Bytes((0..(x % 40)).map(|i| (x >> i) as u8).collect()),
    ])
}

pub(crate) fn frozen(t: &Table<'static, VecPages>) -> u32 {
    native_table::table::root::u32_at(t.host(), t.root(), native_table::table::root::FROZEN_WAITING).unwrap()
}

fn medium(k: u64, v: u64) -> Vec<u8> {
    vec![b'a' + ((k + v) % 26) as u8; HUGE_VALUE as usize - 64 - (k % 7) as usize]
}

/// A value of `len` bytes of one letter but its last twelve, k's decimal digits: every two share all but their
/// last bytes.
pub(crate) fn prefixed(k: u64, len: usize) -> Vec<u8> {
    let mut v = vec![b'p'; len];
    v[len - 12..].copy_from_slice(format!("{k:012}").as_bytes());
    v
}

/// A Nat of `len` LEB128 bytes: every two of the same length share all but their lowest group.
pub(crate) fn big_nat(k: u64, len: usize) -> Vec<u8> {
    let mut v = vec![0xFFu8; len];
    v[0] = 0x80 | (k % 128) as u8;
    v[len - 1] = 0x7F;
    v
}

fn prefix_row(k: u64, salt: u64) -> Row {
    let huge = k % 5 == 1;
    let len = if huge { HUGE_VALUE as usize + 300 } else { HUGE_VALUE as usize - 200 };
    Row(vec![
        C::U(k),
        C::Bytes(prefixed(k * 3 + salt, len)),
        C::Bytes(prefixed(k * 7 + salt, len - 100)),
        C::Nat(big_nat(k + salt, 1_800)),
        C::I(k as i64),
    ])
}

/// Scenario 4's columns: by index i ≥ 1, a Text every 32nd, then a Bool, a u8 of three values, an f64, a
/// ?u8, else u8.
fn wide_kind(i: usize) -> usize {
    if i % 32 == 0 {
        0
    } else {
        (i % 32).min(5)
    }
}

fn wide_cell(i: usize, k: u64, salt: u64) -> C {
    let x = k.wrapping_mul(31).wrapping_add(i as u64).wrapping_add(salt);
    match wide_kind(i) {
        0 => C::Bytes(format!("t{k}.{i}.{salt}").into_bytes()),
        1 => C::B(x % 3 == 0),
        2 => C::U(x % 3),
        3 => C::F((x % 1_000) as f64 / 3.0),
        4 => {
            if x % 5 == 0 {
                C::Null
            } else {
                C::U(x % 200)
            }
        }
        // Every other u8 column holds 0 in every row (a `Seq` leaf, which an update patches).
        _ if i % 2 == 0 && salt == 0 => C::U(0),
        _ => C::U(x % 7),
    }
}

const WIDE: usize = 1_023;

/// Scenario 9's value: 64 KiB of one byte, then k's 16 decimal digits (salted): every two share their first
/// 65,536 bytes.
fn shared_prefix(k: u64, salt: u64) -> Vec<u8> {
    let mut v = vec![b's'; 65_536 + 16];
    v[65_536..].copy_from_slice(format!("{:016}", k * 7 + salt).as_bytes());
    v
}

/// Scenario 7's Text columns.
const TEXTS: usize = 170;

/// Scenario 8's Bool and u8 columns.
const FLAGS: usize = 120;

fn flags_row(k: u64, salt: u64) -> Row {
    let mut r = vec![C::U(k)];
    for c in 1..=FLAGS as u64 {
        let x = k.wrapping_mul(7).wrapping_add(c).wrapping_add(salt);
        r.push(if c % 2 == 0 { C::B(x % 3 == 0) } else { C::U(x % 3) });
    }
    Row(r)
}

/// Scenario 7's row: short Texts, and every 40th row a huge value in one column.
fn texts_row(k: u64, salt: u64) -> Row {
    let mut r = vec![C::U(k)];
    for c in 1..=TEXTS as u64 {
        r.push(if k % 40 == 1 && c == 1 + k % TEXTS as u64 {
            C::Bytes(prefixed(k + salt, HUGE_VALUE as usize + 64))
        } else if salt == 0 {
            C::Bytes(format!("x{k}.{c}").into_bytes())
        } else {
            // Longer than the value it replaces: a patch.
            C::Bytes(format!("x{k}.{c}.{salt}.{salt}.{salt}").into_bytes())
        });
    }
    Row(r)
}

fn wide_row(k: u64) -> Row {
    let mut r = vec![C::U(k)];
    r.extend((1..=WIDE).map(|i| wide_cell(i, k, 0)));
    Row(r)
}

/// Scenario 5's row at schema version `v`, by column id: the columns of the versions before it hold cells
/// too (they are never asked for once dropped).
fn evo_row(k: u64, v: u32) -> Row {
    let cols = if v == 1 { 322 } else { 624 };
    let mut r = Vec::with_capacity(cols);
    for c in 0..cols {
        r.push(match c {
            0 => C::U(k),
            1..=300 => C::U((k + c as u64) % 5),
            301 if k % 10 == 0 => C::Bytes(prefixed(k, HUGE_VALUE as usize + 1_000)),
            302 if k % 10 == 1 => C::Bytes(prefixed(k, LONG_VALUE as usize + 900)),
            301..=320 => C::Bytes(format!("s{k}.{c}").into_bytes()),
            321 => C::U(k),
            322..=621 => C::U((k * 3 + c as u64) % 9),
            _ => C::Bytes(format!("n{k}.{c}").into_bytes()),
        });
    }
    Row(r)
}

/// Builds scenario `s`'s table and leaves its first phase's maintenance pending; returns its rows.
#[no_mangle]
pub extern "C" fn units_setup(s: u32) -> u64 {
    if s >= crate::stress::FIRST {
        return crate::stress::setup(s);
    }
    let host: &'static mut VecPages = Box::leak(Box::new(VecPages { mem: Vec::new(), free: Vec::new() }));
    let mut evo = None;
    let img = match s {
        0 | 6 => schema(&[
            vec![ty::NAT],
            vec![ty::TEXT],
            vec![ty::BOOL],
            vec![ty::I64],
            vec![ty::OPTION, ty::U8],
            vec![ty::U8],
            vec![ty::F64],
            vec![ty::BLOB],
        ]),
        1 => schema(&[vec![ty::NAT], vec![ty::TEXT], vec![ty::I64]]),
        2 => {
            let mut cols = vec![vec![ty::NAT]];
            cols.extend((0..620).map(|_| vec![ty::U8]));
            schema(&cols)
        }
        3 => schema(&[vec![ty::NAT], vec![ty::TEXT], vec![ty::BLOB], vec![ty::NAT], vec![ty::I64]]),
        9 => schema(&[vec![ty::NAT], vec![ty::TEXT]]),
        10 => schema(&[
            vec![ty::NAT],
            vec![ty::INT],
            vec![ty::NAT],
            vec![ty::U8],
            vec![ty::PRINCIPAL],
            vec![ty::TEXT],
            vec![ty::BOOL],
        ]),
        4 => {
            let mut cols = vec![vec![ty::NAT]];
            cols.extend((1..=WIDE).map(|i| match wide_kind(i) {
                0 => vec![ty::TEXT],
                1 => vec![ty::BOOL],
                2 => vec![ty::U8],
                3 => vec![ty::F64],
                4 => vec![ty::OPTION, ty::U8],
                _ => vec![ty::U8],
            }));
            schema(&cols)
        }
        7 => {
            let mut e = Evo::new();
            e.add(&[ty::NAT]);
            for _ in 1..=TEXTS {
                e.add(&[ty::TEXT]);
            }
            let img = e.image();
            evo = Some(e);
            img
        }
        8 => {
            let mut cols = vec![vec![ty::NAT]];
            cols.extend((1..=FLAGS).map(|i| if i % 2 == 0 { vec![ty::BOOL] } else { vec![ty::U8] }));
            schema(&cols)
        }
        _ => {
            let mut e = Evo::new();
            e.add(&[ty::NAT]);
            for _ in 1..=300 {
                e.add(&[ty::U8]);
            }
            for _ in 301..=320 {
                e.add(&[ty::TEXT]);
            }
            e.add(&[ty::NAT]);
            let img = e.image();
            evo = Some(e);
            img
        }
    };
    let img: &'static [u8] = Box::leak(img.into_boxed_slice());
    let (mut t, _) = Table::create(host, 7, img).unwrap();
    let mut g = Gen(0x9E37_79B9_7F4A_7C15);
    let mut k = 0u64;
    if s == 5 {
        let mut e = evo.take().unwrap();
        for _ in 0..200 {
            t.upsert(&mut key_scratch(), &mut evo_row(k, 1)).unwrap();
            k += 1;
        }
        // Version 2: 50 u8 columns and both long Text columns dropped; 300 u8 and two Text added.
        e.version = 2;
        for c in 1..=50 {
            e.drop_col(c);
        }
        e.drop_col(301);
        e.drop_col(302);
        for _ in 322..=621 {
            e.add(&[ty::U8]);
        }
        e.add(&[ty::TEXT]);
        e.add(&[ty::TEXT]);
        let img: &'static [u8] = Box::leak(e.image().into_boxed_slice());
        t.change_schema(img).unwrap();
        evo = Some(e);
    }
    while frozen(&t) == 0 {
        let mut row = match s {
            0 | 6 => mixed_row(&mut g, k),
            1 => Row(vec![C::U(k), C::Bytes(medium(k, 0)), C::I(k as i64)]),
            2 => {
                let mut r = vec![C::U(k)];
                r.extend((0..620).map(|_| C::U(0)));
                Row(r)
            }
            3 => prefix_row(k, 0),
            4 => wide_row(k),
            7 => texts_row(k, 0),
            8 => flags_row(k, 0),
            9 => Row(vec![C::U(k), C::Bytes(shared_prefix(k, 0))]),
            10 => reference_row(&mut g, k),
            _ => evo_row(k, 2),
        };
        t.upsert(&mut key_scratch(), &mut row).unwrap();
        k += 1;
    }
    STATE.with(|st| {
        *st.borrow_mut() = Some(State {
            table: t,
            scenario: s,
            phase: 0,
            rows: k,
            next_key: k,
            evo,
            units: 0,
            gen: Gen(0x5DEE_CE66_D1CE_4E5B ^ u64::from(s)),
            last: (Ran::Nothing, 0, Spent::default()),
            done: 0,
        })
    });
    k
}

/// The next phase's writes, when the pending maintenance is done: `true` when a phase began.
fn next_phase(st: &mut State) -> bool {
    if st.scenario >= crate::stress::FIRST {
        return crate::stress::next_phase(st);
    }
    let t = &mut st.table;
    let n = st.rows;
    let mut g = Gen(0xD1B5_4A32_D192_ED03 ^ u64::from(st.phase));
    st.phase += 1;
    match (st.scenario, st.phase) {
        (0, 1) => {
            for k in 0..1_100 {
                t.remove(&mut key_scratch(), &mut Key(k * 3 % n)).unwrap();
            }
            for i in 0..600u64 {
                let k = (i * 7 + 1) % n;
                if k % 3 == 0 && k < 3_300 {
                    continue;
                }
                let mut row = mixed_row(&mut g, k);
                if i % 2 == 0 {
                    row.0[1] = C::Bytes(format!("u{i}").into_bytes());
                }
                t.upsert(&mut key_scratch(), &mut row).unwrap();
            }
            true
        }
        (0, 2) | (4, 2) => {
            let first = st.next_key;
            let mut k = first;
            while frozen(t) == 0 {
                let mut row = if st.scenario == 0 { mixed_row(&mut g, k) } else { wide_row(k) };
                t.upsert(&mut key_scratch(), &mut row).unwrap();
                k += 1;
            }
            for j in first..k {
                t.remove(&mut key_scratch(), &mut Key(j)).unwrap();
            }
            st.next_key = k;
            true
        }
        (1, 1) => {
            for k in 0..n {
                let mut row = Row(vec![C::U(k), C::Bytes(medium(k, 1)), C::I(k as i64)]);
                t.upsert(&mut key_scratch(), &mut row).unwrap();
            }
            true
        }
        (2, 1) => {
            for unit in 0..600u64 {
                let key = unit % n;
                let mut r = vec![C::U(key)];
                r.extend((0..620u64).map(|c| C::U(if c == unit { 1 + unit % 200 } else { 0 })));
                t.upsert(&mut key_scratch(), &mut Row(r)).unwrap();
            }
            true
        }
        (3, 1) => {
            for k in 0..n {
                if k % 3 == 0 {
                    t.remove(&mut key_scratch(), &mut Key(k)).unwrap();
                } else {
                    t.upsert(&mut key_scratch(), &mut prefix_row(k, 1_000)).unwrap();
                }
            }
            true
        }
        (9, 1) => {
            for k in 0..n {
                if k % 3 == 0 {
                    t.remove(&mut key_scratch(), &mut Key(k)).unwrap();
                } else if k % 3 == 1 {
                    t.upsert(&mut key_scratch(), &mut Row(vec![C::U(k), C::Bytes(shared_prefix(k, 3))])).unwrap();
                }
            }
            true
        }
        (4, 1) => {
            // Patches of 700 units: cells of the u8 leaves that hold 0 in every row, and of every kind.
            let zeros: Vec<usize> = (1..=WIDE).filter(|&i| wide_kind(i) == 5 && i % 2 == 0).collect();
            for i in 0..700u64 {
                let k = (i * 13 + 5) % n;
                let c = if i % 4 == 3 { 1 + (i as usize * 7) % WIDE } else { zeros[i as usize % zeros.len()] };
                set_leaf(t, k, (c as u32) << 16, wide_cell(c, k, 1 + i));
            }
            for i in 0..300u64 {
                t.remove(&mut key_scratch(), &mut Key((i * 11 + 2) % n)).unwrap();
            }
            true
        }
        (5, 1) => {
            // Version 3: eight more Text columns dropped; the release rewrites the sealed group.
            let mut e = st.evo.take().unwrap();
            e.version = 3;
            for c in 303..=310 {
                e.drop_col(c);
            }
            let img: &'static [u8] = Box::leak(e.image().into_boxed_slice());
            t.change_schema(img).unwrap();
            st.evo = Some(e);
            for i in 0..300u64 {
                let k = (i * 17 + 3) % n;
                let c = 351 + (i as u32 % 250);
                set_leaf(t, k, c << 16, C::U(1 + i % 200));
            }
            true
        }
        (6, 1) => {
            // A compaction of the first group.
            for k in 0..1_100 {
                t.remove(&mut key_scratch(), &mut Key(k * 3 % n)).unwrap();
            }
            true
        }
        (7, 1) => {
            for k in 0..n {
                if k % 3 == 0 {
                    t.remove(&mut key_scratch(), &mut Key(k)).unwrap();
                }
            }
            for k in 0..n / 4 {
                if k % 3 != 0 {
                    t.upsert(&mut key_scratch(), &mut texts_row(k, 9)).unwrap();
                }
            }
            true
        }
        (7, 2) => {
            // Version 2: 60 Text columns dropped; the release rewrites the sealed group.
            let mut e = st.evo.take().unwrap();
            e.version = 2;
            for c in 1..=60 {
                e.drop_col(c);
            }
            let img: &'static [u8] = Box::leak(e.image().into_boxed_slice());
            t.change_schema(img).unwrap();
            st.evo = Some(e);
            true
        }
        (8, 1) => {
            // A second group, all of its rows deleted: its seal, then its drop; and a third of the first group's
            // rows updated (patches).
            let mut k = st.next_key;
            let first = k;
            while frozen(t) == 0 {
                t.upsert(&mut key_scratch(), &mut flags_row(k, 0)).unwrap();
                k += 1;
            }
            for j in first..k {
                t.remove(&mut key_scratch(), &mut Key(j)).unwrap();
            }
            st.next_key = k;
            for j in 0..n {
                if j % 3 == 1 {
                    t.upsert(&mut key_scratch(), &mut flags_row(j, 1)).unwrap();
                }
            }
            true
        }
        (6, 2) => {
            // A second group, sealed.
            let mut k = st.next_key;
            while frozen(t) == 0 {
                t.upsert(&mut key_scratch(), &mut mixed_row(&mut g, k)).unwrap();
                k += 1;
            }
            st.next_key = k;
            true
        }
        _ => false,
    }
}

/// Runs one maintenance unit; when none is pending, the scenario's next phase's writes (not part of any
/// unit) and then its first unit. Returns 0 when the scenario is done, else 1.
#[no_mangle]
pub extern "C" fn units_step() -> u64 {
    STATE.with(|st| {
        let mut b = st.borrow_mut();
        let st = b.as_mut().unwrap();
        let mut steps = [0u8; 3];
        for (i, kind) in [seal::SEAL, seal::COMPACTION, seal::RELEASE].into_iter().enumerate() {
            let s = seal::running(st.table.host(), st.table.root(), kind).unwrap();
            if s != 0 {
                steps[i] = seal::step_of(st.table.host(), s).unwrap();
            }
        }
        let (ran, spent) = st.table.unit(&Fuel).unwrap();
        let step = match ran {
            Ran::Seal => steps[0],
            Ran::Compaction => steps[1],
            Ran::Release => steps[2],
            _ => 0,
        };
        st.last = (ran, step, spent);
        u64::from(ran != Ran::Nothing)
    })
}

/// The scenario's writes between two units (not metered): scenario 6's inserts, updates and deletes while its
/// first seal runs; those of the scenarios of `stress`.
#[no_mangle]
pub extern "C" fn units_between() {
    STATE.with(|st| {
        let mut b = st.borrow_mut();
        let st = b.as_mut().unwrap();
        st.units += 1;
        if st.scenario >= crate::stress::FIRST {
            crate::stress::between(&mut b);
            return;
        }
        if st.scenario != 6 {
            return;
        }
        if st.phase == 0 && st.units <= 1_500 {
            let k = st.next_key;
            st.next_key += 1;
            let mut row = mixed_row(&mut st.gen, k);
            st.table.upsert(&mut key_scratch(), &mut row).unwrap();
            let u = st.gen.next() % st.rows;
            let mut row = mixed_row(&mut st.gen, u);
            if u % 2 == 0 {
                row.0[1] = C::Bytes(vec![b'w'; (LONG_VALUE as usize) + (u % 500) as usize]);
            }
            st.table.upsert(&mut key_scratch(), &mut row).unwrap();
            let d = st.gen.next() % st.next_key;
            st.table.remove(&mut key_scratch(), &mut Key(d)).unwrap();
        }
    })
}

/// Starts the scenario's next phase when no unit is pending: 1 when one began, 0 when the scenario is done.
#[no_mangle]
pub extern "C" fn units_phase() -> u64 {
    STATE.with(|st| u64::from(next_phase(st.borrow_mut().as_mut().unwrap())))
}

/// What the last unit ran: 0, its job (1 seal, 2 compaction, 3 drop, 4 release, 5 drain) · 16 + the step
/// its rewrite was at; 1, its estimated instructions; 2, its bytes; 3, its estimated changed blocks.
#[no_mangle]
pub extern "C" fn units_info(sel: u32) -> u64 {
    STATE.with(|st| {
        let b = st.borrow();
        let (ran, step, spent) = b.as_ref().unwrap().last;
        match sel {
            0 => {
                let job = match ran {
                    Ran::Nothing => 0,
                    Ran::Seal => 1,
                    Ran::Compaction => 2,
                    Ran::Drop => 3,
                    Ran::Release => 4,
                    Ran::Drain => 5,
                    Ran::Build => 6,
                };
                job * 16 + u64::from(step)
            }
            1 => spent.work,
            2 => spent.bytes,
            _ => spent.blocks,
        }
    })
}
