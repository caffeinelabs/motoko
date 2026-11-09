//! Updates of sealed rows (in place and in patch columns), rewrites of whole groups (the seal a filling insert
//! runs, compaction, column release), group drops, schema change with defaults, and the maintenance step.
//!
//! Every test runs once per fill pattern (`FILLS`) through `per_fill`, which requires identical results and
//! identical host-call logs from the two runs. Changed-block counts come from `VecHost::end_message`.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use native_table::consts::{COMPACT_DEAD, GROUP_SLOTS, HUGE_VALUE, LONG_VALUE, PATCH_UNITS_PER_DIR};
use native_table::fault::{Fault, SchemaError};
use native_table::mem::{rd_u16, rd_u32, rd_u64};
use native_table::schema::{write_schema, Column, Leaf, SchemaParts, SchemaView};
use native_table::table::group::GroupView;
use native_table::table::jobs::Ran;
use native_table::table::layout::Ty;
use native_table::table::{buffer, page, patch, root, write, Table, Upsert};
use native_table::value::ty;
use native_table::{CorruptAt, Corruption, ImageId, PageHost, Progress, Region, Wake};
use nt_testhost::big::Big;
use nt_testhost::rows::{idl_hash, schema, t, Sink, Src, V};
use nt_testhost::{open_group, Call, TestBudget, VecHost, VecScratch, XorShift, FILLS};

/// Scratch for keys longer than a point operation's stack.
fn key_scratch() -> VecScratch {
    VecScratch::new(FILLS[0])
}

// ---------------------------------------------------------------- harness

/// The model's rows: key → (rid, cells by leaf id in ordinal order).
type Rows = BTreeMap<u64, (u64, Vec<(u32, V)>)>;

/// A digest of host calls, folded as they are taken so long runs keep no log.
#[derive(Default)]
struct LogHash(std::collections::hash_map::DefaultHasher, u64);

impl LogHash {
    fn take(&mut self, h: &mut VecHost) {
        for c in h.take_log() {
            self.1 += 1;
            let r = |r: Region| match r {
                Region::Block(b) => (1u64, b),
                Region::Page(p) => (2u64, p),
            };
            match c {
                Call::Get(a) => (0u8, r(a), (0, 0)).hash(&mut self.0),
                Call::GetMut(a) => (1u8, r(a), (0, 0)).hash(&mut self.0),
                Call::PairMut(a, b) => (2u8, r(a), r(b)).hash(&mut self.0),
                Call::ReadWrite(a, b) => (3u8, r(a), r(b)).hash(&mut self.0),
                Call::PageAlloc(p) => (4u8, (p, 0), (0, 0)).hash(&mut self.0),
                Call::PageFree(p) => (5u8, (p, 0), (0, 0)).hash(&mut self.0),
            }
        }
    }
    fn finish(&self) -> (u64, u64) {
        (self.0.finish(), self.1)
    }
}

/// Runs `run` once per fill pattern on a fresh host; the results and the host-call logs must be equal.
fn per_fill<T: PartialEq + std::fmt::Debug>(run: impl Fn(&mut VecHost, &mut LogHash) -> T) -> T {
    let mut out: Vec<(T, (u64, u64))> = FILLS
        .iter()
        .map(|&f| {
            let mut h = VecHost::new(f);
            let mut lh = LogHash::default();
            let r = run(&mut h, &mut lh);
            lh.take(&mut h);
            (r, lh.finish())
        })
        .collect();
    let b = out.pop().unwrap();
    let a = out.pop().unwrap();
    assert_eq!(a.0, b.0, "results differ between fill patterns");
    assert_eq!(a.1, b.1, "host-call logs differ between fill patterns");
    a.0
}

fn id(col: u32) -> u32 {
    col << 16
}

fn everything() -> TestBudget {
    TestBudget::new(u64::MAX)
}

/// The maintenance steps the tests run, counted by what they ran.
#[derive(Default, Debug, Clone, PartialEq)]
struct Units {
    by: BTreeMap<String, u64>,
}

/// Runs one maintenance step, counting it.
fn one_unit(tb: &mut Table<'_, VecHost>, units: &mut Units) -> Ran {
    let ran = tb.step().unwrap_or_else(|e| panic!("step failed: {e:?}"));
    if ran != Ran::Nothing {
        *units.by.entry(format!("{ran:?}")).or_default() += 1;
    }
    ran
}

/// Runs steps until nothing is pending, counting each.
fn run_all(tb: &mut Table<'_, VecHost>, units: &mut Units) -> u64 {
    let mut n = 0;
    while one_unit(tb, units) != Ran::Nothing {
        n += 1;
        assert!(n < 10_000_000, "maintenance ends");
    }
    assert_eq!(tb.wake().unwrap(), Wake::Idle);
    n
}

fn maintain_all(tb: &mut Table<'_, VecHost>) {
    assert_eq!(tb.maintain(&everything()).unwrap(), Progress::Done);
    assert_eq!(tb.wake().unwrap(), Wake::Idle);
}

/// Reads a row by key; values handed out as long are read back through `read_large`.
fn get_row(tb: &Table<'_, VecHost>, k: u64) -> Option<(u64, Vec<(u32, V)>)> {
    let mut s = Sink::default();
    if !tb.get(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))]), &mut s).unwrap() {
        return None;
    }
    let rid = s.rid.unwrap();
    Some((rid, fetch_long(tb, rid, s.cells)))
}

fn fetch_long(tb: &Table<'_, VecHost>, rid: u64, cells: Vec<(u32, V)>) -> Vec<(u32, V)> {
    cells
        .into_iter()
        .map(|(l, v)| match v {
            V::Arg(mut buf) => {
                let mut at = 0;
                while at < buf.len() {
                    let end = (at + 40_000).min(buf.len());
                    let n = tb.read_large(rid, l, at as u64, &mut buf[at..end]).unwrap();
                    assert!(n > 0);
                    at += n;
                }
                let o = tb.shape().ordinal_of(tb.host(), l).unwrap().unwrap();
                match tb.shape().leaf(tb.host(), o).unwrap().ty {
                    Ty::Nat => (l, V::Nat(buf)),
                    Ty::Int => (l, V::Int(buf)),
                    _ => (l, V::Bytes(buf)),
                }
            }
            other => (l, other),
        })
        .collect()
}

/// The sealed groups of a table: (group, page).
fn sealed_groups(tb: &Table<'_, VecHost>) -> Vec<(u64, u64)> {
    let next = root::u64_at(tb.host(), tb.root(), root::NEXT_GROUP).unwrap();
    (0..next)
        .filter_map(|g| {
            let e = root::group(tb.host(), tb.root(), g).unwrap();
            (e != 0 && e & buffer::DESC_BIT == 0).then_some((g, e))
        })
        .collect()
}

/// The drain's records, over its blocks.
fn drain_records(tb: &Table<'_, VecHost>) -> u64 {
    let mut n = 0;
    let mut b = root::u64_at(tb.host(), tb.root(), root::DRAIN_TOP).unwrap();
    while b != 0 {
        let s = tb.host().get(Region::Block(b)).unwrap();
        n += u64::from(rd_u16(s, 6).unwrap());
        b = rd_u64(s, 8).unwrap();
    }
    n
}

/// The drain's records and its top record's bytes: a drain step that moves on within a record (a walk's
/// position) changes the second, one that pops or pushes the first.
fn drain_state(tb: &Table<'_, VecHost>) -> (u64, Vec<u8>) {
    let top = root::u64_at(tb.host(), tb.root(), root::DRAIN_TOP).unwrap();
    let rec = if top == 0 {
        Vec::new()
    } else {
        let s = tb.host().get(Region::Block(top)).unwrap();
        let n = u64::from(rd_u16(s, 6).unwrap());
        let at = (32 + (n - 1) * 32) as usize;
        s[at..at + 32].to_vec()
    };
    (drain_records(tb), rec)
}

/// The groups sealed so far: the write that fills the open group seals it.
fn seals(tb: &Table<'_, VecHost>) -> u64 {
    open_group(tb.host(), tb.root())
}

// ---------------------------------------------------------------- checks

/// Every sealed group: its live count against its live rows, each of which reads back. Returns the groups
/// checked.
fn check_groups(tb: &Table<'_, VecHost>) -> u64 {
    let h = tb.host();
    let mut checked = 0;
    for (g, pg) in sealed_groups(tb) {
        checked += 1;
        let p = buffer::u16_of(h, pg, page::P_POSITIONS).unwrap();
        let mut live = 0u64;
        for pos in 0..p {
            let mut s = Sink::default();
            if tb.get_rid(g * GROUP_SLOTS + pos, &mut s).unwrap() {
                live += 1;
            }
        }
        assert_eq!(buffer::u16_of(h, pg, page::P_LIVE).unwrap(), live, "group {g} live rows");
    }
    checked
}

/// The integer leaves' totals over every live row, and the buffered live count over the buffered rows.
fn check_running(tb: &Table<'_, VecHost>, model: &Rows) {
    let h = tb.host();
    let tbl = tb.root();
    let shape = *tb.shape();
    let tail = root::u64_at(h, tbl, root::TAIL).unwrap();
    // The buffered live count: the model's rows that lie in a buffer.
    let buffered = model.values().filter(|(rid, _)| buffer::locate(h, tbl, *rid).unwrap().is_some()).count() as u64;
    assert_eq!(buffer::u64_of(h, tail, buffer::H_BUFFERED).unwrap(), buffered, "buffered live rows");
    for o in 0..shape.leaves {
        let l = shape.leaf(h, o).unwrap();
        if l.live() && l.ty.running().is_some() {
            check_total(tb, &l, model);
            check_extremes(tb, &l, model);
        }
    }
}

/// An integer leaf's smallest and largest value over every live row.
fn check_extremes(tb: &Table<'_, VecHost>, l: &native_table::table::layout::LeafInfo, model: &Rows) {
    use std::cmp::Ordering;
    let mut want: Option<(Big, Big)> = None;
    for (_, row) in model.values() {
        let Some(v) = total_value(&row.iter().find(|(x, _)| *x == l.id).unwrap().1) else { continue };
        want = Some(match want {
            None => (v.clone(), v),
            Some((lo, hi)) => {
                let lo = if v.order(&lo) == Ordering::Less { v.clone() } else { lo };
                let hi = if v.order(&hi) == Ordering::Greater { v } else { hi };
                (lo, hi)
            }
        });
    }
    let mut buf = vec![0u8; 1 << 17];
    for largest in [false, true] {
        let got = tb.extreme(l.id, largest, &mut buf).unwrap().map(|t| Big::of_total(t, &buf));
        let w = want.as_ref().map(|(lo, hi)| if largest { hi.clone() } else { lo.clone() });
        assert_eq!(got, w, "leaf {}: largest {largest}", l.ordinal);
    }
}

/// An integer leaf's total, wherever it lies: its sum and non-null count over every live row.
fn check_total(tb: &Table<'_, VecHost>, l: &native_table::table::layout::LeafInfo, model: &Rows) {
    let (mut want, mut n) = (Big::default(), 0u64);
    for (_, row) in model.values() {
        let v = match &row.iter().find(|(x, _)| *x == l.id).unwrap().1 {
            V::Null => continue,
            v => total_value(v).unwrap_or_else(|| panic!("leaf {}: {v:?}", l.ordinal)),
        };
        want = want.add(&v);
        n += 1;
    }
    let mut buf = vec![0u8; 1 << 17];
    let (got, gn) = tb.sum(l.id, &mut buf).unwrap();
    assert_eq!((&Big::of_total(got, &buf), gn), (&want, n), "total of leaf {}", l.ordinal);
}

/// An integer value as a total takes it; `None` for any other value.
fn total_value(v: &V) -> Option<Big> {
    match v {
        V::U(x) => Some(Big::from_i128(i128::from(*x))),
        V::I(x) => Some(Big::from_i128(i128::from(*x))),
        V::Nat(b) => Some(Big::from_leb(b, false)),
        V::Int(b) => Some(Big::from_leb(b, true)),
        V::U128(x) => Some(Big::from_u128(*x)),
        V::I128(x) => Some(Big::from_i128(*x)),
        _ => None,
    }
}

/// Every page of area 15 belongs to exactly one image: the schema image, a sealed group's image, a long value
/// named once by a buffered row, a group's long-value list or a patch entry. Returns the long values found.
fn check_ownership(tb: &Table<'_, VecHost>) -> usize {
    let h = tb.host();
    let tbl = tb.root();
    let shape = *tb.shape();
    let mut pages = BTreeSet::new();
    let mut add = |img: &native_table::image::Image| {
        for k in 0..img.block_count() {
            let p = img.entry(h, k).unwrap();
            assert!(pages.insert(p), "page {p} belongs to two images");
        }
    };
    let sp = root::u64_at(h, tbl, root::SCHEMA_PAGE).unwrap();
    add(&native_table::image::Image {
        length: 1,
        blocks: native_table::image::Blocks::Direct { count: 1, entries: [sp, 0, 0, 0, 0, 0, 0, 0] },
        believed: false,
    });
    let mut longs = 0;
    let patch_longs = |dir: u64, add: &mut dyn FnMut(&native_table::image::Image)| {
        patch::for_each_unit(h, dir, |u| {
            let mut b = u.first;
            let mut first = true;
            while b != 0 {
                let s = h.get(Region::Block(b)).unwrap();
                let n = rd_u16(s, patch::B_ENTRIES).unwrap() as u64;
                let base = if first { patch::FIRST_ENTRIES } else { patch::LATER_ENTRIES };
                for i in 0..n {
                    let at = base + i * patch::ENTRY;
                    let flags = s[at as usize + 2];
                    if flags & patch::F_DEAD == 0 && flags & patch::F_LONG != 0 {
                        let data = rd_u16(s, at + 6).unwrap() as u64;
                        if s[data as usize..data as usize + 24].iter().any(|&x| x != 0) {
                            add(&native_table::image::Image::read_ref(s, data).unwrap());
                        }
                    }
                }
                b = rd_u64(s, patch::B_NEXT).unwrap();
                first = false;
            }
            Ok(())
        })
        .unwrap();
    };
    for (_, pg) in sealed_groups(tb) {
        let gv = GroupView::open(h, pg).unwrap();
        add(&gv.img);
        let n = u64::from(rd_u32(h.get(Region::Block(pg)).unwrap(), page::P_LV_COUNT).unwrap());
        for i in 0..n {
            if let Some(img) = page::lv_get(h, pg, i).unwrap() {
                add(&img);
                longs += 1;
            }
        }
        let dir = buffer::u64_of(h, pg, page::P_PATCH_DIR).unwrap();
        patch_longs(dir, &mut add);
    }
    {
        let d = buffer::open_group(h, tbl).unwrap().1;
        buffer::for_each_slot(h, d, |_, b, s| {
            let Some(loc) = buffer::slot_row(h, b, s)? else { return Ok(()) };
            if let buffer::RowLoc::Long(img) = loc {
                add(&img);
                longs += 1;
            }
            let row = native_table::table::row::RowRef::new(h, loc)?;
            let head = row.head(&shape)?;
            for vi in 0..head.vars {
                if let native_table::table::row::Item::Long { img, .. } = row.item_at(&head, vi)? {
                    add(&img);
                    longs += 1;
                }
            }
            Ok(())
        })
        .unwrap();
    }
    let rec15 = native_table::area::AreaRecord::read(h, tbl, native_table::area::IMAGE_AREA).unwrap();
    assert_eq!(rec15.pages, pages.len() as u64, "area 15 counts exactly the pages its images hold");
    longs
}

// ---------------------------------------------------------------- the model's rows

/// Columns of the model row: key Nat; Text; Bool; i64; ?u8; u32 of three values; Float; Principal; [u8; 4];
/// Int; u128; Char; (); ?Text; Blob; u16.
struct Model {
    img: Vec<u8>,
}

fn model_cols() -> Vec<Vec<u8>> {
    vec![
        t::scalar(ty::NAT),
        t::scalar(ty::TEXT),
        t::scalar(ty::BOOL),
        t::scalar(ty::I64),
        t::opt(t::scalar(ty::U8)),
        t::scalar(ty::U32),
        t::scalar(ty::F64),
        t::scalar(ty::PRINCIPAL),
        t::bytes(4),
        t::scalar(ty::INT),
        t::scalar(ty::U128),
        t::scalar(ty::CHAR),
        t::scalar(ty::UNIT),
        t::opt(t::scalar(ty::TEXT)),
        t::scalar(ty::BLOB),
        t::scalar(ty::U16),
    ]
}

fn model_schema() -> Model {
    let (img, _) = schema(&model_cols());
    Model { img }
}

fn text(r: &mut XorShift, n: usize) -> Vec<u8> {
    let mut s = String::new();
    while s.len() < n {
        match r.below(10) {
            0 => s.push('é'),
            1 => s.push('€'),
            2 => s.push('😀'),
            _ => s.push((b'a' + r.below(26) as u8) as char),
        }
    }
    s.into_bytes()
}

/// The canonical signed LEB128 of an i128.
fn sleb(mut x: i128) -> Vec<u8> {
    let mut v = Vec::new();
    loop {
        let b = (x & 0x7F) as u8;
        x >>= 7;
        let done = (x == 0 && b & 0x40 == 0) || (x == -1 && b & 0x40 != 0);
        v.push(if done { b } else { b | 0x80 });
        if done {
            return v;
        }
    }
}

/// A random value of model column c.
fn model_cell(r: &mut XorShift, c: u32, big: bool) -> V {
    match c {
        1 => {
            let n = match r.below(100) {
                0 if big => (HUGE_VALUE as usize) + 1 + r.below(3_000) as usize,
                1..=3 => LONG_VALUE as usize + 1 + r.below(1_500) as usize,
                4..=40 => 3 + r.below(3) as usize,
                _ => r.below(60) as usize,
            };
            V::Bytes(text(r, n))
        }
        2 => V::Bool(r.below(2) == 0),
        3 => V::I(r.next_u64() as i64 >> r.below(64)),
        4 => {
            if r.below(4) == 0 {
                V::Null
            } else {
                V::U(r.below(256))
            }
        }
        5 => V::U(r.below(3)),
        6 => V::F64((r.below(2_000) as f64 / 7.0 - 100.0).to_bits()),
        7 => V::Bytes((0..r.below(30)).map(|_| r.next_u64() as u8).collect()),
        8 => V::Bytes((0..4).map(|_| r.next_u64() as u8).collect()),
        9 => match r.below(10) {
            0 => V::Int(sleb(i128::from(r.next_u64()) << 70)),
            1 => V::Int(sleb(i128::from(r.next_u64()) << 40)),
            _ => V::Int(sleb(i128::from(r.next_u64() as i64 >> r.below(60)))),
        },
        10 => V::U128(u128::from(r.next_u64()) << r.below(64)),
        11 => V::U(char::from_u32(0x20 + r.below(0xD000) as u32).map_or(0x41, u32::from) as u64),
        12 => V::Unit,
        13 => {
            if r.below(3) == 0 {
                V::Null
            } else {
                let n = r.below(20) as usize;
                V::Bytes(text(r, n))
            }
        }
        14 => V::Bytes((0..r.below(40)).map(|_| r.next_u64() as u8).collect()),
        _ => {
            let top = if r.below(3) == 0 { 65_536 } else { 300 };
            V::U(r.below(top))
        }
    }
}

fn model_row(r: &mut XorShift, k: u64, big: bool) -> Vec<(u32, V)> {
    let mut row = vec![(id(0), V::U(k))];
    for c in 1..16u32 {
        row.push((id(c), model_cell(r, c, big)));
    }
    row
}

/// The value a read hands back for a value written as `v`: a Nat or Int in its smallest cell.
fn read_form(v: &V, col_ty: Ty) -> V {
    match (v, col_ty) {
        (V::Int(b), Ty::Int) => {
            let mut x: i128 = 0;
            let mut shift = 0;
            let mut fits = b.len() <= 18;
            for &byte in b {
                if shift < 128 {
                    x |= i128::from(byte & 0x7F) << shift;
                }
                shift += 7;
            }
            if fits {
                if shift < 128 && b.last().unwrap() & 0x40 != 0 {
                    x |= -1i128 << shift;
                }
                fits = i64::try_from(x).is_ok();
            }
            if fits {
                V::I(x as i64)
            } else {
                v.clone()
            }
        }
        (V::Arg(b), _) => V::Bytes(b.clone()),
        _ => v.clone(),
    }
}

fn expected(tb: &Table<'_, VecHost>, row: &[(u32, V)]) -> Vec<(u32, V)> {
    let shape = tb.shape();
    row.iter()
        .map(|(l, v)| {
            let o = shape.ordinal_of(tb.host(), *l).unwrap().unwrap();
            (*l, read_form(v, shape.leaf(tb.host(), o).unwrap().ty))
        })
        .collect()
}

/// Every model row reads back.
fn check_reads(tb: &Table<'_, VecHost>, model: &Rows) {
    for (k, (rid, row)) in model {
        let (grid, cells) = get_row(tb, *k).unwrap_or_else(|| panic!("key {k} present"));
        assert_eq!(grid, *rid, "key {k} keeps its rid");
        assert_eq!(cells, expected(tb, row), "key {k} reads back");
    }
    assert_eq!(tb.len().unwrap(), model.len() as u64);
}

// ---------------------------------------------------------------- rewrites

/// Upserts `row`, keeping the model: a new key gets a new rid, an existing one keeps its rid.
fn put_row(tb: &mut Table<'_, VecHost>, model: &mut Rows, row: Vec<(u32, V)>) {
    let k = match row[0].1 {
        V::U(k) => k,
        _ => unreachable!(),
    };
    match tb.upsert(&mut key_scratch(), &mut Src::new(&row)).unwrap().0 {
        Upsert::Inserted(rid) => {
            assert!(!model.contains_key(&k));
            model.insert(k, (rid, row));
        }
        Upsert::Replaced(rid) => {
            assert_eq!(model[&k].0, rid, "an update keeps the rid");
            model.insert(k, (rid, row));
        }
    }
}

/// One random write to the model's rows: an update of one or more leaves, a delete, or an insert.
fn random_write(tb: &mut Table<'_, VecHost>, r: &mut XorShift, model: &mut Rows, next_key: &mut u64, big: bool) {
    let op = r.below(10);
    if op < 6 && !model.is_empty() {
        let k = *model.keys().nth(r.below(model.len() as u64) as usize).unwrap();
        let mut row = model[&k].1.clone();
        for _ in 0..1 + r.below(3) {
            let c = 1 + r.below(15) as u32;
            row[c as usize].1 = model_cell(r, c, big);
        }
        put_row(tb, model, row);
    } else if op < 8 && !model.is_empty() {
        let k = *model.keys().nth(r.below(model.len() as u64) as usize).unwrap();
        let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
        assert_eq!(gone, model.remove(&k).map(|x| x.0));
    } else {
        let row = model_row(r, *next_key, big);
        *next_key += 1;
        put_row(tb, model, row);
    }
}

/// The large suite (`NT_LARGE` set; scripts/check.sh runs it in release mode): `rows_model` at 10^6 writes and
/// `rows_model_versions` at 10,000.
fn large() -> bool {
    std::env::var_os("NT_LARGE").is_some()
}

/// Fills the open group until the write that fills it seals it.
fn fill_group(tb: &mut Table<'_, VecHost>, r: &mut XorShift, model: &mut Rows, next_key: &mut u64) {
    let before = seals(tb);
    while seals(tb) == before {
        let row = model_row(r, *next_key, true);
        *next_key += 1;
        put_row(tb, model, row);
    }
}

/// A sealed group's image, whole, with its group number (header @32) zeroed.
fn group_image(tb: &Table<'_, VecHost>, pg: u64) -> Vec<u8> {
    let h = tb.host();
    let gv = GroupView::open(h, pg).unwrap();
    let r = gv.reader();
    let mut out = Vec::new();
    let mut at = 0;
    while at < r.length() {
        let n = (native_table::consts::PAGE - at % native_table::consts::PAGE).min(r.length() - at);
        out.extend_from_slice(r.bytes(at, n).unwrap());
        at += n;
    }
    out[32..40].fill(0);
    out
}

/// A leaf whose cells lie in many segments: a nullable 200-byte array over a group of some 3,600 rows takes
/// a dozen image blocks of cells, which the seal lays out, and its compaction again; every value reads back.
#[test]
fn many_segments() {
    let img = schema(&[t::scalar(ty::NAT), t::opt(t::bytes(200)), t::scalar(ty::U16)]).0;
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 29, &img).unwrap();
        let mut model = BTreeMap::new();
        let mut units = Units::default();
        let cell = |k: u64| -> V {
            if k % 7 == 0 {
                V::Null
            } else {
                V::Bytes((0..200u64).map(|i| (k.wrapping_mul(31) + i) as u8).collect())
            }
        };
        let mut k = 0u64;
        while seals(&tb) == 0 {
            put_row(&mut tb, &mut model, vec![(id(0), V::U(k)), (id(1), cell(k)), (id(2), V::U(k % 1_000))]);
            k += 1;
        }
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        let segs = |tb: &Table<'_, VecHost>| -> u32 {
            let (_, pg) = sealed_groups(tb)[0];
            let gv = GroupView::open(tb.host(), pg).unwrap();
            gv.leaf_by_ordinal(1).unwrap().unwrap().segs
        };
        let sealed = segs(&tb);
        assert!(sealed > 4, "the leaf lies in {sealed} segments");
        // A third of the rows deleted: the compaction lays the leaf out again.
        for key in (0..k).filter(|x| x % 3 == 1) {
            tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(key))])).unwrap();
            model.remove(&key);
        }
        let pg = sealed_groups(&tb)[0].1;
        let tbl = tb.root();
        native_table::table::jobs::queue(tb.host_mut(), tbl, 0, pg, false).unwrap();
        run_all(&mut tb, &mut units);
        assert!(units.by.keys().any(|x| x.starts_with("Compaction")), "{units:?}");
        check_reads(&tb, &model);
        check_groups(&tb);
        (sealed, segs(&tb), model.len())
    });
    eprintln!("many_segments: {digest:?}");
}

/// One rule for the compacted form, and one image for the same live rows: a seal of a group whose rows were
/// deleted before the insert that filled it, and a compaction of the same rows sealed with other values,
/// updated to the first table's values (in place and as patches) and deleted after the seal, give the same
/// bytes, the group number aside. Dead positions below, at and above `COMPACT_DEAD`.
#[test]
fn seal_and_compaction_bytes() {
    let m = model_schema();
    let digest = per_fill(|h, _| {
        let mut out = Vec::new();
        for dead in [37u64, COMPACT_DEAD - 1, COMPACT_DEAD, COMPACT_DEAD + 500] {
            let mut units = Units::default();
            // The final rows, and the group's positions p: every row but the one whose insert seals the group,
            // found on a table of its own.
            let mut r = XorShift(0xB17E ^ dead);
            let mut finals: Vec<Vec<(u32, V)>> = Vec::new();
            let p = {
                let mut h2 = VecHost::new(FILLS[0]);
                let (mut tb, _) = Table::create(&mut h2, 41, &m.img).unwrap();
                while seals(&tb) == 0 {
                    finals.push(model_row(&mut r, finals.len() as u64, true));
                    tb.upsert(&mut key_scratch(), &mut Src::new(finals.last().unwrap())).unwrap();
                }
                finals.len() as u64 - 1
            };
            assert!(p > dead + 100, "the group holds {p} rows");
            // The dead keys: `dead` of the group's keys, spread over it.
            let mut order: Vec<u64> = (0..p).collect();
            order.sort_by_key(|x| (x * 7_919) % p);
            let doomed: Vec<u64> = order[..dead as usize].to_vec();
            let kill = |tb: &mut Table<'_, VecHost>, model: &mut Rows| {
                for key in &doomed {
                    tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(*key))])).unwrap();
                    model.remove(key);
                }
            };
            let images: Vec<Vec<u8>> = (0..2)
                .map(|pass| {
                    // Both tables in the one host, one after the other, with the same uid.
                    let (mut tb, _) = Table::create(&mut *h, 41, &m.img).unwrap();
                    let mut model = BTreeMap::new();
                    for k in 0..=p {
                        if pass == 0 && k == p {
                            // The dead rows deleted before the insert that seals the group.
                            kill(&mut tb, &mut model);
                        }
                        let mut row = finals[k as usize].clone();
                        if pass == 1 && k % 3 == 0 {
                            // Other values first: a u32, a Bool, an i64 and a Text.
                            row[5].1 = V::U((k / 3) % 3);
                            row[2].1 = V::Bool(k % 2 == 0);
                            row[3].1 = V::I(-(k as i64));
                            // A Text of the same length, so the same row fills the group.
                            if let V::Bytes(b) = &row[1].1 {
                                row[1].1 = V::Bytes(vec![b'q'; b.len()]);
                            }
                        }
                        put_row(&mut tb, &mut model, row);
                    }
                    assert_eq!(seals(&tb), 1, "the last insert sealed the group");
                    if pass == 0 {
                        run_all(&mut tb, &mut units);
                    } else {
                        run_all(&mut tb, &mut units);
                        for key in (0..p).filter(|x| x % 3 == 0) {
                            put_row(&mut tb, &mut model, finals[key as usize].clone());
                        }
                        kill(&mut tb, &mut model);
                        let pg = root::group(tb.host(), tb.root(), 0).unwrap();
                        let tbl = tb.root();
                        native_table::table::jobs::queue(tb.host_mut(), tbl, 0, pg, false).unwrap();
                        run_all(&mut tb, &mut units);
                        assert!(units.by.keys().any(|x| x.starts_with("Compaction")), "{units:?}");
                    }
                    check_reads(&tb, &model);
                    check_groups(&tb);
                    check_running(&tb, &model);
                    let pg = root::group(tb.host(), tb.root(), 0).unwrap();
                    let flags = buffer::u16_of(tb.host(), pg, page::P_FLAGS).unwrap();
                    assert_eq!(
                        flags & u64::from(page::PF_COMPACTED) != 0,
                        dead >= COMPACT_DEAD,
                        "{dead} dead positions: the compacted form by the one rule"
                    );
                    group_image(&tb, pg)
                })
                .collect();
            assert_eq!(images[0].len(), images[1].len(), "{dead} dead positions: image lengths");
            if let Some(i) = images[0].iter().zip(&images[1]).position(|(a, b)| a != b) {
                panic!("{dead} dead positions: the seal's and the compaction's images differ at byte {i}");
            }
            out.push((dead, images[0].len()));
        }
        out
    });
    eprintln!("seal_and_compaction_bytes: {digest:?}");
}

/// A Nat of `len` LEB128 bytes: every two of the same length share all but their lowest group.
fn big_nat(k: u64, len: usize) -> Vec<u8> {
    let mut v = vec![0xFFu8; len];
    v[0] = 0x80 | (k % 128) as u8;
    v[len - 1] = 0x7F;
    v
}

/// An Int of `len` signed LEB128 bytes: every 7-bit group 0x7F but the first, which carries k; negative when
/// `neg` (the top group's sign bit set).
fn big_int(k: u64, len: usize, neg: bool) -> Vec<u8> {
    let mut v = vec![0xFFu8; len];
    v[0] = 0x80 | (k % 64) as u8;
    v[len - 1] = if neg { 0x40 } else { 0x3F };
    v
}

/// Values of 60,000 signed LEB128 bytes (the longest a row keeps inline as a medium value), alternating in
/// sign: a seal, then a compaction of half the rows; each time the rows read back and the Int leaf's total is
/// the live rows' exact sum.
#[test]
fn alternating_leb() {
    let img = schema(&[t::scalar(ty::NAT), t::scalar(ty::INT), t::scalar(ty::I64)]).0;
    let len = 60_000usize;
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 26, &img).unwrap();
        let mut units = Units::default();
        let mut model = BTreeMap::new();
        let mut n = 0u64;
        while seals(&tb) == 0 {
            put_row(
                &mut tb,
                &mut model,
                vec![(id(0), V::U(n)), (id(1), V::Int(big_int(n, len, n % 2 == 1))), (id(2), V::I(n as i64))],
            );
            n += 1;
        }
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_running(&tb, &model);
        let keys: Vec<u64> = model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == 0).map(|(k, _)| *k).collect();
        for k in keys.iter().filter(|k| *k % 2 == 0) {
            tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(*k))])).unwrap();
            model.remove(k);
        }
        let pg = root::group(tb.host(), tb.root(), 0).unwrap();
        let tbl = tb.root();
        native_table::table::jobs::queue(tb.host_mut(), tbl, 0, pg, false).unwrap();
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_running(&tb, &model);
        (units, model.len())
    });
    eprintln!("alternating_leb: {digest:?}");
}

/// A row of wide values: key Nat; 64 KiB-class medium Text values.
fn medium_schema() -> Vec<u8> {
    schema(&[t::scalar(ty::NAT), t::scalar(ty::TEXT), t::scalar(ty::I64)]).0
}

/// Rows of 64 KiB-class medium values until the group is full at its out-of-line bytes, sealed by the insert
/// that fills it; then every sealed row updated to another medium value (patches in chains), folded by the
/// compaction.
#[test]
fn medium_values() {
    let img = medium_schema();
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 22, &img).unwrap();
        let mut units = Units::default();
        let mut model = BTreeMap::new();
        let medium = |k: u64, v: u64| -> Vec<u8> {
            vec![b'a' + ((k + v) % 26) as u8; HUGE_VALUE as usize - 64 - (k % 7) as usize]
        };
        let mut n = 0u64;
        while seals(&tb) == 0 {
            put_row(
                &mut tb,
                &mut model,
                vec![(id(0), V::U(n)), (id(1), V::Bytes(medium(n, 0))), (id(2), V::I(n as i64))],
            );
            n += 1;
        }
        let sealed_rows: Vec<u64> =
            model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == 0).map(|(k, _)| *k).collect();
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        for &k in &sealed_rows {
            put_row(
                &mut tb,
                &mut model,
                vec![(id(0), V::U(k)), (id(1), V::Bytes(medium(k, 2))), (id(2), V::I(k as i64))],
            );
        }
        check_reads(&tb, &model);
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_ownership(&tb);
        units
    });
    eprintln!("medium_values: {digest:?}");
}

// ---------------------------------------------------------------- growing updates

fn grow_schema() -> Vec<u8> {
    schema(&[t::scalar(ty::NAT), t::scalar(ty::TEXT), t::scalar(ty::I64), t::scalar(ty::TEXT)]).0
}

fn grow_row(k: u64, a: usize, b: usize) -> Vec<(u32, V)> {
    vec![
        (id(0), V::U(k)),
        (id(1), V::Bytes(vec![b'a' + (k % 26) as u8; a])),
        (id(2), V::I(k as i64 * 3)),
        (id(3), V::Bytes(vec![b'z'; b])),
    ]
}

/// The flags of the slot of buffered row `rid`.
fn slot_flags(tb: &Table<'_, VecHost>, rid: u64) -> u8 {
    let at = buffer::locate(tb.host(), tb.root(), rid).unwrap().unwrap();
    buffer::read_slot(tb.host().get(Region::Block(at.block)).unwrap(), at.slot).unwrap().flags
}

#[test]
fn growing_updates() {
    let img = grow_schema();
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 23, &img).unwrap();
        let mut model = BTreeMap::new();
        let mut log = Vec::new();
        // The open group: alternating lengths, in place, moved to the tail block, and long row images.
        for k in 0..200u64 {
            put_row(&mut tb, &mut model, grow_row(k, 4, 4));
        }
        let open = root::u64_at(tb.host(), tb.root(), root::OPEN_GROUP).unwrap();
        let next = root::u64_at(tb.host(), tb.root(), root::NEXT_GROUP).unwrap();
        // Each update's image goes in place when not longer, to the tail block's room when longer, else to a
        // long row image; images past the inline maximum are long row images whatever their room.
        let mut flags = BTreeSet::new();
        for round in 0..6usize {
            for k in (0..200u64).step_by(3) {
                let (a, b) = match (round + k as usize / 3) % 3 {
                    0 => (4, 4),
                    1 => (300, 40),
                    _ => (2_500, 1_200),
                };
                put_row(&mut tb, &mut model, grow_row(k, a, b));
                flags.insert(slot_flags(&tb, model[&k].0));
            }
            check_reads(&tb, &model);
        }
        assert!(flags.contains(&0), "images rewritten in place: {flags:?}");
        assert!(flags.contains(&buffer::SLOT_MOVED), "images moved to the tail block: {flags:?}");
        assert!(flags.contains(&buffer::SLOT_LONG), "long row images: {flags:?}");
        assert_eq!(root::u64_at(tb.host(), tb.root(), root::OPEN_GROUP).unwrap(), open, "no update freezes");
        assert_eq!(root::u64_at(tb.host(), tb.root(), root::NEXT_GROUP).unwrap(), next);
        log.push(flags.len());
        // An open group with a full descriptor: rows of about 300 bytes until its 240th block; updates that
        // grow then go to the tail block's room or to long row images, never to a new block.
        let mut k = 1_000u64;
        loop {
            let (_, desc, _) = buffer::open_group(tb.host(), tb.root()).unwrap();
            if buffer::u16_of(tb.host(), desc, buffer::D_BLOCKS).unwrap() >= 240 || seals(&tb) > 0 {
                break;
            }
            put_row(&mut tb, &mut model, grow_row(k, 260, 4));
            k += 1;
        }
        let (_, desc, _) = buffer::open_group(tb.host(), tb.root()).unwrap();
        let blocks = buffer::u16_of(tb.host(), desc, buffer::D_BLOCKS).unwrap();
        let seals_before = seals(&tb);
        for kk in (1_000..k).step_by(7) {
            put_row(&mut tb, &mut model, grow_row(kk, 900, 300));
        }
        let (_, desc2, _) = buffer::open_group(tb.host(), tb.root()).unwrap();
        assert_eq!(desc2, desc, "no update freezes the open group");
        assert_eq!(
            buffer::u16_of(tb.host(), desc, buffer::D_BLOCKS).unwrap(),
            blocks,
            "no update takes a block record"
        );
        assert_eq!(seals(&tb), seals_before);
        check_reads(&tb, &model);
        // The group filled and sealed: the seal reads the moved images and the long row images.
        while seals(&tb) == 0 {
            put_row(&mut tb, &mut model, grow_row(k, 8, 8));
            k += 1;
        }
        check_reads(&tb, &model);
        maintain_all(&mut tb);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_ownership(&tb);
        log.push(sealed_groups(&tb).len());
        log
    });
    eprintln!("growing_updates: {digest:?}");
}

// ---------------------------------------------------------------- patch units

/// A key Nat and `n` u8 leaves.
fn many_schema(n: usize) -> Vec<u8> {
    let mut cols = vec![t::scalar(ty::NAT)];
    cols.extend((0..n).map(|_| t::scalar(ty::U8)));
    schema(&cols).0
}

fn many_row(n: usize, k: u64, vals: &BTreeMap<usize, u64>) -> Vec<(u32, V)> {
    let mut r = vec![(id(0), V::U(k))];
    r.extend((0..n).map(|c| (id(c as u32 + 1), V::U(vals.get(&c).copied().unwrap_or(0)))));
    r
}

/// The directory blocks of a group's patch directory.
fn dir_blocks(h: &VecHost, pg: u64) -> u64 {
    let mut b = buffer::u64_of(h, pg, page::P_PATCH_DIR).unwrap();
    let mut n = 0;
    while b != 0 {
        n += 1;
        b = rd_u64(h.get(Region::Block(b)).unwrap(), patch::D_LINK + 8).unwrap();
    }
    n
}

#[test]
fn patch_units_over_255() {
    let n = 620usize;
    let img = many_schema(n);
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 24, &img).unwrap();
        let mut model = BTreeMap::new();
        let mut k = 0u64;
        while seals(&tb) == 0 {
            put_row(&mut tb, &mut model, many_row(n, k, &BTreeMap::new()));
            k += 1;
        }
        maintain_all(&mut tb);
        let (_, pg) = sealed_groups(&tb)[0];
        let rows: Vec<u64> = model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == 0).map(|(k, _)| *k).collect();
        // Every u8 leaf holds 0 in every row: a `Seq` constant, so every update is a patch. 600 distinct
        // units, one row each.
        let mut vals: BTreeMap<u64, BTreeMap<usize, u64>> = BTreeMap::new();
        let mut dirs = Vec::new();
        for unit in 0..600usize {
            let key = rows[unit % rows.len()];
            let v = vals.entry(key).or_default();
            v.insert(unit, 1 + (unit as u64 % 200));
            let row = many_row(n, key, v);
            tb.host_mut().end_message();
            put_row(&mut tb, &mut model, row);
            let changed = tb.host_mut().end_message();
            assert_eq!(root::group(tb.host(), tb.root(), 0).unwrap(), pg, "no write compacts");
            let d = dir_blocks(tb.host(), pg);
            dirs.push(d);
            let units = patch::units(tb.host(), buffer::u64_of(tb.host(), pg, page::P_PATCH_DIR).unwrap()).unwrap();
            assert_eq!(units, unit as u64 + 1);
            let want = (unit as u64) / PATCH_UNITS_PER_DIR + 1;
            assert_eq!(d, want, "unit {} takes directory block {want}", unit + 1);
            assert!(changed <= 8, "a patch write changes few blocks: {changed}");
        }
        assert_eq!(dirs[253], 1);
        assert_eq!(dirs[254], 2, "the 255th unit takes a new directory block");
        assert_eq!(dirs[507], 2);
        assert_eq!(dirs[508], 3, "the 509th unit takes a new directory block");
        check_reads(&tb, &model);
        assert_eq!(tb.wake().unwrap(), Wake::Now, "past 254 units the compaction is urgent");
        // No fold trigger passed: the 255th unit recorded the group in the root for the next step to push,
        // and no write changed the compaction queue.
        assert_eq!(root::u64_at(tb.host(), tb.root(), root::GROUP_TO_QUEUE).unwrap(), 1, "group 0 recorded");
        assert_eq!(root::u64_at(tb.host(), tb.root(), root::COMPACT_HEAD).unwrap(), 0, "the queue is untouched");
        let mut units = Units::default();
        assert_eq!(one_unit(&mut tb, &mut units), Ran::Compaction, "the next step pushes it and compacts");
        assert_eq!(root::u64_at(tb.host(), tb.root(), root::GROUP_TO_QUEUE).unwrap(), 0);
        // The urgent compaction folds them all.
        maintain_all(&mut tb);
        let pg2 = root::group(tb.host(), tb.root(), 0).unwrap();
        assert_eq!(buffer::u64_of(tb.host(), pg2, page::P_PATCH_DIR).unwrap(), 0, "every patch folded");
        check_reads(&tb, &model);
        check_groups(&tb);
        check_ownership(&tb);
        dirs.iter().sum::<u64>()
    });
    eprintln!("patch_units_over_255: {digest:?}");
}

// ---------------------------------------------------------------- no forced steps

/// Maintenance withheld: writes go on past every urgent threshold (patch chains and units) and run no
/// maintenance of their own beyond the seal of the group an insert fills: every group sealed before a write
/// keeps its page (no compaction, column release or drop), and the drain keeps its records.
#[test]
fn no_forced_steps() {
    let m = model_schema();
    let digest = per_fill(|h, lh| {
        let (mut tb, _) = Table::create(h, 25, &m.img).unwrap();
        let mut r = XorShift(0xF0F0);
        let mut model = BTreeMap::new();
        let mut next_key = 0u64;
        let mut units = Units::default();
        // Sealed groups first.
        for _ in 0..2 {
            fill_group(&mut tb, &mut r, &mut model, &mut next_key);
        }
        run_all(&mut tb, &mut units);
        let mut urgent = 0;
        for step in 0..12_000u64 {
            let before = sealed_groups(&tb);
            let drain = drain_records(&tb);
            random_write(&mut tb, &mut r, &mut model, &mut next_key, false);
            let after = sealed_groups(&tb);
            assert!(before.iter().all(|x| after.contains(x)), "step {step}: a write rewrote or dropped a group");
            assert!(after.len() <= before.len() + 1, "step {step}: a write sealed one group at most");
            assert_eq!(drain_records(&tb), drain, "step {step}: a write ran the drain");
            if tb.wake().unwrap() == Wake::Now {
                urgent += 1;
            }
            if step % 4_000 == 3_999 {
                check_reads(&tb, &model);
                lh.take(tb.host_mut());
            }
        }
        assert_ne!(tb.wake().unwrap(), Wake::Idle, "the writes left maintenance waiting");
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_running(&tb, &model);
        (model.len(), urgent, units)
    });
    eprintln!("no_forced_steps: {digest:?}");
}

// ---------------------------------------------------------------- single-leaf updates

/// Sets leaf `leaf` of the row with key k to `v`: an upsert of the model's row with that cell changed, keeping
/// the model.
fn set_cell(tb: &mut Table<'_, VecHost>, model: &mut Rows, k: u64, leaf: u32, v: V) {
    let row = &mut model.get_mut(&k).expect("key present").1;
    row.iter_mut().find(|(l, _)| *l == leaf).unwrap().1 = v;
    let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(row)).unwrap();
    assert!(matches!(u, Upsert::Replaced(_)), "key {k} present");
}

// ---------------------------------------------------------------- schema change

struct LeafVec(Vec<Leaf>);

impl native_table::schema::LeafSink for LeafVec {
    fn leaf(&mut self, l: &Leaf) -> Result<(), Fault> {
        self.0.push(*l);
        Ok(())
    }
}

/// A schema image built version by version: columns added and dropped, record fields added and dropped;
/// every earlier leaf record kept at its ordinal.
#[derive(Clone)]
struct Evo {
    version: u32,
    pool: Vec<u8>,
    cols: Vec<Column>,
    leaves: Vec<Leaf>,
    defaults: Vec<u8>,
}

impl Evo {
    fn new(types: &[Vec<u8>]) -> Evo {
        let mut e = Evo { version: 1, pool: Vec::new(), cols: Vec::new(), leaves: Vec::new(), defaults: Vec::new() };
        for t in types {
            e.add_col(t, None);
        }
        e
    }

    fn flat(&self, col: u32, tpos: u32, key: bool) -> Vec<Leaf> {
        let mut out = LeafVec(Vec::new());
        let mut scratch = vec![0u8; 256 * native_table::value::FRAME];
        let ct = native_table::schema::ColumnType {
            pool: &self.pool,
            column: col,
            ty: tpos,
            version: self.version,
            first_ordinal: self.leaves.len() as u16,
            key,
        };
        native_table::schema::flatten(ct, &mut out, &mut scratch).unwrap();
        out.0
    }

    /// The next version.
    fn bump(&mut self) -> &mut Self {
        self.version += 1;
        self
    }

    /// Adds a column of type `t` with an optional declared default (its VE); returns its id.
    fn add_col(&mut self, t: &[u8], default: Option<&[u8]>) -> u32 {
        let id = self.cols.len() as u32;
        let tpos = self.pool.len() as u32;
        self.pool.extend(t);
        let first = self.leaves.len() as u16;
        let ls = self.flat(id, tpos, id == 0);
        self.leaves.extend(ls);
        let (default_off, default_len) = match default {
            Some(d) => {
                let o = self.defaults.len() as u32;
                self.defaults.extend(d);
                (o, d.len() as u32)
            }
            None => (0, 0),
        };
        self.cols.push(Column { id, flags: 0, first_leaf: first, ty: tpos, default_off, default_len, name: 0 });
        id
    }

    /// Adds a scalar field to record column `col`; returns its leaf id.
    fn add_field(&mut self, col: u32, t: &[u8]) -> u32 {
        self.add_field_with(col, t, None)
    }

    /// Adds a scalar field, with an optional declared default (its VE), to record column `col`: the record's
    /// type is written anew at the pool's end with the field added last, its fields' leaves pointed at their
    /// new types; returns the new leaf's id.
    fn add_field_with(&mut self, col: u32, t: &[u8], default: Option<&[u8]>) -> u32 {
        let old = self.cols[col as usize].ty as usize;
        assert_eq!(self.pool[old], ty::RECORD);
        let n = u16::from_le_bytes([self.pool[old + 1], self.pool[old + 2]]);
        let a = u16::from_le_bytes([self.pool[old + 3], self.pool[old + 4]]);
        let at = self.pool.len();
        let mut expr = vec![ty::RECORD];
        expr.extend(n.to_le_bytes());
        expr.extend((a + 1).to_le_bytes());
        let mut types = Vec::new();
        let mut p = old + 5;
        let mut scratch = vec![0u8; 64 * native_table::value::FRAME];
        for _ in 0..(n + a) {
            let ty_at = p + ty::FIELD_HDR as usize;
            let end = native_table::value::skip_ty(&self.pool, ty_at as u32, &mut scratch).unwrap() as usize;
            types.push(at + expr.len() + ty::FIELD_HDR as usize);
            expr.extend_from_slice(&self.pool[p..end]);
            p = end;
        }
        let k = (n + a) as u32;
        let (doff, dlen) = match default {
            Some(d) => {
                let o = self.defaults.len() as u32;
                self.defaults.extend(d);
                (o, d.len() as u32)
            }
            None => (0, 0),
        };
        expr.extend(idl_hash(format!("added{k}").as_bytes()).to_le_bytes());
        expr.extend(0u32.to_le_bytes());
        expr.push(0);
        expr.extend(doff.to_le_bytes());
        expr.extend(dlen.to_le_bytes());
        let new_ty = at + expr.len();
        expr.extend(t);
        self.pool.extend(expr);
        self.cols[col as usize].ty = at as u32;
        for (i, &tpos) in types.iter().enumerate() {
            let l = self.leaves.iter_mut().find(|l| l.id == (col << 16) | i as u32).unwrap();
            l.path = tpos as u32;
        }
        let mut ls = self.flat(col, new_ty as u32, false);
        assert_eq!(ls.len(), 1);
        ls[0].id = (col << 16) | k;
        self.leaves.push(ls[0]);
        ls[0].id
    }

    fn drop_col(&mut self, col: u32) {
        self.cols[col as usize].flags |= 1;
        let v = self.version;
        for l in self.leaves.iter_mut().filter(|l| l.id >> 16 == col) {
            l.dropped = v;
        }
    }

    /// Drops a record field's leaf: its entry in the record's type flagged dropped.
    fn drop_leaf(&mut self, leaf: u32) {
        let v = self.version;
        let l = self.leaves.iter_mut().find(|l| l.id == leaf).unwrap();
        l.dropped = v;
        let flags = l.path as usize - ty::FIELD_HDR as usize + 8;
        self.pool[flags] |= ty::FIELD_DROPPED;
    }

    fn image(&self) -> Vec<u8> {
        let parts = SchemaParts {
            version: self.version,
            columns: &self.cols,
            leaves: &self.leaves,
            indexes: &[],
            type_pool: &self.pool,
            default_pool: &self.defaults,
            name_pool: &[],
        };
        let mut img = vec![0u8; 65_536];
        let (bytes, _) = write_schema(&parts, 1, &mut img).unwrap();
        img.truncate(bytes as usize);
        img
    }
}

/// Applies a schema change to the model: new leaves read `defaults` (in ordinal order), dropped leaves
/// are no longer read.
fn evolve_model(model: &mut Rows, defaults: &[(u32, V)], dropped: &[u32]) {
    for (_, row) in model.values_mut() {
        row.retain(|(l, _)| !dropped.contains(l));
        row.extend(defaults.iter().cloned());
    }
}

/// The ordinals of a shape's dropped leaves.
fn dropped_ordinals(tb: &Table<'_, VecHost>) -> Vec<u64> {
    (0..tb.shape().leaves).filter(|&o| !tb.shape().leaf(tb.host(), o).unwrap().live()).collect()
}

/// No sealed group's image holds a cell of a dropped leaf.
fn no_dropped_cells(tb: &Table<'_, VecHost>) -> usize {
    let dropped = dropped_ordinals(tb);
    let mut groups = 0;
    for (g, pg) in sealed_groups(tb) {
        let gv = GroupView::open(tb.host(), pg).unwrap();
        for &o in &dropped {
            assert!(gv.leaf_by_ordinal(o).unwrap().is_none(), "group {g} keeps dropped leaf {o}");
        }
        groups += 1;
    }
    groups
}

fn cleaned(tb: &Table<'_, VecHost>) -> u32 {
    root::u32_at(tb.host(), tb.root(), root::CLEANED).unwrap()
}

/// The defaults-and-release table: key Nat; Text; i64; u8; a record {a: u32, b: Text}; Bool.
fn dr_base() -> Evo {
    let base = vec![
        t::scalar(ty::NAT),
        t::scalar(ty::TEXT),
        t::scalar(ty::I64),
        t::scalar(ty::U8),
        t::record(&[("a", t::scalar(ty::U32)), ("b", t::scalar(ty::TEXT))]),
        t::scalar(ty::BOOL),
    ];
    Evo::new(&base)
}

fn dr_row(k: u64) -> Vec<(u32, V)> {
    vec![
        (id(0), V::U(k)),
        (id(1), V::Bytes(format!("row {k}").into_bytes())),
        (id(2), V::I(k as i64 * 7 - 3_000)),
        (id(3), V::U(k % 3)),
        (id(4), V::U(k % 1_000)),
        (id(4) | 1, V::Bytes(vec![b'b'; (k % 40) as usize])),
        (id(5), V::Bool(k % 2 == 0)),
    ]
}

/// Rows in two sealed groups and the open group.
fn dr_fill(tb: &mut Table<'_, VecHost>, model: &mut Rows, units: &mut Units) -> u64 {
    let mut k = 0u64;
    while seals(tb) == 0 {
        put_row(tb, model, dr_row(k));
        k += 1;
    }
    run_all(tb, units);
    while seals(tb) < 2 {
        put_row(tb, model, dr_row(k));
        k += 1;
    }
    for _ in 0..200 {
        put_row(tb, model, dr_row(k));
        k += 1;
    }
    k
}

/// A change refused before anything is written.
fn refused(tb: &mut Table<'_, VecHost>, img: &[u8], want: SchemaError) {
    tb.host_mut().end_message();
    assert_eq!(tb.change_schema(img), Err(Fault::Schema(want)));
    assert_eq!(tb.host_mut().end_message(), 0, "{want:?}: refused before anything is written");
}

#[test]
fn defaults_and_release() {
    let evo = dr_base();
    let img1 = evo.image();
    let digest = per_fill(|h, lh| {
        let (mut tb, _) = Table::create(h, 30, &img1).unwrap();
        let mut model = BTreeMap::new();
        let mut units = Units::default();
        let mut k = dr_fill(&mut tb, &mut model, &mut units);
        check_reads(&tb, &model);
        let mut trace = Vec::new();

        // Version 2: a Nat column with declared default 7, an ?Text column (null), an f64 column (zero), and a
        // Bool field added to the record (false).
        let mut e = evo.clone();
        e.bump();
        let n_id = id(e.add_col(&t::scalar(ty::NAT), Some(&[7])));
        let o_id = id(e.add_col(&t::opt(t::scalar(ty::TEXT)), None));
        let f_id = id(e.add_col(&t::scalar(ty::F64), None));
        let c_id = e.add_field(4, &t::scalar(ty::BOOL));
        tb.change_schema(&e.image()).unwrap();
        evolve_model(&mut model, &[(n_id, V::U(7)), (o_id, V::Null), (f_id, V::F64(0)), (c_id, V::Bool(false))], &[]);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_running(&tb, &model);
        // A write and a read at version 2: a new row; the new columns set in sealed and open rows.
        let mut row = dr_row(k);
        row.extend([
            (n_id, V::U(1 << 40)),
            (o_id, V::text("new")),
            (f_id, V::F64(2.5f64.to_bits())),
            (c_id, V::Bool(true)),
        ]);
        put_row(&mut tb, &mut model, row);
        k += 1;
        for key in [0u64, 1, 2, 4_000, 4_001, k - 2, k - 3] {
            if !model.contains_key(&key) {
                continue;
            }
            set_cell(&mut tb, &mut model, key, n_id, V::U(key + 100));
            set_cell(&mut tb, &mut model, key, o_id, V::text("set"));
            set_cell(&mut tb, &mut model, key, c_id, V::Bool(true));
        }
        check_reads(&tb, &model);
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_running(&tb, &model);
        trace.push((e.version, sealed_groups(&tb).len(), cleaned(&tb)));
        lh.take(tb.host_mut());

        // Version 3: the i64 column dropped, and the record's Text field.
        e.bump();
        e.drop_col(2);
        e.drop_leaf(id(4) | 1);
        let img3 = e.image();
        assert_eq!(tb.change_schema(&img3).unwrap(), Wake::Later, "the column release waits");
        evolve_model(&mut model, &[], &[id(2), id(4) | 1]);
        check_reads(&tb, &model);
        // A write and a read at version 3 before the release runs.
        let mut row = dr_row(k);
        row.retain(|(l, _)| *l != id(2) && *l != id(4) | 1);
        row.extend([(n_id, V::U(9)), (o_id, V::Null), (f_id, V::F64(0)), (c_id, V::Bool(false))]);
        put_row(&mut tb, &mut model, row);
        check_reads(&tb, &model);
        assert_eq!(cleaned(&tb), 0);
        run_all(&mut tb, &mut units);
        assert_eq!(cleaned(&tb), 1, "cleaned counts the dropped columns once the release ends");
        assert!(units.by.keys().any(|x| x.starts_with("Release")), "{units:?}");
        let groups = no_dropped_cells(&tb);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_running(&tb, &model);
        check_ownership(&tb);
        trace.push((e.version, groups, cleaned(&tb)));

        // Refusals, each before anything is written.
        let mut x = e.clone();
        x.bump();
        x.cols[2].flags &= !1;
        for l in x.leaves.iter_mut().filter(|l| l.id >> 16 == 2) {
            l.dropped = 0;
        }
        refused(&mut tb, &x.image(), SchemaError::RestoredDropped);
        let mut x = e.clone();
        x.bump();
        x.drop_col(0);
        refused(&mut tb, &x.image(), SchemaError::KeyChanged);
        // Column 1's type changed to Blob.
        let mut x = e.clone();
        x.bump();
        let tpos = x.pool.len() as u32;
        x.pool.push(ty::BLOB);
        x.cols[1].ty = tpos;
        x.leaves[1].code = ty::BLOB;
        x.leaves[1].path = tpos;
        refused(&mut tb, &x.image(), SchemaError::ColumnChanged);
        let mut x = e.clone();
        x.add_col(&t::scalar(ty::NAT), None);
        refused(&mut tb, &x.image(), SchemaError::ColumnChanged);
        (trace, model.len())
    });
    eprintln!("defaults_and_release: {digest:?}");
}

#[test]
fn release_restart() {
    let evo = dr_base();
    let img1 = evo.image();
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 31, &img1).unwrap();
        let mut model = BTreeMap::new();
        let mut units = Units::default();
        let mut k = dr_fill(&mut tb, &mut model, &mut units);
        // Two more sealed groups, so the walk has groups left when the second drop comes.
        for _ in 0..2 {
            let s0 = seals(&tb);
            while seals(&tb) == s0 {
                put_row(&mut tb, &mut model, dr_row(k));
                k += 1;
            }
            run_all(&mut tb, &mut units);
        }
        let mut e = evo.clone();
        e.bump();
        e.drop_col(2);
        tb.change_schema(&e.image()).unwrap();
        evolve_model(&mut model, &[], &[id(2)]);
        // The walk visits a group, then a second column is dropped: the walk restarts by its generation.
        let mut ran = 0;
        while units.by.get("Release").copied().unwrap_or(0) < 1 {
            one_unit(&mut tb, &mut units);
            ran += 1;
        }
        assert_eq!(cleaned(&tb), 0);
        e.bump();
        e.drop_col(5);
        tb.change_schema(&e.image()).unwrap();
        evolve_model(&mut model, &[], &[id(5)]);
        check_reads(&tb, &model);
        let mut row = dr_row(k);
        row.retain(|(l, _)| *l != id(2) && *l != id(5));
        put_row(&mut tb, &mut model, row);
        run_all(&mut tb, &mut units);
        assert_eq!(cleaned(&tb), 2, "cleaned ends at the number of dropped columns");
        let groups = no_dropped_cells(&tb);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_running(&tb, &model);
        check_ownership(&tb);
        // A drop of a field after the release: cleaned keeps counting columns.
        e.bump();
        e.drop_leaf(id(4) | 1);
        tb.change_schema(&e.image()).unwrap();
        evolve_model(&mut model, &[], &[id(4) | 1]);
        run_all(&mut tb, &mut units);
        assert_eq!(cleaned(&tb), 2);
        no_dropped_cells(&tb);
        check_reads(&tb, &model);
        (ran, groups, units.by.keys().filter(|x| x.starts_with("Release")).count())
    });
    eprintln!("release_restart: {digest:?}");
}

// ---------------------------------------------------------------- the model across changes

/// How the evolving model makes a column's values.
#[derive(Clone, Copy, PartialEq)]
enum Gen {
    /// Model column c's generator.
    Model(u32),
    Nat,
    OptText,
}

fn gen_cell(g: Gen, r: &mut XorShift, big: bool) -> V {
    match g {
        Gen::Model(c) => model_cell(r, c, big),
        Gen::Nat => V::U(r.next_u64() >> r.below(64)),
        Gen::OptText => {
            if r.below(3) == 0 {
                V::Null
            } else {
                let n = r.below(30) as usize;
                V::Bytes(text(r, n))
            }
        }
    }
}

#[test]
fn rows_model() {
    let cols = model_cols();
    let digest = per_fill(|h, lh| {
        let evo0 = Evo::new(&cols);
        let (mut tb, _) = Table::create(h, 32, &evo0.image()).unwrap();
        let mut e = evo0.clone();
        let mut r = XorShift(0x0DE1);
        let mut model: Rows = BTreeMap::new();
        let mut gens: Vec<(u32, Gen)> = (0..16).map(|c| (id(c), Gen::Model(c))).collect();
        let mut units = Units::default();
        let mut next_key = 0u64;
        let mut trace = Vec::new();
        for version in 1..=4u32 {
            if version > 1 {
                e.bump();
                match version {
                    2 => {
                        // Two columns added: a Nat with declared default 300, an ?Text.
                        let a = id(e.add_col(&t::scalar(ty::NAT), Some(&[0xAC, 0x02])));
                        let b = id(e.add_col(&t::opt(t::scalar(ty::TEXT)), None));
                        tb.change_schema(&e.image()).unwrap();
                        evolve_model(&mut model, &[(a, V::U(300)), (b, V::Null)], &[]);
                        gens.push((a, Gen::Nat));
                        gens.push((b, Gen::OptText));
                    }
                    3 => {
                        // The i64 and Blob columns dropped.
                        e.drop_col(3);
                        e.drop_col(14);
                        tb.change_schema(&e.image()).unwrap();
                        evolve_model(&mut model, &[], &[id(3), id(14)]);
                        gens.retain(|(l, _)| *l != id(3) && *l != id(14));
                    }
                    _ => {
                        // The Principal column dropped, and a Bool column added.
                        e.drop_col(7);
                        let b = id(e.add_col(&t::scalar(ty::BOOL), None));
                        tb.change_schema(&e.image()).unwrap();
                        evolve_model(&mut model, &[(b, V::Bool(false))], &[id(7)]);
                        gens.retain(|(l, _)| *l != id(7));
                        gens.push((b, Gen::Model(2)));
                    }
                }
                check_reads(&tb, &model);
                check_running(&tb, &model);
            }
            // Writes at this version, maintenance interleaved: a step after every few writes.
            let (ops, every) = if large() { (200_000u64, 50_000) } else { (5_000, 1_000) };
            for i in 0..ops {
                let op = r.below(10);
                if op < 5 || model.len() < 20 {
                    let mut row = vec![(id(0), V::U(next_key))];
                    for &(l, g) in gens.iter().skip(1) {
                        row.push((l, gen_cell(g, &mut r, true)));
                    }
                    next_key += 1;
                    put_row(&mut tb, &mut model, row);
                } else if op < 7 {
                    let k = *model.keys().nth(r.below(model.len() as u64) as usize).unwrap();
                    tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
                    model.remove(&k);
                } else {
                    let k = *model.keys().nth(r.below(model.len() as u64) as usize).unwrap();
                    let (l, g) = gens[1 + r.below(gens.len() as u64 - 1) as usize];
                    let v = gen_cell(g, &mut r, true);
                    if op == 9 {
                        set_cell(&mut tb, &mut model, k, l, v);
                    } else {
                        let mut row = model[&k].1.clone();
                        row.iter_mut().find(|(x, _)| *x == l).unwrap().1 = v;
                        put_row(&mut tb, &mut model, row);
                    }
                }
                if i % 3 == 0 {
                    one_unit(&mut tb, &mut units);
                }
                if i % every == every - 1 {
                    check_reads(&tb, &model);
                    lh.take(tb.host_mut());
                }
            }
            check_reads(&tb, &model);
            check_groups(&tb);
            check_running(&tb, &model);
            trace.push((version, model.len(), sealed_groups(&tb).len(), seals(&tb)));
        }
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_running(&tb, &model);
        check_ownership(&tb);
        no_dropped_cells(&tb);
        assert_eq!(cleaned(&tb), 3);
        (trace, units)
    });
    eprintln!("rows_model: {digest:?}");
}

// ---------------------------------------------------------------- the schema hash and stale tables

/// The table root's schema hash and schema version.
fn schema_of(tb: &Table<'_, VecHost>) -> (u64, u32) {
    (
        root::u64_at(tb.host(), tb.root(), root::SCHEMA_HASH).unwrap(),
        root::u32_at(tb.host(), tb.root(), root::SCHEMA_VERSION).unwrap(),
    )
}

/// The schema hash is the row type's: a table evolved through four schema changes (four columns added, two
/// with a declared default) and a table created at once from an image of the same row type, built without
/// that history (other versions, ordinals' versions and pool offsets), have equal hashes. The evolved table
/// opened with that image is not stale, and its next write runs no schema change.
#[test]
fn schema_hash_is_the_row_types() {
    let mut e = Evo::new(&[t::scalar(ty::NAT), t::scalar(ty::TEXT), t::scalar(ty::U8)]);
    let img1 = e.image();
    let mut imgs = Vec::new();
    e.bump();
    e.add_col(&t::scalar(ty::NAT), Some(&[7]));
    imgs.push(e.image());
    e.bump();
    e.add_col(&t::opt(t::scalar(ty::TEXT)), None);
    imgs.push(e.image());
    e.bump();
    e.add_col(&t::scalar(ty::F64), None);
    imgs.push(e.image());
    e.bump();
    e.add_col(&t::scalar(ty::BOOL), Some(&[1]));
    imgs.push(e.image());
    // The same row type at version 1, in one go.
    let mut fresh = Evo::new(&[t::scalar(ty::NAT), t::scalar(ty::TEXT), t::scalar(ty::U8)]);
    fresh.add_col(&t::scalar(ty::NAT), Some(&[7]));
    fresh.add_col(&t::opt(t::scalar(ty::TEXT)), None);
    fresh.add_col(&t::scalar(ty::F64), None);
    fresh.add_col(&t::scalar(ty::BOOL), Some(&[1]));
    let fresh_img = fresh.image();
    assert_ne!(&fresh_img, imgs.last().unwrap(), "other bytes: versions and pool offsets");
    let fresh_hash = SchemaView::open(&fresh_img).unwrap().hash;
    per_fill(|h, _| {
        let created = {
            let (tb, _) = Table::create(h, 40, &fresh_img).unwrap();
            schema_of(&tb)
        };
        let (mut tb, root) = Table::create(h, 41, &img1).unwrap();
        let mut sc = key_scratch();
        for k in 0..10u64 {
            let row = vec![(id(0), V::U(k)), (id(1), V::text("x")), (id(2), V::U(0))];
            tb.upsert(&mut sc, &mut Src::new(&row)).unwrap();
        }
        let mut hashes = vec![schema_of(&tb).0];
        for img in &imgs {
            tb.change_schema(img).unwrap();
            hashes.push(schema_of(&tb).0);
        }
        hashes.sort();
        hashes.dedup();
        assert_eq!(hashes.len(), 5, "every change changes the row type");
        let evolved = schema_of(&tb);
        assert_eq!(evolved.0, created.0, "equal row types, equal hashes");
        assert_eq!(evolved.0, fresh_hash);
        assert_eq!((created.1, evolved.1), (1, 5));
        drop(tb);
        // Opened with the image of its row type: not stale; the next write changes no schema.
        let mut tb = Table::open(h, root, &fresh_img).unwrap();
        assert!(!tb.stale());
        let page = root::u64_at(tb.host(), tb.root(), root::SCHEMA_PAGE).unwrap();
        let row = vec![
            (id(0), V::U(99)),
            (id(1), V::text("y")),
            (id(2), V::U(1)),
            (id(3), V::U(3)),
            (id(4), V::Null),
            (id(5), V::F64(1.5f64.to_bits())),
            (id(6), V::Bool(false)),
        ];
        tb.upsert(&mut sc, &mut Src::new(&row)).unwrap();
        assert_eq!(root::u64_at(tb.host(), tb.root(), root::SCHEMA_PAGE).unwrap(), page, "no schema change");
        assert_eq!(schema_of(&tb), evolved);
        let (_, got) = get_row(&tb, 1).unwrap();
        assert!(got.contains(&(id(3), V::U(7))), "the declared default: {got:?}");
        evolved
    });
}

/// A table opened with an image of a newer row type is stale: reads use its stored schema, and its first
/// mutating call adopts the image first (`change_schema`, the schema only), so an upgrade that adds a column
/// can write it at once; an image the change refuses fails that call before anything is written.
#[test]
fn upgrade_add_column_then_write() {
    let mut e = Evo::new(&[t::scalar(ty::NAT), t::scalar(ty::TEXT)]);
    let img1 = e.image();
    e.bump();
    let n_id = id(e.add_col(&t::scalar(ty::NAT), Some(&[7])));
    let img2 = e.image();
    // A changed column type: refused.
    let mut bad = Evo::new(&[t::scalar(ty::NAT), t::scalar(ty::BLOB)]);
    bad.bump();
    let img_bad = bad.image();
    per_fill(|h, _| {
        let (mut tb, root) = Table::create(h, 42, &img1).unwrap();
        let mut sc = key_scratch();
        for k in 0..5u64 {
            tb.upsert(&mut sc, &mut Src::new(&[(id(0), V::U(k)), (id(1), V::text("a"))])).unwrap();
        }
        drop(tb);
        {
            let mut tb = Table::open(h, root, &img_bad).unwrap();
            assert!(tb.stale());
            tb.host_mut().end_message();
            let r = tb.upsert(&mut sc, &mut Src::new(&[(id(0), V::U(9)), (id(1), V::text("b"))]));
            assert_eq!(r.err(), Some(Fault::Schema(SchemaError::ColumnChanged)));
            assert_eq!(tb.host_mut().end_message(), 0, "refused before anything is written");
            assert_eq!(schema_of(&tb).1, 1);
        }
        let mut tb = Table::open(h, root, &img2).unwrap();
        assert!(tb.stale());
        // Reads before the first write use the stored schema.
        let (_, before) = get_row(&tb, 1).unwrap();
        assert!(!before.iter().any(|(l, _)| *l == n_id));
        tb.upsert(&mut sc, &mut Src::new(&[(id(0), V::U(5)), (id(1), V::text("b")), (n_id, V::U(1 << 40))])).unwrap();
        assert!(!tb.stale());
        assert_eq!(schema_of(&tb).1, 2);
        let (_, new) = get_row(&tb, 5).unwrap();
        assert!(new.contains(&(n_id, V::U(1 << 40))), "{new:?}");
        let (_, old) = get_row(&tb, 1).unwrap();
        assert!(old.contains(&(n_id, V::U(7))), "an older row reads the declared default: {old:?}");
        let mut units = Units::default();
        run_all(&mut tb, &mut units);
        tb.len().unwrap()
    });
}

/// A test host's memory through a shared borrow, every write refused: a test can pass `Table::open` the
/// table's own schema page in place, as a runtime that keeps no copy of the image does.
struct InPlace<'a>(&'a VecHost);

impl PageHost for InPlace<'_> {
    fn block_of(&self, page: u64, i: u32) -> u64 {
        self.0.block_of(page, i)
    }
    fn get(&self, r: Region) -> Result<&[u8], Fault> {
        self.0.get(r)
    }
    fn get_mut(&mut self, _r: Region) -> Result<&mut [u8], Fault> {
        Err(Fault::BadRef)
    }
    fn pair_mut(&mut self, _a: Region, _b: Region) -> Result<(&mut [u8], &mut [u8]), Fault> {
        Err(Fault::BadRef)
    }
    fn read_write(&mut self, _r: Region, _wr: Region) -> Result<(&[u8], &mut [u8]), Fault> {
        Err(Fault::BadRef)
    }
    fn page_alloc(&mut self) -> Result<u64, Fault> {
        Err(Fault::BadRef)
    }
    fn page_free(&mut self, _p: u64) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
    fn alloc_changes(&self) -> u64 {
        self.0.alloc_changes()
    }
    fn test_fill(&self) -> Option<u8> {
        self.0.test_fill()
    }
}

/// A table opened with its own schema page, read in place, or with a copy of it is not stale and reads its
/// rows. A corrupted copy is refused: an open checks the hash of an image held anywhere else. The stored page's
/// hash is checked when the table's shape is derived from it, not at every open: a stored page corrupted after
/// its shape was stored opens with the stored shape, and is refused once an open derives the shape from it
/// again (here, after a byte of the stored shape changed).
#[test]
fn open_with_schema_page_in_place() {
    let img = Evo::new(&[t::scalar(ty::NAT), t::scalar(ty::TEXT)]).image();
    per_fill(|h, _| {
        let (root, sp) = {
            let (mut tb, root) = Table::create(h, 45, &img).unwrap();
            let mut sc = key_scratch();
            for k in 0..5u64 {
                tb.upsert(&mut sc, &mut Src::new(&[(id(0), V::U(k)), (id(1), V::text("a"))])).unwrap();
            }
            (root, root::u64_at(tb.host(), root, root::SCHEMA_PAGE).unwrap())
        };
        // The stored page in place, or the given copy.
        let open = |h: &VecHost, copy: Option<&[u8]>| {
            let page = h.get(Region::Page(sp)).unwrap();
            Table::open(&mut InPlace(h), root, copy.unwrap_or(page)).map(|tb| (tb.stale(), tb.len().unwrap()))
        };
        let stored = h.get(Region::Page(sp)).unwrap().to_vec();
        assert_eq!(open(h, None), Ok((false, 5)), "in place");
        assert_eq!(open(h, Some(&stored)), Ok((false, 5)), "a copy");
        // The last column's id raised by one: still in order, but not the row type the hash was taken of.
        let v = SchemaView::open_stored(&stored).unwrap();
        let at = v.layout.columns as usize + 24 * (usize::from(v.columns) - 1);
        let mut bad = stored.clone();
        bad[at] += 1;
        let refused = Err(Fault::Corrupt(CorruptAt { image: ImageId::Runtime, kind: Corruption::Header, offset: 24 }));
        assert_eq!(open(h, Some(&bad)), refused, "a corrupted copy");
        h.get_mut(Region::Page(sp)).unwrap()[at] += 1;
        assert_eq!(open(h, None), Ok((false, 5)), "the corrupted stored page, in place, its shape stored");
        assert_eq!(open(h, Some(&stored)), Ok((false, 5)), "the corrupted stored page, with an intact copy");
        h.get_mut(Region::Block(root)).unwrap()[root::SHAPE as usize] ^= 1;
        assert_eq!(open(h, None), refused, "the corrupted stored page, in place, its shape derived again");
        assert_eq!(open(h, Some(&stored)), refused, "the corrupted stored page, its shape derived again");
    });
}

/// A field added to a record column reads its declared default (its entry's default VE) in the rows written
/// before it, and the zero of its type when it declares none. Also after maintenance seals the rows.
#[test]
fn record_fields_with_declared_defaults() {
    let mut e = Evo::new(&[t::scalar(ty::NAT), t::record(&[("a", t::scalar(ty::U32))])]);
    let img1 = e.image();
    e.bump();
    let d_id = e.add_field_with(1, &t::scalar(ty::U8), Some(&[9]));
    let z_id = e.add_field(1, &t::scalar(ty::U16));
    let img2 = e.image();
    per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 44, &img1).unwrap();
        let mut sc = key_scratch();
        for k in 0..20u64 {
            tb.upsert(&mut sc, &mut Src::new(&[(id(0), V::U(k)), (id(1), V::U(k * 3))])).unwrap();
        }
        tb.change_schema(&img2).unwrap();
        let (_, r) = get_row(&tb, 4).unwrap();
        assert!(r.contains(&(d_id, V::U(9))), "the declared default: {r:?}");
        assert!(r.contains(&(z_id, V::U(0))), "no default: the zero: {r:?}");
        let row = [(id(0), V::U(100)), (id(1), V::U(1)), (d_id, V::U(2)), (z_id, V::U(3))];
        tb.upsert(&mut sc, &mut Src::new(&row)).unwrap();
        let check = |tb: &Table<'_, VecHost>| {
            let (_, r) = get_row(tb, 4).unwrap();
            assert!(r.contains(&(d_id, V::U(9))) && r.contains(&(z_id, V::U(0))), "{r:?}");
            let (_, r) = get_row(tb, 100).unwrap();
            assert!(r.contains(&(d_id, V::U(2))) && r.contains(&(z_id, V::U(3))), "{r:?}");
        };
        check(&tb);
        maintain_all(&mut tb);
        check(&tb);
        tb.len().unwrap()
    });
}

// ---------------------------------------------------------------- schema changes and messages

/// The model rows that do not read back: (key, leaf, wanted, got).
fn bad_reads(tb: &Table<'_, VecHost>, model: &Rows) -> Vec<(u64, u32, V, V)> {
    let mut out = Vec::new();
    for (k, (_, row)) in model {
        let (_, cells) = get_row(tb, *k).unwrap();
        for ((l, w), (_, g)) in expected(tb, row).into_iter().zip(cells) {
            if w != g {
                out.push((*k, l, w, g));
            }
        }
    }
    out
}

/// Every sealed group's image, by group.
fn all_images(tb: &Table<'_, VecHost>) -> Vec<(u64, Vec<u8>)> {
    sealed_groups(tb).into_iter().map(|(g, pg)| (g, group_image(tb, pg))).collect()
}

/// How the versioned model makes a leaf's values.
#[derive(Clone, Copy, PartialEq, Debug)]
enum ModelGen {
    Model(u32),
    Nat,
    OptText,
    U(u64),
    Text(u64),
}

fn versioned_cell(g: ModelGen, r: &mut XorShift) -> V {
    match g {
        ModelGen::Model(c) => model_cell(r, c, true),
        ModelGen::Nat => V::U(r.next_u64() >> r.below(64)),
        ModelGen::OptText => {
            if r.below(3) == 0 {
                V::Null
            } else {
                let n = r.below(30) as usize;
                V::Bytes(text(r, n))
            }
        }
        ModelGen::U(top) => V::U(r.below(top)),
        ModelGen::Text(n) => {
            let n = match r.below(200) {
                0 => LONG_VALUE as usize + 1 + r.below(2_000) as usize,
                1 => HUGE_VALUE as usize + 1 + r.below(2_000) as usize,
                _ => r.below(n) as usize,
            };
            V::Bytes(text(r, n))
        }
    }
}

fn env_u64(name: &str, d: u64) -> u64 {
    std::env::var(name).ok().and_then(|s| s.parse().ok()).unwrap_or(d)
}

/// The model's rows across nine schema versions: writes of every kind (columns and record fields added and
/// dropped), maintenance at a random cadence and in bursts, schema changes between them, and a `clear`;
/// reads, totals and ownership checked. 2,000 writes a version; in the large suite, 10,000 with seed 7.
/// `NT_OPS` and `NT_SEED` set others.
#[test]
fn rows_model_versions() {
    let (d_ops, d_seed) = if large() { (10_000, 7) } else { (2_000, 0xB16) };
    let ops = env_u64("NT_OPS", d_ops);
    let seed = env_u64("NT_SEED", d_seed);
    model_versions(ops, seed);
}

/// `rows_model_versions` with `ops` writes a version and the given seed.
fn model_versions(ops: u64, seed: u64) {
    let cols = model_cols();
    let digest = per_fill(|h, lh| {
        let mut e = Evo::new(&cols);
        let (mut tb, _) = Table::create(h, 70, &e.image()).unwrap();
        let mut r = XorShift(seed);
        let mut model: Rows = BTreeMap::new();
        let mut gens: Vec<(u32, ModelGen)> = (0..16).map(|c| (id(c), ModelGen::Model(c))).collect();
        let mut units = Units::default();
        let mut next_key = 0u64;
        let mut trace = Vec::new();
        let mut rec_col = 0u32;
        let mut extra_text = 0u32;
        for version in 1..=9u32 {
            if version > 1 {
                e.bump();
                let mut defaults: Vec<(u32, V)> = Vec::new();
                let mut dropped: Vec<u32> = Vec::new();
                match version {
                    2 => {
                        let a = id(e.add_col(&t::scalar(ty::NAT), Some(&[0xAC, 0x02])));
                        let b = id(e.add_col(&t::opt(t::scalar(ty::TEXT)), None));
                        defaults.extend([(a, V::U(300)), (b, V::Null)]);
                        gens.push((a, ModelGen::Nat));
                        gens.push((b, ModelGen::OptText));
                    }
                    3 => {
                        e.drop_col(3);
                        e.drop_col(14);
                        dropped.extend([id(3), id(14)]);
                    }
                    4 => {
                        rec_col = e.add_col(&t::record(&[("a", t::scalar(ty::U32)), ("b", t::scalar(ty::TEXT))]), None);
                        defaults.extend([(id(rec_col), V::U(0)), (id(rec_col) | 1, V::Bytes(Vec::new()))]);
                        gens.push((id(rec_col), ModelGen::U(1 << 32)));
                        gens.push((id(rec_col) | 1, ModelGen::Text(50)));
                        e.drop_col(7);
                        dropped.push(id(7));
                    }
                    5 => {
                        let f = e.add_field_with(rec_col, &t::scalar(ty::U8), Some(&[9]));
                        defaults.push((f, V::U(9)));
                        gens.push((f, ModelGen::U(256)));
                        extra_text = e.add_col(&t::scalar(ty::TEXT), None);
                        defaults.push((id(extra_text), V::Bytes(Vec::new())));
                        gens.push((id(extra_text), ModelGen::Text(100)));
                    }
                    6 => {
                        e.drop_leaf(id(rec_col) | 1);
                        dropped.push(id(rec_col) | 1);
                    }
                    7 => {
                        e.drop_col(extra_text);
                        dropped.push(id(extra_text));
                        let b = id(e.add_col(&t::scalar(ty::BOOL), None));
                        defaults.push((b, V::Bool(false)));
                        gens.push((b, ModelGen::Model(2)));
                    }
                    8 => {
                        let f = e.add_field(rec_col, &t::scalar(ty::U16));
                        defaults.push((f, V::U(0)));
                        gens.push((f, ModelGen::U(65_536)));
                        e.drop_col(13);
                        dropped.push(id(13));
                    }
                    _ => {
                        let a = id(e.add_col(&t::scalar(ty::NAT), Some(&[0x05])));
                        defaults.push((a, V::U(5)));
                        gens.push((a, ModelGen::Nat));
                        e.drop_col(1);
                        dropped.push(id(1));
                    }
                }
                tb.change_schema(&e.image()).unwrap();
                evolve_model(&mut model, &defaults, &dropped);
                gens.retain(|(l, _)| !dropped.contains(l));
                let bad = bad_reads(&tb, &model);
                assert!(bad.is_empty(), "version {version}: {} bad reads after the change, first {:?}", bad.len(), bad.first());
                check_running(&tb, &model);
            }
            let cadence = [1u64, 2, 3, 5, 8][r.below(5) as usize];
            for i in 0..ops {
                let op = r.below(20);
                if op < 9 || model.len() < 20 {
                    let mut row = vec![(id(0), V::U(next_key))];
                    for &(l, g) in gens.iter().skip(1) {
                        row.push((l, versioned_cell(g, &mut r)));
                    }
                    next_key += 1;
                    put_row(&mut tb, &mut model, row);
                } else if op < 13 {
                    let k = *model.keys().nth(r.below(model.len() as u64) as usize).unwrap();
                    let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
                    assert_eq!(gone, model.remove(&k).map(|x| x.0));
                } else {
                    let k = *model.keys().nth(r.below(model.len() as u64) as usize).unwrap();
                    if op < 18 {
                        let mut row = model[&k].1.clone();
                        for _ in 0..1 + r.below(3) {
                            let (l, g) = gens[1 + r.below(gens.len() as u64 - 1) as usize];
                            let v = versioned_cell(g, &mut r);
                            row.iter_mut().find(|(x, _)| *x == l).unwrap().1 = v;
                        }
                        put_row(&mut tb, &mut model, row);
                    } else {
                        let (l, g) = gens[1 + r.below(gens.len() as u64 - 1) as usize];
                        let v = versioned_cell(g, &mut r);
                        set_cell(&mut tb, &mut model, k, l, v);
                    }
                }
                if r.below(cadence) == 0 {
                    one_unit(&mut tb, &mut units);
                }
                if r.below(2_000) == 0 {
                    for _ in 0..r.below(400) {
                        one_unit(&mut tb, &mut units);
                    }
                }
                if i % 2_000 == 1_999 {
                    let bad = bad_reads(&tb, &model);
                    assert!(bad.is_empty(), "version {version} op {i}: {} bad reads, first {:?}", bad.len(), bad.first());
                    assert_eq!(tb.len().unwrap(), model.len() as u64);
                    lh.take(tb.host_mut());
                }
            }
            check_reads(&tb, &model);
            check_groups(&tb);
            check_running(&tb, &model);
            trace.push((version, model.len(), sealed_groups(&tb).len(), seals(&tb)));
            eprintln!("version {version}: {:?}", trace.last());
        }
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_running(&tb, &model);
        check_ownership(&tb);
        no_dropped_cells(&tb);
        (trace, units)
    });
    eprintln!("rows_model_versions ({ops} writes a version, seed {seed}): {digest:?}");
}

/// One image for the same live rows across a schema change: rows written at version 1 and sealed at version 2
/// (`write::freeze` after the change seals the group as a filling insert would), against the same rows sealed at
/// version 1 and rewritten (compaction, or the column release) at version 2.
#[test]
fn seal_and_rewrite_bytes_across_versions() {
    let cols = model_cols();
    for variant in 0..6u32 {
        // The rows that fill a group, found on a table of its own.
        let fit = {
            let mut h2 = VecHost::new(FILLS[0]);
            let (mut tb, _) = Table::create(&mut h2, 80, &Evo::new(&cols).image()).unwrap();
            let mut r = XorShift(0xACE0 + u64::from(variant));
            let (mut model, mut next_key) = (BTreeMap::new(), 0u64);
            fill_group(&mut tb, &mut r, &mut model, &mut next_key);
            next_key - 1
        };
        let res = per_fill(|h, _| {
            let mut imgs = Vec::new();
            let mut rows_after = Vec::new();
            for pass in 0..2 {
                let mut e = Evo::new(&cols);
                let (mut tb, _) = Table::create(&mut *h, 80, &e.image()).unwrap();
                let mut r = XorShift(0xACE0 + u64::from(variant));
                let mut model: Rows = BTreeMap::new();
                let mut next_key = 0u64;
                let mut units = Units::default();
                if pass == 0 {
                    // The same rows, the group left open.
                    for _ in 0..fit {
                        let row = model_row(&mut r, next_key, true);
                        next_key += 1;
                        put_row(&mut tb, &mut model, row);
                    }
                } else {
                    fill_group(&mut tb, &mut r, &mut model, &mut next_key);
                }
                // A few dead positions, below the trigger.
                let keys: Vec<u64> = model.keys().copied().filter(|k| k % 97 == 3).collect();
                for k in keys {
                    tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
                    model.remove(&k);
                }
                if pass == 1 {
                    run_all(&mut tb, &mut units);
                }
                e.bump();
                match variant {
                    0 => {
                        let a = id(e.add_col(&t::scalar(ty::NAT), Some(&[0xAC, 0x02])));
                        let b = id(e.add_col(&t::opt(t::scalar(ty::TEXT)), None));
                        evolve_model(&mut model, &[(a, V::U(300)), (b, V::Null)], &[]);
                    }
                    1 | 4 => {
                        let a = id(e.add_col(&t::scalar(ty::TEXT), None));
                        e.drop_col(3);
                        evolve_model(&mut model, &[(a, V::Bytes(Vec::new()))], &[id(3)]);
                    }
                    2 | 5 => {
                        e.drop_col(14);
                        evolve_model(&mut model, &[], &[id(14)]);
                    }
                    _ => {
                        let a = id(e.add_col(&t::scalar(ty::BOOL), None));
                        evolve_model(&mut model, &[(a, V::Bool(false))], &[]);
                    }
                }
                tb.change_schema(&e.image()).unwrap();
                if pass == 0 {
                    let (shape, tbl) = (*tb.shape(), tb.root());
                    write::freeze(tb.host_mut(), tbl, &shape).unwrap();
                }
                if pass == 1 && variant < 4 && root::group(tb.host(), tb.root(), 0).unwrap() & buffer::DESC_BIT == 0 {
                    let pg = root::group(tb.host(), tb.root(), 0).unwrap();
                    let tbl = tb.root();
                    native_table::table::jobs::queue(tb.host_mut(), tbl, 0, pg, false).unwrap();
                }
                run_all(&mut tb, &mut units);
                check_reads(&tb, &model);
                check_groups(&tb);
                let pg = root::group(tb.host(), tb.root(), 0).unwrap();
                imgs.push(group_image(&tb, pg));
                rows_after.push(units.by.keys().cloned().collect::<Vec<_>>());
            }
            let same = imgs[0] == imgs[1];
            let first = imgs[0].iter().zip(&imgs[1]).position(|(a, b)| a != b);
            (same, imgs[0].len(), imgs[1].len(), first, rows_after[1].clone())
        });
        assert!(res.0, "variant {variant}: one image, lengths {} {}, first difference {:?}", res.1, res.2, res.3);
    }
}

/// Messages as a canister runs them (the write-first step, then the write), the table opened anew from its
/// root for every message, and, in the second run, one message in seven trapped (the host's memory put back as
/// it was before it) and run again: both runs end with the same tables.
#[test]
fn crash_equivalence() {
    let cols = model_cols();
    let ops = env_u64("NT_CRASH_OPS", 12_000);
    let run = |trap_every: u64| -> (Vec<(u64, Vec<u8>)>, Vec<(u64, Vec<(u32, V)>)>, (u64, Vec<u8>), usize) {
        let mut e = Evo::new(&cols);
        let img1 = e.image();
        e.bump();
        let a = id(e.add_col(&t::scalar(ty::NAT), Some(&[0x05])));
        e.drop_col(3);
        let img2 = e.image();
        let mut h = VecHost::new(FILLS[0]);
        let root = Table::create(&mut h, 90, &img1).unwrap().1;
        let mut r = XorShift(0xC4A5);
        let mut tr = XorShift(0x7A9);
        let mut model: Rows = BTreeMap::new();
        let mut next_key = 0u64;
        let mut version = 1;
        for i in 0..ops {
            // The message: drawn before it runs, so a replay runs the same one.
            let saved_r = r.0;
            let saved_model = model.clone();
            let saved_next = next_key;
            let change = i == ops / 2;
            let attempts = if trap_every > 0 && tr.below(trap_every) == 0 { 2 } else { 1 };
            let before = if attempts == 2 { Some(h.clone()) } else { None };
            for attempt in 0..attempts {
                if attempt == 1 {
                    h = before.clone().unwrap();
                    r.0 = saved_r;
                    model = saved_model.clone();
                    next_key = saved_next;
                }
                let img = if version == 1 { img1.clone() } else { img2.clone() };
                let mut tb = Table::open(&mut h, root, &img).unwrap();
                tb.write_step(&everything()).unwrap();
                if change {
                    tb.change_schema(&img2).unwrap();
                    if attempt + 1 == attempts {
                        evolve_model(&mut model, &[(a, V::U(5))], &[id(3)]);
                        version = 2;
                    }
                    continue;
                }
                let op = r.below(10);
                if op < 5 || model.len() < 20 {
                    let mut row = vec![(id(0), V::U(next_key))];
                    for c in 1..16u32 {
                        if version == 2 && c == 3 {
                            continue;
                        }
                        row.push((id(c), model_cell(&mut r, c, true)));
                    }
                    if version == 2 {
                        row.push((a, V::U(r.below(1_000))));
                    }
                    next_key += 1;
                    put_row(&mut tb, &mut model, row);
                } else if op < 7 {
                    let k = *model.keys().nth(r.below(model.len() as u64) as usize).unwrap();
                    tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
                    model.remove(&k);
                } else {
                    let k = *model.keys().nth(r.below(model.len() as u64) as usize).unwrap();
                    let mut row = model[&k].1.clone();
                    let n = row.len() as u64;
                    let c = 1 + r.below(n - 1) as usize;
                    let leaf = row[c].0;
                    let col = leaf >> 16;
                    let v = if leaf == a { V::U(r.below(1_000)) } else { model_cell(&mut r, col, true) };
                    row[c].1 = v;
                    put_row(&mut tb, &mut model, row);
                }
            }
        }
        let mut tb = Table::open(&mut h, root, &img2).unwrap();
        maintain_all(&mut tb);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_running(&tb, &model);
        check_ownership(&tb);
        let reads: Vec<(u64, Vec<(u32, V)>)> = model.keys().map(|k| (*k, get_row(&tb, *k).unwrap().1)).collect();
        let n = sealed_groups(&tb).len();
        (all_images(&tb), reads, drain_state(&tb), n)
    };
    let a = run(0);
    let b = run(7);
    assert_eq!(a.3, b.3, "sealed groups");
    assert!(a.0 == b.0, "the same images");
    assert!(a.1 == b.1, "the same rows");
    assert_eq!(a.2, b.2, "the same drain");
    eprintln!("crash_equivalence: {} sealed groups, {} rows", a.3, a.1.len());
}

/// Blocks of area a in use: its pages' blocks less the never-used ones of its newest page and its free stack.
fn area_in_use(tb: &Table<'_, VecHost>, a: u32) -> (u64, u64, u64) {
    let rec = native_table::area::AreaRecord::read(tb.host(), tb.root(), a).unwrap();
    let mut free = 0u64;
    let mut b = rec.free_head;
    while b != 0 {
        free += 1;
        b = rd_u64(tb.host().get(Region::Block(b)).unwrap(), 0).unwrap();
    }
    let never = if rec.newest != 0 { 16 - u64::from(rec.bump) } else { 0 };
    (rec.pages * 16 - never - free, rec.pages, free)
}

/// Cycles of writes (inserts, updates to patches and long values, deletes) with maintenance interleaved, each
/// ended by deleting every row, then filling the open group with small rows until the next group opens and
/// deleting those too, all maintenance run: the blocks and pages in use after each cycle, a group holding one
/// deleted row, must not grow.
#[test]
fn no_leaks_over_cycles() {
    let m = model_schema();
    let cycles = env_u64("NT_CYCLES", 4);
    let ops = env_u64("NT_LEAK_OPS", 15_000);
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 92, &m.img).unwrap();
        let mut r = XorShift(0x1EA4);
        let mut model: Rows = BTreeMap::new();
        let mut next_key = 0u64;
        let mut units = Units::default();
        let mut out = Vec::new();
        for cycle in 0..cycles {
            for i in 0..ops {
                if r.below(10) < 4 {
                    let row = model_row(&mut r, next_key, true);
                    next_key += 1;
                    put_row(&mut tb, &mut model, row);
                } else {
                    random_write(&mut tb, &mut r, &mut model, &mut next_key, true);
                }
                if i % 2 == 0 {
                    one_unit(&mut tb, &mut units);
                }
                // Some sealed groups compacted by their patches and deletes.
                if i % 3_000 == 2_999 {
                    for (g, pg) in sealed_groups(&tb) {
                        if g % 2 == cycle % 2 {
                            let tbl = tb.root();
                            native_table::table::jobs::queue(tb.host_mut(), tbl, g, pg, false).unwrap();
                        }
                    }
                }
            }
            check_reads(&tb, &model);
            let before_delete = (sealed_groups(&tb).len(), model.len());
            let keys: Vec<u64> = model.keys().copied().collect();
            for (i, k) in keys.iter().enumerate() {
                tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(*k))])).unwrap();
                model.remove(k);
                if i % 2 == 0 {
                    one_unit(&mut tb, &mut units);
                }
            }
            let open = root::u64_at(tb.host(), tb.root(), root::OPEN_GROUP).unwrap();
            while root::u64_at(tb.host(), tb.root(), root::OPEN_GROUP).unwrap() == open {
                let row = model_row(&mut r, next_key, false);
                next_key += 1;
                put_row(&mut tb, &mut model, row);
            }
            let keys: Vec<u64> = model.keys().copied().collect();
            for k in &keys {
                tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(*k))])).unwrap();
                model.remove(k);
            }
            run_all(&mut tb, &mut units);
            let after_delete = (area_in_use(&tb, 0), area_in_use(&tb, 1), check_ownership(&tb), tb.host().live_pages());
            out.push((before_delete, after_delete, sealed_groups(&tb).len()));
        }
        out
    });
    let first = digest[1].1;
    for (i, (_, c, _)) in digest.iter().enumerate().skip(1) {
        assert_eq!(c.0 .0, first.0 .0, "cycle {i}: area 0 blocks in use after the deletes");
        assert_eq!(c.1 .0, first.1 .0, "cycle {i}: area 1 blocks in use after the deletes");
    }
}

/// Long patch chains: a few sealed rows' Text updated thousands of times with changing lengths (short, medium
/// in block chains, huge in long values) and no maintenance, the chains far past `PATCH_BLOCKS_MAX`; then
/// writes between the maintenance steps that fold them, twice.
#[test]
fn long_patch_chains() {
    let m = model_schema();
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 97, &m.img).unwrap();
        let mut r = XorShift(0x10C4);
        let mut model: Rows = BTreeMap::new();
        let mut next_key = 0u64;
        let mut units = Units::default();
        fill_group(&mut tb, &mut r, &mut model, &mut next_key);
        run_all(&mut tb, &mut units);
        let pick = |model: &Rows| -> Vec<u64> {
            model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == 0).map(|(k, _)| *k).take(7).collect()
        };
        let len_of = |i: usize, r: &mut XorShift| -> usize {
            match i % 11 {
                0 => LONG_VALUE as usize + 1 + r.below(3_000) as usize,
                5 => HUGE_VALUE as usize + 1 + r.below(2_000) as usize,
                _ => 50 + r.below(600) as usize,
            }
        };
        let mut longest = 0u64;
        for _ in 0..2 {
            let keys = pick(&model);
            for i in 0..1_500usize {
                let k = keys[i % keys.len()];
                let n = len_of(i, &mut r);
                let v = V::Bytes(text(&mut r, n));
                if i % 2 == 0 {
                    set_cell(&mut tb, &mut model, k, id(1), v);
                } else {
                    let mut row = model[&k].1.clone();
                    row[1].1 = v;
                    row[13].1 = if i % 3 == 0 { V::Null } else { V::Bytes(text(&mut r, i % 40)) };
                    put_row(&mut tb, &mut model, row);
                }
                if i % 300 == 299 {
                    check_reads(&tb, &model);
                }
            }
            let pg = root::group(tb.host(), tb.root(), 0).unwrap();
            let dir = buffer::u64_of(tb.host(), pg, page::P_PATCH_DIR).unwrap();
            patch::for_each_unit(tb.host(), dir, |u| {
                longest = longest.max(u.blocks);
                Ok(())
            })
            .unwrap();
            check_reads(&tb, &model);
            let mut i = 0u64;
            while one_unit(&mut tb, &mut units) != Ran::Nothing {
                if i % 4 == 0 {
                    random_write(&mut tb, &mut r, &mut model, &mut next_key, true);
                }
                i += 1;
                if i > 200_000 {
                    break;
                }
            }
            run_all(&mut tb, &mut units);
            check_reads(&tb, &model);
            check_groups(&tb);
            check_running(&tb, &model);
            check_ownership(&tb);
        }
        (longest, model.len())
    });
    assert!(digest.0 > native_table::consts::PATCH_BLOCKS_MAX, "chains past their bound: {digest:?}");
}

// ---------------------------------------------------------------- big values

/// A negative Int of `len` LEB128 bytes whose lowest groups are 0 but its first's low six bits, k's.
fn low_zero_int(k: u64, len: usize) -> Vec<u8> {
    let mut v = vec![0x80u8; len];
    v[0] = 0x80 | (k % 64) as u8;
    v[len - 1] = 0x40;
    v
}

/// Nat and Int values of 100,000 LEB128 bytes on every 32nd row, whose totals pass 65,535 bytes: sealed, then
/// compacted after updates and deletes; the rows read back and the totals stay exact.
#[test]
fn big_values_sealed_and_compacted() {
    let img = schema(&[t::scalar(ty::NAT), t::scalar(ty::NAT), t::scalar(ty::INT)]).0;
    let row = |k: u64, salt: u64| -> Vec<(u32, V)> {
        if k % 32 == 5 {
            vec![(id(0), V::U(k)), (id(1), V::Nat(big_nat(k + salt, 100_000))), (id(2), V::Int(low_zero_int(k + salt, 100_000)))]
        } else {
            vec![(id(0), V::U(k)), (id(1), V::U(k * 7 + salt)), (id(2), V::I(-(k as i64) - salt as i64))]
        }
    };
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 111, &img).unwrap();
        let mut model: Rows = BTreeMap::new();
        let mut units = Units::default();
        let mut k = 0u64;
        while seals(&tb) == 0 {
            put_row(&mut tb, &mut model, row(k, 0));
            k += 1;
        }
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        check_running(&tb, &model);
        let keys: Vec<u64> = model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == 0).map(|(k, _)| *k).collect();
        for key in &keys {
            match key % 3 {
                0 => {
                    tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(*key))])).unwrap();
                    model.remove(key);
                }
                1 => put_row(&mut tb, &mut model, row(*key, 1_000)),
                _ => {}
            }
        }
        check_reads(&tb, &model);
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        check_running(&tb, &model);
        check_ownership(&tb);
        units
    });
    eprintln!("big_values_sealed_and_compacted: {digest:?}");
}

/// A group drop after every row of the group was updated to another value of a u8 column, then deleted: the
/// group is dropped, and the totals stay exact.
#[test]
fn drop_of_a_group_of_updated_rows() {
    let img = schema(&[t::scalar(ty::NAT), t::scalar(ty::U8), t::scalar(ty::I64)]).0;
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 113, &img).unwrap();
        let mut model: Rows = BTreeMap::new();
        let mut units = Units::default();
        let mut k = 0u64;
        while seals(&tb) == 0 {
            put_row(&mut tb, &mut model, vec![(id(0), V::U(k)), (id(1), V::U(k % 200)), (id(2), V::I(k as i64))]);
            k += 1;
        }
        run_all(&mut tb, &mut units);
        let keys: Vec<u64> = model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == 0).map(|(k, _)| *k).collect();
        for key in &keys {
            let c = (key.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40) % 200;
            set_cell(&mut tb, &mut model, *key, id(1), V::U(c));
        }
        check_running(&tb, &model);
        for key in &keys {
            tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(*key))])).unwrap();
            model.remove(key);
        }
        run_all(&mut tb, &mut units);
        assert!(sealed_groups(&tb).iter().all(|(g, _)| *g != 0), "group 0 is dropped");
        check_running(&tb, &model);
        check_ownership(&tb);
        units
    });
    eprintln!("drop_of_a_group_of_updated_rows: {digest:?}");
}

// ---------------------------------------------------------------- stale tables

/// A table opened with a newer schema image while a compaction waits: its steps run on the stored schema and
/// leave it stale; its next write adopts the image.
#[test]
fn stale_table_steps_adopt_nothing() {
    let cols = model_cols();
    let digest = per_fill(|h, _| {
        let mut e = Evo::new(&cols);
        let img1 = e.image();
        let (mut tb, root) = Table::create(h, 114, &img1).unwrap();
        let mut r = XorShift(0x57A1);
        let mut model: Rows = BTreeMap::new();
        let mut next_key = 0u64;
        let mut units = Units::default();
        fill_group(&mut tb, &mut r, &mut model, &mut next_key);
        let keys: Vec<u64> = model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == 0).map(|(k, _)| *k).collect();
        for k in keys.iter().take(COMPACT_DEAD as usize + 10) {
            tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(*k))])).unwrap();
            model.remove(k);
        }
        e.bump();
        let a = id(e.add_col(&t::scalar(ty::NAT), Some(&[0x07])));
        let b = id(e.add_col(&t::opt(t::scalar(ty::TEXT)), None));
        let img2 = e.image();
        drop(tb);
        let mut tb = Table::open(h, root, &img2).unwrap();
        assert!(tb.stale());
        while one_unit(&mut tb, &mut units) != Ran::Nothing {
            assert!(tb.stale(), "a step adopts nothing");
        }
        assert!(units.by.contains_key("Compaction"), "{units:?}");
        evolve_model(&mut model, &[(a, V::U(7)), (b, V::Null)], &[]);
        let key = *model.keys().next().unwrap();
        set_cell(&mut tb, &mut model, key, a, V::U(70));
        assert!(!tb.stale(), "the write adopts the image");
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_running(&tb, &model);
        units
    });
    eprintln!("stale_table_steps_adopt_nothing: {digest:?}");
}

// ---------------------------------------------------------------- big defaults

/// The text of a check's panic, or "ok".
fn checked(f: impl FnOnce()) -> String {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(()) => "ok".to_string(),
        Err(p) => p.downcast_ref::<String>().cloned().unwrap_or_default().chars().take(300).collect(),
    }
}

/// 2^140 - 1 as a Nat's LEB128 bytes (20 groups), and -(2^139) as an Int's.
fn past_64_bits() -> (Vec<u8>, Vec<u8>) {
    let mut nat = vec![0xFFu8; 20];
    nat[19] = 0x7F;
    let mut int = vec![0x80u8; 20];
    int[19] = 0x60;
    (nat, int)
}

/// A column added with a declared default, then the first write of it in a row of a sealed group (a set or an
/// upsert). Kinds 0 to 2 and 6 have defaults past 64 bits (Nat, Int, ?Nat, and an Int that fits 128 bits):
/// the leaf's total counts every row at that default, a big-form sum from the start. Kinds 3 to 5 are a big
/// value written over a small default and long Text and Blob defaults.
fn big_default_case(kind: u32, upsert: bool) -> String {
    checked(|| {
        per_fill(|h, _| {
            let mut e = Evo::new(&[t::scalar(ty::NAT), t::scalar(ty::U16)]);
            let (mut tb, _) = Table::create(h, 330, &e.image()).unwrap();
            let mut model: Rows = BTreeMap::new();
            let mut units = Units::default();
            let mut k = 0u64;
            while seals(&tb) == 0 {
                put_row(&mut tb, &mut model, vec![(id(0), V::U(k)), (id(1), V::U(k % 7))]);
                k += 1;
            }
            run_all(&mut tb, &mut units);
            e.bump();
            let (nat, int) = past_64_bits();
            let (ty_, ve, dv, wv) = match kind {
                0 => (t::scalar(ty::NAT), nat.clone(), V::Nat(nat.clone()), V::U(5)),
                1 => (t::scalar(ty::INT), int.clone(), V::Int(int.clone()), V::I(-5)),
                2 => (t::opt(t::scalar(ty::NAT)), [vec![1u8], nat.clone()].concat(), V::Nat(nat.clone()), V::Null),
                3 => (t::scalar(ty::NAT), vec![0xAC, 0x02], V::U(300), V::Nat(nat.clone())),
                6 => {
                    let wide = sleb(-(1i128 << 100) - 3);
                    (t::scalar(ty::INT), wide.clone(), V::Int(wide), V::I(4))
                }
                4 => {
                    let mut ve = vec![0xB8, 0x17];
                    ve.extend(vec![b't'; 3_000]);
                    (t::scalar(ty::TEXT), ve, V::Bytes(vec![b't'; 3_000]), V::Bytes(b"x".to_vec()))
                }
                _ => {
                    let mut ve = vec![0xB8, 0x17];
                    ve.extend(vec![7u8; 3_000]);
                    (t::scalar(ty::BLOB), ve, V::Bytes(vec![7u8; 3_000]), V::Bytes(vec![1u8; 70_000]))
                }
            };
            let a = id(e.add_col(&ty_, Some(&ve)));
            tb.change_schema(&e.image()).unwrap();
            evolve_model(&mut model, &[(a, dv)], &[]);
            check_reads(&tb, &model);
            // Row 3 is in sealed group 0.
            if upsert {
                let mut row = model[&3].1.clone();
                row.iter_mut().find(|(x, _)| *x == a).unwrap().1 = wv;
                put_row(&mut tb, &mut model, row);
            } else {
                set_cell(&mut tb, &mut model, 3, a, wv);
            }
            check_reads(&tb, &model);
            check_groups(&tb);
            check_running(&tb, &model);
        })
    })
}

#[test]
fn big_default_first_write() {
    let mut out = Vec::new();
    for kind in 0..7 {
        for upsert in [false, true] {
            out.push(format!("kind {kind} upsert {upsert}: {}", big_default_case(kind, upsert)));
        }
    }
    assert!(out.iter().all(|x| x.ends_with(": ok")), "{out:#?}");
}
