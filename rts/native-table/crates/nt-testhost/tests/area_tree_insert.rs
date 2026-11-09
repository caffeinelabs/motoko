//! Area-tree inserts in place (`area_insert::insert_vacant`, at the position a descent with a path found, as
//! the secondary and the primary index insert) against the tree's own `descend` and `insert_at`: after the same
//! sequence of inserts the whole main memory is the same, byte for byte, every insert has the same outcome and
//! changes the same blocks, and the trees have the same root, node count and key log. 252 cases: seven shapes
//! with W ≤ 32 and V ≤ 8, twelve seeds, three workloads (many equal key areas from 2- and 3-letter alphabets,
//! and 256 letters). The sequences hold row entries, count entries, keys shorter than W, keys
//! longer than W through the key log, entries already there, leaf splits, full parents (checked: trees of
//! height 3 or more) and root splits.

use native_table::area::new_root;
use native_table::index::area_insert;
use native_table::index::area_nodes::{AreaNodes, AreaView, KeyLog};
use native_table::key::probe::{count_field, ovf_field, payload, row_field, IndexPending, KeyProbe, Kind};
use native_table::tree::node::{child, is_leaf};
use native_table::tree::{descend, insert_at, insert_first, IndexShape, Path, Search};
use native_table::Fault;
use nt_testhost::{VecHost, XorShift, FILLS};

/// One side: its memory and its tree.
struct Side {
    host: VecHost,
    tbl: u64,
    root: u64,
    nodes: u64,
    log: KeyLog,
    path: Path,
}

impl Side {
    fn new(fill: u8) -> Side {
        let mut host = VecHost::new(fill);
        host.set_logging(false);
        let tbl = new_root(&mut host).unwrap();
        Side { host, tbl, root: 0, nodes: 0, log: KeyLog::default(), path: Path::new() }
    }
}

/// What one insert did.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Inserted,
    Found,
    Failed(Fault),
}

/// The entry: its key, its kind (count entries, row entries), its rid and its value part.
struct Entry {
    key: Vec<u8>,
    kind: Kind,
    base: u64,
    rid: u64,
    value: Vec<u8>,
}

/// The tree's own way: `descend` with a new path, then `insert_at`.
fn reference(s: &mut Side, shape: IndexShape, e: &Entry) -> Outcome {
    let mut m = AreaNodes::new(&mut s.host, s.tbl, 1, shape, s.log);
    m.nodes = s.nodes;
    let field = if e.key.len() as u64 > shape.w {
        let payload = if matches!(e.kind, Kind::Count) { payload(e.base) } else { e.rid };
        ovf_field(e.base, m.new_record(payload, &e.key).unwrap())
    } else {
        e.base
    };
    let probe = KeyProbe { key: &e.key, kind: e.kind };
    let mut p = IndexPending::Borrowed { key: &e.key, field, value: &e.value, child: 0 };
    let r = (|| -> Result<Outcome, Fault> {
        if s.root == 0 {
            s.root = insert_first(shape, &mut m, &p)?;
            return Ok(Outcome::Inserted);
        }
        let mut path = Path::new();
        match descend(shape, &m, s.root, &probe, &mut path)? {
            Search::Vacant(n, j) => {
                insert_at(shape, &mut m, &mut s.root, n, j, &mut p, &mut path)?;
                Ok(Outcome::Inserted)
            }
            Search::Found(..) => Ok(Outcome::Found),
        }
    })();
    s.nodes = m.nodes;
    s.log = m.log;
    r.unwrap_or_else(Outcome::Failed)
}

/// The side under test: a descent with a path, then `insert_vacant` at the position found.
fn in_place(s: &mut Side, shape: IndexShape, e: &Entry) -> Outcome {
    let mut m = AreaNodes::new(&mut s.host, s.tbl, 1, shape, s.log);
    m.nodes = s.nodes;
    let field = if e.key.len() as u64 > shape.w {
        let payload = if matches!(e.kind, Kind::Count) { payload(e.base) } else { e.rid };
        ovf_field(e.base, m.new_record(payload, &e.key).unwrap())
    } else {
        e.base
    };
    let probe = KeyProbe { key: &e.key, kind: e.kind };
    let r = (|| -> Result<Outcome, Fault> {
        if s.root == 0 {
            let p = IndexPending::Borrowed { key: &e.key, field, value: &e.value, child: 0 };
            s.root = insert_first(shape, &mut m, &p)?;
            return Ok(Outcome::Inserted);
        }
        s.path.clear();
        match descend(shape, &m, s.root, &probe, &mut s.path)? {
            Search::Vacant(n, j) => {
                area_insert::insert_vacant(&mut m, &mut s.root, n, j, &e.key, field, &e.value, &mut s.path)?;
                Ok(Outcome::Inserted)
            }
            Search::Found(..) => Ok(Outcome::Found),
        }
    })();
    s.nodes = m.nodes;
    s.log = m.log;
    r.unwrap_or_else(Outcome::Failed)
}

/// The tree's height: 0 empty, 1 a root leaf; 3 or more only after a leaf split reached a full parent.
fn height(s: &Side, shape: IndexShape) -> u64 {
    let m = AreaView::new(&s.host, shape, s.log);
    let (mut n, mut h) = (s.root, 0);
    while n != 0 {
        h += 1;
        if is_leaf(&m, n).unwrap() {
            break;
        }
        n = child(shape, &m, n, 0).unwrap();
    }
    h
}

/// One random sequence of `n` inserts into a tree of shape (w, v), keys of letters below `alphabet`, rids below
/// `rids`; returns how many inserted, found and failed, and the tree's height after them.
fn run(seed: u64, w: u64, v: u64, n: u64, alphabet: u64, rids: u64) -> ([u64; 3], u64) {
    let shape = IndexShape::new(w, v).unwrap();
    let fill = FILLS[(seed % 2) as usize];
    let (mut a, mut b) = (Side::new(fill), Side::new(fill));
    let mut g = XorShift(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut tally = [0u64; 3];
    for step in 0..n {
        // One key in 50 may be longer than W, through the key log.
        let maxlen = if g.below(50) == 0 { w + 24 } else { w };
        let len = g.below(maxlen + 1) as usize;
        let key: Vec<u8> = (0..len).map(|_| g.below(alphabet) as u8).collect();
        let rid = g.below(rids);
        let what = g.below(20);
        let value = if v == 8 { g.next_u64().to_le_bytes().to_vec() } else { Vec::new() };
        let (kind, base) = match what {
            0 => (Kind::Count, count_field((g.below(200) as i64) - 100)),
            _ => (Kind::Row(rid), row_field(rid)),
        };
        let e = Entry { key, kind, base, rid, value };
        let x = reference(&mut a, shape, &e);
        let y = in_place(&mut b, shape, &e);
        assert_eq!(x, y, "outcome, seed {seed} W {w} V {v} step {step}");
        assert_eq!(a.host.changed_set(), b.host.changed_set(), "changed blocks, seed {seed} W {w} V {v} step {step}");
        a.host.end_message();
        b.host.end_message();
        tally[match x {
            Outcome::Inserted => 0,
            Outcome::Found => 1,
            Outcome::Failed(_) => 2,
        }] += 1;
    }
    assert_eq!((a.root, a.nodes, a.log), (b.root, b.nodes, b.log), "tree, seed {seed} W {w} V {v}");
    assert!(a.host.memory() == b.host.memory(), "memory differs: seed {seed} W {w} V {v} n {n}");
    (tally, height(&b, shape))
}

#[test]
fn inserts_in_place_write_the_trees_own_bytes() {
    let mut cases = 0;
    let mut tally = [0u64; 3];
    // Cases whose tree grew to a height of 3 or more: leaf splits that reached a full parent.
    let mut deep = 0u64;
    for (w, v) in [(8u64, 0u64), (8, 8), (16, 0), (16, 8), (24, 0), (32, 0), (32, 8)] {
        for seed in 1..=12u64 {
            for (n, alphabet, rids) in [(3_000u64, 3u64, 50u64), (20_000, 256, 1u64 << 40), (6_000, 2, 8)] {
                let (t, h) = run(seed * 7_919 + w + v, w, v, n, alphabet, rids);
                for (x, y) in tally.iter_mut().zip(t) {
                    *x += y;
                }
                if h >= 3 {
                    deep += 1;
                }
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 252);
    // Every outcome occurs: inserts, entries already there (repeated keys and rids of the small alphabets).
    assert!(tally[0] > 0 && tally[1] > 0, "{tally:?}");
    // Leaves split whose parent is full.
    assert!(deep > 0, "{deep}");
}
