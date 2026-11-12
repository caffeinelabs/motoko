//! Emptying a table with `clear`: tables of buffered rows only, of packed groups with patches, long values, two
//! indexes and a queued compaction, and an empty one. After the clear every read finds no row and the compaction
//! queue is empty; rows written after it read back. Its cost does not grow with the table. Maintenance then gives
//! back every page the old rows used except the root block's, a few pages a step: the host lends exactly the
//! pages the table's areas hold, area 15 holds the schema page alone, and the blocks in use are an empty table's
//! and one radix block. The host refuses any access to a page it took back, so no block of one is still read. A
//! second and a third clear before the first one's memory is all back are reclaimed in order. Rows removed one at
//! a time give back, through the sweeps of maintenance, the pages whose blocks are all free, while rows written
//! during a sweep read back; a clear in the middle of a sweep gives back the rest.

use native_table::area::AreaRecord;
use native_table::consts::{AREAS, GROUP_SLOTS};
use native_table::index::secondary::Start;
use native_table::mem::rd_u64;
use native_table::schema::IndexDecl;
use native_table::table::jobs::{self, Ran};
use native_table::table::root;
use native_table::table::totals::Total;
use native_table::table::{Table, Upsert};
use native_table::tree::Dir;
use native_table::value::ty;
use native_table::{PageHost, Region, Wake};
use nt_testhost::rows::{schema_indexed, t, Sink, Src, V};
use nt_testhost::{Call, VecHost, VecScratch, FILLS};

fn scratch() -> VecScratch {
    VecScratch::new(FILLS[0])
}

/// Key, an indexed Nat, an indexed Text and a Text that is long now and then.
fn image() -> Vec<u8> {
    let ix = |id: u8, leaf: u32| {
        let mut part = [(0, false); 7];
        part[0] = (leaf, false);
        IndexDecl { id, flags: 0, w: 16, parts: 1, part }
    };
    let cols = [t::scalar(ty::NAT), t::scalar(ty::NAT), t::scalar(ty::TEXT), t::scalar(ty::TEXT)];
    schema_indexed(&cols, &[ix(1, 1 << 16), ix(2, 2 << 16)]).0
}

fn row(k: u64, v: u64) -> Vec<(u32, V)> {
    let long = if k % 97 == 0 { "x".repeat(70_000) } else { format!("note {k}") };
    vec![(0, V::U(k)), (1 << 16, V::U(v)), (2 << 16, V::text(&format!("t{}", k % 13))), (3 << 16, V::text(&long))]
}

fn put(tb: &mut Table<'_, VecHost>, k: u64, v: u64) -> Upsert {
    tb.write_step().unwrap();
    tb.upsert(&mut scratch(), &mut Src::new(&row(k, v))).unwrap().0
}

/// Every row a scan hands out, with the walks of both indexes and the sum of the Nat.
fn rows(tb: &Table<'_, VecHost>) -> (Vec<(u32, V)>, usize, u64) {
    let mut s = Sink::default();
    assert_eq!(tb.scan_where(None, 0, u64::MAX, &mut s).unwrap(), None);
    let mut walked = 0;
    for ix in [1, 2] {
        tb.walk(ix, Dir::Fwd, Start::End, u64::MAX, &mut [0; 256], |_, _| Ok(walked += 1)).unwrap();
    }
    let (total, n) = tb.sum(1 << 16, &mut [0; 64]).unwrap();
    let want: u64 = s
        .cells
        .iter()
        .filter_map(|(l, v)| if let (true, V::U(x)) = (*l == 1 << 16, v) { Some(*x) } else { None })
        .sum();
    assert_eq!(total, Total::Small(i128::from(want)), "the sum of the Nat");
    (s.cells, walked, n)
}

/// Pages the table's areas hold.
fn held_pages(h: &VecHost, tbl: u64) -> u64 {
    (0..AREAS as u32).map(|a| nt_testhost::held_pages(h, tbl, a)).sum()
}

/// Blocks in use in area 0: its pages' blocks less the never-used and the free ones.
fn used0(h: &VecHost, tbl: u64) -> u64 {
    let rec = AreaRecord::read(h, tbl, 0).unwrap();
    let mut free = 0;
    let mut b = rec.free_head;
    while b != 0 {
        free += 1;
        b = rd_u64(h.get(Region::Block(b)).unwrap(), 0).unwrap();
    }
    nt_testhost::held_pages(h, tbl, 0) * 16 - (16 - u64::from(rec.bump)) - free
}

/// Runs maintenance steps until none is pending; each step gives back at most 16 pages, but for those that give
/// back a group's images, one a group. Returns the steps.
fn reclaim(tb: &mut Table<'_, VecHost>) -> u64 {
    let (mut steps, mut groups) = (0, 0);
    while tb.wake().unwrap() != Wake::Idle {
        tb.host_mut().take_log();
        let ran = tb.step().unwrap();
        let freed = tb.host_mut().take_log().iter().filter(|c| matches!(c, Call::PageFree(_))).count();
        groups += u64::from(ran == Ran::Free && freed > 16);
        steps += 1;
    }
    assert!(groups <= 4, "{groups} steps gave back more than 16 pages");
    steps
}

/// Clears the table and runs maintenance to the end: the host then lends exactly the pages the table's areas hold,
/// area 15 holds the schema page alone, and the blocks in use are an empty table's and one radix block.
fn cleared(tb: &mut Table<'_, VecHost>, fill: u8) {
    tb.clear().unwrap();
    reclaim(tb);
    let mut fresh = VecHost::new(fill);
    let (_, ftbl) = Table::create(&mut fresh, 7, &image()).unwrap();
    let (h, tbl) = (tb.host(), tb.root());
    assert_eq!(h.live_pages() as u64, held_pages(h, tbl), "the host's pages are the table's");
    assert_eq!(AreaRecord::read(h, tbl, 15).unwrap().pages, 1, "the schema page alone");
    assert_eq!(used0(h, tbl), used0(&fresh, ftbl) + 1, "an empty table's blocks and a radix block");
}

/// A table filled with `n` rows: replaced, removed and long rows among them, a compaction queued once a group is
/// packed.
fn filled(h: &mut VecHost, n: u64) -> u64 {
    let (mut tb, tbl) = Table::create(h, 7, &image()).unwrap();
    for k in 0..n {
        assert!(matches!(put(&mut tb, k, k), Upsert::Inserted(_)));
    }
    for k in (0..n).step_by(3) {
        assert!(matches!(put(&mut tb, k, k + 1), Upsert::Replaced(_)));
    }
    for k in (0..n).step_by(4) {
        tb.remove(&mut scratch(), &mut Src::new(&[(0, V::U(k))])).unwrap();
    }
    tbl
}

#[test]
fn clear_tables() {
    for fill in FILLS {
        for n in [0, 300, GROUP_SLOTS + 700, 3 * GROUP_SLOTS] {
            let mut h = VecHost::new(fill);
            h.set_logging(false);
            let tbl = filled(&mut h, n);
            h.set_logging(true);
            let mut tb = Table::open(&mut h, tbl).unwrap();
            assert_eq!(tb.clear().unwrap(), Wake::Later);
            assert!(!jobs::queued(tb.host(), tbl).unwrap(), "no compaction queued");
            assert_eq!(tb.len().unwrap(), 0);
            assert_eq!(rows(&tb), (vec![], 0, 0));
            for k in [0, 1, 97, n.saturating_sub(1)] {
                assert!(!tb.get(&mut scratch(), &mut Src::new(&[(0, V::U(k))]), &mut Sink::default()).unwrap());
            }
            assert_eq!(tb.count(&mut scratch(), 1, &mut Src::new(&[(1 << 16, V::U(1))])).unwrap(), 0);
            // Rows written after the clear read back, while and after its memory goes back.
            for k in 0..200 {
                assert!(matches!(put(&mut tb, k, 2 * k), Upsert::Inserted(_)));
            }
            let steps = reclaim(&mut tb);
            let (cells, walked, count) = rows(&tb);
            assert_eq!((cells.len(), walked, count), (800, 400, 200), "{n} rows: {steps} steps");
            cleared(&mut tb, fill);
        }
    }
}

/// Area 0 and area 1 of more than 512 pages: area 0's page list has a radix block of each height.
#[test]
fn clear_large_areas() {
    let mut h = VecHost::new(FILLS[0]);
    h.set_logging(false);
    let (_, tbl) = Table::create(&mut h, 7, &image()).unwrap();
    for _ in 0..16 * 1_100 {
        native_table::area::alloc_block(&mut h, tbl, 0).unwrap();
    }
    for _ in 0..16 * 600 {
        native_table::area::alloc_block(&mut h, tbl, 1).unwrap();
    }
    assert_eq!(AreaRecord::read(&h, tbl, 0).unwrap().page_list.height, 2);
    let mut tb = Table::open(&mut h, tbl).unwrap();
    tb.clear().unwrap();
    let steps = reclaim(&mut tb);
    assert!(steps >= (1_100 + 600) / 16, "{steps} steps");
    let h = tb.host();
    assert_eq!(h.live_pages() as u64, held_pages(h, tbl));
}

#[test]
fn clear_while_reclaiming() {
    let mut h = VecHost::new(FILLS[1]);
    h.set_logging(false);
    let tbl = filled(&mut h, 2 * GROUP_SLOTS + 100);
    let mut tb = Table::open(&mut h, tbl).unwrap();
    tb.clear().unwrap();
    for k in 0..GROUP_SLOTS + 50 {
        put(&mut tb, k, k);
    }
    // Part of the first clear's memory is back; a second clear parks the rows written since, and a third, in the
    // middle of the second's pages, parks the rows written since that.
    tb.clear().unwrap();
    for k in 0..300 {
        put(&mut tb, k, k);
    }
    for _ in 0..40 {
        tb.step().unwrap();
    }
    tb.clear().unwrap();
    for k in 0..100 {
        put(&mut tb, k, 1);
    }
    reclaim(&mut tb);
    assert_eq!(tb.len().unwrap(), 100);
    assert_eq!(rows(&tb).2, 100);
    cleared(&mut tb, FILLS[1]);
}

#[test]
fn removals_give_back_pages() {
    for fill in FILLS {
        let n = 3 * GROUP_SLOTS;
        let mut h = VecHost::new(fill);
        h.set_logging(false);
        let tbl = filled(&mut h, n);
        let mut tb = Table::open(&mut h, tbl).unwrap();
        reclaim(&mut tb);
        let own01 = |h: &VecHost| nt_testhost::held_pages(h, tbl, 0) + nt_testhost::held_pages(h, tbl, 1);
        let full = own01(tb.host());
        let (mut kept, mut swept) = (0u64, 0u64);
        for k in 0..n {
            tb.write_step().unwrap();
            swept += u64::from(root::u8_at(tb.host(), tbl, root::SWEEP_STAGE).unwrap() != 0);
            tb.remove(&mut scratch(), &mut Src::new(&[(0, V::U(k))])).unwrap();
            if k % 512 == 0 {
                put(&mut tb, n + k, k);
                kept += k;
            }
        }
        reclaim(&mut tb);
        let h = tb.host();
        let (live, own) = (h.live_pages() as u64, held_pages(h, tbl));
        assert_eq!(live, own, "the host's pages are the table's");
        assert!(swept > 0 && 4 * own01(h) < full, "{swept} steps in a sweep: {full} pages, then {}", own01(h));
        let (cells, walked, count) = rows(&tb);
        assert_eq!((cells.len() as u64, walked as u64, count), (4 * n / 512, 2 * n / 512, n / 512));
        let (total, _) = tb.sum(1 << 16, &mut [0; 64]).unwrap();
        assert_eq!(total, Total::Small(i128::from(kept)));
        // A clear while a sweep has blocks taken.
        for k in 0..n {
            put(&mut tb, k, k);
        }
        for k in 0..n {
            tb.remove(&mut scratch(), &mut Src::new(&[(0, V::U(k))])).unwrap();
        }
        while root::u64_at(tb.host(), tbl, root::SWEPT_N).unwrap() == 0 {
            assert_ne!(tb.step().unwrap(), Ran::Nothing, "a sweep takes blocks");
        }
        cleared(&mut tb, fill);
    }
}
