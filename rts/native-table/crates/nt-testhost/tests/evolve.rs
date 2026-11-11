//! A change of a table's row type (`Table::evolve`): every refused image leaves every byte of the table as it
//! was, a dropped field coming back included; an image equal to the stored one changes nothing, an open group
//! whose rows are all deleted is sealed and the next insert works, an added integer field's total starts at
//! zero beside the others, and its minimum and maximum over the groups packed before it was added come from
//! their extremes records, without a row read; on a table that had no integer field, whose groups have no
//! extremes record, min, max and filters on an added one skip those groups; after a drop that shrinks the
//! integer fields' totals, rows that fit a block only under the smaller totals area are stored in place and
//! read back, and the sums hold.

use std::ops::Bound;

use native_table::consts::{BLOCK, PAGE};
use native_table::mem::{rd_u32, PageHost, Region};
use native_table::schema::IndexDecl;
use native_table::table::buffer::{RowLoc, DESC_BIT, SLOT};
use native_table::table::extremes::{self, Rec, HAS};
use native_table::table::filter::{Cond, Filter};
use native_table::table::totals::Total;
use native_table::table::{buffer, root, write, CellIn, Table, Upsert};
use native_table::value::ty;
use native_table::{Fault, Progress, SchemaError, Wake};
use nt_testhost::rows::{schema_indexed, schema_versioned, t, Sink, Src, VCol, V};
use nt_testhost::{TestBudget, VecHost, VecScratch, FILLS};

fn id(col: u32) -> u32 {
    col << 16
}

fn scratch() -> VecScratch {
    VecScratch::new(FILLS[0])
}

fn decl(index: u8, w: u16, parts: &[(u32, bool)]) -> IndexDecl {
    let mut part = [(0u32, false); 7];
    for (p, &x) in part.iter_mut().zip(parts) {
        *p = x;
    }
    IndexDecl { id: index, flags: 0, w, parts: parts.len() as u8, part }
}

/// The columns of version `version`: each type expression with the version that added it.
fn cols(version: u32, c: &[(Vec<u8>, u32)]) -> Vec<VCol> {
    assert!(c.iter().all(|(_, added)| *added <= version));
    c.iter().map(|(ty, added)| VCol { ty: ty.clone(), added: *added, dropped: 0, default: None }).collect()
}

fn image(version: u32, c: &[(Vec<u8>, u32)], decls: &[IndexDecl]) -> Vec<u8> {
    schema_versioned(version, &cols(version, c), decls).0
}

fn open_group(tb: &Table<'_, VecHost>) -> u64 {
    root::u64_at(tb.host(), tb.root(), root::OPEN_GROUP).unwrap()
}

fn open_positions(tb: &Table<'_, VecHost>) -> u64 {
    let tail = root::u64_at(tb.host(), tb.root(), root::TAIL).unwrap();
    buffer::u16_of(tb.host(), tail, buffer::H_OPEN_POS).unwrap()
}

#[test]
fn a_refused_image_changes_no_byte_and_an_equal_image_changes_nothing() {
    // Key Nat, Text (index 1), i64, ?Nat.
    let v1 =
        [(t::scalar(ty::NAT), 1), (t::scalar(ty::TEXT), 1), (t::scalar(ty::I64), 1), (t::opt(t::scalar(ty::NAT)), 1)];
    let decls = [decl(1, 16, &[(id(1), false)])];
    let (img, _) = schema_indexed(&v1.iter().map(|c| c.0.clone()).collect::<Vec<_>>(), &decls);
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &img).unwrap();
    // A sealed group and an open group holding rows, which an accepted change would seal.
    for k in 0..5_000u64 {
        let row =
            [(id(0), V::U(k)), (id(1), V::text(&format!("r{}", k % 7))), (id(2), V::I(-(k as i64))), (id(3), V::Null)];
        tb.upsert(&mut scratch(), &mut Src::new(&row)).unwrap();
    }
    assert!(open_positions(&tb) > 0);

    let opt_text = t::opt(t::scalar(ty::TEXT));
    let with = |c: (Vec<u8>, u32)| {
        let mut v = v1.to_vec();
        v.push(c);
        v
    };
    let mut retyped = v1.to_vec();
    retyped[2].0 = t::scalar(ty::I32);
    let mut rekeyed = v1.to_vec();
    rekeyed[0].0 = t::scalar(ty::INT);
    let refused: Vec<(&str, Vec<u8>, SchemaError)> = vec![
        ("a field's type changes", image(2, &retyped, &decls), SchemaError::ColumnChanged),
        ("a new field is not nullable", image(2, &with((t::scalar(ty::TEXT), 2)), &decls), SchemaError::ColumnChanged),
        ("the key's type changes", image(2, &rekeyed, &decls), SchemaError::KeyChanged),
        (
            "a new secondary index",
            image(2, &v1, &[decls[0], decl(2, 8, &[(id(2), false)])]),
            SchemaError::ColumnChanged,
        ),
        ("the version skips one", image(3, &with((opt_text.clone(), 3)), &decls), SchemaError::ColumnChanged),
        ("the version stays", image(1, &with((opt_text.clone(), 1)), &decls), SchemaError::ColumnChanged),
    ];
    for (what, im, e) in refused {
        let before = tb.host().memory().to_vec();
        let pages = tb.host().live_pages();
        assert_eq!(tb.evolve(&im), Err(Fault::Schema(e)), "{what}");
        assert!(tb.host().memory() == &before[..] && tb.host().live_pages() == pages, "{what}: a byte changed");
    }

    // The stored image again: nothing changes, the open group included.
    let before = tb.host().memory().to_vec();
    tb.evolve(&img).unwrap();
    assert!(tb.host().memory() == &before[..]);
}

#[test]
fn an_open_group_whose_rows_are_all_deleted_is_sealed_and_the_next_insert_works() {
    for fill in FILLS {
        let v1 = [(t::scalar(ty::NAT), 1), (t::scalar(ty::TEXT), 1)];
        let mut h = VecHost::new(fill);
        let (mut tb, _) = Table::create(&mut h, 1, &image(1, &v1, &[])).unwrap();
        for k in 0..100u64 {
            tb.upsert(&mut scratch(), &mut Src::new(&[(id(0), V::U(k)), (id(1), V::text("gone"))])).unwrap();
        }
        for k in 0..100u64 {
            assert_eq!(tb.remove(&mut scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap().0, Some(k));
        }
        assert_eq!((tb.len().unwrap(), open_group(&tb), open_positions(&tb)), (0, 0, 100));

        let mut v2 = v1.to_vec();
        v2.push((t::opt(t::scalar(ty::TEXT)), 2));
        tb.evolve(&image(2, &v2, &[])).unwrap();
        assert_eq!((tb.shape().version, tb.len().unwrap(), open_group(&tb), open_positions(&tb)), (2, 0, 1, 0));

        let row = [(id(0), V::U(7)), (id(1), V::text("new")), (id(2), V::text("added"))];
        tb.upsert(&mut scratch(), &mut Src::new(&row)).unwrap();
        assert_eq!(tb.maintain(&TestBudget::new(u64::MAX)).unwrap(), Progress::Done);
        assert_eq!(tb.wake().unwrap(), Wake::Idle);
        let mut s = Sink::default();
        assert!(tb.get(&mut scratch(), &mut Src::new(&[(id(0), V::U(7))]), &mut s).unwrap());
        assert_eq!(s.cells, row.to_vec());
        assert_eq!(tb.len().unwrap(), 1);
    }
}

#[test]
fn an_added_integer_field_sums_to_zero_and_the_other_sums_stay() {
    for fill in FILLS {
        // Key Nat, Nat, i64: three totals.
        let v1 = [(t::scalar(ty::NAT), 1), (t::scalar(ty::NAT), 1), (t::scalar(ty::I64), 1)];
        let mut h = VecHost::new(fill);
        let (mut tb, _) = Table::create(&mut h, 1, &image(1, &v1, &[])).unwrap();
        for k in 0..10_000u64 {
            let row = [(id(0), V::U(k)), (id(1), V::U(3 * k)), (id(2), V::I(-(k as i64)))];
            tb.upsert(&mut scratch(), &mut Src::new(&row)).unwrap();
        }
        let mut buf = [0u8; 64];
        let sums = |tb: &Table<'_, VecHost>, buf: &mut [u8]| {
            (0..3).map(|c| tb.sum(id(c), buf).unwrap()).collect::<Vec<(Total, u64)>>()
        };
        let before = sums(&tb, &mut buf);
        let n = 10_000i128;
        assert_eq!(
            before,
            vec![
                (Total::Small(n * (n - 1) / 2), 10_000),
                (Total::Small(3 * n * (n - 1) / 2), 10_000),
                (Total::Small(-n * (n - 1) / 2), 10_000)
            ]
        );

        let mut v2 = v1.to_vec();
        v2.push((t::opt(t::scalar(ty::NAT)), 2));
        tb.evolve(&image(2, &v2, &[])).unwrap();
        assert_eq!(tb.sum(id(3), &mut buf).unwrap(), (Total::Small(0), 0));
        assert_eq!(sums(&tb, &mut buf), before);
    }
}

/// Every group's extremes block, by group number.
fn extremes_blocks(tb: &Table<'_, VecHost>) -> Vec<u64> {
    let h = tb.host();
    (0..=open_group(tb))
        .map(|g| {
            let e = root::group(h, tb.root(), g).unwrap();
            if e & DESC_BIT != 0 {
                extremes::of_desc(h, e & !DESC_BIT).unwrap()
            } else {
                extremes::of_page(h, e).unwrap()
            }
        })
        .collect()
}

/// What `run` returns, and the reads of main memory it took, every page watched.
fn reads<T>(tb: &mut Table<'_, VecHost>, run: impl FnOnce(&Table<'_, VecHost>) -> T) -> (T, u64) {
    let h = tb.host_mut();
    h.watched = (1..=h.memory().len() as u64 / PAGE).map(|k| k * PAGE).collect();
    h.watched_reads.set(0);
    let out = run(tb);
    let n = tb.host().watched_reads.get();
    tb.host_mut().watched.clear();
    (out, n)
}

/// Leaf `leaf`'s minimum (`largest`: maximum) over the table, and the reads of main memory it took.
fn extreme_reads(tb: &mut Table<'_, VecHost>, leaf: u32, largest: bool) -> (Option<Total>, u64) {
    reads(tb, |tb| tb.extreme(leaf, largest, &mut [0u8; 64]).unwrap())
}

#[test]
fn an_added_integer_fields_extremes_come_from_the_records_of_groups_packed_before() {
    for fill in FILLS {
        // Key Nat, Nat, Text: 10,000 rows, two full groups and one open group, which the change packs.
        let v1 = [(t::scalar(ty::NAT), 1), (t::scalar(ty::NAT), 1), (t::scalar(ty::TEXT), 1)];
        let mut h = VecHost::new(fill);
        let (mut tb, _) = Table::create(&mut h, 1, &image(1, &v1, &[])).unwrap();
        let old = |k: u64, x: V| [(id(0), V::U(k)), (id(1), V::U(3 * k)), (id(2), V::text("old")), (id(3), x)];
        for k in 0..10_000u64 {
            tb.upsert(&mut scratch(), &mut Src::new(&old(k, V::Null)[..3])).unwrap();
        }
        let mut v2 = v1.to_vec();
        v2.push((t::opt(t::scalar(ty::NAT)), 2));
        tb.evolve(&image(2, &v2, &[])).unwrap();
        assert_eq!(open_group(&tb), 3);
        let added = tb.shape().leaf(tb.host(), 3).unwrap();
        let blocks = extremes_blocks(&tb);
        assert_eq!(blocks.len(), 4);
        for &blk in &blocks {
            // Made before the change: the added leaf lies past the leaves the block's records were made for.
            assert_eq!(rd_u32(tb.host().get(Region::Block(blk)).unwrap(), extremes::LEAVES).unwrap(), 3);
            assert_eq!(extremes::record(tb.host(), blk, &added).unwrap(), Some(Rec::default()));
        }

        // Before any value is written: no value, at the cost of the old Nat field's, whose records give its
        // extremes.
        for largest in [false, true] {
            let (got, n) = extreme_reads(&mut tb, id(3), largest);
            let (kept, kept_n) = extreme_reads(&mut tb, id(1), largest);
            assert_eq!(got, None);
            assert_eq!(kept, Some(Total::Small(if largest { 3 * 9_999 } else { 0 })));
            assert_eq!(n, kept_n, "largest {largest}: no row read");
        }

        // Values written: one old row of each group packed before (an update the patch path takes, as the old
        // groups hold no cell of the added leaf), and new rows into the open group.
        for (k, x) in [(5u64, 7u64), (4_100, 900_000), (9_000, 42)] {
            tb.upsert(&mut scratch(), &mut Src::new(&old(k, V::U(x)))).unwrap();
        }
        for k in 10_000..10_100u64 {
            tb.upsert(&mut scratch(), &mut Src::new(&old(k, V::U(500 + k % 37)))).unwrap();
        }
        let has = |x: u128, y: u128| Some(Rec { flags: HAS, min: x, max: y });
        let want = [has(7, 7), has(900_000, 900_000), has(42, 42), has(500, 536)];
        for (g, &blk) in blocks.iter().enumerate() {
            assert_eq!(extremes::record(tb.host(), blk, &added).unwrap(), want[g], "group {g}");
        }
        for largest in [false, true] {
            let (got, n) = extreme_reads(&mut tb, id(3), largest);
            let (_, kept_n) = extreme_reads(&mut tb, id(1), largest);
            assert_eq!(got, Some(Total::Small(if largest { 900_000 } else { 7 })));
            assert_eq!(n, kept_n, "largest {largest}: no row read");
        }
        let mut s = Sink::default();
        assert!(tb.get(&mut scratch(), &mut Src::new(&[(id(0), V::U(10))]), &mut s).unwrap());
        assert_eq!(s.cells, old(10, V::Null).to_vec());
    }
}

#[test]
fn an_integer_field_added_to_a_table_without_one_skips_the_groups_packed_before() {
    for fill in FILLS {
        // Key Text, Text, ?Text: no integer field, so no group has an extremes block. 10,000 rows: two full
        // groups and an open group, which the change packs.
        let v1 = [(t::scalar(ty::TEXT), 1), (t::scalar(ty::TEXT), 1), (t::opt(t::scalar(ty::TEXT)), 1)];
        let mut h = VecHost::new(fill);
        h.set_logging(false);
        let (mut tb, _) = Table::create(&mut h, 1, &image(1, &v1, &[])).unwrap();
        let row =
            |k: u64, x: V| [(id(0), V::text(&format!("k{k}"))), (id(1), V::text("c")), (id(2), V::Null), (id(3), x)];
        for k in 0..10_000u64 {
            tb.upsert(&mut scratch(), &mut Src::new(&row(k, V::Null)[..3])).unwrap();
        }
        let mut v2 = v1.to_vec();
        v2.push((t::opt(t::scalar(ty::NAT)), 2));
        tb.evolve(&image(2, &v2, &[])).unwrap();
        assert_eq!(open_group(&tb), 3);
        // The groups packed before have no extremes block; the group the change opened has the new schema's.
        let blocks = extremes_blocks(&tb);
        assert_eq!(blocks[..3], [0; 3]);
        let added = tb.shape().leaf(tb.host(), 3).unwrap();
        assert_eq!(extremes::record(tb.host(), blocks[3], &added).unwrap(), Some(Rec::default()));

        let ext = |tb: &Table<'_, VecHost>, largest: bool| tb.extreme(id(3), largest, &mut [0u8; 64]).unwrap();
        let at_least_1 = Filter { leaf: id(3), cond: Cond::Range(Bound::Included(CellIn::U64(1)), Bound::Unbounded) };
        let count = |tb: &Table<'_, VecHost>| tb.count_where(&at_least_1).unwrap();
        // No value yet: no row read, a few reads a group.
        let (got, n) = reads(&mut tb, |tb| (ext(tb, false), ext(tb, true), count(tb)));
        assert_eq!(got, (None, None, 0));
        assert!(n < 100, "{n} reads: a row was read");

        // Values in new rows only: the open group's record gives them, the groups packed before are skipped.
        for k in 10_000..10_100u64 {
            tb.upsert(&mut scratch(), &mut Src::new(&row(k, V::U(500 + k % 37)))).unwrap();
        }
        let (got, n) = reads(&mut tb, |tb| (ext(tb, false), ext(tb, true)));
        assert_eq!(got, (Some(Total::Small(500)), Some(Total::Small(536))));
        assert!(n < 100, "{n} reads: a row was read");
        assert_eq!(count(&tb), 100);

        // Values in one old row of each group packed before (patches), then the answers again.
        for (k, x) in [(5u64, 7u64), (4_100, 900_000), (9_000, 42)] {
            tb.upsert(&mut scratch(), &mut Src::new(&row(k, V::U(x)))).unwrap();
        }
        assert_eq!((ext(&tb, false), ext(&tb, true)), (Some(Total::Small(7)), Some(Total::Small(900_000))));
        assert_eq!(count(&tb), 103);
        let mut s = Sink::default();
        assert!(tb.get(&mut scratch(), &mut Src::new(&row(4_100, V::Null)[..1]), &mut s).unwrap());
        assert_eq!(s.cells, row(4_100, V::U(900_000)).to_vec());
    }
}

#[test]
fn a_dropped_field_coming_back_is_refused_and_changes_no_byte() {
    // Key Nat, Text, ?Nat; version 2 drops the Text field.
    let col = |ty: Vec<u8>| VCol { ty, added: 1, dropped: 0, default: None };
    let v1 = [col(t::scalar(ty::NAT)), col(t::scalar(ty::TEXT)), col(t::opt(t::scalar(ty::NAT)))];
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &schema_versioned(1, &v1, &[]).0).unwrap();
    for k in 0..5_000u64 {
        let row = [(id(0), V::U(k)), (id(1), V::text("old")), (id(2), V::U(k))];
        tb.upsert(&mut scratch(), &mut Src::new(&row)).unwrap();
    }
    let mut v2 = v1.to_vec();
    v2[1].dropped = 2;
    tb.evolve(&schema_versioned(2, &v2, &[]).0).unwrap();
    for k in 5_000..5_100u64 {
        let row = [(id(0), V::U(k)), (id(1), V::Null), (id(2), V::U(k))];
        tb.upsert(&mut scratch(), &mut Src::new(&row)).unwrap();
    }

    // Version 3 declares the dropped field again, as version 1 did.
    let before = tb.host().memory().to_vec();
    let pages = tb.host().live_pages();
    assert_eq!(tb.evolve(&schema_versioned(3, &v1, &[]).0), Err(Fault::Schema(SchemaError::RestoredDropped)));
    assert!(tb.host().memory() == &before[..] && tb.host().live_pages() == pages, "a byte changed");
}

#[test]
fn after_a_drop_shrinks_the_totals_rows_that_fit_only_under_the_smaller_area_are_stored_and_read_back() {
    for fill in FILLS {
        // Key Nat, 30 Nat fields, Blob: 31 totals, all in the hot state. Version 2 drops Nat fields 21 to 30.
        let col = |ty: Vec<u8>| VCol { ty, added: 1, dropped: 0, default: None };
        let mut v1: Vec<VCol> = (0..31).map(|_| col(t::scalar(ty::NAT))).collect();
        v1.push(col(t::scalar(ty::BLOB)));
        let mut v2 = v1.clone();
        for c in &mut v2[21..31] {
            c.dropped = 2;
        }
        let row = |k: u64, blob: usize, dropped: bool| -> Vec<(u32, V)> {
            let mut r: Vec<(u32, V)> = (0..31u32)
                .map(|c| (id(c), if dropped && c >= 21 { V::Null } else { V::U(k + u64::from(c)) }))
                .collect();
            r.push((id(31), V::Bytes(vec![(k % 251) as u8; blob])));
            r
        };
        let mut sums = [0i128; 31];
        let mut add = |k: u64| {
            for (c, s) in (0u64..).zip(sums.iter_mut()) {
                *s += i128::from(k + c);
            }
        };
        let mut h = VecHost::new(fill);
        let (mut tb, _) = Table::create(&mut h, 1, &schema_versioned(1, &v1, &[]).0).unwrap();
        for k in 0..5_000u64 {
            tb.upsert(&mut scratch(), &mut Src::new(&row(k, 16, false))).unwrap();
            add(k);
        }
        let old_h = tb.shape().hot_h;
        tb.evolve(&schema_versioned(2, &v2, &[]).0).unwrap();
        let new_h = tb.shape().hot_h;
        assert_eq!((old_h, new_h), (1_248, 912));
        let opened = open_group(&tb);

        // Images of 3,000 bytes: past an empty block's room under the old first row byte, within it under the
        // new one. Enough of them to fill the open group's blocks, which seals it.
        let rows = 5_000..5_300u64;
        for k in rows.clone() {
            let r = row(k, 2_557, true);
            let Upsert::Inserted(rid) = tb.upsert(&mut scratch(), &mut Src::new(&r)).unwrap().0 else {
                panic!("row {k} replaced one")
            };
            add(k);
            let at = buffer::locate(tb.host(), tb.root(), rid).unwrap().unwrap();
            let Some(RowLoc::Inline { len, .. }) = buffer::slot_row(tb.host(), at.block, at.slot).unwrap() else {
                panic!("row {k} is not stored in its block")
            };
            let need = write::reserve(len) + SLOT;
            assert!(len == 3_000 && need > BLOCK - old_h && need <= BLOCK - new_h, "row {k}: {len} bytes");
        }
        assert_eq!(tb.maintain(&TestBudget::new(u64::MAX)).unwrap(), Progress::Done);
        // The group the change opened was filled and packed: its dropped fields hold no cells.
        assert!(open_group(&tb) > opened);

        for k in rows {
            let mut s = Sink::default();
            assert!(tb.get(&mut scratch(), &mut Src::new(&[(id(0), V::U(k))]), &mut s).unwrap());
            let rid = s.rid.unwrap();
            let got: Vec<(u32, V)> = s
                .cells
                .into_iter()
                .map(|(l, v)| match v {
                    V::Arg(mut b) => {
                        assert_eq!(tb.read_large(rid, l, 0, &mut b).unwrap(), b.len());
                        (l, V::Bytes(b))
                    }
                    v => (l, v),
                })
                .collect();
            assert_eq!(got, row(k, 2_557, true), "row {k}");
        }
        let mut buf = [0u8; 64];
        for (c, s) in (0..21u32).zip(sums) {
            assert_eq!(tb.sum(id(c), &mut buf).unwrap(), (Total::Small(s), 5_300), "field {c}");
        }
    }
}
