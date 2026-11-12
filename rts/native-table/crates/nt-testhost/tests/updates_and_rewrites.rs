//! Updates of sealed rows (in place and in patch columns), rewrites of whole groups (the seal a filling insert
//! runs, compaction), group drops, and the maintenance step.
//!
//! Every test runs once per fill pattern (`FILLS`) through `per_fill`, which requires identical results and
//! identical host-call logs from the two runs. Changed-block counts come from `VecHost::end_message`.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use native_table::area::{AreaRecord, IMAGE_AREA};
use native_table::consts::{COMPACT_DEAD, GROUP_SLOTS, HUGE_VALUE, LONG_VALUE, PATCH_UNITS_PER_DIR};
use native_table::fault::Fault;
use native_table::mem::{rd_u16, rd_u32, rd_u64};
use native_table::schema::SchemaView;
use native_table::table::group::GroupView;
use native_table::table::jobs::Ran;
use native_table::table::layout::Ty;
use native_table::table::row::{Item, RowRef};
use native_table::table::{buffer, page, patch, root, CellIn, RowSource, Table, Upsert};
use native_table::value::ty;
use native_table::{CorruptAt, Corruption, ImageId, PageHost, Progress, Region, Wake};
use nt_testhost::big::Big;
use nt_testhost::rows::{schema, schema_versioned, t, uleb, Sink, Src, VCol, V};
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
        if l.ty.running().is_some() {
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
            let map = rd_u64(h.get(Region::Block(u.first)).unwrap(), patch::B_MAP).unwrap();
            let mut b = u.first;
            let mut j = 0u64;
            while b != 0 {
                let s = h.get(Region::Block(b)).unwrap();
                let n = rd_u16(s, patch::B_ENTRIES).unwrap() as u64;
                let base = if j == 0 { patch::FIRST_ENTRIES } else { patch::LATER_ENTRIES };
                for i in 0..n {
                    let at = base + i * patch::ENTRY;
                    let flags = s[at as usize + 2];
                    // The position map names the block of each live entry; a chain without one has one block.
                    if flags & patch::F_DEAD == 0 {
                        let pos = rd_u16(s, at).unwrap() as usize;
                        let k = if map == 0 { 0 } else { h.get(Region::Block(map)).unwrap()[pos] as u64 };
                        assert_eq!(k, j.min(255), "the position map at position {pos}");
                    }
                    if flags & patch::F_DEAD == 0 && flags & patch::F_LONG != 0 {
                        let data = rd_u16(s, at + 6).unwrap() as u64;
                        if s[data as usize..data as usize + 24].iter().any(|&x| x != 0) {
                            add(&native_table::image::Image::read_ref(s, data).unwrap());
                        }
                    }
                }
                b = rd_u64(s, patch::B_NEXT).unwrap();
                j += 1;
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
                        native_table::table::jobs::queue(tb.host_mut(), tbl, 0, pg).unwrap();
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

// ---------------------------------------------------------------- growing updates

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
        assert_eq!(tb.wake().unwrap(), Wake::Later, "past 254 units a compaction is pending");
        // No fold trigger passed: the 255th unit recorded the group in the root for the next step to push,
        // and no write changed the compaction queue.
        assert_eq!(root::u64_at(tb.host(), tb.root(), root::GROUP_TO_QUEUE).unwrap(), 1, "group 0 recorded");
        assert_eq!(root::u64_at(tb.host(), tb.root(), root::COMPACT_HEAD).unwrap(), 0, "the queue is untouched");
        let mut units = Units::default();
        assert_eq!(one_unit(&mut tb, &mut units), Ran::Compaction, "the next step pushes it and compacts");
        assert_eq!(root::u64_at(tb.host(), tb.root(), root::GROUP_TO_QUEUE).unwrap(), 0);
        // The compaction folds them all.
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

/// Maintenance withheld: writes go on past every threshold that queues a compaction (patch chains and units) and run no
/// maintenance of their own beyond the seal of the group an insert fills: every group sealed before a write
/// keeps its page (no compaction or drop).
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
        for step in 0..12_000u64 {
            let before = sealed_groups(&tb);
            random_write(&mut tb, &mut r, &mut model, &mut next_key, false);
            let after = sealed_groups(&tb);
            assert!(before.iter().all(|x| after.contains(x)), "step {step}: a write rewrote or dropped a group");
            assert!(after.len() <= before.len() + 1, "step {step}: a write sealed one group at most");
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
        (model.len(), units)
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

// ---------------------------------------------------------------- the model

// ---------------------------------------------------------------- opening a table in place

/// A test host's memory through a shared borrow, every write refused: a test can open a table in place, as a
/// runtime does.
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
    fn test_fill(&self) -> Option<u8> {
        self.0.test_fill()
    }
}

/// A table opened in place reads its rows. The stored page's hash is checked when the table's shape is derived
/// from it, not at every open: a stored page corrupted after its shape was stored opens with the stored shape,
/// and is refused once an open derives the shape from it again (here, after a byte of the stored shape
/// changed).
#[test]
fn open_with_schema_page_in_place() {
    let (img, _) = schema(&[t::scalar(ty::NAT), t::scalar(ty::TEXT)]);
    per_fill(|h, _| {
        let (root, sp) = {
            let (mut tb, root) = Table::create(h, 45, &img).unwrap();
            let mut sc = key_scratch();
            for k in 0..5u64 {
                tb.upsert(&mut sc, &mut Src::new(&[(id(0), V::U(k)), (id(1), V::text("a"))])).unwrap();
            }
            (root, root::u64_at(tb.host(), root, root::SCHEMA_PAGE).unwrap())
        };
        let open = |h: &VecHost| Table::open(&mut InPlace(h), root).map(|tb| tb.len().unwrap());
        assert_eq!(open(h), Ok(5), "in place");
        // The last column's id raised by one: still in order, but not the row type the hash was taken of.
        let stored = h.get(Region::Page(sp)).unwrap().to_vec();
        let v = SchemaView::open(&stored).unwrap();
        let at = v.layout.columns as usize + 24 * (usize::from(v.columns) - 1);
        let refused = Err(Fault::Corrupt(CorruptAt { image: ImageId::Runtime, kind: Corruption::Header, offset: 24 }));
        h.get_mut(Region::Page(sp)).unwrap()[at] += 1;
        assert_eq!(open(h), Ok(5), "the corrupted stored page, its shape stored");
        h.get_mut(Region::Block(root)).unwrap()[root::SHAPE as usize] ^= 1;
        assert_eq!(open(h), refused, "the corrupted stored page, its shape derived again");
    });
}

// ---------------------------------------------------------------- messages

/// Every sealed group's image, by group.
fn all_images(tb: &Table<'_, VecHost>) -> Vec<(u64, Vec<u8>)> {
    sealed_groups(tb).into_iter().map(|(g, pg)| (g, group_image(tb, pg))).collect()
}

fn env_u64(name: &str, d: u64) -> u64 {
    std::env::var(name).ok().and_then(|s| s.parse().ok()).unwrap_or(d)
}

/// Messages as a canister runs them (the write-first step, then the write), the table opened anew from its
/// root for every message, and, in the second run, one message in seven trapped (the host's memory put back as
/// it was before it) and run again: both runs end with the same tables.
#[test]
fn crash_equivalence() {
    let cols = model_cols();
    let ops = env_u64("NT_CRASH_OPS", 12_000);
    let run = |trap_every: u64| -> (Vec<(u64, Vec<u8>)>, Vec<(u64, Vec<(u32, V)>)>, usize) {
        let (img, _) = schema(&cols);
        let mut h = VecHost::new(FILLS[0]);
        let root = Table::create(&mut h, 90, &img).unwrap().1;
        let mut r = XorShift(0xC4A5);
        let mut tr = XorShift(0x7A9);
        let mut model: Rows = BTreeMap::new();
        let mut next_key = 0u64;
        for _ in 0..ops {
            // The message: drawn before it runs, so a replay runs the same one.
            let saved_r = r.0;
            let saved_model = model.clone();
            let saved_next = next_key;
            let attempts = if trap_every > 0 && tr.below(trap_every) == 0 { 2 } else { 1 };
            let before = if attempts == 2 { Some(h.clone()) } else { None };
            for attempt in 0..attempts {
                if attempt == 1 {
                    h = before.clone().unwrap();
                    r.0 = saved_r;
                    model = saved_model.clone();
                    next_key = saved_next;
                }
                let mut tb = Table::open(&mut h, root).unwrap();
                tb.write_step().unwrap();
                let op = r.below(10);
                if op < 5 || model.len() < 20 {
                    let mut row = vec![(id(0), V::U(next_key))];
                    for c in 1..16u32 {
                        row.push((id(c), model_cell(&mut r, c, true)));
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
                    let col = row[c].0 >> 16;
                    row[c].1 = model_cell(&mut r, col, true);
                    put_row(&mut tb, &mut model, row);
                }
            }
        }
        let mut tb = Table::open(&mut h, root).unwrap();
        maintain_all(&mut tb);
        check_reads(&tb, &model);
        check_groups(&tb);
        check_running(&tb, &model);
        check_ownership(&tb);
        let reads: Vec<(u64, Vec<(u32, V)>)> = model.keys().map(|k| (*k, get_row(&tb, *k).unwrap().1)).collect();
        let n = sealed_groups(&tb).len();
        (all_images(&tb), reads, n)
    };
    let a = run(0);
    let b = run(7);
    assert_eq!(a.2, b.2, "sealed groups");
    assert!(a.0 == b.0, "the same images");
    assert!(a.1 == b.1, "the same rows");
    eprintln!("crash_equivalence: {} sealed groups, {} rows", a.2, a.1.len());
}

/// Long patch chains: a few sealed rows' Text updated thousands of times with changing lengths (short, medium
/// in block chains, huge in long values) and no maintenance, the chains far past `PATCH_FOLD_BLOCKS`; then
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
    assert!(digest.0 > 3 * native_table::consts::PATCH_FOLD_BLOCKS, "chains far past their bound: {digest:?}");
}

/// A chain past its first block has a position map naming each live entry's block (`check_ownership`), and a
/// compaction gives the map back with the chain: from the second round on, area 0 holds the blocks it held before
/// the round's changes.
#[test]
fn patch_position_maps() {
    let m = model_schema();
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 97, &m.img).unwrap();
    let (mut r, mut model, mut next_key, mut units) = (XorShift(0x3A9), Rows::new(), 0u64, Units::default());
    fill_group(&mut tb, &mut r, &mut model, &mut next_key);
    run_all(&mut tb, &mut units);
    let keys: Vec<u64> = model.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == 0).map(|(k, _)| *k).take(300).collect();
    for round in 0..3 {
        let used = nt_testhost::used_blocks(tb.host(), tb.root(), 0);
        for (i, k) in keys.iter().enumerate() {
            set_cell(&mut tb, &mut model, *k, id(1), V::Bytes(text(&mut r, 30 + (i * 7 + round) % 60)));
        }
        let pg = root::group(tb.host(), tb.root(), 0).unwrap();
        let dir = buffer::u64_of(tb.host(), pg, page::P_PATCH_DIR).unwrap();
        let u = patch::unit(tb.host(), dir, 1).unwrap().unwrap();
        assert!(u.blocks > 1 && rd_u64(tb.host().get(Region::Block(u.first)).unwrap(), patch::B_MAP).unwrap() != 0);
        check_reads(&tb, &model);
        check_ownership(&tb);
        let tbl = tb.root();
        native_table::table::jobs::queue(tb.host_mut(), tbl, 0, pg).unwrap();
        run_all(&mut tb, &mut units);
        check_reads(&tb, &model);
        if round > 0 {
            assert_eq!(nt_testhost::used_blocks(tb.host(), tb.root(), 0), used, "round {round}");
        }
    }
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
            vec![
                (id(0), V::U(k)),
                (id(1), V::Nat(big_nat(k + salt, 100_000))),
                (id(2), V::Int(low_zero_int(k + salt, 100_000))),
            ]
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

/// An update of one leaf of a buffered row that holds a long Nat, a long Int and a long Text: the other leaves
/// read back as they were, and the long values keep their images.
#[test]
fn update_of_one_leaf_keeps_long_values() {
    let cols = [t::scalar(ty::NAT), t::scalar(ty::NAT), t::scalar(ty::INT), t::scalar(ty::TEXT), t::scalar(ty::NAT)];
    let img = schema(&cols).0;
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &img).unwrap();
    let mut row = vec![
        (id(0), V::U(1)),
        (id(1), V::Nat(big_nat(3, 100_000))),
        (id(2), V::Int(low_zero_int(5, 100_000))),
        (id(3), V::Bytes(vec![b'x'; 70_000])),
        (id(4), V::Nat(big_nat(7, 40))),
    ];
    tb.upsert(&mut key_scratch(), &mut Src::new(&row)).unwrap();
    let before = (check_ownership(&tb), tb.host().live_pages());
    row[4] = (id(4), V::U(9));
    let key = [(id(0), V::U(1))];
    tb.update(&mut key_scratch(), &mut Src::new(&key), &[id(4)], &mut Src::new(&row[4..]), false).unwrap();
    assert_eq!(get_row(&tb, 1).unwrap().1, row);
    assert_eq!((check_ownership(&tb), tb.host().live_pages()), before, "the long values keep their images");
}

/// Updates of some leaves of a buffered row, each giving the row a new image made from its stored one: a Text
/// grown past the row's place (the image moves to the tail block), a Text made a long value beside a Nat whose
/// total moves, that long value replaced by one from the message's argument, a Text that makes the image a long
/// row image, then short values again. After each, every row reads back, the totals and extremes follow, and
/// every long value has one owner; at the end the table holds the pages it held before.
#[test]
fn update_of_some_leaves_of_a_buffered_row() {
    let cols =
        [t::scalar(ty::NAT), t::scalar(ty::NAT), t::scalar(ty::TEXT), t::opt(t::scalar(ty::INT)), t::scalar(ty::BLOB)];
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &schema(&cols).0).unwrap();
    let mut model = Rows::new();
    for k in 0..3u64 {
        let row = vec![
            (id(0), V::U(k)),
            (id(1), V::U(10 * k)),
            (id(2), V::text("a")),
            (id(3), V::I(-1)),
            (id(4), V::Bytes(vec![7; 70_000])),
        ];
        put_row(&mut tb, &mut model, row);
    }
    let pages = tb.host().live_pages();
    let steps: [&[(u32, V)]; 5] = [
        &[(id(2), V::Bytes(vec![b'b'; 200]))],
        &[(id(1), V::U(5)), (id(2), V::Bytes(vec![b'c'; 70_000]))],
        &[(id(2), V::Arg(vec![b'd'; 80_000]))],
        &[(id(3), V::Null), (id(2), V::Bytes(vec![b'e'; 3_000]))],
        &[(id(2), V::text("a")), (id(1), V::U(10)), (id(3), V::I(-1))],
    ];
    let rid = model[&1].0;
    let place = |tb: &Table<'_, VecHost>| {
        let at = buffer::locate(tb.host(), tb.root(), rid).unwrap().unwrap();
        match buffer::slot_row(tb.host(), at.block, at.slot).unwrap().unwrap() {
            buffer::RowLoc::Inline { block, off, .. } => Some((block, off)),
            buffer::RowLoc::Long(_) => None,
        }
    };
    let mut places = vec![place(&tb)];
    for cells in steps {
        let leaves: Vec<u32> = cells.iter().map(|c| c.0).collect();
        let key = [(id(0), V::U(1))];
        tb.update(&mut key_scratch(), &mut Src::new(&key), &leaves, &mut Src::new(cells), false).unwrap();
        let row = &mut model.get_mut(&1).unwrap().1;
        for (l, v) in cells {
            row.iter_mut().find(|c| c.0 == *l).unwrap().1 = v.clone();
        }
        check_reads(&tb, &model);
        check_running(&tb, &model);
        check_ownership(&tb);
        places.push(place(&tb));
    }
    assert!(places[1].is_some() && places[1] != places[0], "moved to the tail block");
    assert!(places[2] == places[1] && places[3] == places[1], "then in place");
    assert!(places[4].is_none() && places[5].is_some(), "a long row image, then inline");
    assert_eq!(tb.host().live_pages(), pages, "the long values and the long row image are given back");
    // The row's other long value outlived the replaced one: its removal gives it back.
    assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(1))])).unwrap().0.is_some());
    check_ownership(&tb);
}

/// Writes that grow a buffered row past its place and the tail block's room: the row moves to a new tail block,
/// and no image page is taken for it.
#[test]
fn growing_a_buffered_row_past_the_tail_blocks_room() {
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &schema(&[t::scalar(ty::NAT), t::scalar(ty::TEXT)]).0).unwrap();
    let mut model = Rows::new();
    for k in 0..3u64 {
        put_row(&mut tb, &mut model, vec![(id(0), V::U(k)), (id(1), V::Bytes(vec![b'a'; [1, 1_500, 1_500][k as usize]]))]);
    }
    let images = |tb: &Table<'_, VecHost>| AreaRecord::read(tb.host(), tb.root(), IMAGE_AREA).unwrap().pages;
    let pages = images(&tb);
    for (k, n) in [(0u64, 2_000), (1, 2_040)] {
        let tail = root::u64_at(tb.host(), tb.root(), root::TAIL).unwrap();
        let cells = [(id(1), V::Bytes(vec![b'b'; n]))];
        if k == 0 {
            tb.update(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))]), &[id(1)], &mut Src::new(&cells), false)
                .unwrap();
        } else {
            tb.upsert(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k)), cells[0].clone()])).unwrap();
        }
        model.get_mut(&k).unwrap().1[1] = cells[0].clone();
        check_reads(&tb, &model);
        check_running(&tb, &model);
        check_ownership(&tb);
        let at = buffer::locate(tb.host(), tb.root(), model[&k].0).unwrap().unwrap();
        let Some(buffer::RowLoc::Inline { block, .. }) = buffer::slot_row(tb.host(), at.block, at.slot).unwrap() else {
            panic!("key {k}: not inline");
        };
        assert_ne!(block, tail, "key {k}: in a new tail block");
        assert_eq!(block, root::u64_at(tb.host(), tb.root(), root::TAIL).unwrap());
        assert_eq!(images(&tb), pages, "no image page");
    }
    // Once the open group has its most buffer blocks, a long row image takes the row.
    for i in 0..600 {
        let cells = [(id(1), V::Bytes(vec![b'c'; 2_000 + 40 * (i % 2)]))];
        tb.update(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(2))]), &[id(1)], &mut Src::new(&cells), false)
            .unwrap();
        model.get_mut(&2).unwrap().1[1] = cells[0].clone();
    }
    check_reads(&tb, &model);
    check_ownership(&tb);
    let at = buffer::locate(tb.host(), tb.root(), model[&2].0).unwrap().unwrap();
    assert!(matches!(buffer::slot_row(tb.host(), at.block, at.slot).unwrap(), Some(buffer::RowLoc::Long(_))));
}

/// Scratch of a fixed size, refusing a longer request.
struct Short(Vec<u8>);

impl native_table::Scratch for Short {
    fn get(&mut self, need: usize) -> Result<&mut [u8], Fault> {
        self.0.get_mut(..need).ok_or(Fault::Scratch { need: need as u64 })
    }
}

/// Whole-row writes of a buffered row whose key is longer than a point operation's stack and whose image is
/// longer than an inline one: the insert asks the scratch once for both, and a scratch too short for the image
/// refuses it with nothing written; a replace keeps a long value it gives again with the same bytes, and frees
/// one it changes.
#[test]
fn whole_row_writes_with_a_long_key_and_image() {
    let cols = [t::scalar(ty::TEXT), t::scalar(ty::NAT), t::scalar(ty::BLOB), t::scalar(ty::TEXT)];
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &schema(&cols).0).unwrap();
    let key = (id(0), V::Bytes(vec![b'k'; 100]));
    let row = |n: u64, b: u8| {
        vec![key.clone(), (id(1), V::U(n)), (id(2), V::Bytes(vec![b; 70_000])), (id(3), V::Bytes(vec![b't'; 4_000]))]
    };
    tb.host_mut().end_message();
    let got = tb.upsert(&mut Short(vec![0; 1_000]), &mut Src::new(&row(1, 7)));
    assert!(matches!(got, Err(Fault::Scratch { .. })), "{got:?}");
    assert_eq!(tb.host_mut().end_message(), 0, "nothing written");
    let mut s = key_scratch();
    tb.upsert(&mut s, &mut Src::new(&row(1, 7))).unwrap();
    assert_eq!(s.grows, 1, "one request holds the key and the image");
    let read = |tb: &Table<'_, VecHost>| {
        let mut s = Sink::default();
        assert!(tb.get(&mut key_scratch(), &mut Src::new(&[key.clone()]), &mut s).unwrap());
        let rid = s.rid.unwrap();
        let at = buffer::locate(tb.host(), tb.root(), rid).unwrap().unwrap();
        let loc = buffer::slot_row(tb.host(), at.block, at.slot).unwrap().unwrap();
        assert!(matches!(loc, buffer::RowLoc::Long(_)), "a long row image");
        let item = RowRef::new(tb.host(), loc).unwrap().item(tb.shape(), 2).unwrap();
        let Item::Long { img, .. } = item else { panic!("{item:?}") };
        (fetch_long(tb, rid, s.cells), img)
    };
    let (cells, img) = read(&tb);
    assert_eq!(cells, row(1, 7));
    let pages = tb.host().live_pages();
    tb.upsert(&mut key_scratch(), &mut Src::new(&row(2, 7))).unwrap();
    assert_eq!(read(&tb), (row(2, 7), img), "the same bytes keep their long value");
    tb.upsert(&mut key_scratch(), &mut Src::new(&row(3, 8))).unwrap();
    let (cells, other) = read(&tb);
    assert_eq!(cells, row(3, 8));
    assert_ne!(other, img);
    check_ownership(&tb);
    assert_eq!(tb.host().live_pages(), pages, "the changed long value's pages are given back");
}

/// An update of one leaf of a buffered row whose key is a long Text: the key is read from the row's image, and
/// the row reads back with the leaf changed.
#[test]
fn update_of_one_leaf_with_a_long_key() {
    let img = schema(&[t::scalar(ty::TEXT), t::scalar(ty::NAT)]).0;
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &img).unwrap();
    let key = [(id(0), V::Bytes(vec![b'k'; 70_000]))];
    tb.upsert(&mut key_scratch(), &mut Src::new(&[key[0].clone(), (id(1), V::U(1))])).unwrap();
    let rid = tb.update(&mut key_scratch(), &mut Src::new(&key), &[id(1)], &mut Src::new(&[(id(1), V::U(2))]), false);
    let mut s = Sink::default();
    assert!(tb.get(&mut key_scratch(), &mut Src::new(&key), &mut s).unwrap());
    assert_eq!(rid.unwrap().0, s.rid);
    assert_eq!(fetch_long(&tb, s.rid.unwrap(), s.cells), vec![key[0].clone(), (id(1), V::U(2))]);
}

/// A row's cells the test owns, which `append` replaces with the lists it gives.
struct Owned(Vec<(u32, V)>);

impl RowSource for Owned {
    fn cell(&mut self, leaf: u32) -> Result<CellIn<'_>, Fault> {
        match self.0.iter().find(|c| c.0 == leaf).map(|c| &c.1) {
            Some(V::Bytes(b)) => Ok(CellIn::Bytes(b)),
            Some(V::Null) => Ok(CellIn::Null),
            Some(V::U(x)) => Ok(CellIn::U64(*x)),
            _ => Err(Fault::BadValue { leaf }),
        }
    }

    fn fill(&mut self, _: u32, _: u64, _: &mut [u8]) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
}

/// `append` to lists of u8: two lists of a sealed row in one call, a null list taken as empty, a null given adding
/// nothing, a count whose LEB128 grows a byte and long lists, read in pieces, in a buffered row and a sealed one.
/// A key with no row writes nothing, and a leaf that is not a Blob is refused. Every row reads back, after
/// maintenance too.
#[test]
fn append_adds_elements_at_the_end_of_lists() {
    let list = |e: &[u8]| V::Bytes([uleb(e.len() as u128), e.to_vec()].concat());
    let cols = [t::scalar(ty::NAT), vec![ty::LIST, ty::U8], t::opt(vec![ty::LIST, ty::U8]), t::scalar(ty::NAT)];
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &schema(&cols).0).unwrap();
    let mut model = Rows::new();
    let mut k = 0;
    while seals(&tb) == 0 || model.len() < 2 + GROUP_SLOTS as usize {
        let opt = if k % 2 == 0 { V::Null } else { list(&[3]) };
        put_row(&mut tb, &mut model, vec![(id(0), V::U(k)), (id(1), list(&[1, 2])), (id(2), opt), (id(3), V::U(k))]);
        k += 1;
    }
    let (long, more): (Vec<u8>, Vec<u8>) = ((0..70_000).map(|i| i as u8).collect(), (0..125).collect());
    let steps: [(u64, Vec<(u32, V)>, Vec<(u32, V)>); 8] = [
        (
            1,
            vec![(id(1), list(&[5])), (id(2), list(&[6, 7]))],
            vec![(id(1), list(&[1, 2, 5])), (id(2), list(&[3, 6, 7]))],
        ),
        (2, vec![(id(2), list(&[9]))], vec![(id(2), list(&[9]))]),
        (3, vec![(id(2), V::Null)], vec![(id(2), list(&[3]))]),
        (4, vec![(id(2), V::Null)], vec![(id(2), V::Null)]),
        (k - 1, vec![(id(1), list(&more))], vec![(id(1), list(&[&[1, 2], &more[..]].concat()))]),
        (k - 1, vec![(id(1), list(&long))], vec![(id(1), list(&[&[1, 2], &more[..], &long].concat()))]),
        (5, vec![(id(1), list(&long))], vec![(id(1), list(&[&[1, 2], &long[..]].concat()))]),
        (5, vec![(id(1), list(&[8]))], vec![(id(1), list(&[&[1, 2], &long[..], &[8]].concat()))]),
    ];
    let append = |tb: &mut Table<'_, VecHost>, k: u64, cells: Vec<(u32, V)>| {
        let leaves: Vec<u32> = cells.iter().map(|c| c.0).collect();
        let give = |s: &mut Owned, l, b: &[u8]| s.0.iter_mut().find(|c| c.0 == l).unwrap().1 = V::Bytes(b.to_vec());
        let key = [(id(0), V::U(k))];
        tb.append(&mut key_scratch(), &mut Src::new(&key), &leaves, &mut Owned(cells), give).map(|r| r.0)
    };
    for (k, cells, after) in steps {
        assert_eq!(append(&mut tb, k, cells).unwrap(), Some(model[&k].0));
        for (l, v) in after {
            model.get_mut(&k).unwrap().1.iter_mut().find(|c| c.0 == l).unwrap().1 = v;
        }
        check_reads(&tb, &model);
    }
    tb.host_mut().end_message();
    let none = append(&mut tb, k, vec![(id(1), list(&[1]))]).unwrap();
    assert_eq!((none, tb.host_mut().end_message()), (None, 0), "no row, nothing written");
    assert_eq!(append(&mut tb, 1, vec![(id(3), V::U(1))]).unwrap_err(), Fault::BadValue { leaf: id(3) });
    maintain_all(&mut tb);
    check_reads(&tb, &model);
    check_ownership(&tb);
}

/// On a table `without_null_rows`, an update that leaves a row null in every leaf but the key's removes it, in the
/// write buffer and in a sealed group, and one that leaves a leaf set keeps it. Before, the table keeps such a row.
/// `clear` keeps the setting, and so does `evolve`, after which a dropped leaf's old value does not count.
#[test]
fn update_removes_a_row_it_leaves_null_on_a_table_without_null_rows() {
    let cols = [t::scalar(ty::NAT), t::opt(t::scalar(ty::NAT)), t::opt(t::scalar(ty::TEXT))];
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &schema(&cols).0).unwrap();
    let mut model = Rows::new();
    let row = |k: u64| vec![(id(0), V::U(k)), (id(1), V::U(k)), (id(2), V::text("x"))];
    let mut k = 0;
    while seals(&tb) == 0 || model.len() < 2 + GROUP_SLOTS as usize {
        put_row(&mut tb, &mut model, row(k));
        k += 1;
    }
    let update = |tb: &mut Table<'_, VecHost>, k: u64, cells: &[(u32, V)]| {
        let leaves: Vec<u32> = cells.iter().map(|c| c.0).collect();
        let key = [(id(0), V::U(k))];
        tb.update(&mut key_scratch(), &mut Src::new(&key), &leaves, &mut Src::new(cells), false).unwrap().0
    };
    let nulls = [(id(1), V::Null), (id(2), V::Null)];
    let removed = |tb: &mut Table<'_, VecHost>, model: &mut Rows, k: u64, cells: &[(u32, V)]| {
        let rid = model.remove(&k).unwrap().0;
        assert_eq!(update(tb, k, cells), Some(rid));
        assert!(get_row(tb, k).is_none(), "row {k} removed");
    };
    assert_eq!(update(&mut tb, 0, &nulls), Some(model[&0].0));
    model.get_mut(&0).unwrap().1 = [vec![(id(0), V::U(0))], nulls.to_vec()].concat();
    check_reads(&tb, &model);
    tb.without_null_rows().unwrap();
    removed(&mut tb, &mut model, 0, &nulls[..1]);
    for j in [1, k - 1] {
        for (l, v) in [(1, V::Null), (2, V::text("y"))] {
            assert_eq!(update(&mut tb, j, &[(id(l), v.clone())]), Some(model[&j].0));
            model.get_mut(&j).unwrap().1[l as usize].1 = v;
        }
        removed(&mut tb, &mut model, j, &nulls[1..]);
    }
    removed(&mut tb, &mut model, 2, &nulls);
    check_reads(&tb, &model);
    maintain_all(&mut tb);
    check_reads(&tb, &model);
    check_ownership(&tb);
    tb.clear().unwrap();
    put_row(&mut tb, &mut Rows::new(), row(0));
    let v2: Vec<VCol> = cols
        .iter()
        .zip([0, 0, 2])
        .map(|(ty, dropped)| VCol { ty: ty.clone(), added: 1, dropped, default: None })
        .collect();
    tb.evolve(&schema_versioned(2, &v2, &[]).0).unwrap();
    assert!(update(&mut tb, 0, &nulls[..1]).is_some() && get_row(&tb, 0).is_none(), "the dropped leaf does not count");
    assert_eq!(tb.len().unwrap(), 0);
}

/// A sealed row's u64 cell, in a group whose values all fit one byte, updated to one that does not: the cell
/// cannot take it in place, a patch holds it, and the row reads it back, after maintenance too.
#[test]
fn update_wider_than_its_sealed_cell() {
    let (img, _) = schema(&[t::scalar(ty::NAT), t::scalar(ty::U64)]);
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &img).unwrap();
    let mut k = 0;
    while seals(&tb) == 0 {
        tb.upsert(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k)), (id(1), V::U(k % 200))])).unwrap();
        k += 1;
    }
    tb.upsert(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(1)), (id(1), V::U(70_000))])).unwrap();
    assert_eq!(get_row(&tb, 1).unwrap().1[1], (id(1), V::U(70_000)));
    maintain_all(&mut tb);
    assert_eq!(get_row(&tb, 1).unwrap().1[1], (id(1), V::U(70_000)));
    assert_eq!(get_row(&tb, 2).unwrap().1[1], (id(1), V::U(2)));
}

/// A sealed row's option Bool, in a group packed with no null, set to null and then to another value: a patch
/// holds the value, and the row reads it back.
#[test]
fn option_bool_set_after_null() {
    let (img, _) = schema(&[t::scalar(ty::NAT), t::opt(t::scalar(ty::BOOL))]);
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &img).unwrap();
    for k in 0..5_000 {
        tb.upsert(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k)), (id(1), V::Bool(true))])).unwrap();
    }
    assert!(seals(&tb) > 0);
    tb.upsert(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(3)), (id(1), V::Null)])).unwrap();
    assert_eq!(get_row(&tb, 3).unwrap().1[1], (id(1), V::Null));
    tb.upsert(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(3)), (id(1), V::Bool(false))])).unwrap();
    assert_eq!(get_row(&tb, 3).unwrap().1[1], (id(1), V::Bool(false)));
    assert_eq!(get_row(&tb, 2).unwrap().1[1], (id(1), V::Bool(true)));
}
