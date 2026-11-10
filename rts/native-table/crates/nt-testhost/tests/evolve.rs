//! A change of a table's row type (`Table::evolve`): every refused image leaves every byte of the table as it
//! was, an image equal to the stored one changes nothing, an open group whose rows are all deleted is sealed
//! and the next insert works, and an added integer field's total starts at zero beside the others.

use native_table::schema::IndexDecl;
use native_table::table::totals::Total;
use native_table::table::{buffer, root, Table};
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
