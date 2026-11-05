//! The golden typed nodes: the crate's side.
//!
//! Every script of `nt_testhost::golden_nodes` runs on the crate's tree over the typed test host twice, once
//! with the fill pattern P = 0xA5 and once with P = 0x5A. The two runs must give identical results, host-call
//! logs and node references; a byte equal in both runs is written, a byte that differs is unwritten (and must
//! hold P). After each imaged step every node reachable from the root is normalized
//! (`golden_nodes::normalize`) and the script's text (format: `golden/nodes/README.md`) is compared with the
//! frozen file `golden/nodes/<script>.txt`. Any difference fails. The files are never rewritten by this test;
//! `NT_GOLDEN_NODES_OUT=<dir>` writes the text it produced to `<dir>` for inspection (refused for
//! `golden/nodes` itself).
//!
//! Each imaged step names the cases it exercises; an independent count model of `PBTreeMap`'s rules,
//! reading only the node counts and children before the step, must find exactly those cases.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use native_table::tree::node::{child, count, is_leaf};
use native_table::tree::{descend, Path, Search, Shape};
use nt_testhost::golden_nodes::{self, Kind, Observed, Op, Script, Writer};
use nt_testhost::typed::{NodeCall, Part, TMap, TProbe};
use nt_testhost::FILLS;

/// A key type of the scripts.
trait GtKey: Part {
    const KIND: Kind;
    fn of(label: u64) -> Self;
}

impl GtKey for u64 {
    const KIND: Kind = Kind::U64;
    fn of(label: u64) -> Self {
        golden_nodes::key_u64(label)
    }
}

impl GtKey for [u8; 16] {
    const KIND: Kind = Kind::A16;
    fn of(label: u64) -> Self {
        golden_nodes::key_array(label, 16).try_into().unwrap()
    }
}

impl GtKey for [u8; 1012] {
    const KIND: Kind = Kind::A1012;
    fn of(label: u64) -> Self {
        golden_nodes::key_array(label, 1012).try_into().unwrap()
    }
}

/// What one run of a script gave at one step.
#[derive(Debug, PartialEq, Eq)]
struct RawStep {
    result: Option<u64>,
    log: Vec<NodeCall>,
    root: u64,
    len: u64,
    images: Option<Vec<(u64, Vec<u8>)>>,
}

/// Runs `script` on a fresh map with fill pattern `fill`; for every imaged step, also returns the cases an
/// independent count model finds in the tree before the step.
fn run<K: GtKey>(script: &Script, fill: u8) -> (Vec<RawStep>, Vec<String>) {
    assert_eq!(K::KIND, script.kind);
    let mut t: TMap<K, u64> = TMap::new(fill);
    let mut out = Vec::new();
    let mut cases = Vec::new();
    for st in &script.steps {
        if st.cases.is_some() {
            cases.push(classify(&t, st.op));
        }
        let before = t.nodes.log.len();
        let result = match st.op {
            Op::Insert(l) => t.insert(K::of(l), golden_nodes::value(l)),
            Op::Remove(l) => t.remove(K::of(l)),
        };
        let images = st.cases.map(|_| {
            t.check();
            t.reachable().into_iter().map(|n| (n, t.nodes.node(n).to_vec())).collect()
        });
        out.push(RawStep { result, log: t.nodes.log[before..].to_vec(), root: t.root, len: t.len, images });
    }
    t.check();
    (out, cases)
}

/// The cases a step exercises, from the node counts before it, by `PBTreeMap`'s rules:
/// "first insert", "leaf room", "<level> split i<h|i=h|i>h", "root split", "leaf remove", "predecessor",
/// "<level> merge", "<level> rotation from the left|right", "root collapse (internal|leaf)".
fn classify<K: GtKey>(t: &TMap<K, u64>, op: Op) -> String {
    let s = TMap::<K, u64>::S;
    let m = &t.nodes;
    let leaf = |n: u64| is_leaf(m, n).unwrap();
    let cnt = |n: u64| count(s, m, n).unwrap();
    let kid = |n: u64, j: u64| child(s, m, n, j).unwrap();
    let cap = |l: bool| s.shape(l).cap;
    let level = |l: bool| if l { "leaf" } else { "internal" };
    let mut cases: Vec<String> = Vec::new();
    match op {
        Op::Insert(l) => {
            if t.root == 0 {
                return "first insert".into();
            }
            let mut path = Path::new();
            let Search::Vacant(mut n, mut i) = descend(s, m, t.root, &TProbe(K::of(l)), &mut path).unwrap() else {
                panic!("insert of a present key");
            };
            let mut frames = path.frames().to_vec();
            loop {
                let c = cap(leaf(n));
                if cnt(n) < c {
                    if cases.is_empty() {
                        cases.push("leaf room".into());
                    }
                    break;
                }
                let h = c.div_ceil(2);
                let rel = match i.cmp(&h) {
                    std::cmp::Ordering::Less => "i<h",
                    std::cmp::Ordering::Equal => "i=h",
                    std::cmp::Ordering::Greater => "i>h",
                };
                cases.push(format!("{} split {rel}", level(leaf(n))));
                match frames.pop() {
                    Some((p, j)) => (n, i) = (p, j),
                    None => {
                        cases.push("root split".into());
                        break;
                    }
                }
            }
        }
        Op::Remove(l) => {
            let mut path = Path::new();
            let Search::Found(n, i) = descend(s, m, t.root, &TProbe(K::of(l)), &mut path).unwrap() else {
                panic!("removal of an absent key");
            };
            let mut frames = path.frames().to_vec();
            let mut x = n;
            if !leaf(n) {
                cases.push("predecessor".into());
                frames.push((n, i));
                x = kid(n, i);
                while !leaf(x) {
                    frames.push((x, cnt(x)));
                    x = kid(x, cnt(x));
                }
            }
            let mut c_x = cnt(x) - 1;
            loop {
                let lx = leaf(x);
                let Some((p, j)) = frames.pop() else {
                    if c_x == 0 {
                        cases.push(format!("root collapse ({})", level(lx)));
                    }
                    break;
                };
                let c = cap(lx);
                if c_x >= c / 2 {
                    break;
                }
                let (lc, rc) = if j > 0 { (cnt(kid(p, j - 1)), c_x) } else { (c_x, cnt(kid(p, 1))) };
                if lc + rc < c {
                    cases.push(format!("{} merge", level(lx)));
                    x = p;
                    c_x = cnt(p) - 1;
                } else {
                    cases.push(format!("{} rotation from the {}", level(lx), if j > 0 { "left" } else { "right" }));
                    break;
                }
            }
            if cases.is_empty() {
                cases.push("leaf remove".into());
            }
        }
    }
    cases.join(", ")
}

/// Runs `script` under both fill patterns and returns its observations, its text and the model's cases.
fn observe<K: GtKey>(script: &Script) -> (Vec<Observed>, String, Vec<String>) {
    let (a, cases) = run::<K>(script, FILLS[0]);
    let (b, cases_b) = run::<K>(script, FILLS[1]);
    assert_eq!(cases, cases_b);
    // Allocation ordinals: positions among the script's alloc results, all made before its first hand-back.
    let mut ord: BTreeMap<u64, u64> = BTreeMap::new();
    let mut handed_back = false;
    for (k, (x, y)) in a.iter().zip(&b).enumerate() {
        assert_eq!(x.result, y.result, "{}: step {}: the two fill patterns give different results", script.name, k + 1);
        assert_eq!(x.log, y.log, "{}: step {}: different host-call logs", script.name, k + 1);
        assert_eq!((x.root, x.len), (y.root, y.len), "{}: step {}", script.name, k + 1);
        let want = match script.steps[k].op {
            Op::Insert(_) => None,
            Op::Remove(l) => Some(golden_nodes::value(l)),
        };
        assert_eq!(x.result, want, "{}: step {}: the result", script.name, k + 1);
        for c in &x.log {
            match *c {
                NodeCall::Alloc { node, .. } => {
                    assert!(!handed_back, "{}: an alloc after a hand-back", script.name);
                    let n = ord.len() as u64 + 1;
                    assert!(ord.insert(node, n).is_none(), "{}: node {node:#x} allocated twice", script.name);
                }
                NodeCall::HandBack(_) => handed_back = true,
                NodeCall::Release(_) => panic!("{}: a release", script.name),
            }
        }
    }
    let ord_of = |w: u64| ord.get(&w).copied();
    let mut obs = Vec::new();
    let mut writer = Writer::new(script);
    for (k, (x, y)) in a.iter().zip(&b).enumerate() {
        let mut o = Observed { root: if x.root == 0 { 0 } else { ord[&x.root] }, len: x.len, ..Default::default() };
        for c in &x.log {
            match *c {
                NodeCall::Alloc { node, leaf } => o.allocs.push((ord[&node], leaf)),
                NodeCall::HandBack(n) => assert!(o.hand_backs.insert(ord[&n])),
                NodeCall::Release(_) => unreachable!(),
            }
        }
        if let (Some(ia), Some(ib)) = (&x.images, &y.images) {
            let mut imgs = Vec::new();
            for ((na, ra), (nb, rb)) in ia.iter().zip(ib) {
                assert_eq!(na, nb);
                let written: Vec<bool> = ra.iter().zip(rb).map(|(p, q)| p == q).collect();
                for (j, (&p, &q)) in ra.iter().zip(rb).enumerate() {
                    assert!(
                        p == q || (p == FILLS[0] && q == FILLS[1]),
                        "{}: step {}: node {na:#x} byte {j} is {p:#04x} and {q:#04x}, not the fill patterns",
                        script.name,
                        k + 1
                    );
                }
                let img = golden_nodes::normalize(script.kind, ra, &written, (1, 2), ord[na], &ord_of)
                    .unwrap_or_else(|e| panic!("{}: step {}: {e}", script.name, k + 1));
                imgs.push(img);
            }
            assert_eq!(ia.len(), ib.len());
            o.images = Some(imgs);
        }
        writer.step(k + 1, &script.steps[k], &o);
        obs.push(o);
    }
    (obs, writer.finish(), cases)
}

fn observe_any(script: &Script) -> (Vec<Observed>, String, Vec<String>) {
    match script.kind {
        Kind::U64 => observe::<u64>(script),
        Kind::A16 => observe::<[u8; 16]>(script),
        Kind::A1012 => observe::<[u8; 1012]>(script),
    }
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../golden/nodes")
}

/// The first line where two texts differ, for the failure message.
fn first_difference(want: &str, got: &str) -> String {
    for (no, (w, g)) in want.lines().zip(got.lines()).enumerate() {
        if w != g {
            return format!("line {}:\n  golden:   {w}\n  produced: {g}", no + 1);
        }
    }
    format!("lengths differ: golden {} lines, produced {} lines", want.lines().count(), got.lines().count())
}

#[test]
fn golden_typed_nodes() {
    // The scripts' own node geometry is the crate's.
    for kind in Kind::ALL {
        for leaf in [true, false] {
            let g = kind.geometry(leaf);
            let n = native_table::tree::shape::node_shape(kind.ks() as u64, 8, 8, !leaf);
            assert_eq!((g.cap as u64, g.voff as u64, g.coff as u64), (n.cap, n.voff, n.coff), "{}", kind.name());
        }
    }
    let out_dir = std::env::var_os("NT_GOLDEN_NODES_OUT").map(PathBuf::from);
    if let Some(d) = &out_dir {
        let g = golden_dir().canonicalize().ok();
        let frozen = "NT_GOLDEN_NODES_OUT never names golden/nodes: the golden typed nodes are frozen";
        assert!(d.canonicalize().ok() != g || g.is_none(), "{frozen}");
        std::fs::create_dir_all(d).unwrap();
    }
    let mut failures = Vec::new();
    for script in golden_nodes::scripts() {
        script.present_after_all();
        let (obs, text, cases) = observe_any(&script);
        // The model's cases are the script's.
        let named: Vec<&str> = script.steps.iter().filter_map(|s| s.cases).collect();
        assert_eq!(named.len(), cases.len());
        for (k, (n, c)) in named.iter().zip(&cases).enumerate() {
            assert_eq!(n, c, "{}: imaged step {} names cases the model does not find", script.name, k + 1);
        }
        // The text reads back to the observations it was made from.
        let parsed = golden_nodes::parse(&text).unwrap();
        let printed: Vec<(usize, &Observed)> = obs
            .iter()
            .enumerate()
            .filter(|(_, o)| !o.allocs.is_empty() || !o.hand_backs.is_empty() || o.images.is_some())
            .map(|(k, o)| (k + 1, o))
            .collect();
        assert_eq!(parsed.len(), printed.len());
        for (p, (k, o)) in parsed.iter().zip(printed) {
            assert_eq!(p.index, k);
            assert_eq!(&p.observed, o, "{}: step {k} does not read back", script.name);
        }
        if let Some(d) = &out_dir {
            std::fs::write(d.join(format!("{}.txt", script.name)), &text).unwrap();
        }
        let path = golden_dir().join(format!("{}.txt", script.name));
        match std::fs::read_to_string(&path) {
            Ok(want) if want == text => {}
            Ok(want) => failures.push(format!("{}: {}", script.name, first_difference(&want, &text))),
            Err(e) => failures.push(format!("{}: {}: {e}", script.name, path.display())),
        }
    }
    assert!(failures.is_empty(), "the golden typed nodes differ from golden/nodes:\n{}", failures.join("\n"));
}

/// Every case the scripts must exercise, each named by an imaged step and found there by the count model.
const LEAF_CASES: [&str; 13] = [
    "first insert",
    "leaf room",
    "leaf split i<h",
    "leaf split i=h",
    "leaf split i>h",
    "root split",
    "leaf remove",
    "predecessor",
    "leaf merge",
    "leaf rotation from the left",
    "leaf rotation from the right",
    "root collapse (internal)",
    "root collapse (leaf)",
];
const INTERNAL_CASES: [&str; 6] = [
    "internal split i<h",
    "internal split i=h",
    "internal split i>h",
    "internal merge",
    "internal rotation from the left",
    "internal rotation from the right",
];

/// The depth-3 scripts (`<u64, u64>` and `<[u8; 1012], u64>`) exercise every case at the leaf level and at an
/// internal level; the depth-2 script (`<[u8; 16], u64>`) every case at the leaf level.
#[test]
fn golden_typed_nodes_cover_every_case() {
    let mut seen: BTreeMap<Kind, BTreeSet<&str>> = BTreeMap::new();
    for script in golden_nodes::scripts() {
        let names: Vec<&str> = script.steps.iter().filter_map(|s| s.cases).flat_map(|c| c.split(", ")).collect();
        for n in &names {
            assert!(LEAF_CASES.contains(n) || INTERNAL_CASES.contains(n), "{}: unknown case {n:?}", script.name);
        }
        seen.entry(script.kind).or_default().extend(names);
    }
    for kind in Kind::ALL {
        let s = &seen[&kind];
        let want: Vec<&str> = match kind {
            Kind::A16 => LEAF_CASES.to_vec(),
            Kind::U64 | Kind::A1012 => LEAF_CASES.iter().chain(&INTERNAL_CASES).copied().collect(),
        };
        let missing: Vec<&&str> = want.iter().filter(|c| !s.contains(**c)).collect();
        assert!(missing.is_empty(), "{}: no imaged step exercises {missing:?}", kind.name());
    }
}
