//! The crate's typed tree over `RawTypedNodes`, unchecked and checked, against the transcription of
//! today's `PBTreeMap`: the same answers, the same tree after every operation, the same node and box
//! allocations, and, for `u64` keys, the same number of key comparisons per operation. Keys: `u64`,
//! strings (comparisons read the string's buffer) and boxed 1,013-byte arrays.

use std::cell::Cell;
use std::cmp::Ordering;
use std::ops::Bound;

use nt_ref::mem::{live, select, Region};
use nt_ref::walk::{check, digest};
use nt_ref::{CrateMap, MapShapeChecked, PKey, PString, Persistent, RefMap, TestKey};

thread_local! {
    static COMPARISONS: Cell<u64> = const { Cell::new(0) };
}

fn comparisons() -> u64 {
    COMPARISONS.with(|c| c.replace(0))
}

/// A `u64` key that counts its comparisons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
struct Counted(u64);

impl Persistent for Counted {
    const SIZE: u32 = 8;
    const ALIGN: u32 = 8;
}

// SAFETY: the lookup form is the wrapped value, ordered by its `Ord`.
unsafe impl PKey for Counted {
    type Lookup<'a> = u64;
    fn as_lookup(&self) -> u64 {
        self.0
    }
    fn cmp_lookup<'a, 'b>(a: u64, b: u64) -> Ordering
    where
        Self: 'a + 'b,
    {
        COMPARISONS.with(|c| c.set(c.get() + 1));
        a.cmp(&b)
    }
    fn from_lookup(k: u64) -> Self {
        Counted(k)
    }
}

impl TestKey for Counted {
    fn from_x(x: u64) -> Self {
        Counted(x)
    }
    fn lookup_x(k: u64) -> u64 {
        k
    }
    fn with_lookup<R>(x: u64, f: impl FnOnce(u64) -> R) -> R {
        f(x)
    }
}

struct Gen(u64);

impl Gen {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

trait Val: Persistent + Copy + PartialEq + std::fmt::Debug {
    fn make(x: u64) -> Self;
}

impl Val for u64 {
    fn make(x: u64) -> Self {
        x
    }
}

impl Val for [u8; 1024] {
    fn make(x: u64) -> Self {
        let mut v = [0u8; 1024];
        v[..8].copy_from_slice(&x.to_le_bytes());
        v[1023] = x as u8;
        v
    }
}

type Checked<K, V> = CrateMap<K, V, MapShapeChecked<K, V>>;

struct Trio<K: TestKey, V: Val> {
    r: RefMap<K, V>,
    u: CrateMap<K, V>,
    c: Checked<K, V>,
}

impl<K: TestKey, V: Val> Trio<K, V> {
    fn new() -> Self {
        Trio { r: RefMap::new(), u: CrateMap::new(), c: CrateMap::new() }
    }

    /// Runs `f` on each map with its own arena selected; the results and comparison counts must agree.
    fn each<R: PartialEq + std::fmt::Debug>(
        &mut self,
        what: &str,
        fr: impl FnOnce(&mut RefMap<K, V>) -> R,
        fu: impl FnOnce(&mut CrateMap<K, V>) -> R,
        fc: impl FnOnce(&mut Checked<K, V>) -> R,
    ) -> R {
        comparisons();
        select(0);
        let a = fr(&mut self.r);
        let ca = comparisons();
        select(1);
        let b = fu(&mut self.u);
        let cb = comparisons();
        select(2);
        let c = fc(&mut self.c);
        let cc = comparisons();
        assert_eq!(a, b, "{what}: unchecked answer");
        assert_eq!(a, c, "{what}: checked answer");
        assert_eq!(ca, cb, "{what}: unchecked comparison count");
        assert_eq!(ca, cc, "{what}: checked comparison count");
        a
    }

    fn same(&self) {
        let d = digest::<K, V>(self.r.root());
        assert_eq!(d, digest::<K, V>(self.u.root()), "unchecked tree");
        assert_eq!(d, digest::<K, V>(self.c.root()), "checked tree");
        assert_eq!(self.r.len(), self.u.len());
        assert_eq!(self.r.len(), self.c.len());
        assert_eq!(live(0), live(1), "unchecked allocations");
        assert_eq!(live(0), live(2), "checked allocations");
    }
}

fn bound(sel: u64) -> u64 {
    sel % 3
}

macro_rules! range_of {
    ($m:expr, $lo:expr, $hi:expr, $x:expr, $e:expr) => {{
        // Owned keys, so both bounds borrow for one lifetime; a string's buffer comes from the map's arena
        // and goes back to it.
        let (a, b) = (K::from_x($x), K::from_x($e));
        let lo = match $lo {
            0 => Bound::Unbounded,
            1 => Bound::Included(a.as_lookup()),
            _ => Bound::Excluded(a.as_lookup()),
        };
        let hi = match $hi {
            0 => Bound::Unbounded,
            1 => Bound::Included(b.as_lookup()),
            _ => Bound::Excluded(b.as_lookup()),
        };
        let out = $m.range((lo, hi)).take(300).map(|(k, v)| (K::lookup_x(k), *v)).collect::<Vec<_>>();
        out
    }};
}

fn run<K: TestKey, V: Val>(seed: u64, ops: u64, space: u64, arena: u64) {
    let _region = Region::install(arena);
    let mut t = Trio::<K, V>::new();
    let mut g = Gen(seed);
    for step in 0..ops {
        let r = g.next();
        let x = r % space;
        match (r >> 32) % 9 {
            0..=3 => {
                let v = V::make(r);
                t.each(
                    "insert",
                    |m| m.insert(K::from_x(x), v),
                    |m| m.insert(K::from_x(x), v),
                    |m| m.insert(K::from_x(x), v),
                );
            }
            4 => {
                let v = V::make(r);
                t.each(
                    "entry",
                    |m| K::with_lookup(x, |k| *m.entry(k).or_insert(v)),
                    |m| K::with_lookup(x, |k| *m.entry(k).or_insert(v)),
                    |m| K::with_lookup(x, |k| *m.entry(k).or_insert(v)),
                );
            }
            5 | 6 => {
                t.each(
                    "remove",
                    |m| K::with_lookup(x, |k| m.remove(k)),
                    |m| K::with_lookup(x, |k| m.remove(k)),
                    |m| K::with_lookup(x, |k| m.remove(k)),
                );
            }
            7 => {
                t.each(
                    "get",
                    |m| K::with_lookup(x, |k| m.get(k).copied()),
                    |m| K::with_lookup(x, |k| m.get(k).copied()),
                    |m| K::with_lookup(x, |k| m.get(k).copied()),
                );
            }
            _ => {
                let (lo, hi, e) = (bound(r >> 40), bound(r >> 48), x.wrapping_add(r >> 54));
                t.each(
                    "range",
                    |m| range_of!(m, lo, hi, x, e),
                    |m| range_of!(m, lo, hi, x, e),
                    |m| range_of!(m, lo, hi, x, e),
                );
            }
        }
        if step % 512 == 0 {
            t.same();
            assert_eq!(check::<K, V>(t.r.root()), t.r.len());
        }
    }
    t.same();
    assert_eq!(check::<K, V>(t.r.root()), t.r.len());
    // Drain everything: merges and rotations up to the root's collapse.
    select(0);
    let all: Vec<u64> = t.r.iter().map(|(k, _)| K::lookup_x(k)).collect();
    for (i, x) in all.into_iter().enumerate() {
        t.each(
            "drain",
            |m| K::with_lookup(x, |k| m.remove(k)),
            |m| K::with_lookup(x, |k| m.remove(k)),
            |m| K::with_lookup(x, |k| m.remove(k)),
        );
        if i % 256 == 0 {
            t.same();
        }
    }
    t.same();
    assert_eq!(t.r.root(), 0);
    assert_eq!(live(0), (0, 0));
}

#[test]
fn small_values_agree() {
    run::<Counted, u64>(0x2545_F491_4F6C_DD1D, 60_000, 20_000, 32 << 20);
}

#[test]
fn boxed_values_agree() {
    run::<Counted, [u8; 1024]>(0x9E37_79B9_7F4A_7C15, 8_000, 3_000, 32 << 20);
}

#[test]
fn string_keys_agree() {
    run::<PString, u64>(0x5851_F42D_4C95_7F2D, 40_000, 15_000, 32 << 20);
}

#[test]
fn boxed_keys_agree() {
    run::<[u8; 1013], u64>(0x1405_7B7E_F767_814F, 8_000, 3_000, 32 << 20);
}

#[test]
fn sequential_fill_and_drain_agree() {
    let _region = Region::install(32 << 20);
    let mut t = Trio::<Counted, u64>::new();
    for k in 0..40_000u64 {
        t.each("insert", |m| m.insert(Counted(k), k), |m| m.insert(Counted(k), k), |m| m.insert(Counted(k), k));
    }
    t.same();
    for k in (0..40_000u64).rev().step_by(3) {
        t.each("remove", |m| m.remove(k), |m| m.remove(k), |m| m.remove(k));
    }
    t.same();
    for k in 0..40_000u64 {
        t.each("remove", |m| m.remove(k), |m| m.remove(k), |m| m.remove(k));
    }
    t.same();
    assert_eq!(live(0), (0, 0));
}
