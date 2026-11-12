//! The ends of a secondary index's sorted walk (`index::secondary::sorted`): the smallest and largest keys whose
//! first part is not null.

use std::collections::BTreeSet;

use native_table::area::new_root;
use native_table::index::secondary::Secondary;
use native_table::key::{KeyKind, KeyPart, KeySpec};
use native_table::tree::Path;
use nt_testhost::{VecHost, VecSink, FILLS};

// ---------------------------------------------------------------- the walk

// ---------------------------------------------------------------- ends with nulls

/// The smallest and largest keys whose first part is not null, for each kind whose smallest non-null value
/// encodes as the tag 01 followed by zero bytes only.
#[test]
fn ends_skip_nulls() {
    use native_table::table::cell::{enc_key, Val};
    use native_table::table::layout::{LeafInfo, Ty};
    fn enc(ty: Ty, v: Val<'_>) -> Vec<u8> {
        let l = LeafInfo { ordinal: 0, id: 0, ty, nullable: true, key: true, path: 0 };
        let mut s = VecSink::default();
        enc_key(&mut s, &l, &v).unwrap();
        s.0
    }
    let part = |kind| [KeyPart { leaf: 0, kind, nullable: true }];
    let cases: [([KeyPart; 1], Ty, [Val<'static>; 3]); 7] = [
        (part(KeyKind::Bool), Ty::Bool, [Val::Null, Val::Bool(false), Val::Bool(true)]),
        // A smallest value of 40 zero bytes, a key longer than the tree's W.
        (part(KeyKind::Fixed(40)), Ty::Fixed(40), [Val::Null, Val::Bytes(&[0; 40]), Val::Bytes(&[7; 40])]),
        (part(KeyKind::U32), Ty::U(4), [Val::Null, Val::U(0), Val::U(7)]),
        (part(KeyKind::U64), Ty::U(8), [Val::Null, Val::U(0), Val::U(1 << 40)]),
        (part(KeyKind::Nat), Ty::Nat, [Val::Null, Val::U(0), Val::U(5)]),
        (part(KeyKind::Text), Ty::Text, [Val::Null, Val::Bytes(b""), Val::Bytes(b"a")]),
        (part(KeyKind::Unit), Ty::Unit, [Val::Null, Val::Unit, Val::Unit]),
    ];
    for (parts, ty, values) in &cases {
        let spec = KeySpec { parts };
        let keys: BTreeSet<Vec<u8>> = values.iter().map(|&v| enc(*ty, v)).collect();
        let held: Vec<&Vec<u8>> = keys.iter().filter(|k| k[0] != 0).collect();
        let want = (held.first().map(|k| k.to_vec()), held.last().map(|k| k.to_vec()));
        let mut h = VecHost::new(FILLS[0]);
        let tbl = new_root(&mut h).unwrap();
        let mut ix = Secondary::create(1, 8).unwrap();
        let mut path = Path::new();
        for (i, k) in keys.iter().enumerate() {
            assert!(ix.insert(&mut h, tbl, k, i as u64 + 1, &mut path).unwrap());
        }
        let mut out = vec![0u8; 64];
        let min = ix.min_key(&h, &spec, &mut out).unwrap().map(|n| out[..n].to_vec());
        let max = ix.max_key(&h, &spec, &mut out).unwrap().map(|n| out[..n].to_vec());
        assert_eq!((min, max), want, "{:?}", parts[0].kind);
    }
}
