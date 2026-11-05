//! The schema: flattening every kind of column type into leaves, refusing the type codes the schema does not
//! hold, and the schema image round trip.

use native_table::schema::{
    flatten, node, write_schema, Column, ColumnType, IndexDecl, Leaf, LeafSink, SchemaParts, SchemaView, LEAF_KEY,
    LEAF_NULLABLE,
};
use native_table::value::{ty, FRAME};
use native_table::{Corruption, Fault};
use nt_testhost::rows::idl_hash;

#[derive(Default)]
struct Pool(Vec<u8>);

impl Pool {
    fn p(&mut self, c: u8) -> &mut Self {
        self.0.push(c);
        self
    }
    fn u16(&mut self, v: u16) -> &mut Self {
        self.0.extend(v.to_le_bytes());
        self
    }
    fn u32(&mut self, v: u32) -> &mut Self {
        self.0.extend(v.to_le_bytes());
        self
    }
    fn at(&self) -> u32 {
        self.0.len() as u32
    }
    /// A record header: n main fields, 0 added.
    fn record(&mut self, n: u16) -> &mut Self {
        self.p(ty::RECORD).u16(n).u16(0)
    }
    fn field(&mut self, name: &str, flags: u8) -> &mut Self {
        self.u32(idl_hash(name.as_bytes())).u32(0).p(flags).u32(0).u32(0)
    }
}

struct Leaves(Vec<Leaf>);

impl LeafSink for Leaves {
    fn leaf(&mut self, l: &Leaf) -> Result<(), Fault> {
        self.0.push(*l);
        Ok(())
    }
}

#[test]
fn flattening_rules_and_the_schema_image() {
    let mut p = Pool::default();
    let mut cols = Vec::new();
    // 0: key Nat.
    cols.push(p.at());
    p.p(ty::NAT);
    // 1: Text.
    cols.push(p.at());
    p.p(ty::TEXT);
    // 2: ?Text: a nullable Text.
    cols.push(p.at());
    p.p(ty::OPTION).p(ty::TEXT);
    // 3: { a: Nat; b: Bool; gone: Text (dropped) }: the live fields' leaves.
    cols.push(p.at());
    p.record(3).field("a", 0).p(ty::NAT).field("b", 0).p(ty::BOOL).field("gone", ty::FIELD_DROPPED).p(ty::TEXT);
    // 4: (Char, [u8; 5]).
    cols.push(p.at());
    p.p(ty::TUPLE).u16(2).p(ty::CHAR).p(ty::BYTES).u16(5);
    // 5: ().
    cols.push(p.at());
    p.p(ty::UNIT);

    let mut out = Leaves(Vec::new());
    let mut columns = Vec::new();
    let mut scratch = vec![0u8; 256 * FRAME];
    for (i, &t) in cols.iter().enumerate() {
        let first = out.0.len() as u16;
        let ct = ColumnType { pool: &p.0, column: i as u32, ty: t, version: 1, first_ordinal: first, key: i == 0 };
        let n = flatten(ct, &mut out, &mut scratch).unwrap();
        assert_eq!(n as usize, out.0.len() - first as usize);
        columns.push(Column {
            id: i as u32,
            flags: 0,
            first_leaf: first,
            ty: t,
            default_off: 0,
            default_len: 0,
            name: 0,
        });
    }
    let l = &out.0;
    let k = |i: usize| (l[i].kind, l[i].code, l[i].flags);
    assert_eq!(k(0), (node::SCALAR, ty::NAT, LEAF_KEY));
    assert_eq!(k(1), (node::SCALAR, ty::TEXT, 0));
    assert_eq!(k(2), (node::SCALAR, ty::TEXT, LEAF_NULLABLE));
    assert_eq!(k(3), (node::SCALAR, ty::NAT, 0));
    assert_eq!(k(4), (node::SCALAR, ty::BOOL, 0));
    assert_eq!(k(5), (node::SCALAR, ty::CHAR, 0));
    assert_eq!((l[6].kind, l[6].code, l[6].width), (node::SCALAR, ty::BYTES, 5));
    assert_eq!(k(7), (node::UNIT, 0, 0));
    assert_eq!(l.len(), 8, "the dropped field makes no leaf");
    // Leaf ids: column · 65,536 + the ordinal within the column.
    assert_eq!(l[4].id, 3 * 65_536 + 1);
    assert_eq!(l[6].id, 4 * 65_536 + 1);

    // The schema image round trip.
    let names = b"\x03\x00key".to_vec();
    let mut decl = IndexDecl { id: 0, flags: 1, w: 16, parts: 1, part: [(0, false); 7] };
    decl.part[0] = (0, false);
    let parts = SchemaParts {
        version: 1,
        columns: &columns,
        leaves: l,
        indexes: &[decl],
        type_pool: &p.0,
        default_pool: &[],
        name_pool: &names,
    };
    let mut img = vec![0xA5u8; 8_192];
    let (bytes, hash) = write_schema(&parts, 77, &mut img).unwrap();
    let v = SchemaView::open(&img[..bytes as usize]).unwrap();
    assert_eq!((v.columns, v.leaves, v.indexes, v.hash), (6, 8, 1, hash));
    for (i, lf) in l.iter().enumerate() {
        assert_eq!(&v.leaf(i as u16).unwrap(), lf);
    }
    assert_eq!(v.column(3).unwrap(), columns[3]);
    // The type pool, then zeros up to the next section's 8-byte alignment.
    assert_eq!(&v.types()[..p.0.len()], &p.0[..]);
    assert!(v.types()[p.0.len()..].iter().all(|&x| x == 0) && v.types().len() < p.0.len() + 8);
    assert!(v.leaf(8).is_err());
    // Same parts, same bytes; a changed byte breaks the hash.
    let mut img2 = vec![0x5Au8; 8_192];
    assert_eq!(write_schema(&parts, 77, &mut img2).unwrap(), (bytes, hash));
    assert_eq!(&img[..bytes as usize], &img2[..bytes as usize]);
    img[100] ^= 1;
    assert!(matches!(SchemaView::open(&img[..bytes as usize]), Err(Fault::Corrupt(c)) if c.kind == Corruption::Header));
    // Too small an output asks for more.
    assert!(matches!(write_schema(&parts, 77, &mut [0u8; 64]), Err(Fault::Scratch { .. })));
}

/// Variants, lists, recursive references, tables and sets (type codes 0x23 to 0x27), and options of anything
/// but a scalar, are refused by flattening and by the schema image's canonical form.
#[test]
fn types_the_schema_does_not_hold_are_refused() {
    let mut rec = Pool::default();
    rec.p(ty::OPTION).record(1).field("v", 0).p(ty::U8);
    let pools: Vec<Vec<u8>> = vec![
        vec![0x23, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, ty::UNIT],
        vec![0x24, ty::TEXT],
        vec![ty::OPTION, 0x25, 0, 0, 0, 0],
        vec![0x26, ty::NAT, ty::TEXT],
        vec![0x27, ty::NAT],
        rec.0,
        vec![ty::OPTION, ty::OPTION, ty::TEXT],
        vec![ty::OPTION, ty::UNIT],
    ];
    for pool in &pools {
        let mut out = Leaves(Vec::new());
        let ct = ColumnType { pool, column: 1, ty: 0, version: 1, first_ordinal: 0, key: false };
        let r = flatten(ct, &mut out, &mut vec![0u8; 64 * FRAME]);
        assert!(matches!(r, Err(Fault::Corrupt(c)) if c.kind == Corruption::Encoding), "{pool:x?}: {r:?}");
    }
    // A column of one of the removed types in an image: its canonical form, and so the image, is refused.
    let leaf = Leaf { id: 0, kind: node::SCALAR, code: ty::NAT, flags: LEAF_KEY, path: 0, added: 1, ..Leaf::default() };
    for pool in &pools[..5] {
        let column = Column { id: 0, flags: 0, first_leaf: 0, ty: 0, default_off: 0, default_len: 0, name: 0 };
        let parts = SchemaParts {
            version: 1,
            columns: &[column],
            leaves: &[leaf],
            indexes: &[],
            type_pool: pool,
            default_pool: &[],
            name_pool: &[],
        };
        let r = write_schema(&parts, 1, &mut vec![0u8; 4_096]);
        assert!(matches!(r, Err(Fault::Corrupt(c)) if c.kind == Corruption::Encoding), "{pool:x?}: {r:?}");
    }
}

#[test]
fn deep_types_need_scratch_not_a_cap() {
    // Records nest by frames: 300 levels need more scratch than 16 frames, and succeed with enough.
    let mut p = Pool::default();
    for _ in 0..300 {
        p.record(1).field("v", 0);
    }
    p.p(ty::U8);
    let ct = ColumnType { pool: &p.0, column: 1, ty: 0, version: 1, first_ordinal: 0, key: false };
    let mut out = Leaves(Vec::new());
    assert!(matches!(flatten(ct, &mut out, &mut vec![0u8; 16 * FRAME]), Err(Fault::Scratch { .. })));
    out.0.clear();
    flatten(ct, &mut out, &mut vec![0u8; 4_096 * FRAME]).unwrap();
    assert_eq!(out.0.len(), 1);
}
