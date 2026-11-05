//! Updates of sealed rows (in place and in patch columns), rewrites in units of bounded work (seal,
//! compaction, column release) with writes between their units, group drops, schema change with defaults,
//! and the maintenance step that runs exactly one unit.
//!
//! Every test runs once per fill pattern (`FILLS`) through `per_fill`, which requires identical results and
//! identical host-call logs from the two runs. Changed-block counts come from `VecHost::end_message`; a
//! unit's estimated instructions come from its rewrite's state block.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use native_table::consts::{
    BLOCK, BYTE_UNIT, COMPACT_DEAD, GROUP_SLOTS, HUGE_VALUE, LONG_VALUE, PATCH_UNITS_PER_DIR, UNIT_BLOCKS_MAX,
};
use native_table::fault::{Fault, SchemaError};
use native_table::mem::{rd_u16, rd_u32, rd_u64};
use native_table::schema::{write_schema, Column, Leaf, SchemaParts, SchemaView};
use native_table::table::group::GroupView;
use native_table::table::jobs::{Ran, UNIT_WORK};
use native_table::table::layout::Ty;
use native_table::table::{buffer, page, patch, read, root, seal, Table, Upsert};
use native_table::value::ty;
use native_table::{CorruptAt, Corruption, ImageId, Owner, PageHost, Progress, Region, Wake};
use nt_testhost::big::Big;
use nt_testhost::rows::{idl_hash, schema, t, Sink, Src, V};
use nt_testhost::{Call, TestBudget, VecHost, VecScratch, XorShift, FILLS};

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

/// What every unit the tests run spent, by the job that ran it: the most changed 4 KiB blocks and the most
/// estimated instructions of one unit, and the number of units.
#[derive(Default, Debug, Clone, PartialEq)]
struct Units {
    by: BTreeMap<String, (u64, u64, u64)>,
}

/// The name of a rewrite step.
fn step_name(step: u8) -> &'static str {
    match step {
        0 => "begin",
        seal::STEP_SNAPSHOT => "snapshot",
        seal::STEP_PASS1 => "pass1",
        seal::STEP_LAYOUT => "layout",
        seal::STEP_WRITE => "write",
        seal::STEP_FLIP => "flip",
        seal::STEP_LAST => "last",
        _ => "other",
    }
}

impl Units {
    fn add(&mut self, ran: Ran, step: u8, blocks: u64, work: u64) {
        let name = match ran {
            Ran::Seal | Ran::Compaction | Ran::Release => format!("{ran:?} {}", step_name(step)),
            _ => format!("{ran:?}"),
        };
        let e = self.by.entry(name).or_insert((0, 0, 0));
        e.0 = e.0.max(blocks);
        e.1 = e.1.max(work);
        e.2 += 1;
    }
}

/// Runs one unit, measuring it: every unit changes at most `UNIT_BLOCKS_MAX` 4 KiB blocks, moves at most
/// `BYTE_UNIT` bytes (a piece past it by at most one block's), and its estimated instructions stay within
/// `UNIT_WORK`; every unit that finds work makes progress.
fn one_unit(tb: &mut Table<'_, VecHost>, units: &mut Units) -> Ran {
    one_unit_m(tb, units, &everything())
}

fn one_unit_m<B: native_table::Budget>(tb: &mut Table<'_, VecHost>, units: &mut Units, meter: &B) -> Ran {
    // The allocator's own blocks count among the unit's.
    tb.host_mut().count_allocator(true);
    tb.host_mut().end_message();
    let before: Vec<u64> = (1..=3).map(|k| seal::running(tb.host(), tb.root(), k).unwrap()).collect();
    let steps: Vec<u8> =
        before.iter().map(|&st| if st == 0 { 0 } else { seal::step_of(tb.host(), st).unwrap() }).collect();
    let drain_top = {
        let top = root::u64_at(tb.host(), tb.root(), root::DRAIN_TOP).unwrap();
        if top == 0 {
            (0, 0)
        } else {
            let b = tb.host().get(Region::Block(top)).unwrap();
            let n = u64::from(rd_u16(b, 6).unwrap());
            (b[(32 + (n - 1) * 32) as usize], b[(32 + (n - 1) * 32 + 1) as usize])
        }
    };
    let (ran, spent) = tb.unit(meter).unwrap_or_else(|e| panic!("unit failed: {e:?} (steps before {steps:?})"));
    let blocks = tb.host_mut().end_message();
    let (work, bytes) = (spent.work, spent.bytes);
    assert!(bytes <= BYTE_UNIT + BLOCK, "{ran:?} moved {bytes} bytes");
    assert!(
        blocks <= UNIT_BLOCKS_MAX,
        "{ran:?} changed {blocks} blocks (steps before {steps:?}, drain top {drain_top:?}, {spent:?})"
    );
    assert!(
        meter.used() > 0 || work <= UNIT_WORK,
        "{ran:?} spent an estimated {work} instructions (steps before {steps:?})"
    );
    assert!(ran == Ran::Nothing || blocks > 0, "{ran:?} made progress");
    let step = match ran {
        Ran::Seal => steps[0],
        Ran::Compaction => steps[1],
        Ran::Release => steps[2],
        _ => 0,
    };
    if ran != Ran::Nothing {
        units.add(ran, step, blocks, work);
    }
    ran
}

/// Runs units until nothing is pending, measuring each.
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
    let mut sc = VecScratch::new(FILLS[0]);
    assert_eq!(tb.maintain(&mut sc, &everything()).unwrap(), Progress::Done);
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

/// The drain's records and its top record's bytes: a drain unit that moves on within a record (a walk's
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

fn frozen(tb: &Table<'_, VecHost>) -> u32 {
    root::u32_at(tb.host(), tb.root(), root::FROZEN_WAITING).unwrap()
}

/// Whether a rewrite of any kind is in progress.
fn rewriting(tb: &Table<'_, VecHost>) -> bool {
    (1..=3).any(|k| seal::running(tb.host(), tb.root(), k).unwrap() != 0)
}

// ---------------------------------------------------------------- checks

/// Every sealed group no rewrite holds: its live count against its live rows, each of which reads back.
/// Returns the groups checked.
fn check_groups(tb: &Table<'_, VecHost>) -> u64 {
    let h = tb.host();
    let mut checked = 0;
    for (g, pg) in sealed_groups(tb) {
        if buffer::u64_of(h, pg, page::P_REWRITE).unwrap() != 0 {
            continue;
        }
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

/// The model's totals, sum and non-null count by leaf id, moved by one write: the `old` row's integer values
/// leave them and the `new` row's enter them.
fn move_totals(want: &mut BTreeMap<u32, (Big, u64)>, old: Option<&[(u32, V)]>, new: Option<&[(u32, V)]>) {
    for (row, add) in [(old, false), (new, true)] {
        for (leaf, v) in row.into_iter().flatten() {
            if let Some(b) = total_value(v) {
                let (sum, n) = want.entry(*leaf).or_default();
                (*sum, *n) = if add { (sum.add(&b), *n + 1) } else { (sum.sub(&b), *n - 1) };
            }
        }
    }
}

/// Each leaf's total in `want` (as `move_totals` keeps it) is the table's.
fn check_totals(tb: &Table<'_, VecHost>, want: &BTreeMap<u32, (Big, u64)>) {
    let mut buf = vec![0u8; 1 << 17];
    for (leaf, (sum, n)) in want {
        let (got, gn) = tb.sum(*leaf, &mut buf).unwrap();
        assert_eq!((&Big::of_total(got, &buf), gn), (sum, *n), "total of leaf {leaf:#x}");
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
    let mut descs = Vec::new();
    let mut d = root::u64_at(h, tbl, root::FROZEN_HEAD).unwrap();
    while d != 0 {
        descs.push(d);
        d = buffer::u64_of(h, d, buffer::D_NEXT).unwrap();
    }
    descs.push(buffer::open_group(h, tbl).unwrap().1);
    for d in descs {
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

// ---------------------------------------------------------------- rewrites in units

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

/// The units a test interleaves writes with before it lets maintenance finish.
const WRITE_UNITS: u64 = 3_000;

/// The large suite (`NT_LARGE` set; scripts/check.sh runs it in release mode): `rows_model` at 10^6 writes,
/// `rows_model_versions` at 10,000, `rewrite_units` writing between a compaction's units for as long as it
/// runs, post-snapshot patches of more than 254 units at every rewrite step, and a clear during the last units
/// of a wide group.
fn large() -> bool {
    std::env::var_os("NT_LARGE").is_some()
}

/// Fills the open group until it freezes.
fn fill_group(tb: &mut Table<'_, VecHost>, r: &mut XorShift, model: &mut Rows, next_key: &mut u64) {
    let before = frozen(tb);
    while frozen(tb) == before {
        let row = model_row(r, *next_key, true);
        *next_key += 1;
        put_row(tb, model, row);
    }
}

#[test]
fn rewrite_units() {
    let m = model_schema();
    let digest = per_fill(|h, lh| {
        let (mut tb, _) = Table::create(h, 21, &m.img).unwrap();
        let mut r = XorShift(0x5EA1);
        let mut model = BTreeMap::new();
        let mut next_key = 0u64;
        let mut units = Units::default();
        let mut trace = Vec::new();
        // A seal with writes between every two units: updates, deletes and inserts of the frozen group's rows
        // and of others.
        fill_group(&mut tb, &mut r, &mut model, &mut next_key);
        let mut between = 0;
        loop {
            let ran = one_unit(&mut tb, &mut units);
            if ran == Ran::Nothing {
                break;
            }
            for _ in 0..1 + r.below(3) {
                random_write(&mut tb, &mut r, &mut model, &mut next_key, true);
                between += 1;
            }
            if between % 50 == 0 {
                check_reads(&tb, &model);
            }
            // New groups freeze as the writes go on; a few seals are enough.
            if sealed_groups(&tb).len() >= 3 {
                break;
            }
        }
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        let g1 = check_groups(&tb);
        check_running(&tb, &model);
        trace.push(("sealed", sealed_groups(&tb).len(), g1));
        // Compactions: a quarter of a sealed group's rows deleted, then units with writes between them.
        let groups = sealed_groups(&tb);
        let (g0, _) = groups[0];
        let keys: Vec<u64> = model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == g0).map(|(k, _)| *k).collect();
        for k in keys.iter().take((COMPACT_DEAD as usize + 10).min(keys.len())) {
            tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(*k))])).unwrap();
            model.remove(k);
        }
        // Writes between the units while the compaction runs (and for at most `WRITE_UNITS` units, so the
        // maintenance they make ends; in the large suite, for as long as the compaction runs).
        let mut n = 0;
        loop {
            let ran = one_unit(&mut tb, &mut units);
            if ran == Ran::Nothing {
                break;
            }
            n += 1;
            let compacting = seal::running(tb.host(), tb.root(), seal::COMPACTION).unwrap() != 0;
            if n <= WRITE_UNITS || (large() && compacting) {
                for _ in 0..r.below(3) {
                    random_write(&mut tb, &mut r, &mut model, &mut next_key, true);
                }
            }
        }
        check_reads(&tb, &model);
        let g2 = check_groups(&tb);
        check_running(&tb, &model);
        check_ownership(&tb);
        trace.push(("compacted", sealed_groups(&tb).len(), g2));
        lh.take(tb.host_mut());
        // A group whose rows are all deleted is dropped in one unit.
        let (g_last, _) = *sealed_groups(&tb).last().unwrap();
        let keys: Vec<u64> =
            model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == g_last).map(|(k, _)| *k).collect();
        for k in keys {
            tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
            model.remove(&k);
        }
        run_all(&mut tb, &mut units);
        assert_eq!(root::group(tb.host(), tb.root(), g_last).unwrap(), 0, "the empty group is dropped");
        check_reads(&tb, &model);
        check_running(&tb, &model);
        check_ownership(&tb);
        assert!(units.by.contains_key("Drop"), "{units:?}");
        trace.push(("dropped", sealed_groups(&tb).len(), 0));
        (trace, units, model.len())
    });
    eprintln!("rewrite_units: {digest:?}");
}

/// Reads while a compaction runs, with writes after its snapshot between its units: every row, the integer
/// leaves' totals and the buffered live count equal the model's.
#[test]
fn reads_during_compaction() {
    let m = model_schema();
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 24, &m.img).unwrap();
        let mut r = XorShift(0xDE17A);
        let mut model = BTreeMap::new();
        let mut next_key = 0u64;
        let mut units = Units::default();
        fill_group(&mut tb, &mut r, &mut model, &mut next_key);
        fill_group(&mut tb, &mut r, &mut model, &mut next_key);
        run_all(&mut tb, &mut units);
        let (g0, _) = sealed_groups(&tb)[0];
        let keys: Vec<u64> = model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == g0).map(|(k, _)| *k).collect();
        for k in keys.iter().take(COMPACT_DEAD as usize + 10) {
            tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(*k))])).unwrap();
            model.remove(k);
        }
        let (mut n, mut under) = (0u64, 0u64);
        loop {
            let ran = one_unit(&mut tb, &mut units);
            if ran == Ran::Nothing {
                break;
            }
            n += 1;
            if n <= 400 {
                // Writes to the compacted group's rows above all: updates, deletes and inserts.
                for _ in 0..2 {
                    let k = *model.keys().nth(r.below(model.len() as u64) as usize).unwrap();
                    if r.below(4) == 0 {
                        let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
                        assert_eq!(gone, model.remove(&k).map(|x| x.0));
                    } else {
                        let mut row = model[&k].1.clone();
                        for _ in 0..1 + r.below(4) {
                            let c = 1 + r.below(15) as u32;
                            row[c as usize].1 = model_cell(&mut r, c, true);
                        }
                        put_row(&mut tb, &mut model, row);
                    }
                }
            }
            if n % 7 == 0 {
                let held = rewriting(&tb);
                under += u64::from(held);
                check_running(&tb, &model);
            }
        }
        assert!(under >= 10, "reads ran while the rewrite held its group ({under})");
        check_reads(&tb, &model);
        check_groups(&tb);
        check_running(&tb, &model);
        check_ownership(&tb);
        (n, under, model.len())
    });
    eprintln!("reads_during_compaction: {digest:?}");
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
/// a dozen image blocks of cells, which the seal's layout lays out a few segments a piece, and its compaction
/// too; every value reads back.
#[test]
fn layout_in_pieces() {
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
        while frozen(&tb) == 0 {
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
    eprintln!("layout_in_pieces: {digest:?}");
}

/// One rule for the compacted form, and one image for the same live rows: a seal of a frozen group whose
/// rows were deleted before it, and a compaction of the same rows sealed with other values, updated to the
/// first table's values (in place and as patches) and deleted after the seal, give the same bytes, the group
/// number aside. Dead positions below, at and above `COMPACT_DEAD`.
#[test]
fn seal_and_compaction_bytes() {
    let m = model_schema();
    let digest = per_fill(|h, _| {
        let mut out = Vec::new();
        for dead in [37u64, COMPACT_DEAD - 1, COMPACT_DEAD, COMPACT_DEAD + 500] {
            let mut units = Units::default();
            // The final rows, and the group's positions: every row but the one that froze the group.
            let mut r = XorShift(0xB17E ^ dead);
            let mut finals: Vec<Vec<(u32, V)>> = Vec::new();
            let images: Vec<Vec<u8>> = (0..2)
                .map(|pass| {
                    // Both tables in the one host, one after the other, with the same uid.
                    let (mut tb, _) = Table::create(&mut *h, 41, &m.img).unwrap();
                    let mut model = BTreeMap::new();
                    let mut k = 0u64;
                    while frozen(&tb) == 0 {
                        if pass == 0 {
                            finals.push(model_row(&mut r, k, true));
                        }
                        let mut row = finals[k as usize].clone();
                        if pass == 1 && k % 3 == 0 {
                            // Other values first: a u32, a Bool, an i64 and a Text.
                            row[5].1 = V::U((k / 3) % 3);
                            row[2].1 = V::Bool(k % 2 == 0);
                            row[3].1 = V::I(-(k as i64));
                            // A Text of the same length, so the group freezes at the same row.
                            if let V::Bytes(b) = &row[1].1 {
                                row[1].1 = V::Bytes(vec![b'q'; b.len()]);
                            }
                        }
                        put_row(&mut tb, &mut model, row);
                        k += 1;
                    }
                    let p = k - 1;
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
                    if pass == 0 {
                        kill(&mut tb, &mut model);
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
fn rewrite_units_alternating_leb() {
    let img = schema(&[t::scalar(ty::NAT), t::scalar(ty::INT), t::scalar(ty::I64)]).0;
    let len = 60_000usize;
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 26, &img).unwrap();
        let mut units = Units::default();
        let mut model = BTreeMap::new();
        let mut n = 0u64;
        while frozen(&tb) == 0 {
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
    eprintln!("rewrite_units_alternating_leb: {digest:?}");
}

/// A row of wide values: key Nat; 64 KiB-class medium Text values.
fn medium_schema() -> Vec<u8> {
    schema(&[t::scalar(ty::NAT), t::scalar(ty::TEXT), t::scalar(ty::I64)]).0
}

#[test]
fn rewrite_units_medium_values() {
    let img = medium_schema();
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 22, &img).unwrap();
        let mut units = Units::default();
        let mut model = BTreeMap::new();
        let medium = |k: u64, v: u64| -> Vec<u8> {
            vec![b'a' + ((k + v) % 26) as u8; HUGE_VALUE as usize - 64 - (k % 7) as usize]
        };
        // Rows of 64 KiB-class medium values until the group freezes at its out-of-line bytes; then every
        // frozen row updated to another medium value (a long value each, adopted by the seal).
        let mut n = 0u64;
        while frozen(&tb) == 0 {
            put_row(
                &mut tb,
                &mut model,
                vec![(id(0), V::U(n)), (id(1), V::Bytes(medium(n, 0))), (id(2), V::I(n as i64))],
            );
            n += 1;
        }
        let frozen_rows: Vec<u64> =
            model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == 0).map(|(k, _)| *k).collect();
        for &k in &frozen_rows {
            put_row(
                &mut tb,
                &mut model,
                vec![(id(0), V::U(k)), (id(1), V::Bytes(medium(k, 1))), (id(2), V::I(k as i64))],
            );
        }
        assert_eq!(frozen(&tb), 1, "no update freezes or seals");
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        // Every sealed row updated again to another medium value: patches in chains, folded by the compaction.
        for &k in &frozen_rows {
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
    eprintln!("rewrite_units_medium_values: {digest:?}");
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
            if buffer::u16_of(tb.host(), desc, buffer::D_BLOCKS).unwrap() >= 240 || frozen(&tb) > 0 {
                break;
            }
            put_row(&mut tb, &mut model, grow_row(k, 260, 4));
            k += 1;
        }
        let (_, desc, _) = buffer::open_group(tb.host(), tb.root()).unwrap();
        let blocks = buffer::u16_of(tb.host(), desc, buffer::D_BLOCKS).unwrap();
        let frozen_before = frozen(&tb);
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
        assert_eq!(frozen(&tb), frozen_before);
        check_reads(&tb, &model);
        // Frozen groups: their rows grow into the open group's tail block or long row images, and their medium
        // values become long values; the seals read the moved images.
        while frozen(&tb) == 0 {
            put_row(&mut tb, &mut model, grow_row(k, 8, 8));
            k += 1;
        }
        let g_frozen: Vec<u64> = model.iter().filter(|(_, (rid, _))| {
            matches!(read::find(tb.host(), tb.root(), *rid).unwrap(), Some(read::Row::Buffered(at)) if at.frozen)
        }).map(|(k, _)| *k).collect();
        let waiting = frozen(&tb);
        for (i, &kk) in g_frozen.iter().enumerate().step_by(5) {
            let row = match i % 3 {
                0 => grow_row(kk, 120, 60),
                1 => grow_row(kk, LONG_VALUE as usize + 100, 10),
                _ => grow_row(kk, 4, 2_900),
            };
            put_row(&mut tb, &mut model, row);
        }
        assert_eq!(frozen(&tb), waiting, "no update freezes or seals");
        assert!(!rewriting(&tb), "no write runs a seal");
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

/// Frozen rows grown into the open group's tail block (moved stubs, each with a long item) are read through
/// their stubs by the frozen group's descriptor walk after its seal. While that walk is under way, new rows
/// fill and seal the next groups, whose descriptors wait behind it: the walks run in the order their
/// descriptors reached the drain, so the blocks of the group the stubs name, which new rows then reuse, are
/// freed only after the walk that reads them.
#[test]
fn descriptor_walks_in_order() {
    let img = grow_schema();
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 24, &img).unwrap();
        let mut units = Units::default();
        let mut model = BTreeMap::new();
        let mut k = 0u64;
        while frozen(&tb) == 0 {
            put_row(&mut tb, &mut model, grow_row(k, 4, 4));
            k += 1;
        }
        let mut moved = 0u64;
        for j in (0..k).step_by(3) {
            put_row(&mut tb, &mut model, grow_row(j, LONG_VALUE as usize + 100, 200));
            moved += u64::from(slot_flags(&tb, model[&j].0) == buffer::SLOT_MOVED);
        }
        assert!(moved > 0, "frozen rows moved to the tail block");
        while rewriting(&tb) || frozen(&tb) > 0 {
            one_unit(&mut tb, &mut units);
        }
        assert!(drain_records(&tb) > 0, "the sealed group's walk is under way");
        // Rows of about 2 KiB between units: each block the drain frees is soon a new row's.
        let mut waited = false;
        for n in 0..1_500u64 {
            one_unit(&mut tb, &mut units);
            put_row(&mut tb, &mut model, grow_row(1_000_000 + n, 1_500, 400));
            let (walking, last) = walks(&tb);
            waited |= walking != 0 && last != walking;
        }
        assert!(waited, "a descriptor waited behind another's walk");
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_ownership(&tb);
        (moved, units)
    });
    eprintln!("descriptor_walks_in_order: {digest:?}");
}

/// The drain's descriptor walks: the descriptor whose walk is under way, and the last one waiting (root job
/// slot 7).
fn walks(tb: &Table<'_, VecHost>) -> (u64, u64) {
    let at = root::JOB_SLOTS + 7 * 32;
    (root::u64_at(tb.host(), tb.root(), at).unwrap(), root::u64_at(tb.host(), tb.root(), at + 8).unwrap())
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
        while frozen(&tb) == 0 {
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
            assert!(!rewriting(&tb), "no write compacts");
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
        // No fold trigger passed: the 255th unit recorded the group in the root for the next unit to push,
        // and no write changed the compaction queue.
        assert_eq!(root::u64_at(tb.host(), tb.root(), root::GROUP_TO_QUEUE).unwrap(), 1, "group 0 recorded");
        assert_eq!(root::u64_at(tb.host(), tb.root(), root::COMPACT_HEAD).unwrap(), 0, "the queue is untouched");
        let mut units = Units::default();
        assert_eq!(one_unit(&mut tb, &mut units), Ran::Compaction, "the next unit pushes it and compacts");
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
        // Maintenance withheld: writes go on past every urgent threshold (frozen groups, patch chains and
        // units) and never run a unit of their own: no rewrite starts, none advances, no drain record goes.
        let mut urgent = 0;
        for step in 0..12_000u64 {
            let drain_top = root::u64_at(tb.host(), tb.root(), root::DRAIN_TOP).unwrap();
            let drain_n =
                if drain_top == 0 { 0 } else { rd_u16(tb.host().get(Region::Block(drain_top)).unwrap(), 6).unwrap() };
            let seals = sealed_groups(&tb).len();
            random_write(&mut tb, &mut r, &mut model, &mut next_key, false);
            assert!(!rewriting(&tb), "step {step}: a write ran a rewrite unit");
            assert!(sealed_groups(&tb).len() <= seals, "step {step}: a write sealed or compacted a group");
            let top2 = root::u64_at(tb.host(), tb.root(), root::DRAIN_TOP).unwrap();
            if top2 == drain_top && drain_top != 0 {
                let n2 = rd_u16(tb.host().get(Region::Block(top2)).unwrap(), 6).unwrap();
                assert!(n2 >= drain_n, "step {step}: a write ran the drain");
            }
            if tb.wake().unwrap() == Wake::Now {
                urgent += 1;
            }
            if step % 4_000 == 3_999 {
                check_reads(&tb, &model);
                lh.take(tb.host_mut());
            }
        }
        // Inserts until two groups wait frozen: the seal is urgent, and still no write runs it.
        while frozen(&tb) < 2 {
            let row = model_row(&mut r, next_key, false);
            next_key += 1;
            put_row(&mut tb, &mut model, row);
            assert!(!rewriting(&tb), "an insert ran a rewrite unit");
        }
        assert_eq!(tb.wake().unwrap(), Wake::Now);
        urgent += 1;
        check_reads(&tb, &model);
        // The write-first step runs one unit, a bound shared across jobs: a cheap flip is followed by further
        // pending jobs (the drain) inside that bound, never by the seal's next unit or the next seal.
        let mut sc = VecScratch::new(FILLS[0]);
        loop {
            let st = seal::running(tb.host(), tb.root(), seal::SEAL).unwrap();
            if st != 0 && seal::step_of(tb.host(), st).unwrap() == seal::STEP_FLIP {
                break;
            }
            one_unit(&mut tb, &mut units);
        }
        // Rows with huge values inserted and removed: their long values wait in the drain.
        for _ in 0..4 {
            let mut row = model_row(&mut r, next_key, false);
            row[1].1 = V::Bytes(vec![b'h'; HUGE_VALUE as usize + 10]);
            tb.upsert(&mut key_scratch(), &mut Src::new(&row)).unwrap();
            tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(next_key))])).unwrap();
            next_key += 1;
        }
        let waiting = frozen(&tb);
        let drain_before = drain_state(&tb);
        assert!(drain_before.0 > 0, "drain records wait behind the flip");
        tb.host_mut().end_message();
        tb.write_step(&mut sc, &everything()).unwrap();
        let changed = tb.host_mut().end_message();
        let st = seal::running(tb.host(), tb.root(), seal::SEAL).unwrap();
        assert_eq!(
            seal::step_of(tb.host(), st).unwrap(),
            seal::STEP_LAST,
            "the step ran the flip, and no seal unit after it"
        );
        assert_eq!(frozen(&tb), waiting - 1, "the next seal waits");
        assert!(drain_state(&tb) != drain_before, "the drain went on inside the same bound");
        assert!(changed <= UNIT_BLOCKS_MAX, "{changed} blocks");
        // Run to a state with drain records and a frozen group behind them: the step runs the next seal's
        // snapshot, and nothing of the seal after it.
        while seal::running(tb.host(), tb.root(), seal::SEAL).unwrap() != 0 {
            one_unit(&mut tb, &mut units);
        }
        for _ in 0..3 {
            let row = model_row(&mut r, next_key, true);
            next_key += 1;
            put_row(&mut tb, &mut model, row);
        }
        assert!(drain_records(&tb) > 0, "drain records wait");
        tb.host_mut().end_message();
        tb.write_step(&mut sc, &everything()).unwrap();
        let changed = tb.host_mut().end_message();
        let st = seal::running(tb.host(), tb.root(), seal::SEAL).unwrap();
        assert!(st != 0, "the next seal's snapshot ran");
        assert_eq!(seal::step_of(tb.host(), st).unwrap(), seal::STEP_SNAPSHOT, "and no seal unit after it");
        assert!(changed <= UNIT_BLOCKS_MAX, "{changed} blocks");
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

/// Rows in sealed groups, a frozen group and the open group.
fn dr_fill(tb: &mut Table<'_, VecHost>, model: &mut Rows, units: &mut Units) -> u64 {
    let mut k = 0u64;
    while frozen(tb) == 0 {
        put_row(tb, model, dr_row(k));
        k += 1;
    }
    run_all(tb, units);
    while frozen(tb) == 0 {
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
        // A write and a read at version 2: a new row; the new columns set in sealed, frozen and open rows.
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
            while frozen(&tb) < 2 {
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
        // The walk runs a few units, then a second column is dropped: the walk restarts by its generation.
        let mut ran = 0;
        while !units.by.keys().any(|x| x == "Release last") {
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
            // Writes at this version, maintenance interleaved: a unit after every few writes.
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
            trace.push((version, model.len(), sealed_groups(&tb).len(), frozen(&tb)));
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

// ---------------------------------------------------------------- unit bound

/// Every job's units, measured: a seal with writes between its units, a compaction of deletes and patches,
/// a group drop, the drain, seals and compactions of 64 KiB-class values and of 621 leaves with 600 patch
/// units, and a column release. Each unit is checked by `one_unit` against `UNIT_BLOCKS_MAX` changed 4 KiB
/// blocks (`VecHost::end_message`), `BYTE_UNIT` bytes, and `UNIT_WORK` estimated instructions.
#[test]
fn unit_bound() {
    let mut all = Units::default();
    let mut merge = |u: &Units| {
        for (k, v) in &u.by {
            let e = all.by.entry(k.clone()).or_insert((0, 0, 0));
            e.0 = e.0.max(v.0);
            e.1 = e.1.max(v.1);
            e.2 += v.2;
        }
    };
    // The model's rows: seals with writes between units, compactions, a drop.
    let m = model_schema();
    let u = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 40, &m.img).unwrap();
        let mut r = XorShift(0xB0B);
        let mut model = BTreeMap::new();
        let mut next_key = 0u64;
        let mut units = Units::default();
        for _ in 0..3 {
            fill_group(&mut tb, &mut r, &mut model, &mut next_key);
        }
        let mut n = 0;
        while one_unit(&mut tb, &mut units) != Ran::Nothing {
            n += 1;
            if n <= WRITE_UNITS {
                for _ in 0..2 {
                    random_write(&mut tb, &mut r, &mut model, &mut next_key, true);
                }
            }
        }
        for (g, _) in sealed_groups(&tb).into_iter().take(2) {
            let keys: Vec<u64> = model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == g).map(|(k, _)| *k).collect();
            let n = if g == 0 { keys.len() } else { COMPACT_DEAD as usize + 1 };
            for k in keys.iter().take(n) {
                tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(*k))])).unwrap();
                model.remove(k);
            }
        }
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        units
    });
    merge(&u);
    // 64 KiB-class values.
    let img = medium_schema();
    let u = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 41, &img).unwrap();
        let mut units = Units::default();
        let mut model = BTreeMap::new();
        let mut n = 0u64;
        while frozen(&tb) == 0 {
            let v = vec![b'm'; HUGE_VALUE as usize - 8 - (n % 5) as usize];
            put_row(&mut tb, &mut model, vec![(id(0), V::U(n)), (id(1), V::Bytes(v)), (id(2), V::I(n as i64))]);
            n += 1;
        }
        run_all(&mut tb, &mut units);
        for k in 0..n / 2 {
            tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
            model.remove(&k);
        }
        for k in n / 2..n {
            let v = vec![b'q'; HUGE_VALUE as usize - 100 - (k % 7) as usize];
            put_row(&mut tb, &mut model, vec![(id(0), V::U(k)), (id(1), V::Bytes(v)), (id(2), V::I(-(k as i64)))]);
        }
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        units
    });
    merge(&u);
    // 621 leaves, 600 patch units, the urgent compaction.
    let nl = 620usize;
    let img = many_schema(nl);
    let u = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 42, &img).unwrap();
        let mut units = Units::default();
        let mut model = BTreeMap::new();
        let mut k = 0u64;
        while frozen(&tb) == 0 {
            put_row(&mut tb, &mut model, many_row(nl, k, &BTreeMap::new()));
            k += 1;
        }
        run_all(&mut tb, &mut units);
        let mut vals: BTreeMap<u64, BTreeMap<usize, u64>> = BTreeMap::new();
        for unit in 0..600usize {
            let key = unit as u64 % k;
            let v = vals.entry(key).or_default();
            v.insert(unit, 1 + unit as u64 % 200);
            let row = many_row(nl, key, v);
            put_row(&mut tb, &mut model, row);
        }
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        units
    });
    merge(&u);
    // A column release.
    let evo = dr_base();
    let img1 = evo.image();
    let u = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 43, &img1).unwrap();
        let mut model = BTreeMap::new();
        let mut units = Units::default();
        dr_fill(&mut tb, &mut model, &mut units);
        let mut e = evo.clone();
        e.bump();
        e.drop_col(1);
        tb.change_schema(&e.image()).unwrap();
        evolve_model(&mut model, &[], &[id(1)]);
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        units
    });
    merge(&u);
    eprintln!("unit_bound: job and step | most changed 4 KiB blocks | most estimated instructions | units");
    for (k, (b, w, n)) in &all.by {
        eprintln!("unit_bound: {k:<20} | {b:>2} | {w:>6} | {n}");
        assert!(*b <= UNIT_BLOCKS_MAX && *w <= UNIT_WORK);
    }
    for job in ["Seal", "Compaction", "Drop", "Release", "Drain"] {
        assert!(all.by.keys().any(|k| k.starts_with(job)), "{job} measured");
    }
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
    fn get_mut(&mut self, _r: Region, _w: Owner) -> Result<&mut [u8], Fault> {
        Err(Fault::BadRef)
    }
    fn pair_mut(&mut self, _a: Region, _b: Region, _w: Owner) -> Result<(&mut [u8], &mut [u8]), Fault> {
        Err(Fault::BadRef)
    }
    fn read_write(&mut self, _r: Region, _wr: Region, _w: Owner) -> Result<(&[u8], &mut [u8]), Fault> {
        Err(Fault::BadRef)
    }
    fn page_alloc(&mut self, _w: Owner) -> Result<u64, Fault> {
        Err(Fault::BadRef)
    }
    fn page_free(&mut self, _p: u64, _w: Owner) -> Result<(), Fault> {
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
        h.get_mut(Region::Page(sp), Owner(0)).unwrap()[at] += 1;
        assert_eq!(open(h, None), Ok((false, 5)), "the corrupted stored page, in place, its shape stored");
        assert_eq!(open(h, Some(&stored)), Ok((false, 5)), "the corrupted stored page, with an intact copy");
        h.get_mut(Region::Block(root), Owner(0)).unwrap()[root::SHAPE as usize] ^= 1;
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

// ---------------------------------------------------------------- schema changes, meters and messages during rewrites

/// A column added while a seal reads its frozen group's liveness (before the snapshot), and a frozen row's
/// new column written then: the value must survive the seal.
#[test]
fn schema_change_before_seal_snapshot() {
    let cols = model_cols();
    let digest = per_fill(|h, _| {
        let mut e = Evo::new(&cols);
        let (mut tb, _) = Table::create(h, 51, &e.image()).unwrap();
        let mut r = XorShift(0x5EA7);
        let mut model: Rows = BTreeMap::new();
        let mut next_key = 0u64;
        let mut units = Units::default();
        fill_group(&mut tb, &mut r, &mut model, &mut next_key);
        // The seal's first unit: begin (allocations only); the liveness pass is still to run.
        assert_eq!(one_unit(&mut tb, &mut units), Ran::Seal);
        let st = seal::running(tb.host(), tb.root(), seal::SEAL).unwrap();
        assert_ne!(st, 0);
        assert_eq!(seal::step_of(tb.host(), st).unwrap(), seal::STEP_SNAPSHOT);
        e.bump();
        let a = id(e.add_col(&t::scalar(ty::NAT), Some(&[0xAC, 0x02])));
        tb.change_schema(&e.image()).unwrap();
        evolve_model(&mut model, &[(a, V::U(300))], &[]);
        // Frozen rows written at the new version before the snapshot.
        let keys: Vec<u64> = model.keys().copied().take(40).collect();
        for (i, k) in keys.iter().enumerate() {
            if i % 2 == 0 {
                set_cell(&mut tb, &mut model, *k, a, V::U(7_000 + i as u64));
            } else {
                let mut row = model[k].1.clone();
                row.iter_mut().find(|(x, _)| *x == a).unwrap().1 = V::U(9_000 + i as u64);
                put_row(&mut tb, &mut model, row);
            }
        }
        check_reads(&tb, &model);
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_running(&tb, &model);
        model.len()
    });
    eprintln!("schema_change_before_seal_snapshot: {digest:?}");
}

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

/// A column added at each step of a rewrite (seal or compaction), and the rewrite's group's rows given a value
/// of it right after: every value survives the rewrite.
#[test]
fn schema_change_at_every_rewrite_step() {
    let cols = model_cols();
    let targets: Vec<(u8, usize)> = vec![
        (seal::STEP_SNAPSHOT, 0),
        (seal::STEP_SNAPSHOT, 5),
        (seal::STEP_PASS1, 0),
        (seal::STEP_PASS1, 100),
        (seal::STEP_LAYOUT, 0),
        (seal::STEP_WRITE, 0),
        (seal::STEP_WRITE, 500),
        (seal::STEP_FLIP, 0),
        (seal::STEP_LAST, 0),
        (seal::STEP_LAST, 10),
    ];
    for compaction in [false, true] {
        for &(target, extra) in &targets {
            let res = per_fill(|h, _| {
                let mut e = Evo::new(&cols);
                let (mut tb, _) = Table::create(h, 52, &e.image()).unwrap();
                let mut r = XorShift(0x5EA8);
                let mut model: Rows = BTreeMap::new();
                let mut next_key = 0u64;
                let mut units = Units::default();
                fill_group(&mut tb, &mut r, &mut model, &mut next_key);
                if compaction {
                    run_all(&mut tb, &mut units);
                    let keys: Vec<u64> = model.keys().copied().filter(|k| k % 5 == 1).collect();
                    for k in keys {
                        tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
                        model.remove(&k);
                    }
                    let pg = sealed_groups(&tb)[0].1;
                    let tbl = tb.root();
                    native_table::table::jobs::queue(tb.host_mut(), tbl, 0, pg, false).unwrap();
                }
                let kind = if compaction { seal::COMPACTION } else { seal::SEAL };
                let mut step;
                let mut left = extra;
                loop {
                    one_unit(&mut tb, &mut units);
                    let st = seal::running(tb.host(), tb.root(), kind).unwrap();
                    if st == 0 {
                        step = 255;
                        break;
                    }
                    step = seal::step_of(tb.host(), st).unwrap();
                    if step > target || (step == target && left == 0) {
                        break;
                    }
                    if step == target {
                        left -= 1;
                    }
                }
                e.bump();
                let a = id(e.add_col(&t::scalar(ty::NAT), Some(&[0xAC, 0x02])));
                tb.change_schema(&e.image()).unwrap();
                evolve_model(&mut model, &[(a, V::U(300))], &[]);
                let keys: Vec<u64> = model.keys().copied().take(60).collect();
                for (i, k) in keys.iter().enumerate() {
                    if i % 2 == 0 {
                        set_cell(&mut tb, &mut model, *k, a, V::U(7_000 + i as u64));
                    } else {
                        let mut row = model[k].1.clone();
                        row.iter_mut().find(|(x, _)| *x == a).unwrap().1 = V::U(9_000 + i as u64);
                        put_row(&mut tb, &mut model, row);
                    }
                }
                let mid = bad_reads(&tb, &model).len();
                run_all(&mut tb, &mut units);
                let bad = bad_reads(&tb, &model);
                if bad.is_empty() {
                    check_groups(&tb);
                    check_running(&tb, &model);
                    check_ownership(&tb);
                }
                (step, mid, bad.len(), bad.first().map(|x| format!("{:?}", x)))
            });
            assert_eq!(res.2, 0, "compaction {compaction} at step {target}+{extra}: bad reads after, first {:?}", res.3);
        }
    }
}

/// A host meter that advances by a random amount at every reading, so units end after a random number of
/// pieces.
struct RandMeter {
    used: std::cell::Cell<u64>,
    r: std::cell::RefCell<XorShift>,
    max: u64,
    /// One reading in `big_every` jumps by up to `big`.
    big_every: u64,
    big: u64,
}

impl RandMeter {
    fn new(seed: u64, max: u64) -> Self {
        RandMeter { used: std::cell::Cell::new(0), r: std::cell::RefCell::new(XorShift(seed)), max, big_every: 0, big: 0 }
    }
    fn spiky(seed: u64, max: u64, big_every: u64, big: u64) -> Self {
        RandMeter { used: std::cell::Cell::new(0), r: std::cell::RefCell::new(XorShift(seed)), max, big_every, big }
    }
}

impl native_table::Budget for RandMeter {
    fn used(&self) -> u64 {
        let mut r = self.r.borrow_mut();
        let step = if self.big_every > 0 && r.below(self.big_every) == 0 { r.below(self.big) + 1 } else { r.below(self.max) + 1 };
        self.used.set(self.used.get() + step);
        self.used.get()
    }
    fn limit(&self) -> u64 {
        u64::MAX
    }
}

/// Units with meter `b` until nothing is pending; each within the page bound and making progress.
fn run_all_with(tb: &mut Table<'_, VecHost>, b: &dyn native_table::Budget) -> u64 {
    struct W<'a>(&'a dyn native_table::Budget);
    impl native_table::Budget for W<'_> {
        fn used(&self) -> u64 {
            self.0.used()
        }
        fn limit(&self) -> u64 {
            self.0.limit()
        }
    }
    let mut n = 0;
    loop {
        tb.host_mut().count_allocator(true);
        tb.host_mut().end_message();
        let (ran, spent) = tb.unit(&W(b)).unwrap_or_else(|e| panic!("unit {n} failed: {e:?}"));
        let blocks = tb.host_mut().end_message();
        assert!(blocks <= UNIT_BLOCKS_MAX, "{ran:?} changed {blocks} blocks");
        assert!(spent.bytes <= BYTE_UNIT + BLOCK, "{ran:?} moved {} bytes", spent.bytes);
        if ran == Ran::Nothing {
            break;
        }
        assert!(blocks > 0, "{ran:?} made progress");
        n += 1;
        if n > 300_000 {
            let steps: Vec<(u64, u8, u64, u64)> = (1..=3u8)
                .map(|k| {
                    let st = seal::running(tb.host(), tb.root(), k).unwrap();
                    if st == 0 {
                        (0, 0, 0, 0)
                    } else {
                        let b = tb.host().get(Region::Block(st)).unwrap();
                        (st, seal::step_of(tb.host(), st).unwrap(), rd_u32(b, seal::RW_LEAF).unwrap() as u64, rd_u64(b, seal::RW_SUB).unwrap())
                    }
                })
                .collect();
            eprintln!("unit {n}: {ran:?} {spent:?} blocks {blocks} rewrites (st, step, leaf, sub) {steps:?} drain {:?}", drain_state(tb));
            if n > 300_010 {
                panic!("maintenance does not end");
            }
        }
    }
    n
}

/// Every sealed group's image, by group.
fn all_images(tb: &Table<'_, VecHost>) -> Vec<(u64, Vec<u8>)> {
    sealed_groups(tb).into_iter().map(|(g, pg)| (g, group_image(tb, pg))).collect()
}

/// The same writes and rewrites under hosts whose meters end units at different pieces: the same images.
#[test]
fn meter_cuts_same_bytes() {
    let cols = model_cols();
    let scenario = |h: &mut VecHost, meter: &dyn native_table::Budget| -> (Vec<(u64, Vec<u8>)>, u64, usize) {
        let mut e = Evo::new(&cols);
        let (mut tb, _) = Table::create(h, 61, &e.image()).unwrap();
        let mut r = XorShift(0xC075);
        let mut model: Rows = BTreeMap::new();
        let mut next_key = 0u64;
        fill_group(&mut tb, &mut r, &mut model, &mut next_key);
        fill_group(&mut tb, &mut r, &mut model, &mut next_key);
        let mut units = run_all_with(&mut tb, meter);
        // Deletes past the trigger in group 0, updates of group 1's rows (patches, long and huge ones too).
        let g0: Vec<u64> = model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == 0).map(|(k, _)| *k).collect();
        for k in g0.iter().take(COMPACT_DEAD as usize + 10) {
            tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(*k))])).unwrap();
            model.remove(k);
        }
        let g1: Vec<u64> = model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == 1).map(|(k, _)| *k).collect();
        for (i, k) in g1.iter().enumerate().take(900) {
            let mut row = model[k].1.clone();
            for c in [1u32, 2, 3, 5, 9, 13] {
                if (i as u32 + c) % 3 == 0 {
                    row[c as usize].1 = model_cell(&mut r, c, true);
                }
            }
            put_row(&mut tb, &mut model, row);
        }
        let pg1 = root::group(tb.host(), tb.root(), 1).unwrap();
        let tbl = tb.root();
        native_table::table::jobs::queue(tb.host_mut(), tbl, 1, pg1, false).unwrap();
        units += run_all_with(&mut tb, meter);
        check_reads(&tb, &model);
        check_groups(&tb);
        // A dropped column: the release rewrites every group.
        e.bump();
        e.drop_col(9);
        tb.change_schema(&e.image()).unwrap();
        evolve_model(&mut model, &[], &[id(9)]);
        units += run_all_with(&mut tb, meter);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_running(&tb, &model);
        check_ownership(&tb);
        (all_images(&tb), units, model.len())
    };
    let out = per_fill(|h, _| {
        let base = scenario(h, &everything());
        let mut res = vec![(0u64, base.1)];
        for (seed, max, every, big) in [(1u64, 3_000u64, 0u64, 0u64), (2, 20_000, 0, 0), (3, 2_000, 40, 90_000), (4, 500, 10, 30_000)] {
            let mut h2 = VecHost::new(h.fill_pattern());
            let got = scenario(&mut h2, &RandMeter::spiky(seed, max, every, big));
            assert_eq!(got.2, base.2);
            assert_eq!(got.0.len(), base.0.len(), "meter {max}: groups");
            for ((g, a), (g2, b)) in base.0.iter().zip(&got.0) {
                assert_eq!(g, g2);
                assert_eq!(a.len(), b.len(), "meter {max}: group {g} image length");
                if let Some(i) = a.iter().zip(b).position(|(x, y)| x != y) {
                    panic!("meter {max}: group {g} images differ at byte {i}");
                }
            }
            res.push((max, got.1));
        }
        res
    });
    eprintln!("meter_cuts_same_bytes: {out:?}");
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
/// dropped), maintenance at a random cadence and in bursts under a meter whose readings jump, schema changes at
/// whatever point the rewrites under way have reached (a seal's liveness pass included), and a `clear`;
/// reads, totals and ownership checked. 2,000 writes a version; in the large suite, 10,000 with seed 7 and
/// meter 3. `NT_OPS`, `NT_SEED` and `NT_METER` set others.
#[test]
fn rows_model_versions() {
    let (d_ops, d_seed, d_meter) = if large() { (10_000, 7, 3) } else { (2_000, 0xB16, 1) };
    let ops = env_u64("NT_OPS", d_ops);
    let seed = env_u64("NT_SEED", d_seed);
    let metered = env_u64("NT_METER", d_meter);
    model_versions(ops, seed, metered);
}

/// `rows_model_versions` with `ops` writes a version, the given seed, and a meter whose readings jump by up to
/// `metered` × 3,000 (0: no meter).
fn model_versions(ops: u64, seed: u64, metered: u64) {
    let cols = model_cols();
    let digest = per_fill(|h, lh| {
        let meter = RandMeter::new(seed ^ 0x3E7E, metered.max(1) * 3_000);
        let one_unit = |tb: &mut Table<'_, VecHost>, units: &mut Units| -> Ran {
            if metered > 0 {
                one_unit_m(tb, units, &meter)
            } else {
                one_unit(tb, units)
            }
        };
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
            trace.push((version, model.len(), sealed_groups(&tb).len(), frozen(&tb)));
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
    eprintln!("rows_model_versions ({ops} writes a version, seed {seed}, meter {metered}): {digest:?}");
}

/// One image for the same live rows across a schema change: rows written at version 1 and sealed at version 2,
/// against the same rows sealed at version 1 and rewritten (compaction, or the column release) at version 2.
#[test]
fn seal_and_rewrite_bytes_across_versions() {
    let cols = model_cols();
    for variant in 0..6u32 {
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
                fill_group(&mut tb, &mut r, &mut model, &mut next_key);
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
/// root for every message under a meter whose readings jump, and, in the second run, one message in seven
/// trapped (the host's memory and the meter put back as they were before it) and run again: both runs end
/// with the same tables.
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
        let meter = RandMeter::new(5, 3_000);
        for i in 0..ops {
            // The message: drawn before it runs, so a replay runs the same one.
            let saved_r = r.0;
            let saved_model = model.clone();
            let saved_next = next_key;
            let change = i == ops / 2;
            let attempts = if trap_every > 0 && tr.below(trap_every) == 0 { 2 } else { 1 };
            let before = if attempts == 2 { Some(h.clone()) } else { None };
            // The meter's state is the message's: a replay reads what the trapped attempt read.
            let saved_meter = (meter.used.get(), meter.r.borrow().0);
            for attempt in 0..attempts {
                if attempt == 1 {
                    h = before.clone().unwrap();
                    r.0 = saved_r;
                    model = saved_model.clone();
                    next_key = saved_next;
                    meter.used.set(saved_meter.0);
                    meter.r.borrow_mut().0 = saved_meter.1;
                }
                let img = if version == 1 { img1.clone() } else { img2.clone() };
                let mut tb = Table::open(&mut h, root, &img).unwrap();
                tb.write_step(&mut key_scratch(), &meter).unwrap();
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

/// Post-snapshot patches of more than 254 units (a chained post-snapshot directory) while a compaction, then a
/// seal, holds the group, with deletes; at every step of the rewrite; reads, totals and ownership after.
/// In the large suite.
#[test]
fn post_snapshot_units_over_255() {
    if !large() {
        return;
    }
    let n = 620usize;
    let img = many_schema(n);
    for seal_it in [false, true] {
        for target in [seal::STEP_PASS1, seal::STEP_WRITE, seal::STEP_FLIP, seal::STEP_LAST] {
            let res = per_fill(|h, _| {
                let (mut tb, _) = Table::create(h, 91, &img).unwrap();
                let mut model = BTreeMap::new();
                let mut units = Units::default();
                let mut k = 0u64;
                while frozen(&tb) == 0 {
                    put_row(&mut tb, &mut model, many_row(n, k, &BTreeMap::new()));
                    k += 1;
                }
                let kind = if seal_it { seal::SEAL } else { seal::COMPACTION };
                if !seal_it {
                    run_all(&mut tb, &mut units);
                    let (_, pg) = sealed_groups(&tb)[0];
                    // A few patches before the rewrite.
                    let rows: Vec<u64> = model.keys().copied().take(30).collect();
                    for (i, key) in rows.iter().enumerate() {
                        let mut v = BTreeMap::new();
                        v.insert(i * 3, 7u64);
                        put_row(&mut tb, &mut model, many_row(n, *key, &v));
                    }
                    let tbl = tb.root();
                    native_table::table::jobs::queue(tb.host_mut(), tbl, 0, pg, false).unwrap();
                }
                let mut step = 0;
                let mut guard = 0;
                while step < target {
                    one_unit(&mut tb, &mut units);
                    let st = seal::running(tb.host(), tb.root(), kind).unwrap();
                    step = if st == 0 { 255 } else { seal::step_of(tb.host(), st).unwrap() };
                    guard += 1;
                    assert!(guard < 1_000_000);
                }
                // 600 distinct units over the group's rows, and deletes.
                let rows: Vec<u64> = model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == 0).map(|(k, _)| *k).collect();
                let mut vals: BTreeMap<u64, BTreeMap<usize, u64>> = BTreeMap::new();
                for unit in 0..600usize {
                    let key = rows[(unit * 7) % rows.len()];
                    let v = vals.entry(key).or_default();
                    v.insert(unit, 1 + (unit as u64 % 200));
                    put_row(&mut tb, &mut model, many_row(n, key, v));
                }
                for key in rows.iter().skip(1).step_by(11).take(100) {
                    tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(*key))])).unwrap();
                    model.remove(key);
                }
                let mid = bad_reads(&tb, &model).len();
                // Writes after each further unit.
                let mut i = 0usize;
                while rewriting(&tb) {
                    one_unit(&mut tb, &mut units);
                    let key = rows[(i * 13 + 5) % rows.len()];
                    if model.contains_key(&key) {
                        let v = vals.entry(key).or_default();
                        v.insert(600 + (i % 19), 3);
                        put_row(&mut tb, &mut model, many_row(n, key, v));
                    }
                    i += 1;
                }
                run_all(&mut tb, &mut units);
                let bad = bad_reads(&tb, &model);
                if bad.is_empty() {
                    check_groups(&tb);
                    check_ownership(&tb);
                }
                (step, mid, bad.len(), bad.first().map(|x| format!("{x:?}")))
            });
            assert_eq!(res.2, 0, "seal {seal_it} target {target}: bad reads after, first {:?}", res.3);
        }
    }
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
        let meter = RandMeter::new(9, 3_000);
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
                    one_unit_m(&mut tb, &mut units, &meter);
                }
                // Some sealed groups compacted by their patches and deletes.
                if i % 3_000 == 2_999 {
                    for (g, pg) in sealed_groups(&tb) {
                        if buffer::u64_of(tb.host(), pg, page::P_REWRITE).unwrap() == 0 && g % 2 == cycle % 2 {
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
                    one_unit_m(&mut tb, &mut units, &meter);
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

/// A seal under a meter whose readings jump by up to 60,000: every unit moves its cursors on, whatever was
/// spent before its pieces (a value of more than 64 bytes read as a piece of its own included).
#[test]
fn progress_under_a_jumpy_meter() {
    let m = model_schema();
    let max = env_u64("NT_STALL_MAX", 60_000);
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 93, &m.img).unwrap();
    let mut r = XorShift(0xC075);
    let mut model: Rows = BTreeMap::new();
    let mut next_key = 0u64;
    fill_group(&mut tb, &mut r, &mut model, &mut next_key);
    let meter = RandMeter::new(3, max);
    let mut last = (0u8, 0u64, 0u64, 0u64);
    let mut same = 0u64;
    for n in 0..200_000u64 {
        let (ran, _) = tb.unit(&meter).unwrap();
        if ran == Ran::Nothing {
            return;
        }
        let st = seal::running(tb.host(), tb.root(), seal::SEAL).unwrap();
        if st == 0 {
            continue;
        }
        let b = tb.host().get(Region::Block(st)).unwrap();
        let now = (
            seal::step_of(tb.host(), st).unwrap(),
            rd_u32(b, seal::RW_LEAF).unwrap() as u64,
            rd_u64(b, seal::RW_SUB).unwrap(),
            rd_u32(b, seal::RW_Q).unwrap() as u64,
        );
        if now == last {
            same += 1;
            if same == 1_000 {
                let q = now.3;
                let row = model.values().find(|(rid, _)| *rid == q).map(|(_, r)| r[1].1.clone());
                let len = match row {
                    Some(V::Bytes(b)) => b.len(),
                    _ => 0,
                };
                panic!("no progress over 1,000 units at (step, leaf, sub, q) {now:?}, unit {n}: a Text of {len} bytes");
            }
        } else {
            same = 0;
            last = now;
        }
    }
}

/// A seal (or compaction) past its snapshot, the group's rows' u32 values changed after the snapshot, and a
/// schema change (a column added and one dropped) at a given step: every row reads back its changed value.
#[test]
fn post_snapshot_updates_with_schema_change() {
    let cols = model_cols();
    let targets: Vec<(u8, usize)> = vec![
        (seal::STEP_PASS1, 0),
        (seal::STEP_PASS1, 300),
        (seal::STEP_LAYOUT, 0),
        (seal::STEP_WRITE, 0),
        (seal::STEP_FLIP, 0),
        (seal::STEP_LAST, 0),
    ];
    let change = env_u64("NT_POST_SNAPSHOT_CHANGE", 1);
    for compaction in [false, true] {
        for &(target, extra) in &targets {
            let res = per_fill(|h, _| {
                let mut e = Evo::new(&cols);
                let (mut tb, _) = Table::create(h, 94, &e.image()).unwrap();
                let mut r = XorShift(0x7A65);
                let mut model: Rows = BTreeMap::new();
                let mut next_key = 0u64;
                let mut units = Units::default();
                fill_group(&mut tb, &mut r, &mut model, &mut next_key);
                if compaction {
                    run_all(&mut tb, &mut units);
                    let pg = sealed_groups(&tb)[0].1;
                    let tbl = tb.root();
                    native_table::table::jobs::queue(tb.host_mut(), tbl, 0, pg, false).unwrap();
                }
                let kind = if compaction { seal::COMPACTION } else { seal::SEAL };
                let mut left = extra;
                let mut step;
                loop {
                    one_unit(&mut tb, &mut units);
                    let st = seal::running(tb.host(), tb.root(), kind).unwrap();
                    step = if st == 0 { 255 } else { seal::step_of(tb.host(), st).unwrap() };
                    if step > target || (step == target && left == 0) {
                        break;
                    }
                    if step == target {
                        left -= 1;
                    }
                }
                // Rows of the group whose u32 changes after the snapshot: 0 to 1, 1 to 2.
                let keys: Vec<u64> = model.keys().copied().take(300).collect();
                let mut changed = 0;
                for k in &keys {
                    let row = &model[k].1;
                    let to = match row[5].1 {
                        V::U(x) if x < 2 => Some(x + 1),
                        _ => None,
                    };
                    if let Some(to) = to {
                        if changed % 2 == 0 {
                            set_cell(&mut tb, &mut model, *k, id(5), V::U(to));
                        } else {
                            let mut row = model[k].1.clone();
                            row[5].1 = V::U(to);
                            put_row(&mut tb, &mut model, row);
                        }
                        changed += 1;
                    }
                }
                if change == 1 {
                    e.bump();
                    let a = id(e.add_col(&t::scalar(ty::BOOL), None));
                    e.drop_col(13);
                    tb.change_schema(&e.image()).unwrap();
                    evolve_model(&mut model, &[(a, V::Bool(false))], &[id(13)]);
                }
                run_all(&mut tb, &mut units);
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    check_reads(&tb, &model);
                    check_groups(&tb);
                    check_running(&tb, &model);
                }));
                (step, changed, res.is_ok())
            });
            assert!(res.2, "compaction {compaction} target {target}+{extra}: (at step, changed) {:?}", (res.0, res.1));
        }
    }
}

/// Runs units until the rewrite of `kind` reaches `target` and `extra` more units in it (or ends).
fn units_to(tb: &mut Table<'_, VecHost>, units: &mut Units, kind: u8, target: u8, extra: usize) -> u8 {
    let mut left = extra;
    let mut guard = 0;
    while seal::running(tb.host(), tb.root(), kind).unwrap() == 0 {
        assert_ne!(one_unit(tb, units), Ran::Nothing, "the rewrite starts");
        guard += 1;
        assert!(guard < 100_000, "the rewrite starts");
    }
    loop {
        let st = seal::running(tb.host(), tb.root(), kind).unwrap();
        let step = if st == 0 { 255 } else { seal::step_of(tb.host(), st).unwrap() };
        if step > target || (step == target && left == 0) {
            return step;
        }
        if step == target {
            left -= 1;
        }
        one_unit(tb, units);
    }
}

/// Long patch chains: a few sealed rows' Text updated thousands of times with changing lengths (short, medium
/// in block chains, huge in long values) and no maintenance, the chains far past `PATCH_BLOCKS_MAX`; then the
/// same while a compaction holds the group (post-snapshot chains); writes between the units after.
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
        for round in 0..2 {
            if round == 1 {
                // A compaction past its snapshot.
                let pg = root::group(tb.host(), tb.root(), 0).unwrap();
                let tbl = tb.root();
                native_table::table::jobs::queue(tb.host_mut(), tbl, 0, pg, false).unwrap();
                units_to(&mut tb, &mut units, seal::COMPACTION, seal::STEP_PASS1, 3);
            }
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
            let dir = if round == 0 {
                buffer::u64_of(tb.host(), pg, page::P_PATCH_DIR).unwrap()
            } else {
                let st = seal::running(tb.host(), tb.root(), seal::COMPACTION).unwrap();
                let pending = rd_u64(tb.host().get(Region::Block(st)).unwrap(), seal::RW_PENDING).unwrap();
                buffer::u64_of(tb.host(), pending, page::P_POST_PATCH).unwrap()
            };
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

// ---------------------------------------------------------------- values past one unit

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
        while frozen(&tb) == 0 {
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

/// Big Nat and Int values changed after a seal's snapshot, and again after a compaction's, with writes between
/// every unit of the rewrite's write step and after it: the totals are exact after every write, every row reads
/// back and every group keeps its count.
#[test]
fn big_values_changed_during_rewrites() {
    let img = schema(&[t::scalar(ty::NAT), t::scalar(ty::NAT), t::scalar(ty::INT)]).0;
    let row = |k: u64, salt: u64| -> Vec<(u32, V)> {
        let len = if (k + salt) % 7 == 0 { 3_000 } else if (k + salt) % 5 == 0 { 40 } else { 0 };
        if len > 0 {
            vec![(id(0), V::U(k)), (id(1), V::Nat(big_nat(k + salt, len))), (id(2), V::Int(low_zero_int(k + salt, len)))]
        } else {
            vec![(id(0), V::U(k)), (id(1), V::U(k * 7 + salt)), (id(2), V::I(-(k as i64) - salt as i64))]
        }
    };
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 112, &img).unwrap();
        let mut model: Rows = BTreeMap::new();
        let mut units = Units::default();
        let mut k = 0u64;
        while frozen(&tb) == 0 {
            put_row(&mut tb, &mut model, row(k, 0));
            k += 1;
        }
        let keys: Vec<u64> = model.keys().copied().filter(|k| *k < 4_096).collect();
        let mut want = BTreeMap::new();
        for (_, r) in model.values() {
            move_totals(&mut want, None, Some(r));
        }
        let mut salt = 1u64;
        for kind in [seal::SEAL, seal::COMPACTION] {
            if kind == seal::COMPACTION {
                let pg = root::group(tb.host(), tb.root(), 0).unwrap();
                let tbl = tb.root();
                native_table::table::jobs::queue(tb.host_mut(), tbl, 0, pg, false).unwrap();
            }
            // Changes after the snapshot.
            units_to(&mut tb, &mut units, kind, seal::STEP_PASS1, 0);
            for key in keys.iter().step_by(13) {
                if let Some((_, old)) = model.get(key).cloned() {
                    put_row(&mut tb, &mut model, row(*key, salt));
                    move_totals(&mut want, Some(&old), Some(&model[key].1));
                    check_totals(&tb, &want);
                }
            }
            units_to(&mut tb, &mut units, kind, seal::STEP_WRITE, 0);
            // Writes between the rewrite's units: updates to big and small values, and deletes.
            let mut i = 0usize;
            while seal::running(tb.host(), tb.root(), kind).unwrap() != 0 {
                one_unit(&mut tb, &mut units);
                salt += 1;
                let key = keys[(i * 37 + 11) % keys.len()];
                if let Some((_, old)) = model.get(&key).cloned() {
                    if i % 9 == 8 {
                        tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(key))])).unwrap();
                        model.remove(&key);
                    } else {
                        put_row(&mut tb, &mut model, row(key, salt));
                    }
                    move_totals(&mut want, Some(&old), model.get(&key).map(|(_, r)| r.as_slice()));
                    check_totals(&tb, &want);
                }
                i += 1;
            }
            run_all(&mut tb, &mut units);
            check_reads(&tb, &model);
            check_groups(&tb);
        }
        check_ownership(&tb);
        units
    });
    eprintln!("big_values_changed_during_rewrites: {digest:?}");
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
        while frozen(&tb) == 0 {
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

// ---------------------------------------------------------------- stale tables and the drain

/// A table opened with a newer schema image while its seal runs: its units run on the stored schema, each
/// within the bound, and leave it stale; its next write adopts the image.
#[test]
fn stale_table_units_adopt_nothing() {
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
        for _ in 0..40 {
            one_unit(&mut tb, &mut units);
        }
        e.bump();
        let a = id(e.add_col(&t::scalar(ty::NAT), Some(&[0x07])));
        let b = id(e.add_col(&t::opt(t::scalar(ty::TEXT)), None));
        let img2 = e.image();
        drop(tb);
        let mut tb = Table::open(h, root, &img2).unwrap();
        assert!(tb.stale());
        for _ in 0..200 {
            one_unit(&mut tb, &mut units);
            assert!(tb.stale(), "a unit adopts nothing");
        }
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
    eprintln!("stale_table_units_adopt_nothing: {digest:?}");
}

/// A frozen group none of whose rows holds a long value: its walk on the drain reads no row, so freeing its
/// buffer takes a few drain units; a group with a long row image, or a row with a long value, has its rows
/// read and their long values freed.
#[test]
fn drain_reads_rows_only_with_long_values() {
    let img = schema(&[t::scalar(ty::NAT), t::scalar(ty::NAT), t::scalar(ty::NAT), t::scalar(ty::TEXT)]).0;
    let digest = per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 115, &img).unwrap();
        let mut model: Rows = BTreeMap::new();
        let mut units = Units::default();
        let mut drains = Vec::new();
        for long in [false, true] {
            let first = model.len() as u64;
            let before = frozen(&tb);
            let mut k = first;
            while frozen(&tb) == before {
                let text = if long && k % 512 == 3 { vec![b'x'; HUGE_VALUE as usize + 10] } else { b"t".to_vec() };
                put_row(&mut tb, &mut model, vec![(id(0), V::U(k)), (id(1), V::U(k)), (id(2), V::U(k * 3)), (id(3), V::Bytes(text))]);
                k += 1;
            }
            let desc = root::u64_at(tb.host(), tb.root(), root::FROZEN_HEAD).unwrap();
            let flags = buffer::u16_of(tb.host(), desc, buffer::D_FLAGS).unwrap();
            assert_eq!(flags & u64::from(buffer::LONG_ROWS) != 0, long, "the frozen group's long-values flag");
            if long {
                // A row of the frozen group updated to a long row image too.
                let key = *model.keys().nth(first as usize + 7).unwrap();
                let mut row = model[&key].1.clone();
                row[3].1 = V::Bytes(vec![b'y'; 5_000]);
                put_row(&mut tb, &mut model, row);
            }
            let mut n = 0u64;
            while one_unit(&mut tb, &mut units) != Ran::Nothing {
                n += 1;
            }
            drains.push((units.by.get("Drain").map(|x| x.2).unwrap_or(0), n));
            check_reads(&tb, &model);
            check_ownership(&tb);
        }
        drains
    });
    eprintln!("drain_reads_rows_only_with_long_values: (drain units so far, units) {digest:?}");
    assert!(digest[0].0 <= 40, "a group with no long value is freed in a few drain units: {digest:?}");
}

// ---------------------------------------------------------------- leaves added or dropped while a seal runs

/// A column added after a seal's snapshot, a frozen row's value of it written during the seal (a
/// post-snapshot patch, so the sealed image lacks the leaf), the seal ended, then the same row's value written
/// again by `set` or by upsert, to another value and to null: every row reads back.
#[test]
fn late_leaf_written_again_after_seal() {
    let cols = model_cols();
    for (second, upsert) in [
        (V::Bytes(b"xyz".to_vec()), false),
        (V::Bytes(b"xyz".to_vec()), true),
        (V::Null, false),
        (V::Null, true),
    ] {
        let res = per_fill(|h, _| {
            let mut e = Evo::new(&cols);
            let (mut tb, _) = Table::create(h, 120, &e.image()).unwrap();
            let mut r = XorShift(0x7A7E);
            let mut model: Rows = BTreeMap::new();
            let mut next_key = 0u64;
            let mut units = Units::default();
            fill_group(&mut tb, &mut r, &mut model, &mut next_key);
            units_to(&mut tb, &mut units, seal::SEAL, seal::STEP_PASS1, 0);
            e.bump();
            let a = id(e.add_col(&t::opt(t::scalar(ty::TEXT)), None));
            tb.change_schema(&e.image()).unwrap();
            evolve_model(&mut model, &[(a, V::Null)], &[]);
            let k = *model.keys().next().unwrap();
            set_cell(&mut tb, &mut model, k, a, V::Bytes(b"abc".to_vec()));
            run_all(&mut tb, &mut units);
            check_reads(&tb, &model);
            if upsert {
                let mut row = model[&k].1.clone();
                row.iter_mut().find(|(x, _)| *x == a).unwrap().1 = second.clone();
                put_row(&mut tb, &mut model, row);
            } else {
                set_cell(&mut tb, &mut model, k, a, second.clone());
            }
            check_reads(&tb, &model);
            check_groups(&tb);
            check_running(&tb, &model);
            model.len()
        });
        eprintln!("late_leaf_written_again_after_seal: {second:?}, upsert {upsert}: {res:?}");
    }
}

/// As `late_leaf_written_again_after_seal`, for a Bool and a Nat column added after the seal's snapshot: the
/// second write succeeds, the rows read back and the Nat's total matches the live rows.
#[test]
fn late_leaf_kinds_written_again_after_seal() {
    let cols = model_cols();
    let kinds: Vec<(&str, Vec<u8>, Option<Vec<u8>>, V, V, V)> = vec![
        ("bool", t::scalar(ty::BOOL), None, V::Bool(false), V::Bool(true), V::Bool(false)),
        ("nat", t::scalar(ty::NAT), Some(vec![0x05]), V::U(5), V::U(1_000), V::U(7)),
    ];
    for (name, ty_img, dflt, def, first, second) in kinds {
        let res = per_fill(|h, _| {
            let mut e = Evo::new(&cols);
            let (mut tb, _) = Table::create(h, 121, &e.image()).unwrap();
            let mut r = XorShift(0x7A7F);
            let mut model: Rows = BTreeMap::new();
            let mut next_key = 0u64;
            let mut units = Units::default();
            fill_group(&mut tb, &mut r, &mut model, &mut next_key);
            units_to(&mut tb, &mut units, seal::SEAL, seal::STEP_PASS1, 0);
            e.bump();
            let a = id(e.add_col(&ty_img, dflt.as_deref()));
            tb.change_schema(&e.image()).unwrap();
            evolve_model(&mut model, &[(a, def.clone())], &[]);
            let k = *model.keys().next().unwrap();
            set_cell(&mut tb, &mut model, k, a, first.clone());
            run_all(&mut tb, &mut units);
            set_cell(&mut tb, &mut model, k, a, second.clone());
            check_reads(&tb, &model);
            check_groups(&tb);
            check_running(&tb, &model);
            // Every row of the group again, in turn to the first value and the second.
            let keys: Vec<u64> = model.keys().copied().take(40).collect();
            for (i, k) in keys.iter().enumerate() {
                set_cell(&mut tb, &mut model, *k, a, if i % 2 == 0 { first.clone() } else { second.clone() });
            }
            check_reads(&tb, &model);
            check_groups(&tb);
            check_running(&tb, &model);
            model.len()
        });
        eprintln!("late_leaf_kinds_written_again_after_seal: {name}: {res:?}");
    }
}

/// A column dropped once a seal has taken its snapshot (its pass 1 begun): the seal writes the dropped leaf,
/// and the column release cleans the group once the seal has flipped, before it clears the release flag.
#[test]
fn drop_during_seal_pass1_released() {
    let cols = model_cols();
    let res = per_fill(|h, _| {
        let mut e = Evo::new(&cols);
        let (mut tb, _) = Table::create(h, 132, &e.image()).unwrap();
        let mut r = XorShift(0xB0DA);
        let mut model: Rows = BTreeMap::new();
        let mut next_key = 0u64;
        let mut units = Units::default();
        fill_group(&mut tb, &mut r, &mut model, &mut next_key);
        units_to(&mut tb, &mut units, seal::SEAL, seal::STEP_PASS1, 0);
        e.bump();
        e.drop_col(13);
        tb.change_schema(&e.image()).unwrap();
        evolve_model(&mut model, &[], &[id(13)]);
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        no_dropped_cells(&tb);
        let flags = root::u8_at(tb.host(), tb.root(), root::FLAGS).unwrap();
        (flags & root::FLAG_RELEASE, root::u32_at(tb.host(), tb.root(), root::CLEANED).unwrap())
    });
    assert_eq!(res, (0, 1), "(release flag, cleaned columns)");
}

/// A column dropped at unit boundaries all through a seal (every `stride`th unit and the first four of each
/// step), with and without writes of the group's rows after the drop: once maintenance ends, no sealed group
/// holds the dropped leaf and the release flag is clear.
#[test]
fn drop_at_seal_unit_boundaries() {
    let cols = model_cols();
    let stride = env_u64("NT_STRIDE", if large() { 23 } else { 500 });
    let mut h = VecHost::new(FILLS[0]);
    let mut model: Rows = BTreeMap::new();
    let mut units = Units::default();
    let e0 = Evo::new(&cols);
    let img0 = e0.image();
    let root = {
        let (mut tb, root) = Table::create(&mut h, 133, &img0).unwrap();
        let mut r = XorShift(0xB0DA);
        let mut next_key = 0u64;
        fill_group(&mut tb, &mut r, &mut model, &mut next_key);
        root
    };
    let mut fails = Vec::new();
    let (mut n, mut branches, mut started, mut last_step, mut in_step) = (0u64, 0u64, false, 255u8, 0u64);
    loop {
        let step = {
            let tb = Table::open(&mut h, root, &img0).unwrap();
            let st = seal::running(tb.host(), root, seal::SEAL).unwrap();
            if st == 0 { 255 } else { seal::step_of(tb.host(), st).unwrap() }
        };
        if step != 255 {
            started = true;
        }
        if started && step == 255 {
            break;
        }
        if step != last_step {
            last_step = step;
            in_step = 0;
        }
        if n % stride == 0 || in_step < 4 {
            for writes in [false, true] {
                branches += 1;
                let mut h2 = h.clone();
                let mut model2 = model.clone();
                let mut e = Evo::new(&cols);
                e.bump();
                e.drop_col(13);
                let b = id(e.add_col(&t::scalar(ty::BOOL), None));
                let img = e.image();
                let mut tb = Table::open(&mut h2, root, &img0).unwrap();
                tb.change_schema(&img).unwrap();
                evolve_model(&mut model2, &[(b, V::Bool(false))], &[id(13)]);
                if writes {
                    let keys: Vec<u64> = model2.keys().copied().take(60).collect();
                    for (i, k) in keys.iter().enumerate() {
                        set_cell(&mut tb, &mut model2, *k, b, V::Bool(i % 2 == 0));
                    }
                }
                let mut u2 = Units::default();
                let mut guard = 0u64;
                while one_unit(&mut tb, &mut u2) != Ran::Nothing {
                    guard += 1;
                    assert!(guard < 1_000_000, "maintenance ends");
                }
                check_reads(&tb, &model2);
                let dropped = dropped_ordinals(&tb);
                let clean = sealed_groups(&tb).iter().all(|&(_, pg)| {
                    let gv = GroupView::open(tb.host(), pg).unwrap();
                    dropped.iter().all(|&o| gv.leaf_by_ordinal(o).unwrap().is_none())
                });
                let flag = root::u8_at(tb.host(), tb.root(), root::FLAGS).unwrap() & root::FLAG_RELEASE;
                if !clean || flag != 0 {
                    fails.push((n, step, writes, clean, flag));
                }
            }
        }
        let mut tb = Table::open(&mut h, root, &img0).unwrap();
        assert_ne!(one_unit(&mut tb, &mut units), Ran::Nothing);
        n += 1;
        in_step += 1;
    }
    eprintln!("drop_at_seal_unit_boundaries: {n} seal units, {branches} branches");
    assert!(fails.is_empty(), "(unit, step, writes, no dropped cells, release flag) {fails:?}");
}

/// The model's rows across schema versions at 10,000 writes a version, with no meter, for two seeds that
/// once met a column added after a seal's snapshot and written again after the seal.
#[test]
fn rows_model_versions_late_leaves() {
    if !large() && env_u64("NT_LATE", 0) == 0 {
        eprintln!("rows_model_versions_late_leaves runs in the large suite (NT_LARGE=1)");
        return;
    }
    for seed in [3u64, 46] {
        model_versions(10_000, seed, 0);
    }
}

// ---------------------------------------------------------------- leaves added during rewrites

/// The text of a check's panic, or "ok".
fn checked(f: impl FnOnce()) -> String {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(()) => "ok".to_string(),
        Err(p) => p.downcast_ref::<String>().cloned().unwrap_or_default().chars().take(300).collect(),
    }
}

/// Columns added while a seal holds a frozen group past its snapshot, and some of the group's rows given
/// values of them during the seal (one row written, then deleted): after the flip every row reads back and
/// the new Nat leaf's total counts those values.
#[test]
fn late_leaf_record_through_a_seal() {
    let cols = model_cols();
    let out = per_fill(|h, _| {
        let mut e = Evo::new(&cols);
        let (mut tb, _) = Table::create(h, 171, &e.image()).unwrap();
        let mut r = XorShift(0x1A7E);
        let mut model: Rows = BTreeMap::new();
        let mut next_key = 0u64;
        let mut units = Units::default();
        fill_group(&mut tb, &mut r, &mut model, &mut next_key);
        units_to(&mut tb, &mut units, seal::SEAL, seal::STEP_PASS1, 0);
        e.bump();
        let a = id(e.add_col(&t::scalar(ty::NAT), Some(&[0xAC, 0x02])));
        let b = id(e.add_col(&t::opt(t::scalar(ty::TEXT)), None));
        let c = id(e.add_col(&t::scalar(ty::BOOL), None));
        tb.change_schema(&e.image()).unwrap();
        evolve_model(&mut model, &[(a, V::U(300)), (b, V::Null), (c, V::Bool(false))], &[]);
        let g0: Vec<u64> =
            model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == 0).map(|(k, _)| *k).take(12).collect();
        for k in g0.iter().take(10) {
            set_cell(&mut tb, &mut model, *k, a, V::U(7));
            set_cell(&mut tb, &mut model, *k, b, V::Bytes(b"late".to_vec()));
            set_cell(&mut tb, &mut model, *k, c, V::Bool(true));
        }
        // A row written, then deleted, during the seal.
        set_cell(&mut tb, &mut model, g0[10], a, V::U(9));
        let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(g0[10]))])).unwrap();
        assert_eq!(gone, model.remove(&g0[10]).map(|x| x.0));
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        check_running(&tb, &model);
        check_groups(&tb);
        model.values().filter(|(rid, _)| rid / GROUP_SLOTS == 0).count() as u32
    });
    assert!(out > 1_000, "the group's live rows: {out}");
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
            while frozen(&tb) == 0 {
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

/// A Nat column with a declared default past 64 bits added after a compaction's or a seal's snapshot, then a
/// row of the rewrite's group given a value of it: the leaf is later than the snapshot; every row reads back
/// and its total counts every row.
fn big_default_under(seal_it: bool) -> String {
    checked(|| {
        per_fill(|h, _| {
            let mut e = Evo::new(&[t::scalar(ty::NAT), t::scalar(ty::U16)]);
            let (mut tb, _) = Table::create(h, 332, &e.image()).unwrap();
            let mut model: Rows = BTreeMap::new();
            let mut units = Units::default();
            let mut k = 0u64;
            while frozen(&tb) == 0 {
                put_row(&mut tb, &mut model, vec![(id(0), V::U(k)), (id(1), V::U(k % 7))]);
                k += 1;
            }
            let kind = if seal_it {
                seal::SEAL
            } else {
                run_all(&mut tb, &mut units);
                let pg = root::group(tb.host(), tb.root(), 0).unwrap();
                let tbl = tb.root();
                native_table::table::jobs::queue(tb.host_mut(), tbl, 0, pg, false).unwrap();
                seal::COMPACTION
            };
            units_to(&mut tb, &mut units, kind, seal::STEP_PASS1, 0);
            e.bump();
            let (nat, _) = past_64_bits();
            let a = id(e.add_col(&t::scalar(ty::NAT), Some(&nat)));
            tb.change_schema(&e.image()).unwrap();
            evolve_model(&mut model, &[(a, V::Nat(nat.clone()))], &[]);
            set_cell(&mut tb, &mut model, 3, a, V::U(5));
            run_all(&mut tb, &mut units);
            check_reads(&tb, &model);
            check_groups(&tb);
            check_running(&tb, &model);
        })
    })
}

#[test]
fn big_default_first_write_under_rewrite() {
    let out: Vec<String> = [false, true].iter().map(|&s| format!("seal {s}: {}", big_default_under(s))).collect();
    assert!(out.iter().all(|x| x.ends_with(": ok")), "{out:#?}");
}

/// The same frozen rows give one group image whatever their history. A group is sealed as inserted, and
/// again after a quarter of its rows were updated while frozen: a Bool set and set back by `set` (`how` 0)
/// or by upsert (1), or by `set` after a schema change that adds a column (2). Such an update writes each of
/// the row's items longer than `LONG_VALUE` as a long value, written or not. The seal places a value of at
/// most `HUGE_VALUE` bytes by its length, out of line, as the first seal and a compaction of the same rows
/// do; it adopts only longer ones. A compaction of the second group gives the same image again, and every
/// page of area 15 is owned once after each.
fn frozen_update_images(how: u32) -> (usize, u32, u32) {
    let cols = model_cols();
    per_fill(|h, _| {
        let mut out = Vec::new();
        let mut updated_medium = 0u32;
        for pass in 0..2 {
            // Both tables in the one host, one after the other, with the same uid.
            let mut e = Evo::new(&cols);
            let (mut tb, _) = Table::create(&mut *h, 400, &e.image()).unwrap();
            let mut r = XorShift(0xB0F);
            let mut model: Rows = BTreeMap::new();
            let mut next_key = 0u64;
            let mut units = Units::default();
            fill_group(&mut tb, &mut r, &mut model, &mut next_key);
            if how == 2 {
                e.bump();
                let a = id(e.add_col(&t::scalar(ty::NAT), Some(&[0xAC, 0x02])));
                tb.change_schema(&e.image()).unwrap();
                evolve_model(&mut model, &[(a, V::U(300))], &[]);
            }
            if pass == 1 {
                let keys: Vec<u64> = model.keys().copied().filter(|k| k % 4 == 1).collect();
                for k in keys {
                    if matches!(&model[&k].1[1].1, V::Bytes(b) if b.len() as u64 > LONG_VALUE) {
                        updated_medium += 1;
                    }
                    let was = model[&k].1[2].1.clone();
                    let flipped = V::Bool(was != V::Bool(true));
                    if how == 1 {
                        for v in [flipped, was] {
                            let mut row = model[&k].1.clone();
                            row[2].1 = v;
                            put_row(&mut tb, &mut model, row);
                        }
                    } else {
                        set_cell(&mut tb, &mut model, k, id(2), flipped);
                        set_cell(&mut tb, &mut model, k, id(2), was);
                    }
                }
            }
            run_all(&mut tb, &mut units);
            check_reads(&tb, &model);
            check_groups(&tb);
            check_ownership(&tb);
            let pg = root::group(tb.host(), tb.root(), 0).unwrap();
            let lv = rd_u32(tb.host().get(Region::Block(pg)).unwrap(), page::P_LV_COUNT).unwrap();
            out.push((group_image(&tb, pg), lv));
            if pass == 1 {
                let tbl = tb.root();
                native_table::table::jobs::queue(tb.host_mut(), tbl, 0, pg, false).unwrap();
                run_all(&mut tb, &mut units);
                assert!(units.by.keys().any(|x| x.starts_with("Compaction")), "{units:?}");
                check_reads(&tb, &model);
                check_groups(&tb);
                check_ownership(&tb);
                let pg = root::group(tb.host(), tb.root(), 0).unwrap();
                let lv = rd_u32(tb.host().get(Region::Block(pg)).unwrap(), page::P_LV_COUNT).unwrap();
                out.push((group_image(&tb, pg), lv));
            }
        }
        assert!(updated_medium > 0, "some updated rows hold a medium Text");
        for (i, what) in [(1, "the seal after updates"), (2, "the compaction")] {
            assert_eq!(
                (out[0].0.len(), out[0].1),
                (out[i].0.len(), out[i].1),
                "how {how}: image length and long-value list of the first seal and {what}"
            );
            if let Some(b) = out[0].0.iter().zip(&out[i].0).position(|(x, y)| x != y) {
                panic!("how {how}: the first seal's image and {what}'s differ at byte {b}");
            }
        }
        (out[0].0.len(), out[0].1, updated_medium)
    })
}

#[test]
fn frozen_update_seal_bytes() {
    for how in 0..3 {
        let (len, lv, medium) = frozen_update_images(how);
        eprintln!("how {how}: image {len} bytes, long-value list {lv}, updated rows with a medium Text {medium}");
    }
}

/// A host meter whose every reading advances by the same `jump`.
struct JumpMeter {
    used: std::cell::Cell<u64>,
    jump: u64,
}

impl native_table::Budget for JumpMeter {
    fn used(&self) -> u64 {
        self.used.set(self.used.get() + self.jump);
        self.used.get()
    }
    fn limit(&self) -> u64 {
        u64::MAX
    }
}

// ---------------------------------------------------------------- rewrites of groups with no live row

/// The write step's phases whose walk over the leaves a snapshot with no live row leaves without output: the
/// cells (3) and the segment table (6); and the step's end (7).
const W_CELLS: u64 = 3;
const W_SEGTABLE: u64 = 6;
const W_DONE: u64 = 7;

/// Where the write step of the rewrite at state block `st` stands: (phase, leaf), before the step (0, 0),
/// after it (`W_DONE`, 0).
fn write_cursor(tb: &Table<'_, VecHost>, st: u64) -> (u64, u64) {
    if st == 0 {
        return (W_DONE, 0);
    }
    let step = seal::step_of(tb.host(), st).unwrap();
    if step < seal::STEP_WRITE {
        return (0, 0);
    }
    if step > seal::STEP_WRITE {
        return (W_DONE, 0);
    }
    let b = tb.host().get(Region::Block(st)).unwrap();
    (rd_u64(b, seal::RW_SUB).unwrap() & 0xFF, u64::from(rd_u32(b, seal::RW_LEAF).unwrap()))
}

/// The leaves of a table of `leaves` leaves crossed in the cells and segment-table phases from write cursor
/// `from` to `to`.
fn leaves_crossed(from: (u64, u64), to: (u64, u64), leaves: u64) -> u64 {
    [W_CELLS, W_SEGTABLE]
        .iter()
        .map(|&p| {
            if p < from.0 || p > to.0 {
                return 0;
            }
            let a = if p == from.0 { from.1 } else { 0 };
            let b = if p == to.0 { to.1 } else { leaves };
            b.saturating_sub(a)
        })
        .sum()
}

/// Runs units on `meter` until the rewrite of `kind` ends, the others too; returns the most leaves one unit
/// crossed in its write step's cells and segment-table phases, and the live rows of its snapshot (each write
/// step's).
fn run_crossing<B: native_table::Budget>(
    tb: &mut Table<'_, VecHost>,
    units: &mut Units,
    meter: &B,
    kind: u8,
    leaves: u64,
) -> (u64, Vec<u64>) {
    let mut most = 0;
    let mut live = Vec::new();
    let mut guard = 0;
    loop {
        let st = seal::running(tb.host(), tb.root(), kind).unwrap();
        let from = write_cursor(tb, st);
        if st != 0 && seal::step_of(tb.host(), st).unwrap() == seal::STEP_WRITE {
            let n = u64::from(rd_u16(tb.host().get(Region::Block(st)).unwrap(), seal::RW_N).unwrap());
            if live.last() != Some(&n) {
                live.push(n);
            }
        }
        if one_unit_m(tb, units, meter) == Ran::Nothing {
            break;
        }
        // The same state block until the rewrite ends.
        let after = seal::running(tb.host(), tb.root(), kind).unwrap();
        if st != 0 && (after == st || after == 0) {
            most = most.max(leaves_crossed(from, write_cursor(tb, after), leaves));
        }
        guard += 1;
        assert!(guard < 1_000_000, "maintenance ends");
    }
    (most, live)
}

/// A rewrite whose snapshot holds no live row, in compacted form (the group has at least `COMPACT_DEAD` dead
/// positions, so its stored count is 0): every leaf's cells are empty and no leaf has a segment, yet each leaf
/// the write step crosses costs the replica several thousand instructions (the leaf found, its plan loaded,
/// its segments walked). Two such rewrites of a group of a key and 200 u8 leaves: a seal of a frozen group
/// whose rows were all deleted before it began (`compaction` false), and a compaction of a sealed group
/// whose last live rows were deleted at its snapshot (true). With estimates only (`metered` false) and under
/// a meter whose readings advance by 5,000, no unit may cross more than `UNIT_WORK / 4_000` leaves in the two
/// phases. Returns the most one unit crossed and the snapshot's live rows.
fn empty_snapshot_case(compaction: bool, metered: bool) -> (u64, Vec<u64>) {
    let n = 200usize;
    let leaves = n as u64 + 1;
    let img = many_schema(n);
    per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 460, &img).unwrap();
        let mut model = BTreeMap::new();
        let mut units = Units::default();
        let mut k = 0u64;
        while frozen(&tb) == 0 {
            put_row(&mut tb, &mut model, many_row(n, k, &BTreeMap::new()));
            k += 1;
        }
        let rows: Vec<u64> = model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == 0).map(|(k, _)| *k).collect();
        assert!(rows.len() as u64 >= COMPACT_DEAD, "the group has {} rows", rows.len());
        let remove = |tb: &mut Table<'_, VecHost>, model: &mut Rows, keys: &[u64]| {
            for k in keys {
                tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(*k))])).unwrap();
                model.remove(k);
            }
        };
        let kind = if compaction {
            run_all(&mut tb, &mut units);
            // More than `COMPACT_DEAD` rows deleted queue the compaction; the rest at its snapshot.
            let cut = COMPACT_DEAD as usize + 10;
            remove(&mut tb, &mut model, &rows[..cut]);
            units_to(&mut tb, &mut units, seal::COMPACTION, seal::STEP_SNAPSHOT, 0);
            remove(&mut tb, &mut model, &rows[cut..]);
            seal::COMPACTION
        } else {
            remove(&mut tb, &mut model, &rows);
            seal::SEAL
        };
        let meter = JumpMeter { used: std::cell::Cell::new(0), jump: 5_000 };
        let out = if metered {
            run_crossing(&mut tb, &mut units, &meter, kind, leaves)
        } else {
            run_crossing(&mut tb, &mut units, &everything(), kind, leaves)
        };
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_ownership(&tb);
        out
    })
}

#[test]
fn empty_snapshot_units_bounded() {
    let mut out = Vec::new();
    for compaction in [false, true] {
        for metered in [false, true] {
            let mut res = None;
            let ran = checked(|| res = Some(empty_snapshot_case(compaction, metered)));
            let ok = matches!(&res, Some((most, live)) if *most <= UNIT_WORK / 4_000 && live == &[0]);
            out.push((
                ok,
                format!("compaction {compaction}, metered {metered}: {ran}, (most leaves a unit, live rows) {res:?}"),
            ));
        }
    }
    for (_, s) in &out {
        eprintln!("{s}");
    }
    assert!(out.iter().all(|(ok, _)| *ok), "{out:#?}");
}
