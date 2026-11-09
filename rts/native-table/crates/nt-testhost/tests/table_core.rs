//! The table core over flat row types (root, hot state with running records, buffer with item tables and
//! long values, freezes, the seal, group images and group pages, get, upsert, remove, clear).
//!
//! Every test runs once per fill pattern (`FILLS`) through `per_fill`, which requires identical results and
//! identical host-call logs from the two runs: a byte that differs between them was read without being
//! written. Changed-block counts come from `VecHost::end_message`.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use native_table::area::{AreaRecord, IMAGE_AREA};
use native_table::consts::{BLOCK, GROUP_SLOTS, HUGE_VALUE, LONG_VALUE, PAGE, RADIX_STEP};
use native_table::fault::Fault;
use native_table::mem::rd_u32;
use native_table::table::group::{self, GroupView, LeafDir, Seg};
use native_table::table::hot::RunCursor;
use native_table::table::layout::Ty;
use native_table::table::totals::At;
use native_table::table::{buffer, extremes, page, read, root, Table, Upsert};
use native_table::value::ty;
use native_table::{PageHost, Progress, Region, Wake};
use nt_testhost::big::Big;
use nt_testhost::rows::{rid_of, schema, t, Sink, Src, V};
use nt_testhost::{Call, TestBudget, VecHost, VecScratch, XorShift, FILLS};

/// Scratch for keys longer than a point operation's stack.
fn key_scratch() -> VecScratch {
    VecScratch::new(FILLS[0])
}

// ---------------------------------------------------------------- harness

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

/// Runs maintenance until nothing is pending.
fn maintain_all(tb: &mut Table<'_, VecHost>) {
    assert_eq!(tb.maintain(&everything()).unwrap(), Progress::Done);
    assert_eq!(tb.wake().unwrap(), Wake::Idle);
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

/// A big Int's canonical signed LEB128: `groups` 7-bit groups, the sign in the last.
fn big_int(r: &mut XorShift, groups: usize, negative: bool) -> Vec<u8> {
    let mut v: Vec<u8> = (0..groups).map(|_| (r.next_u64() as u8 & 0x7F) | 0x80).collect();
    let last = v.last_mut().unwrap();
    *last = if negative { 0x40 | (*last & 0x3E) } else { (*last & 0x3F) | 1 };
    if !negative && *last == 0 {
        *last = 1;
    }
    v
}

/// The value a read hands back for a value written as `v` (Nat and Int in their smallest cell).
fn read_form(v: &V, col_ty: Ty) -> V {
    match (v, col_ty) {
        (V::Int(b), Ty::Int) => {
            // A value that fits i64 comes back as I64.
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

/// Reads a row by key; values handed out as long are read back through `read_large`.
fn get_row(tb: &Table<'_, VecHost>, key: &[(u32, V)]) -> Option<(u64, Vec<(u32, V)>)> {
    let mut s = Sink::default();
    if !tb.get(&mut key_scratch(), &mut Src::new(key), &mut s).unwrap() {
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

// ---------------------------------------------------------------- the model's schema

/// Columns of the flat model row: key Nat; Text; Bool; i64; ?u8; u32; Float; Principal; [u8; 4]; Int; u128;
/// Char; (); ?Text; Blob.
struct Model {
    img: Vec<u8>,
    tys: Vec<Ty>,
}

fn model_schema() -> Model {
    let cols = vec![
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
    ];
    let tys = vec![
        Ty::Nat,
        Ty::Text,
        Ty::Bool,
        Ty::I(8),
        Ty::U(1),
        Ty::U(4),
        Ty::F64,
        Ty::Principal,
        Ty::Fixed(4),
        Ty::Int,
        Ty::U128,
        Ty::Char,
        Ty::Unit,
        Ty::Text,
        Ty::Blob,
    ];
    let (img, _) = schema(&cols);
    Model { img, tys }
}

fn text(r: &mut XorShift, n: usize) -> Vec<u8> {
    // ASCII with some 2-, 3- and 4-byte characters, cut at a character boundary.
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

fn model_row(r: &mut XorShift, k: u64, big: bool) -> Vec<(u32, V)> {
    let text_len = match r.below(100) {
        0 if big => (HUGE_VALUE as usize) + 1 + r.below(3_000) as usize,
        1..=3 => LONG_VALUE as usize + 1 + r.below(1_500) as usize,
        _ => r.below(60) as usize,
    };
    let int = match r.below(10) {
        0 => {
            let (g, neg) = (20 + r.below(5) as usize, r.below(2) == 0);
            V::Int(big_int(r, g, neg))
        }
        1 => V::Int(sleb(i128::from(r.next_u64()) << 40)),
        _ => V::Int(sleb(i128::from(r.next_u64() as i64 >> r.below(60)))),
    };
    let principal: Vec<u8> = (0..r.below(30)).map(|_| r.next_u64() as u8).collect();
    let fixed: Vec<u8> = (0..4).map(|_| r.next_u64() as u8).collect();
    let blob: Vec<u8> = (0..r.below(40)).map(|_| r.next_u64() as u8).collect();
    vec![
        (id(0), V::U(k)),
        (id(1), V::Bytes(text(r, text_len))),
        (id(2), V::Bool(r.below(2) == 0)),
        (id(3), V::I(r.next_u64() as i64 >> r.below(64))),
        (id(4), if r.below(4) == 0 { V::Null } else { V::U(r.below(256)) }),
        (id(5), V::U(r.below(3))),
        (id(6), V::F64((r.below(2_000) as f64 / 7.0 - 100.0).to_bits())),
        (id(7), V::Bytes(principal)),
        (id(8), V::Bytes(fixed)),
        (id(9), int),
        (id(10), V::U128(u128::from(r.next_u64()) << r.below(64))),
        (id(11), V::U(char::from_u32(0x20 + r.below(0xD000) as u32).map_or(0x41, u32::from) as u64)),
        (id(12), V::Unit),
        (
            id(13),
            if r.below(3) == 0 {
                V::Null
            } else {
                let n = r.below(20) as usize;
                V::Bytes(text(r, n))
            },
        ),
        (id(14), V::Bytes(blob)),
    ]
}

fn expected(m: &Model, row: &[(u32, V)]) -> Vec<(u32, V)> {
    row.iter().enumerate().map(|(i, (l, v))| (*l, read_form(v, m.tys[i]))).collect()
}

// ---------------------------------------------------------------- checks

/// Checks every sealed group's live count against its live rows, and returns the number of groups checked.
fn check_groups(tb: &Table<'_, VecHost>) -> u64 {
    let h = tb.host();
    let tbl = tb.root();
    let next = root::u64_at(h, tbl, root::NEXT_GROUP).unwrap();
    let mut checked = 0;
    for g in 0..next {
        let e = root::group(h, tbl, g).unwrap();
        if e == 0 || e & buffer::DESC_BIT != 0 {
            continue;
        }
        checked += 1;
        let p = buffer::u16_of(h, e, page::P_POSITIONS).unwrap();
        let mut live = 0;
        for pos in 0..p {
            let mut s = Sink::default();
            if tb.get_rid(g * GROUP_SLOTS + pos, &mut s).unwrap() {
                live += 1;
            }
        }
        assert_eq!(buffer::u16_of(h, e, page::P_LIVE).unwrap(), live, "group {g} live rows");
    }
    checked
}

/// Checks the integer leaves' totals in the hot state against the model, over every live row.
fn check_running(tb: &Table<'_, VecHost>, model: &BTreeMap<u64, (u64, Vec<(u32, V)>)>) {
    let h = tb.host();
    let tbl = tb.root();
    let shape = *tb.shape();
    let tail = root::u64_at(h, tbl, root::TAIL).unwrap();
    let hot = h.get(Region::Block(tail)).unwrap();
    let mut rc = RunCursor::open(h, tail).unwrap();
    for o in 0..shape.leaves {
        let l = shape.leaf(h, o).unwrap();
        if !matches!(rc.place(hot, &l).unwrap(), At::Hot(_)) {
            continue;
        }
        let (mut n, mut want) = (0u64, Big::default());
        for row in model.values().map(|(_, row)| &row[o as usize].1) {
            let v = match row {
                V::Null => continue,
                V::U(x) => Big::from_i128(i128::from(*x)),
                V::I(x) => Big::from_i128(i128::from(*x)),
                V::Nat(b) => Big::from_leb(b, false),
                V::Int(b) => Big::from_leb(b, true),
                V::U128(x) => Big::from_u128(*x),
                V::I128(x) => Big::from_i128(*x),
                other => panic!("leaf {o}: {other:?}"),
            };
            n += 1;
            want = want.add(&v);
        }
        let mut buf = vec![0u8; 64];
        let (t, got_n) = tb.sum(l.id, &mut buf).unwrap();
        assert_eq!((Big::of_total(t, &buf), got_n), (want, n), "the total of leaf {o}");
    }
}

// ---------------------------------------------------------------- rows_model

#[test]
fn rows_model() {
    let m = model_schema();
    let digest = per_fill(|h, lh| {
        let (mut tb, _) = Table::create(h, 11, &m.img).unwrap();
        let mut r = XorShift(0x1234_5678);
        // key → (rid, row as written)
        let mut model: BTreeMap<u64, (u64, Vec<(u32, V)>)> = BTreeMap::new();
        let mut seals = 0u64;
        let mut refused = 0u64;
        let mut trace = Vec::new();
        let mut groups_checked = 0;
        for step in 0..36_000u64 {
            let op = r.below(100);
            if op < 70 {
                // An upsert: a new key, or one in the model.
                let k = if model.is_empty() || r.below(4) != 0 {
                    r.below(1 << 40)
                } else {
                    *model.keys().nth(r.below(model.len() as u64) as usize).unwrap()
                };
                let row = model_row(&mut r, k, true);
                match tb.upsert(&mut key_scratch(), &mut Src::new(&row)) {
                    Ok((Upsert::Inserted(rid), _)) => {
                        assert!(!model.contains_key(&k));
                        model.insert(k, (rid, row));
                    }
                    Ok((Upsert::Replaced(rid), _)) => {
                        let (old_rid, _) = model.get(&k).unwrap();
                        assert_eq!(rid, *old_rid, "an update keeps the rid");
                        model.insert(k, (rid, row));
                    }
                    Err(Fault::Busy) => {
                        // The row is sealed and its values differ: this version refuses before writing.
                        refused += 1;
                        let (rid, _) = model[&k];
                        assert!(matches!(
                            read::find(tb.host(), tb.root(), rid).unwrap(),
                            Some(read::Row::Sealed { .. })
                        ));
                        let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
                        assert_eq!(gone, Some(rid));
                        let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(&row)).unwrap();
                        let Upsert::Inserted(rid2) = u else { panic!("re-insert") };
                        assert!(rid2 > rid, "rids are never reused");
                        model.insert(k, (rid2, row));
                    }
                    Err(e) => panic!("upsert: {e:?}"),
                }
            } else if op < 84 {
                let k = if model.is_empty() || r.below(5) == 0 {
                    r.below(1 << 40)
                } else {
                    *model.keys().nth(r.below(model.len() as u64) as usize).unwrap()
                };
                let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
                assert_eq!(gone, model.remove(&k).map(|x| x.0));
            } else if op < 97 {
                let k = if model.is_empty() || r.below(5) == 0 {
                    r.below(1 << 40)
                } else {
                    *model.keys().nth(r.below(model.len() as u64) as usize).unwrap()
                };
                let got = get_row(&tb, &[(id(0), V::U(k))]);
                match model.get(&k) {
                    None => assert!(got.is_none()),
                    Some((rid, row)) => {
                        let (grid, cells) = got.expect("present");
                        assert_eq!(grid, *rid);
                        assert_eq!(cells, expected(&m, row), "key {k}");
                    }
                }
            } else {
                // Maintenance at a random point: a write's one unit, or all of it.
                let before = root::u32_at(tb.host(), tb.root(), root::FROZEN_WAITING).unwrap();
                if r.below(2) == 0 {
                    tb.write_step(&everything()).unwrap();
                } else {
                    maintain_all(&mut tb);
                    groups_checked += check_groups(&tb);
                }
                let after = root::u32_at(tb.host(), tb.root(), root::FROZEN_WAITING).unwrap();
                seals += u64::from(before - after);
            }
            assert_eq!(tb.len().unwrap(), model.len() as u64, "step {step}");
            if step % 2_000 == 1_999 {
                check_running(&tb, &model);
                lh.take(tb_host(&mut tb));
                trace.push((step, model.len()));
            }
        }
        maintain_all(&mut tb);
        groups_checked += check_groups(&tb);
        check_running(&tb, &model);
        for (k, (rid, row)) in &model {
            let (grid, cells) = get_row(&tb, &[(id(0), V::U(*k))]).expect("present");
            assert_eq!((grid, cells), (*rid, expected(&m, row)));
        }
        assert!(seals >= 3, "several groups sealed: {seals}");
        assert!(groups_checked >= 3);
        (model.len(), seals, refused, trace)
    });
    eprintln!("rows_model: {digest:?}");
}

/// The host of a table, writable, for taking its log between steps.
fn tb_host<'a>(tb: &'a mut Table<'_, VecHost>) -> &'a mut VecHost {
    tb.host_mut()
}

// ---------------------------------------------------------------- group images

/// Group g's page, once sealed.
fn group_page(tb: &Table<'_, VecHost>, g: u64) -> u64 {
    let e = root::group(tb.host(), tb.root(), g).unwrap();
    assert!(e != 0 && e & buffer::DESC_BIT == 0, "group {g} is sealed");
    e
}

/// A sealed group's image, whole.
fn image_bytes(h: &VecHost, pg: u64) -> Vec<u8> {
    let gv = GroupView::open(h, pg).unwrap();
    let r = gv.reader();
    let mut out = Vec::new();
    let mut at = 0;
    while at < r.length() {
        let n = (PAGE - at % PAGE).min(r.length() - at);
        out.extend_from_slice(r.bytes(at, n).unwrap());
        at += n;
    }
    out
}

/// The sealed groups of a table.
fn sealed_groups(tb: &Table<'_, VecHost>) -> Vec<(u64, u64)> {
    let next = root::u64_at(tb.host(), tb.root(), root::NEXT_GROUP).unwrap();
    (0..next)
        .filter_map(|g| {
            let e = root::group(tb.host(), tb.root(), g).unwrap();
            (e != 0 && e & buffer::DESC_BIT == 0).then_some((g, e))
        })
        .collect()
}

/// The byte-layout checks of one image, as plain bytes: header fields, the leaf directory and segment
/// table at 32-byte boundaries, no cell segment, `Seq` part or out-of-line directory across an
/// image block, the cell segments of each leaf covering its stored slots in order, the parts of the cell area
/// disjoint and inside it, each leaf's run inside the inline data region, and the image's length at the
/// segment table's end. Returns the number of segments checked.
fn check_layout(img: &[u8]) -> u64 {
    let u16a = |o: usize| u16::from_le_bytes(img[o..o + 2].try_into().unwrap()) as u64;
    let u32a = |o: usize| u32::from_le_bytes(img[o..o + 4].try_into().unwrap()) as u64;
    let u64a = |o: usize| u64::from_le_bytes(img[o..o + 8].try_into().unwrap());
    assert_eq!(&img[0..4], b"NTIM");
    assert_eq!(img[6], group::KIND_GROUP);
    assert_eq!(u64a(8), img.len() as u64, "the header's length is the image's");
    let (n, l, flags, s) = (u16a(42), u16a(44), u16a(46), u32a(52));
    let hh = u32a(60);
    let segs_at = u64a(112);
    assert_eq!(segs_at % 32, 0);
    assert_eq!(segs_at + 32 * s, img.len() as u64);
    assert_eq!(u32a(68), 128);
    if flags & u64::from(group::GF_COMPACTED) != 0 {
        assert_eq!(u32a(64), 128 + 32 * l);
    }
    let ool_at = u64a(88);
    let ool_bytes = u64a(96);
    assert!(ool_at + ool_bytes <= segs_at);
    let mut parts: Vec<(u64, u64)> = Vec::new();
    let mut inline_lo = u64::MAX;
    for o in 0..l {
        let d = LeafDir::read(&img[(128 + 32 * o) as usize..]).unwrap();
        assert_eq!(u64::from(d.ordinal), o);
        let mut next_slot = 0u64;
        for i in d.first_seg..d.first_seg + d.segs {
            let sg = Seg::read(&img[(segs_at + 32 * u64::from(i)) as usize..]).unwrap();
            assert!(img[(segs_at + 32 * u64::from(i) + 21) as usize..(segs_at + 32 * u64::from(i) + 32) as usize]
                .iter()
                .all(|&b| b == 0));
            let end = sg.off + u64::from(sg.bytes);
            if sg.kind == group::seg::RUN {
                inline_lo = inline_lo.min(sg.off);
                assert!(end <= ool_at, "a run lies before the out-of-line area");
                continue;
            }
            assert!(sg.off >= hh && end <= ool_at.min(segs_at));
            if sg.bytes > 0 {
                assert_eq!(sg.off / PAGE, (end - 1) / PAGE, "leaf {o} segment {i} crosses an image block");
            }
            assert_eq!(sg.off % 8, 0);
            if sg.kind == group::seg::CELLS {
                assert_eq!(u64::from(sg.first), next_slot, "cell segments in slot order");
                next_slot += u64::from(sg.positions);
            }
            parts.push((sg.off, end));
        }
        if group::has_cells(d.form, d.flags & group::LF_VALIDITY != 0) {
            assert_eq!(next_slot, n, "leaf {o}'s cell segments cover its slots");
        }
    }
    parts.sort();
    for w in parts.windows(2) {
        assert!(w[0].1 <= w[1].0, "parts of the cell area are disjoint");
    }
    if let Some(&(_, last)) = parts.last() {
        assert!(last <= inline_lo.min(ool_at));
    }
    parts.len() as u64
}

/// A small schema whose rows are about 80 bytes: key Nat, i32, Text, Bool, u8, Float.
fn small_schema() -> Vec<u8> {
    let (img, _) = schema(&[
        t::scalar(ty::NAT),
        t::scalar(ty::I32),
        t::scalar(ty::TEXT),
        t::scalar(ty::BOOL),
        t::scalar(ty::U8),
        t::scalar(ty::F64),
    ]);
    img
}

fn small_row(k: u64, text: Vec<u8>) -> Vec<(u32, V)> {
    vec![
        (id(0), V::U(k)),
        (id(1), V::I((k as i64 * 7919) % 100_000 - 50_000)),
        (id(2), V::Bytes(text)),
        (id(3), V::Bool(k % 3 == 0)),
        (id(4), V::U(k % 4)),
        (id(5), V::F64((k as f64 * 0.25 - 77.0).to_bits())),
    ]
}

/// The final text of row k in `seal_canonical`: mostly short, some medium, two huge.
fn canon_text(k: u64) -> Vec<u8> {
    match k {
        100 | 2_000 => vec![b'h' + (k % 7) as u8; HUGE_VALUE as usize + 1_000 + k as usize],
        k if k % 97 == 0 => vec![b'm'; LONG_VALUE as usize + 1 + (k % 500) as usize],
        k if k % 131 == 0 => vec![b'r'; 3_200],
        k => format!("text of {k}").into_bytes(),
    }
}

#[test]
fn seal_canonical() {
    let img = small_schema();
    per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 5, &img).unwrap();
        let n = GROUP_SLOTS;
        // Phase 1: the rows inserted as they are; the next insert freezes the group.
        for k in 0..n {
            tb.upsert(&mut key_scratch(), &mut Src::new(&small_row(k, canon_text(k)))).unwrap();
        }
        for k in (0..n).filter(|k| k % 11 == 5) {
            tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
        }
        tb.upsert(&mut key_scratch(), &mut Src::new(&small_row(n, b"next".to_vec()))).unwrap();
        maintain_all(&mut tb);
        let a = image_bytes(tb.host(), group_page(&tb, 0));
        // Phase 2, in a later group of the same table: every row of phase 1 deleted and its group dropped, the
        // open group filled up behind the row that froze group 0, so that the same rows start a group of their
        // own, reached through updates that rewrite images in place, move them to the tail block and make long
        // row images.
        for k in (0..n).filter(|k| k % 11 != 5) {
            tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
        }
        maintain_all(&mut tb);
        let g1 = root::u64_at(tb.host(), tb.root(), root::OPEN_GROUP).unwrap();
        for f in 1..n {
            tb.upsert(&mut key_scratch(), &mut Src::new(&small_row(2 * n + f, b"f".to_vec()))).unwrap();
        }
        assert_eq!(root::u64_at(tb.host(), tb.root(), root::OPEN_GROUP).unwrap(), g1, "group 1 holds n rows");
        let g2 = g1 + 1;
        for k in 0..n {
            let first = if k % 5 == 0 { b"x".to_vec() } else { canon_text(k) };
            tb.upsert(&mut key_scratch(), &mut Src::new(&small_row(k, first))).unwrap();
            if k % 5 == 0 {
                // Grows: moved to the tail block, or into a long row image.
                tb.upsert(&mut key_scratch(), &mut Src::new(&small_row(k, canon_text(k)))).unwrap();
            }
        }
        for k in (0..n).filter(|k| k % 5 == 0 && k % 2 == 0) {
            // Shrinks back and grows again: in place, then moved.
            tb.upsert(&mut key_scratch(), &mut Src::new(&small_row(k, b"y".to_vec()))).unwrap();
            tb.upsert(&mut key_scratch(), &mut Src::new(&small_row(k, canon_text(k)))).unwrap();
        }
        for k in (0..n).filter(|k| k % 11 == 5) {
            tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
        }
        tb.upsert(&mut key_scratch(), &mut Src::new(&small_row(3 * n, b"next".to_vec()))).unwrap();
        maintain_all(&mut tb);
        let mut b = image_bytes(tb.host(), group_page(&tb, g2));
        assert_eq!(u64::from_le_bytes(b[32..40].try_into().unwrap()), g2);
        b[32..40].copy_from_slice(&0u64.to_le_bytes());
        assert_eq!(a.len(), b.len());
        assert!(a == b, "sealing the same rows twice gives the same bytes");
        check_layout(&a);
        // The rows read back from the sealed group.
        for k in (0..n).step_by(13) {
            let got = get_row(&tb, &[(id(0), V::U(k))]);
            if k % 11 == 5 {
                assert!(got.is_none());
            } else {
                let want = small_row(k, canon_text(k));
                assert_eq!(got.unwrap().1, want, "row {k}");
            }
        }
        (a.len(), nt_testhost_hash(&a))
    });
}

fn nt_testhost_hash(b: &[u8]) -> u64 {
    native_table::hash::nt_hash64(b)
}

#[test]
fn no_crossing() {
    let m = model_schema();
    per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 9, &m.img).unwrap();
        let mut r = XorShift(77);
        let mut out = Vec::new();
        // Several groups: plain, with medium and huge values, and one with 1,100 dead positions (compacted).
        let mut k = 0u64;
        for round in 0..3 {
            let start = k;
            while root::u32_at(tb.host(), tb.root(), root::FROZEN_WAITING).unwrap() == 0 {
                tb.upsert(&mut key_scratch(), &mut Src::new(&model_row(&mut r, k, true))).unwrap();
                k += 1;
            }
            if round == 2 {
                for x in (start..k - 1).take(1_100) {
                    tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(x))])).unwrap();
                }
            }
            maintain_all(&mut tb);
        }
        let mut compacted = 0;
        for (g, pg) in sealed_groups(&tb) {
            let img = image_bytes(tb.host(), pg);
            let segs = check_layout(&img);
            if u16::from_le_bytes(img[46..48].try_into().unwrap()) & group::GF_COMPACTED != 0 {
                compacted += 1;
            }
            out.push((g, img.len(), segs, nt_testhost_hash(&img)));
        }
        assert_eq!(compacted, 1, "the group with 1,100 dead positions is written compacted");
        // Every row of every group reads back.
        out.push((u64::MAX, tb.len().unwrap() as usize, 0, 0));
        out
    });
}

// ---------------------------------------------------------------- long values

/// The pages of an image, through its block entries.
fn image_pages(h: &VecHost, img: &native_table::image::Image) -> Vec<u64> {
    (0..img.block_count()).map(|k| img.entry(h, k).unwrap()).collect()
}

/// Ownership of the table's images: every image page belongs to exactly one image (the schema image,
/// a sealed group's image, a long value named once by a buffered row or a group's long-value list), and
/// area 15's count is their number. Returns the long values found.
fn check_ownership(tb: &Table<'_, VecHost>) -> usize {
    let h = tb.host();
    let tbl = tb.root();
    let shape = *tb.shape();
    let mut pages = std::collections::BTreeSet::new();
    let mut add = |img: &native_table::image::Image| {
        for p in image_pages(h, img) {
            assert!(pages.insert(p), "page {p} belongs to two images");
        }
    };
    let schema = root::u64_at(h, tbl, root::SCHEMA_PAGE).unwrap();
    add(&native_table::image::Image {
        length: 1,
        blocks: native_table::image::Blocks::Direct { count: 1, entries: [schema, 0, 0, 0, 0, 0, 0, 0] },
        believed: false,
    });
    let mut longs = 0;
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
    }
    // The buffered groups: the frozen queue, then the open group.
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
            for vi in 0..shape.vars {
                if let native_table::table::row::Item::Long { img, .. } = row.item(&shape, vi)? {
                    add(&img);
                    longs += 1;
                }
            }
            Ok(())
        })
        .unwrap();
    }
    let rec15 = AreaRecord::read(h, tbl, IMAGE_AREA).unwrap();
    assert_eq!(rec15.pages, pages.len() as u64, "area 15 counts exactly the pages its images hold");
    longs
}

/// Text of `len` bytes: ASCII filler with a `width`-byte character starting one byte before each content
/// offset in `edges`, so that it straddles the image block edge there.
fn straddling_text(len: usize, edges: &[usize], width: usize) -> Vec<u8> {
    let ch = match width {
        2 => "é",
        3 => "€",
        _ => "😀",
    }
    .as_bytes();
    let mut v = vec![b'a'; len];
    for &e in edges {
        let at = e - 1;
        v[at..at + width].copy_from_slice(ch);
    }
    assert!(std::str::from_utf8(&v).is_ok());
    v
}

#[test]
fn long_values() {
    let (img, _) = schema(&[t::scalar(ty::NAT), t::scalar(ty::TEXT), t::scalar(ty::BLOB)]);
    // Content offsets that are image block edges of a long value: its content starts at image offset 64.
    let e1 = (PAGE - 64) as usize;
    let e2 = (2 * PAGE - 64) as usize;
    per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 3, &img).unwrap();
        let mut log = Vec::new();
        let lens = [2_049usize, 65_536, 65_537, 150_000, 1 << 20];
        let blob = |n: usize, seed: u64| -> Vec<u8> {
            let mut r = XorShift(seed | 1);
            (0..n).map(|_| r.next_u64() as u8).collect()
        };
        // Inserts from the heap and from the argument.
        let mut rows: BTreeMap<u64, Vec<(u32, V)>> = BTreeMap::new();
        for (i, &n) in lens.iter().enumerate() {
            for arg in [false, true] {
                let k = (i * 2 + usize::from(arg)) as u64;
                let b = blob(n, k);
                let cell = if arg { V::Arg(b.clone()) } else { V::Bytes(b.clone()) };
                let row = vec![(id(0), V::U(k)), (id(1), V::text("t")), (id(2), cell)];
                let mut src = Src::new(&row);
                tb.upsert(&mut key_scratch(), &mut src).unwrap();
                // One copy per byte, by the host, for a value in the argument; none from the heap.
                assert_eq!(src.filled, if arg { n as u64 } else { 0 });
                assert!(src.fills <= (n as u64).div_ceil(BLOCK) + 2);
                log.push((k, src.filled, src.fills));
                rows.insert(k, vec![(id(0), V::U(k)), (id(1), V::text("t")), (id(2), V::Bytes(b))]);
            }
        }
        // Text whose characters straddle image block edges of its long value, from the argument and the heap.
        for (j, width) in [2usize, 3, 4].into_iter().enumerate() {
            for arg in [false, true] {
                let k = 100 + (j * 2 + usize::from(arg)) as u64;
                let t = straddling_text(200_000, &[e1, e2], width);
                let cell = if arg { V::Arg(t.clone()) } else { V::Bytes(t.clone()) };
                let row = vec![(id(0), V::U(k)), (id(1), cell), (id(2), V::Bytes(vec![1, 2, 3]))];
                let mut src = Src::new(&row);
                tb.upsert(&mut key_scratch(), &mut src).unwrap();
                if arg {
                    assert_eq!(src.filled, 200_000);
                }
                rows.insert(k, vec![(id(0), V::U(k)), (id(1), V::Bytes(t)), (id(2), V::Bytes(vec![1, 2, 3]))]);
            }
        }
        let check_all = |tb: &Table<'_, VecHost>, rows: &BTreeMap<u64, Vec<(u32, V)>>| {
            for (k, row) in rows {
                let (_, got) = get_row(tb, &[(id(0), V::U(*k))]).expect("present");
                assert!(&got == row, "row {k} reads back byte for byte");
            }
            assert_eq!(tb.len().unwrap(), rows.len() as u64);
            check_ownership(tb)
        };
        let longs_buffered = check_all(&tb, &rows);
        // The seal adopts every huge value: the list names the same pages, and the image holds 8 bytes per
        // huge value.
        let before: std::collections::BTreeSet<u64> = {
            let h = tb.host();
            let mut s = std::collections::BTreeSet::new();
            let (_, d, _) = buffer::open_group(h, tb.root()).unwrap();
            buffer::for_each_slot(h, d, |_, b, sl| {
                let loc = buffer::slot_row(h, b, sl)?.unwrap();
                let row = native_table::table::row::RowRef::new(h, loc)?;
                for vi in 0..tb.shape().vars {
                    if let native_table::table::row::Item::Long { img, .. } = row.item(tb.shape(), vi)? {
                        s.extend(image_pages(h, &img));
                    }
                }
                Ok(())
            })
            .unwrap();
            s
        };
        let mut k = 1_000u64;
        while root::u32_at(tb.host(), tb.root(), root::FROZEN_WAITING).unwrap() == 0 {
            tb.upsert(
                &mut key_scratch(),
                &mut Src::new(&[(id(0), V::U(k)), (id(1), V::text("f")), (id(2), V::Bytes(vec![]))]),
            )
            .unwrap();
            rows.insert(k, vec![(id(0), V::U(k)), (id(1), V::text("f")), (id(2), V::Bytes(vec![]))]);
            k += 1;
        }
        maintain_all(&mut tb);
        let (_, pg) = sealed_groups(&tb)[0];
        let gimg = GroupView::open(tb.host(), pg).unwrap().img;
        let huge_total: u64 =
            lens.iter().filter(|&&n| n as u64 > HUGE_VALUE).map(|&n| 2 * n as u64).sum::<u64>() + 6 * 200_000;
        assert!(gimg.length < huge_total / 4, "huge values are adopted, not copied: image of {} bytes", gimg.length);
        let after: std::collections::BTreeSet<u64> = {
            let h = tb.host();
            let n = u64::from(rd_u32(h.get(Region::Block(pg)).unwrap(), page::P_LV_COUNT).unwrap());
            (0..n).flat_map(|i| image_pages(h, &page::lv_get(h, pg, i).unwrap().unwrap())).collect()
        };
        assert!(after.is_subset(&before), "the list's values are the rows' own long values");
        let longs_sealed = check_all(&tb, &rows);
        // Updates of buffered rows: a huge value replaced, then the row deleted.
        let k1 = 1_000_000u64;
        let v1 = blob(300_000, 9);
        let v2 = blob(70_000, 10);
        tb.upsert(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k1)), (id(1), V::text("u")), (id(2), V::Arg(v1))]))
            .unwrap();
        tb.upsert(
            &mut key_scratch(),
            &mut Src::new(&[(id(0), V::U(k1)), (id(1), V::text("u2")), (id(2), V::Bytes(v2.clone()))]),
        )
        .unwrap();
        // The same huge value written again: the new image takes the old image's long value over, and the
        // drain leaves it alone.
        tb.upsert(
            &mut key_scratch(),
            &mut Src::new(&[(id(0), V::U(k1)), (id(1), V::text("u3")), (id(2), V::Bytes(v2.clone()))]),
        )
        .unwrap();
        rows.insert(k1, vec![(id(0), V::U(k1)), (id(1), V::text("u3")), (id(2), V::Bytes(v2))]);
        maintain_all(&mut tb);
        check_all(&tb, &rows);
        // Deletes, sealed and buffered: every long value goes to the drain and its pages back to the host.
        let keys: Vec<u64> = rows.keys().copied().collect();
        for k in keys {
            if k < 1_000 || k == k1 || k % 3 == 0 {
                tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
                rows.remove(&k);
            }
        }
        maintain_all(&mut tb);
        let longs_end = check_all(&tb, &rows);
        assert_eq!(longs_end, 0);
        // An argument value that is not UTF-8 faults while it is copied (the host traps on it).
        let mut bad = straddling_text(100_000, &[e1], 3);
        bad.truncate(e1);
        let r = tb.upsert(
            &mut key_scratch(),
            &mut Src::new(&[(id(0), V::U(9_999)), (id(1), V::Arg(bad)), (id(2), V::Bytes(vec![]))]),
        );
        assert!(matches!(r, Err(Fault::BadValue { .. })), "{r:?}");
        (log, longs_buffered, longs_sealed, longs_end)
    });
}

// ---------------------------------------------------------------- changed blocks per write

/// The primary index's node count.
fn m_nodes(tb: &Table<'_, VecHost>) -> u64 {
    native_table::table::primary::Primary::read(tb.host(), tb.root()).unwrap().map_or(0, |p| p.nodes)
}

fn area0_pages(tb: &Table<'_, VecHost>) -> u64 {
    AreaRecord::read(tb.host(), tb.root(), 0).unwrap().pages
        + (AreaRecord::read(tb.host(), tb.root(), 1).unwrap().pages << 32)
}

/// Ends a simulated message: the blocks it changed besides the groups' extremes blocks, of which a write
/// changes at most one, its row's group's, when a value it writes or removes moves the group's extremes.
fn end_apart(tb: &mut Table<'_, VecHost>, ext_changes: &mut u64) -> u64 {
    let h = tb.host();
    let open = root::u64_at(h, tb.root(), root::OPEN_GROUP).unwrap();
    let mut ext = std::collections::BTreeSet::new();
    for g in 0..=open {
        let e = root::group(h, tb.root(), g).unwrap();
        if e == 0 {
            continue;
        }
        let blk = if e & buffer::DESC_BIT != 0 {
            extremes::of_desc(h, e & !buffer::DESC_BIT).unwrap()
        } else {
            extremes::of_page(h, e).unwrap()
        };
        ext.insert(blk);
    }
    let e = h.changed_set().into_iter().filter(|b| ext.contains(b)).count() as u64;
    assert!(e <= 1, "{e} extremes blocks changed");
    *ext_changes += e;
    tb.host_mut().end_message() - e
}

#[test]
fn changed_blocks() {
    let img = small_schema();
    per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 6, &img).unwrap();
        let mut seen: BTreeMap<&'static str, Vec<u64>> = BTreeMap::new();
        let mut k = 0u64;
        let row = |k: u64| small_row(k, format!("v{k}").into_bytes());
        // A group's extremes block, when a write moves the group's extremes, is counted apart (`end_apart`).
        let mut ext_changes = 0u64;
        // Inserts, sorted by what they did: a row into the tail block (2: the tail block, M's leaf), one that
        // opens a tail block (4: + the new block, the descriptor, the root), one that freezes the group
        // (6: the old and the new descriptor, the root, the new tail block, the directory's leaf, M's leaf;
        // 7 when a frozen group waits: its descriptor's queue link). Inserts that also took a new area page or
        // split M's leaf are counted apart.
        while root::u32_at(tb.host(), tb.root(), root::FROZEN_WAITING).unwrap() < 2 {
            let tail = root::u64_at(tb.host(), tb.root(), root::TAIL).unwrap();
            let waiting = root::u32_at(tb.host(), tb.root(), root::FROZEN_WAITING).unwrap();
            let (pages, nodes) = (area0_pages(&tb), m_nodes(&tb));
            tb.host_mut().end_message();
            tb.upsert(&mut key_scratch(), &mut Src::new(&row(k))).unwrap();
            let c = end_apart(&mut tb, &mut ext_changes);
            k += 1;
            if k < 10 {
                continue;
            }
            let split = m_nodes(&tb) != nodes;
            let page = area0_pages(&tb) != pages;
            let froze = root::u32_at(tb.host(), tb.root(), root::FROZEN_WAITING).unwrap() != waiting;
            let new_tail = root::u64_at(tb.host(), tb.root(), root::TAIL).unwrap() != tail;
            let what = match (froze, new_tail, split, page) {
                (true, _, false, false) if waiting == 0 => "freeze",
                (true, _, false, false) => "freeze, a group waiting",
                (false, true, false, false) => "new tail block",
                (false, false, false, false) => "insert",
                (false, false, true, false) => "insert splitting M's leaf",
                _ => "other",
            };
            seen.entry(what).or_default().push(c);
        }
        let only = |v: &[u64], want: u64, what: &str| {
            assert!(!v.is_empty(), "{what}: measured");
            assert!(v.iter().all(|&c| c == want), "{what}: {v:?}");
        };
        only(&seen["insert"], 2, "lone insert");
        only(&seen["new tail block"], 4, "insert that opens a tail block");
        only(&seen["freeze"], 6, "insert that freezes");
        only(&seen["freeze, a group waiting"], 7, "insert that freezes with a group waiting");
        // A split of M's leaf: the tail block, the leaf, its new sibling, the parent, the root (area 1's
        // record) and the index directory block (M's node count).
        only(&seen["insert splitting M's leaf"], 6, "insert that splits M's leaf");
        maintain_all(&mut tb);
        // Deletes: a sealed row (3: M's leaf, the group page, the tail block); a buffered row outside the tail
        // block (3: M's leaf, its block, the tail block) and in it (2). A removal that rebalances M (keys
        // inserted in order leave M's leaves half full) changes more of M; those are counted apart.
        let area1 = |tb: &Table<'_, VecHost>| -> std::collections::BTreeSet<u64> {
            let list = AreaRecord::read(tb.host(), tb.root(), 1).unwrap().page_list;
            let mut s = std::collections::BTreeSet::new();
            let mut from = 0;
            loop {
                let w = list.walk_max(tb.host(), from, RADIX_STEP, |_, p| {
                    s.insert(p);
                    Ok(())
                });
                match w.unwrap().next {
                    Some(n) => from = n,
                    None => return s,
                }
            }
        };
        let mut rebalanced = 0;
        let mut del = |tb: &mut Table<'_, VecHost>, key: u64| -> u64 {
            tb.host_mut().end_message();
            tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(key))])).unwrap();
            let pages = area1(tb);
            let m = tb.host().changed_set().into_iter().filter(|b| pages.contains(&(b - b % PAGE))).count();
            let c = end_apart(tb, &mut ext_changes);
            if m == 1 {
                c
            } else {
                rebalanced += 1;
                0
            }
        };
        let sealed: Vec<u64> = (100..140).map(|x| del(&mut tb, x)).filter(|&c| c != 0).collect();
        for _ in 0..300 {
            tb.upsert(&mut key_scratch(), &mut Src::new(&row(k))).unwrap();
            k += 1;
        }
        let tail = root::u64_at(tb.host(), tb.root(), root::TAIL).unwrap();
        let in_tail = |tb: &Table<'_, VecHost>, key: u64| {
            let rid = rid_of(tb, &[(id(0), V::U(key))]).unwrap();
            let f = read::find(tb.host(), tb.root(), rid).unwrap();
            matches!(f, Some(read::Row::Buffered(at)) if at.block == tail)
        };
        let buffered_keys: Vec<u64> = (k - 300..k - 9).collect();
        let (mut out_tail, mut at_tail) = (Vec::new(), Vec::new());
        let tail_keys: Vec<u64> = (k - 8..k).rev().collect();
        for key in buffered_keys.iter().step_by(17).chain(tail_keys.iter()) {
            let t = in_tail(&tb, *key);
            let c = del(&mut tb, *key);
            if c == 0 {
                continue;
            }
            if t {
                at_tail.push(c);
            } else {
                out_tail.push(c);
            }
        }
        only(&sealed, 3, "delete of a sealed row");
        only(&out_tail, 3, "delete of a buffered row outside the tail block");
        only(&at_tail, 2, "delete of a buffered row in the tail block");
        // Updates of buffered rows: in place, a counted leaf (2: its block, the tail block's running record);
        // growing, into the tail block (2: the old block's stub, the tail block).
        let mut upd = |tb: &mut Table<'_, VecHost>, r: Vec<(u32, V)>| -> u64 {
            tb.host_mut().end_message();
            let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(&r)).unwrap();
            assert!(matches!(u, Upsert::Replaced(_)));
            end_apart(tb, &mut ext_changes)
        };
        let mut in_place = Vec::new();
        let mut growing = Vec::new();
        for key in buffered_keys.iter().skip(3).step_by(29) {
            if rid_of(&tb, &[(id(0), V::U(*key))]).is_none() || in_tail(&tb, *key) {
                continue;
            }
            let mut r = row(*key);
            r[1].1 = V::I(1);
            in_place.push(upd(&mut tb, r.clone()));
            r[2].1 = V::Bytes(vec![b'g'; 60]);
            growing.push(upd(&mut tb, r));
        }
        only(&in_place, 2, "update in place of a counted leaf");
        only(&growing, 2, "growing update into the tail block");
        assert!(ext_changes > 0, "writes moved extremes");
        (seen, sealed, out_tail, at_tail, in_place, growing, ext_changes)
    });
}

#[test]
fn changed_blocks_of_a_total_in_the_totals_space() {
    // 80 integer leaves: the hot state holds the running records of the first ones, the totals space the
    // totals of the others, leaf 71's among them. An update in place of a buffered row outside the tail block,
    // inside its block's bounds, of a leaf whose total lies in the totals space changes that block and the
    // totals block alone.
    let n = 80;
    let img = wide_schema(n);
    per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 12, &img).unwrap();
        {
            let (h, shape) = (tb.host(), tb.shape());
            let tail = root::u64_at(h, tb.root(), root::TAIL).unwrap();
            let l = shape.leaf(h, 71).unwrap();
            let at = native_table::table::hot::place_of(h, shape, tail, &l).unwrap();
            assert!(matches!(at, native_table::table::totals::At::Space(_)), "{at:?}");
        }
        let vals = |k: u64| -> Vec<i64> { (1..n).map(|c| (k as i64 * 31 + c as i64) % 1_000).collect() };
        for k in 0..60u64 {
            tb.upsert(&mut key_scratch(), &mut Src::new(&wide_row(n, k, &vals(k), 4))).unwrap();
        }
        let tail = root::u64_at(tb.host(), tb.root(), root::TAIL).unwrap();
        let mut counts = Vec::new();
        for k in 0..20u64 {
            let rid = rid_of(&tb, &[(id(0), V::U(k))]).unwrap();
            let Some(read::Row::Buffered(at)) = read::find(tb.host(), tb.root(), rid).unwrap() else { panic!() };
            if at.block == tail {
                continue;
            }
            // A leaf whose total lies in the totals space, and one whose total the hot state holds set to the
            // value it holds.
            let mut v = vals(k);
            v[70] += 1;
            v[3] = vals(k)[3];
            tb.host_mut().end_message();
            tb.upsert(&mut key_scratch(), &mut Src::new(&wide_row(n, k, &v, 4))).unwrap();
            counts.push(tb.host_mut().end_message());
        }
        assert!(!counts.is_empty() && counts.iter().all(|&c| c == 2), "{counts:?}");
        counts
    });
}

// ---------------------------------------------------------------- wide rows

/// A schema of a Nat key, `n − 1` i64 leaves and a Text: n integer leaves.
fn wide_schema(n: usize) -> Vec<u8> {
    let mut cols = vec![t::scalar(ty::NAT)];
    cols.extend((1..n).map(|_| t::scalar(ty::I64)));
    cols.push(t::scalar(ty::TEXT));
    schema(&cols).0
}

fn wide_row(n: usize, k: u64, vals: &[i64], text: usize) -> Vec<(u32, V)> {
    let mut r = vec![(id(0), V::U(k))];
    r.extend((1..n).map(|c| (id(c as u32), V::I(vals[c - 1]))));
    r.push((id(n as u32), V::Bytes(vec![b't'; text])));
    r
}

// ---------------------------------------------------------------- golden groups

/// Compares an image with `golden/groups/<name>.bin`, or writes it when `NT_GOLDEN_WRITE` is set.
fn golden(name: &str, img: &[u8]) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../golden/groups");
    let path = dir.join(format!("{name}.bin"));
    if std::env::var_os("NT_GOLDEN_WRITE").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, img).unwrap();
        return;
    }
    let want = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(want.len(), img.len(), "{name}: length");
    if let Some(i) = want.iter().zip(img).position(|(a, b)| a != b) {
        panic!("{name}: first differing byte at {i}");
    }
}

/// Fills the open group until it freezes (the freezing row goes to the next group), deletes the keys `kill`
/// says, seals, and returns the sealed group's page and the live rows.
fn one_group(
    tb: &mut Table<'_, VecHost>,
    mut row: impl FnMut(u64) -> Vec<(u32, V)>,
    kill: impl Fn(u64) -> bool,
) -> (u64, BTreeMap<u64, Vec<(u32, V)>>) {
    let g = root::u64_at(tb.host(), tb.root(), root::OPEN_GROUP).unwrap();
    let mut rows = BTreeMap::new();
    let mut k = 0;
    loop {
        let r = row(k);
        tb.upsert(&mut key_scratch(), &mut Src::new(&r)).unwrap();
        if root::u32_at(tb.host(), tb.root(), root::FROZEN_WAITING).unwrap() > 0 {
            break;
        }
        rows.insert(k, r);
        k += 1;
    }
    for key in rows.keys().copied().filter(|&x| kill(x)).collect::<Vec<_>>() {
        tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), rows[&key][0].1.clone())])).unwrap();
        rows.remove(&key);
    }
    maintain_all(tb);
    (group_page(tb, g), rows)
}

/// Every row of `rows` reads back from the table as written.
fn reads_back(tb: &Table<'_, VecHost>, m: Option<&Model>, rows: &BTreeMap<u64, Vec<(u32, V)>>) {
    for row in rows.values() {
        let (_, got) = get_row(tb, &row[..1]).expect("present");
        let want = match m {
            Some(m) => expected(m, row),
            None => row.clone(),
        };
        assert_eq!(got, want);
    }
}

/// Text of `len` bytes: `pre` ASCII bytes, then `ch` repeated while it fits, then ASCII.
fn fixed_text(len: usize, pre: usize, ch: &str) -> Vec<u8> {
    let mut v = vec![b'x'; pre];
    while v.len() + ch.len() <= len {
        v.extend_from_slice(ch.as_bytes());
    }
    v.resize(len, b'y');
    v
}

/// The live value of a Text leaf holding image offset `b`, given the leaf's value places in the image: its
/// key and the offset of `b` in it.
fn value_at(tb: &Table<'_, VecHost>, pg: u64, o: u64, rows: &BTreeMap<u64, Vec<(u32, V)>>, b: u64) -> (u64, u64) {
    let gv = GroupView::open(tb.host(), pg).unwrap();
    let ld = gv.leaf(o).unwrap();
    for (key, _) in rows {
        let rid = rid_of(tb, &[(id(0), V::U(*key))]).unwrap();
        let pos = rid % GROUP_SLOTS;
        let slot = gv.slot(pos).unwrap().unwrap();
        if let group::GCell::Bytes { off, len } = gv.cell(&ld, slot, pos).unwrap() {
            if off <= b && b < off + len {
                return (*key, b - off);
            }
        }
    }
    panic!("no value holds image offset {b}");
}

/// Builds a group whose Text leaf (ordinal 1) has characters of the given widths straddling image offsets
/// 65,536 and 131,072: values of fixed length, so the places do not move; a first build finds the values
/// holding the two offsets, a second gives them the characters.
fn straddling_group(h: &mut VecHost, len: usize, live: u64, widths: [usize; 2]) -> Vec<u8> {
    let (img, _) = schema(&[t::scalar(ty::NAT), t::scalar(ty::TEXT)]);
    let ch = |w: usize| match w {
        2 => "é",
        3 => "€",
        _ => "😀",
    };
    let mut plan: BTreeMap<u64, (usize, &str)> = BTreeMap::new();
    for round in 0..2 {
        let (mut tb, _) = Table::create(&mut *h, 1, &img).unwrap();
        let (pg, rows) = one_group(
            &mut tb,
            |k| {
                let (pre, c) = plan.get(&k).copied().unwrap_or((0, "a"));
                vec![(id(0), V::U(k)), (id(1), V::Bytes(fixed_text(len, pre, c)))]
            },
            |k| k >= live,
        );
        if round == 0 {
            for (i, edge) in [PAGE, 2 * PAGE].into_iter().enumerate() {
                let (key, o) = value_at(&tb, pg, 1, &rows, edge);
                let w = widths[i];
                assert!(o >= 1 && o as usize + w < len, "an edge inside a value");
                // A character starts one byte before the edge, so it straddles it.
                let pre = (o as usize - 1) % w;
                plan.insert(key, (pre, ch(w)));
            }
            continue;
        }
        let bytes = image_bytes(tb.host(), pg);
        for edge in [PAGE, 2 * PAGE] {
            assert_eq!(bytes[edge as usize] & 0xC0, 0x80, "a character straddles image offset {edge}");
            assert!(bytes[edge as usize - 1] >= 0xC0, "it starts one byte before");
        }
        reads_back(&tb, None, &rows);
        return bytes;
    }
    unreachable!()
}

#[test]
fn golden_groups() {
    let m = model_schema();
    per_fill(|h, _| {
        let mut out = Vec::new();
        // Every flat kind, nulls, medium and huge values and big integers, compacted: 4,000 of 4,096 rows
        // deleted.
        {
            let (mut tb, _) = Table::create(&mut *h, 21, &m.img).unwrap();
            let mut r = XorShift(2_024);
            let (pg, rows) = one_group(
                &mut tb,
                |k| {
                    let mut row = model_row(&mut r, 10 + 3 * k, k % 1_000 == 7);
                    if k % 1_000 == 7 {
                        row[1].1 = V::Bytes(vec![b'h'; HUGE_VALUE as usize + 1 + k as usize]);
                    }
                    row
                },
                |k| k % 43 != 0,
            );
            let img = image_bytes(tb.host(), pg);
            assert_ne!(u16::from_le_bytes(img[46..48].try_into().unwrap()) & group::GF_COMPACTED, 0);
            check_layout(&img);
            reads_back(&tb, Some(&m), &rows);
            golden("flat_kinds", &img);
            out.push(("flat_kinds", img.len(), nt_testhost_hash(&img)));
        }
        // Narrow leaves over every position, 300 dead positions stored as nulls; the key a `Seq`.
        {
            let (img, _) = schema(&[
                t::scalar(ty::NAT),
                t::scalar(ty::I32),
                t::scalar(ty::BOOL),
                t::scalar(ty::U8),
                t::opt(t::scalar(ty::U16)),
                t::scalar(ty::I64),
            ]);
            let (mut tb, _) = Table::create(&mut *h, 22, &img).unwrap();
            let (pg, rows) = one_group(
                &mut tb,
                |k| {
                    vec![
                        (id(0), V::U(1_000 + 2 * k)),
                        (id(1), V::I(k as i64 % 300 - 150)),
                        (id(2), V::Bool(k % 5 == 0)),
                        (id(3), V::U(k % 3)),
                        (id(4), if k % 7 == 0 { V::Null } else { V::U(k % 60_000) }),
                        (id(5), V::I(-7)),
                    ]
                },
                |k| k % 13 == 4,
            );
            let img = image_bytes(tb.host(), pg);
            assert_eq!(u16::from_le_bytes(img[46..48].try_into().unwrap()) & group::GF_COMPACTED, 0);
            let gv = GroupView::open(tb.host(), pg).unwrap();
            assert_eq!(gv.leaf(0).unwrap().form, group::form::SEQ, "keys 1,000 + 2·position");
            assert_eq!(gv.leaf(5).unwrap().form, group::form::SEQ, "a constant");
            assert_eq!(gv.leaf(1).unwrap().form, group::form::I16);
            check_layout(&img);
            reads_back(&tb, None, &rows);
            golden("flat_plain", &img);
            out.push(("flat_plain", img.len(), nt_testhost_hash(&img)));
        }
        // Text across image blocks: 2- and 3-byte characters in the inline data region; 4- and 2-byte ones
        // in the out-of-line area.
        let img = straddling_group(h, 1_000, 150, [2, 3]);
        check_layout(&img);
        golden("flat_inline", &img);
        out.push(("flat_inline", img.len(), nt_testhost_hash(&img)));
        let img = straddling_group(h, 2_500, 60, [4, 2]);
        check_layout(&img);
        assert!(u64::from_le_bytes(img[88..96].try_into().unwrap()) < PAGE, "the out-of-line area holds the edges");
        golden("flat_ool", &img);
        out.push(("flat_ool", img.len(), nt_testhost_hash(&img)));
        out
    });
}

// ---------------------------------------------------------------- the probe's entry points

#[test]
fn probe_entry_points() {
    assert_eq!(nt_probe::nt_selftest(), 0);
    let mut x = XorShift(99);
    for _ in 0..300 {
        let (a, b) = (x.next_u64(), x.next_u64());
        nt_probe::nt_group_read(a, b, x.below(8), x.below(5_000));
    }
    for s in 0..6 {
        let v = nt_probe::nt_table(s, 280);
        assert!(v < u64::MAX - 64, "nt_table faulted: {v}");
    }
}

// ---------------------------------------------------------------- keys of any length

/// A key of `n` ASCII bytes, none of them zero, which E(k) escapes into `n + 2` bytes; `salt` makes keys
/// distinct.
fn long_key(n: usize, salt: u8) -> Vec<u8> {
    (0..n).map(|i| 1 + ((i as u64 * 31 + u64::from(salt)) % 126) as u8).collect()
}

/// Point operations take keys whose E(k) passes the stack's `KEY_STACK` bytes: E(k) is built in the
/// scratch, and stored keys are compared in pieces. Text keys of E(k) = 4,097 bytes (one past the stack),
/// 65,538 (a 64 KiB key) and 10^6 bytes, and a Nat key of 6,000 LEB128 bytes, through upsert, get and remove,
/// with short keys beside them; then the same reads after maintenance.
#[test]
fn keys_of_any_length() {
    use native_table::table::KEY_STACK;
    assert_eq!(KEY_STACK, 4_096);
    let (img, _) = schema(&[t::scalar(ty::TEXT), t::scalar(ty::I64)]);
    per_fill(|h, lh| {
        let fill = h.fill_pattern();
        let (mut tb, _) = Table::create(h, 21, &img).unwrap();
        let mut sc = VecScratch::new(fill);
        let keys: Vec<Vec<u8>> =
            vec![long_key(4_095, 1), long_key(65_536, 2), long_key(999_998, 3), b"short".to_vec(), long_key(4_094, 4)];
        let key = |k: &[u8]| vec![(id(0), V::Bytes(k.to_vec()))];
        let value = |tb: &Table<'_, VecHost>, sc: &mut VecScratch, k: &[u8]| -> Option<i64> {
            let mut out = Sink { cells: Vec::new(), rid: None };
            if !tb.get(sc, &mut Src::new(&key(k)), &mut out).unwrap() {
                return None;
            }
            out.cells.iter().find(|(l, _)| *l == id(1)).map(|(_, v)| match v {
                V::I(x) => *x,
                v => panic!("{v:?}"),
            })
        };
        let mut rids = Vec::new();
        for (i, k) in keys.iter().enumerate() {
            let mut row = key(k);
            row.push((id(1), V::I(i as i64)));
            let (u, _) = tb.upsert(&mut sc, &mut Src::new(&row)).unwrap();
            let Upsert::Inserted(rid) = u else { panic!("a new key inserts: {u:?}") };
            rids.push(rid);
            assert_eq!(tb.upsert(&mut sc, &mut Src::new(&row)).unwrap().0, Upsert::Replaced(rid), "a present key");
            lh.take(tb.host_mut());
        }
        for (i, k) in keys.iter().enumerate() {
            assert_eq!(value(&tb, &mut sc, k), Some(i as i64), "key of {} bytes", k.len());
            assert_eq!(rid_of(&tb, &key(k)), Some(rids[i]));
            // A key that differs only in its last byte is another key.
            let mut other = k.clone();
            *other.last_mut().unwrap() ^= 0x40;
            assert_eq!(value(&tb, &mut sc, &other), None);
        }
        for (i, k) in keys.iter().enumerate() {
            let mut row = key(k);
            row.push((id(1), V::I(200 + i as i64)));
            let (u, _) = tb.upsert(&mut sc, &mut Src::new(&row)).unwrap();
            assert_eq!(u, Upsert::Replaced(rids[i]));
            assert_eq!(value(&tb, &mut sc, k), Some(200 + i as i64));
        }
        // The 10^6-byte key goes, and comes back under a new rid.
        let (gone, _) = tb.remove(&mut sc, &mut Src::new(&key(&keys[2]))).unwrap();
        assert_eq!(gone, Some(rids[2]));
        assert_eq!(value(&tb, &mut sc, &keys[2]), None);
        assert_eq!(tb.remove(&mut sc, &mut Src::new(&key(&keys[2]))).unwrap().0, None);
        let mut row = key(&keys[2]);
        row.push((id(1), V::I(7)));
        assert!(matches!(tb.upsert(&mut sc, &mut Src::new(&row)).unwrap().0, Upsert::Inserted(_)));
        maintain_all(&mut tb);
        for (i, k) in keys.iter().enumerate() {
            let want = if i == 2 { 7 } else { 200 + i as i64 };
            assert_eq!(value(&tb, &mut sc, k), Some(want), "key of {} bytes after maintenance", k.len());
        }
        assert!(sc.grows > 0, "the long keys were built in the scratch");
        lh.take(tb.host_mut());
        tb.len().unwrap()
    });
    // A Nat key of 6,000 LEB128 bytes: E(k) about 5,250 bytes, its stored LEB128 bytes read through a window.
    let (img, _) = schema(&[t::scalar(ty::NAT), t::scalar(ty::I64)]);
    per_fill(|h, _| {
        let fill = h.fill_pattern();
        let (mut tb, _) = Table::create(h, 22, &img).unwrap();
        let mut sc = VecScratch::new(fill);
        let nat = |salt: u8| {
            let mut v: Vec<u8> = (0..6_000u32).map(|i| 0x80 | (((i as u8).wrapping_mul(7) ^ salt) & 0x7F)).collect();
            *v.last_mut().unwrap() = 0x55;
            V::Nat(v)
        };
        for s in 0..3u8 {
            let row = vec![(id(0), nat(s)), (id(1), V::I(i64::from(s)))];
            assert!(matches!(tb.upsert(&mut sc, &mut Src::new(&row)).unwrap().0, Upsert::Inserted(_)));
        }
        for s in 0..3u8 {
            assert!(rid_of(&tb, &[(id(0), nat(s))]).is_some());
            let row = vec![(id(0), nat(s)), (id(1), V::I(9))];
            assert!(matches!(tb.upsert(&mut sc, &mut Src::new(&row)).unwrap().0, Upsert::Replaced(_)));
        }
        assert!(rid_of(&tb, &[(id(0), nat(9))]).is_none());
        assert!(tb.remove(&mut sc, &mut Src::new(&[(id(0), nat(1))])).unwrap().0.is_some());
        tb.len().unwrap()
    });
}
