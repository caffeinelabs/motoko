//! The schema: every kind of column type as its leaf, refusing the types the schema does not hold, and the
//! schema image round trip.

use native_table::schema::{
    flatten, node, write_schema, Column, ColumnType, IndexDecl, Leaf, LeafSink, SchemaParts, SchemaView, LEAF_KEY,
    LEAF_NULLABLE,
};
use native_table::value::ty;
use native_table::{Corruption, Fault};

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
    fn at(&self) -> u32 {
        self.0.len() as u32
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
    // 3: Char.
    cols.push(p.at());
    p.p(ty::CHAR);
    // 4: ?[u8; 5].
    cols.push(p.at());
    p.p(ty::OPTION).p(ty::BYTES).u16(5);
    // 5: ().
    cols.push(p.at());
    p.p(ty::UNIT);
    // 6: [Text].
    cols.push(p.at());
    p.p(ty::LIST).p(ty::TEXT);
    // 7: ?[?Nat].
    cols.push(p.at());
    p.p(ty::OPTION).p(ty::LIST).p(ty::OPTION).p(ty::NAT);
    // 8: [{a : Nat; b : ?Text}]: each field its type, u32 label hash, u16 name length and name.
    cols.push(p.at());
    p.p(ty::LIST).p(ty::RECORD).u16(2);
    p.p(ty::NAT).u16(7).u16(0).u16(1).p(b'a');
    p.p(ty::OPTION).p(ty::TEXT).u16(9).u16(0).u16(1).p(b'b');
    let list = cols[8] as usize;

    let mut out = Leaves(Vec::new());
    let mut columns = Vec::new();
    for (i, &t) in cols.iter().enumerate() {
        let first = out.0.len() as u16;
        let ct = ColumnType { pool: &p.0, column: i as u32, ty: t, version: 1, first_ordinal: first, key: i == 0 };
        assert_eq!(flatten(ct, &mut out).unwrap(), 1);
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
    assert_eq!(k(3), (node::SCALAR, ty::CHAR, 0));
    assert_eq!((l[4].kind, l[4].code, l[4].flags, l[4].width), (node::SCALAR, ty::BYTES, LEAF_NULLABLE, 5));
    assert_eq!(k(5), (node::UNIT, 0, 0));
    // A list is one byte-string leaf.
    assert_eq!(k(6), (node::SCALAR, ty::BLOB, 0));
    assert_eq!(k(7), (node::SCALAR, ty::BLOB, LEAF_NULLABLE));
    assert_eq!(k(8), (node::SCALAR, ty::BLOB, 0));
    assert_eq!((l[7].path, l[8].path), (cols[7] + 1, cols[8]));
    assert_eq!(l.len(), 9);
    // Leaf ids: column · 65,536.
    assert_eq!(l[4].id, 4 * 65_536);

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
    assert_eq!((v.columns, v.leaves, v.indexes, v.hash), (9, 9, 1, hash));
    for (i, lf) in l.iter().enumerate() {
        assert_eq!(&v.leaf(i as u16).unwrap(), lf);
    }
    assert_eq!(v.column(3).unwrap(), columns[3]);
    // The type pool, then zeros up to the next section's 8-byte alignment.
    assert_eq!(&v.types()[..p.0.len()], &p.0[..]);
    assert!(v.types()[p.0.len()..].iter().all(|&x| x == 0) && v.types().len() < p.0.len() + 8);
    assert!(v.leaf(9).is_err());
    // Same parts, same bytes; a changed byte breaks the hash.
    let mut img2 = vec![0x5Au8; 8_192];
    assert_eq!(write_schema(&parts, 77, &mut img2).unwrap(), (bytes, hash));
    assert_eq!(&img[..bytes as usize], &img2[..bytes as usize]);
    img[100] ^= 1;
    assert!(matches!(SchemaView::open(&img[..bytes as usize]), Err(Fault::Corrupt(c)) if c.kind == Corruption::Header));
    // Too small an output asks for more.
    assert!(matches!(write_schema(&parts, 77, &mut [0u8; 64]), Err(Fault::Scratch { .. })));
    // The hash covers a list's element: another name of a record's field gives another hash.
    let mut renamed = p.0.clone();
    renamed[list + 11] = b'c';
    let parts = SchemaParts { type_pool: &renamed, ..parts };
    assert_ne!(write_schema(&parts, 77, &mut img2).unwrap().1, hash);
}

/// Records outside a list, tuples, variants, recursive references, tables and sets (type codes 0x21 to 0x27),
/// lists of anything but a scalar, an option of one or a record of them, and options of anything but a scalar
/// or a list, are refused by flattening and by the schema image's canonical form.
#[test]
fn types_the_schema_does_not_hold_are_refused() {
    let pools: Vec<Vec<u8>> = vec![
        vec![0x21, 1, 0, ty::U8, 0, 0, 0, 0, 0, 0],
        vec![0x22, 1, 0, ty::U8],
        vec![0x23, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, ty::UNIT],
        vec![ty::LIST, ty::LIST, ty::TEXT],
        vec![ty::OPTION, 0x25, 0, 0, 0, 0],
        vec![0x26, ty::NAT, ty::TEXT],
        vec![0x27, ty::NAT],
        vec![ty::OPTION, ty::OPTION, ty::TEXT],
        vec![ty::LIST, ty::UNIT],
        vec![ty::LIST, ty::OPTION, ty::OPTION, ty::TEXT],
        vec![ty::LIST, ty::OPTION, ty::RECORD, 1, 0, ty::U8, 0, 0, 0, 0, 0, 0],
        vec![ty::LIST, ty::RECORD, 1, 0, ty::LIST, ty::U8, 0, 0, 0, 0, 0, 0],
        // A field's name past the pool's end.
        vec![ty::LIST, ty::RECORD, 1, 0, ty::U8, 0, 0, 0, 0, 0, 1, b'a'],
        vec![ty::OPTION, ty::UNIT],
    ];
    for pool in &pools {
        let mut out = Leaves(Vec::new());
        let ct = ColumnType { pool, column: 1, ty: 0, version: 1, first_ordinal: 0, key: false };
        let r = flatten(ct, &mut out);
        assert!(matches!(r, Err(Fault::Corrupt(c)) if c.kind == Corruption::Encoding), "{pool:x?}: {r:?}");
    }
    // A column of one of the removed types in an image: its canonical form, and so the image, is refused.
    let leaf = Leaf { id: 0, kind: node::SCALAR, code: ty::NAT, flags: LEAF_KEY, path: 0, added: 1, ..Leaf::default() };
    for pool in &pools[..pools.len() - 1] {
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
