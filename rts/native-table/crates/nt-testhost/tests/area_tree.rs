//! Index trees in table memory: entries with keys of up to 10,000 bytes through the key log, scans and
//! prefix ranges against a model, removal down to an empty tree, record payload rewrites, under both fill
//! patterns with identical results and host-call logs.

use std::ops::Bound::*;

use native_table::area::new_root;
use native_table::index::area_nodes::{AreaNodes, KeyLog};
use native_table::key::enc_bytes;
use native_table::key::probe::{
    count_field, is_ovf, ovf_field, payload, row_field, IndexPending, KeyProbe, Kind, ResolveKey,
};
use native_table::tree::{
    self, descend, insert_at, insert_first, remove_found, Cursor, Fwd, IndexShape, Path, Search, Shape,
};
use native_table::NodeMem;
use nt_testhost::{VecHost, VecSink, XorShift, FILLS};

fn key(r: &mut XorShift, long: bool) -> Vec<u8> {
    let n = if long { 500 + r.below(9_500) as usize } else { r.below(20) as usize };
    let t: Vec<u8> = (0..n).map(|_| [b'a', b'b', 0, 0xFF][r.below(4) as usize]).collect();
    let mut s = VecSink::default();
    enc_bytes(&mut s, &t).unwrap();
    s.0
}

/// (key, rid) entries, row entries only.
fn run(fill: u8, w: u64, v: u64) -> (Vec<nt_testhost::Call>, Vec<(Vec<u8>, u64)>) {
    let shape = IndexShape::new(w, v).unwrap();
    let mut host = VecHost::new(fill);
    let tbl = new_root(&mut host).unwrap();
    let mut log = KeyLog::default();
    let mut root = 0u64;
    let mut model: Vec<(Vec<u8>, u64)> = Vec::new();
    let mut r = XorShift(1234 + w + v);
    let value = vec![0x33u8; v as usize];
    for step in 0..3_000u64 {
        let long = r.below(5) == 0;
        let k = key(&mut r, long);
        let rid = step;
        let mut m = AreaNodes::new(&mut host, tbl, 1, shape, log);
        let field =
            if k.len() as u64 > w { ovf_field(row_field(rid), m.new_record(rid, &k).unwrap()) } else { row_field(rid) };
        let probe = KeyProbe { key: &k, kind: Kind::Row(rid) };
        if root == 0 {
            let p = IndexPending::Borrowed { key: &k, field, value: &value, child: 0 };
            root = insert_first(shape, &mut m, &p).unwrap();
        } else {
            let mut path = Path::new();
            match descend(shape, &m, root, &probe, &mut path).unwrap() {
                Search::Vacant(n, j) => {
                    let mut p = IndexPending::Borrowed { key: &k, field, value: &value, child: 0 };
                    insert_at(shape, &mut m, &mut root, n, j, &mut p, &mut path).unwrap();
                }
                Search::Found(..) => unreachable!("rids are unique"),
            }
        }
        log = m.log;
        let pos = model.binary_search(&(k.clone(), rid)).unwrap_err();
        model.insert(pos, (k, rid));
        // Remove one in five, from the model's middle.
        if step % 5 == 4 {
            let (k, rid) = model.remove(model.len() / 2);
            let mut m = AreaNodes::new(&mut host, tbl, 1, shape, log);
            let mut path = Path::new();
            match descend(shape, &m, root, &KeyProbe { key: &k, kind: Kind::Row(rid) }, &mut path).unwrap() {
                Search::Found(n, i) => remove_found(shape, &mut m, &mut root, n, i, &mut path).unwrap(),
                Search::Vacant(..) => panic!("entry missing"),
            }
            log = m.log;
        }
    }
    // Full scan equals the model, keys resolved through the log.
    let m = AreaNodes::new(&mut host, tbl, 1, shape, log);
    let mut c: Cursor<Fwd> = Cursor::seek(shape, &m, root, Unbounded::<&KeyProbe<'_>>).unwrap();
    let mut got = Vec::new();
    while let Some((n, i)) = c.next(shape, &m, Unbounded::<&KeyProbe<'_>>).unwrap() {
        let leaf = tree::node::is_leaf(&m, n).unwrap();
        let sh = shape.shape(leaf);
        let part = m.bytes(n, tree::node::ko(shape, &sh, i), w + 8).unwrap().to_vec();
        let field = u64::from_be_bytes(part[w as usize..].try_into().unwrap());
        let full = if is_ovf(field) {
            let (pl, len) = m.record(field).unwrap();
            let mut kb = part[..w as usize].to_vec();
            let mut at = w;
            let mut piece = [0u8; 64];
            while at < len {
                let got = m.key_piece(field, at, &mut piece).unwrap();
                assert!(got > 0);
                kb.extend_from_slice(&piece[..got]);
                at += got as u64;
            }
            assert_eq!(kb.len() as u64, len);
            (kb, pl)
        } else {
            (part[..w as usize].to_vec(), payload(field))
        };
        got.push(full);
    }
    let want: Vec<(Vec<u8>, u64)> = model
        .iter()
        .map(|(k, rid)| {
            let mut kk = k.clone();
            if kk.len() as u64 <= w {
                kk.resize(w as usize, 0);
            }
            (kk, *rid)
        })
        .collect();
    assert_eq!(got, want);
    // A count record's payload is rewritten in place.
    let mut m = AreaNodes::new(&mut host, tbl, 1, shape, log);
    let long = vec![7u8; 3_000];
    let loc = m.new_record(count_field(5) & native_table::consts::PAYLOAD, &long).unwrap();
    m.set_record_payload(loc, 9).unwrap();
    assert_eq!(m.record(ovf_field(count_field(0), loc)).unwrap(), (9, 3_000));
    // Empty the tree: every node block goes back.
    for (k, rid) in model.clone() {
        let mut path = Path::new();
        match descend(shape, &m, root, &KeyProbe { key: &k, kind: Kind::Row(rid) }, &mut path).unwrap() {
            Search::Found(n, i) => remove_found(shape, &mut m, &mut root, n, i, &mut path).unwrap(),
            Search::Vacant(..) => panic!("entry missing"),
        }
    }
    assert_eq!(root, 0);
    (host.take_log(), model)
}

#[test]
fn area_index_trees_with_key_logs() {
    for (w, v) in [(8, 0), (16, 8), (32, 0)] {
        let a = run(FILLS[0], w, v);
        let b = run(FILLS[1], w, v);
        assert_eq!(a, b, "identical results and host-call logs under both fill patterns, W {w} V {v}");
    }
}
