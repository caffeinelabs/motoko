//! A key of several parts: a table whose key is a tuple of a Text, a Nat and an Int has a key leaf per part,
//! the ids of column 0. Rows are inserted, replaced, removed and read back by their keys, and a key that differs
//! from a row's in one part finds no row; a scan and an index walk hand each row out with its key parts first; a
//! compaction and a change of the row type keep every row under its key; rows removed by a range of the index
//! are found by the keys read from them.

use native_table::consts::GROUP_SLOTS;
use native_table::index::secondary::Start;
use native_table::schema::IndexDecl;
use native_table::table::{Table, Upsert};
use native_table::tree::Dir;
use native_table::value::ty;
use native_table::{Progress, Wake};
use nt_testhost::rows::{schema_versioned, t, Sink, Src, VCol, V};
use nt_testhost::{TestBudget, VecHost, VecScratch, FILLS};

fn scratch() -> VecScratch {
    VecScratch::new(FILLS[0])
}

/// Row k's key: a Text of k mod 7, a Nat of k / 7 and an Int of −k, parts 0 to 2 of column 0.
fn key(k: u64) -> Vec<(u32, V)> {
    vec![(0, V::text(&format!("part {}", k % 7))), (1, V::U(k / 7)), (2, V::I(-(k as i64)))]
}

/// Row k: its key, a Nat of 2k + `v` and a Text.
fn row(k: u64, v: u64) -> Vec<(u32, V)> {
    let mut r = key(k);
    r.extend([(1 << 16, V::U(2 * k + v)), (2 << 16, V::text(&format!("row {k}")))]);
    r
}

/// The schema image of version 1: the key, a Nat with an index, a Text; version 2 adds an optional Nat.
fn image(version: u32) -> Vec<u8> {
    let col = |ty: Vec<u8>, added: u32| VCol { ty, added, dropped: 0, default: None };
    let mut cols = vec![
        col(vec![ty::TUPLE, 3, 0, ty::TEXT, ty::NAT, ty::INT], 1),
        col(t::scalar(ty::NAT), 1),
        col(t::scalar(ty::TEXT), 1),
    ];
    if version == 2 {
        cols.push(col(t::opt(t::scalar(ty::NAT)), 2));
    }
    let mut part = [(0, false); 7];
    part[0] = (1 << 16, false);
    schema_versioned(version, &cols, &[IndexDecl { id: 1, flags: 0, w: 16, parts: 1, part }]).0
}

fn get(tb: &Table<'_, VecHost>, k: &[(u32, V)]) -> Option<Vec<(u32, V)>> {
    let mut s = Sink::default();
    tb.get(&mut scratch(), &mut Src::new(k), &mut s).unwrap().then_some(s.cells)
}

/// Every live row as a scan hands it out, and as the index walk on the Nat does.
fn scans(tb: &Table<'_, VecHost>) -> (Vec<(u32, V)>, Vec<(u32, V)>) {
    let mut s = Sink::default();
    assert_eq!(tb.scan_where(None, 0, u64::MAX, &mut s).unwrap(), None);
    let mut rids = Vec::new();
    tb.walk(1, Dir::Fwd, Start::End, u64::MAX, &mut [0; 256], |_, rid| Ok(rids.push(rid))).unwrap();
    let mut w = Sink::default();
    for rid in rids {
        assert!(tb.get_rid(rid, &mut w).unwrap());
    }
    (s.cells, w.cells)
}

#[test]
fn tuple_keys() {
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &image(1)).unwrap();
    let n = GROUP_SLOTS + 500;
    for k in 0..n {
        assert!(matches!(tb.upsert(&mut scratch(), &mut Src::new(&row(k, 0))).unwrap().0, Upsert::Inserted(_)));
    }
    for k in (0..n).step_by(5) {
        assert!(matches!(tb.upsert(&mut scratch(), &mut Src::new(&row(k, 1))).unwrap().0, Upsert::Replaced(_)));
    }
    // A third of the packed group removed: a compaction is queued.
    for k in (0..n).step_by(3) {
        assert!(tb.remove(&mut scratch(), &mut Src::new(&key(k))).unwrap().0.is_some());
    }
    assert_ne!(tb.wake().unwrap(), Wake::Idle);
    let live: Vec<u64> = (0..n).filter(|k| k % 3 != 0).collect();
    let want = |k: u64, extra: &[(u32, V)]| [row(k, u64::from(k % 5 == 0)), extra.to_vec()].concat();
    for added in [false, true] {
        let extra = if added { vec![(3 << 16, V::Null)] } else { vec![] };
        for k in 0..n {
            assert_eq!(get(&tb, &key(k)), (k % 3 != 0).then(|| want(k, &extra)), "row {k}");
            for p in 0..3 {
                let mut other = key(k);
                other[p].1 = match &other[p].1 {
                    V::U(x) => V::U(x + 1),
                    V::I(x) => V::I(x - 1),
                    _ => V::text("part 7"),
                };
                assert_eq!(get(&tb, &other), None, "row {k}, part {p} changed");
            }
        }
        let (scanned, walked) = scans(&tb);
        let rows: Vec<(u32, V)> = live.iter().flat_map(|&k| want(k, &extra)).collect();
        assert_eq!((scanned, walked), (rows.clone(), rows));
        if !added {
            assert_eq!(tb.maintain(&TestBudget::new(u64::MAX)).unwrap(), Progress::Done);
            assert_eq!(tb.wake().unwrap(), Wake::Idle);
            tb.evolve(&image(2)).unwrap();
        }
    }
    let enc = |tb: &Table<'_, VecHost>, x: u64| {
        let mut b = [0u8; 32];
        let n = tb.index_key(1, &mut Src::new(&[(1 << 16, V::U(x))]), &mut b).unwrap();
        b[..n].to_vec()
    };
    let (from, to) = (enc(&tb, 1_000), enc(&tb, 3_000));
    let gone = |k: u64| k % 3 != 0 && (1_000..=3_000).contains(&(2 * k + u64::from(k % 5 == 0)));
    let (removed, _) = tb.remove_range(&mut scratch(), 1, Some(&from), Some(&to)).unwrap();
    assert_eq!(removed, (0..n).filter(|&k| gone(k)).count() as u64);
    for k in 0..n {
        let extra = [(3 << 16, V::Null)];
        assert_eq!(get(&tb, &key(k)), (k % 3 != 0 && !gone(k)).then(|| want(k, &extra)), "row {k} after the range");
    }
}
