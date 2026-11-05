//! The `cost_vs_ref` module: the same map operations run three ways in one wasm64 module, so that a
//! runner can meter each call with wasmtime fuel:
//!
//! - `ref`: the transcription of today's `PBTreeMap` (`nt_ref::RefMap`);
//! - `unc`: the crate's tree, `CHECKED` false, over `RawTypedNodes` (`nt_ref::CrateMap`);
//! - `chk`: the same with `CHECKED` true, for information.
//!
//! Shapes: `a` is `<u64, u64>`; `b` is `<u64, [u8; 1024]>`, whose values are boxed, as `PBTreeMap` stores
//! parts above 1,012 bytes; `c` is `<PString, u64>`, whose key comparisons read each string's buffer; `d` is
//! `<[u8; 1013], u64>`, whose keys are boxed. The three maps of a shape are built by the same inserts, each in
//! its own arena, so they hold the same tree and see the same allocator states; every operation is applied
//! to all three.
//!
//! Metered exports, per shape and way: `get`, `ins`, `rem`, `ent` (`entry(k).or_insert(v)`), `seek` (a range
//! iterator from a start bound to an end bound and its first step), `next` (one step of an iterator `open`
//! left in a slot), `scan` (an `Iter` loop over every entry) and `span` (a loop over a bounded range). Keys that live in memory are
//! made by the unmetered `prep` first: the lookup forms of x and e in two static slots, and, before an
//! insert, an owned key per way. Unmetered `setup` builds the trees; `plan` picks the key of the next
//! operation of a category and first brings the trees into the state the category needs (a full leaf, a
//! leaf at its minimum beside a sibling at its minimum, an internal node at its minimum, ...); `check`
//! compares the three trees.

mod stress;
pub mod units;

use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::ops::Bound;
use std::ptr::{addr_of, addr_of_mut};

use nt_ref::map::{Iter, Node};
use nt_ref::mem::{live, select, trap, Region};
use nt_ref::typed::CIter;
use nt_ref::walk::{self, key_array, key_text, TestKey};
use nt_ref::{CrateMap, MapShape, MapShapeChecked, PString, Persistent, RefMap};

/// The boxed value of shape `b`.
pub type Big = [u8; 1024];
/// The boxed key of shape `d`.
pub type BoxKey = [u8; 1013];

/// Arena of each way: allocations of `ref`, `unc` and `chk` come from arenas 0, 1 and 2.
pub const REF: usize = 0;
pub const UNC: usize = 1;
pub const CHK: usize = 2;

// ---- keys and values ----

/// A test key as the metered exports see it.
pub trait HKey: TestKey + 'static {
    /// The lookup form of x in slot s (0 or 1), as `prep` left it there.
    fn lookup(x: u64, s: usize) -> Self::Lookup<'static>;
    /// Writes x's lookup form into slot s.
    fn prep_lookup(x: u64, s: usize);
    /// The owned key of x for way w, as `prep_owned` made it.
    fn owned(x: u64, w: usize) -> Self;
    /// Makes x's owned key for each way, its buffer from that way's arena.
    fn prep_owned(x: u64);
    /// A word of a lookup form, the same in every map.
    fn word(k: Self::Lookup<'_>) -> u64;
}

impl HKey for u64 {
    #[inline(always)]
    fn lookup(x: u64, _s: usize) -> u64 {
        x
    }
    fn prep_lookup(_x: u64, _s: usize) {}
    #[inline(always)]
    fn owned(x: u64, _w: usize) -> u64 {
        x
    }
    fn prep_owned(_x: u64) {}
    #[inline(always)]
    fn word(k: u64) -> u64 {
        k
    }
}

static mut TEXT: [[u8; 17]; 2] = [[b'k'; 17]; 2];
static mut OWNED_TEXT: [Option<PString>; 3] = [None, None, None];

impl HKey for PString {
    #[inline(always)]
    fn lookup(_x: u64, s: usize) -> &'static str {
        // SAFETY: single-threaded; `prep_lookup` wrote ASCII hex into the slot, and no `&mut` to it lives.
        unsafe { core::str::from_utf8_unchecked(&(*addr_of!(TEXT))[s]) }
    }
    fn prep_lookup(x: u64, s: usize) {
        // SAFETY: single-threaded; no reference into the slots lives between metered calls.
        unsafe { (*addr_of_mut!(TEXT))[s] = key_text(x) };
    }
    #[inline(always)]
    fn owned(_x: u64, w: usize) -> PString {
        // SAFETY: single-threaded.
        match unsafe { (*addr_of_mut!(OWNED_TEXT))[w].take() } {
            Some(k) => k,
            None => trap("no prepared key"),
        }
    }
    fn prep_owned(x: u64) {
        for w in [REF, UNC, CHK] {
            select(w);
            // SAFETY: single-threaded; a key left over from an earlier `prep` drops into its own arena.
            unsafe { (*addr_of_mut!(OWNED_TEXT))[w] = Some(PString::from_x(x)) };
        }
    }
    #[inline(always)]
    fn word(k: &str) -> u64 {
        k.len() as u64
    }
}

static mut ARRAY: [BoxKey; 2] = [[0; 1013]; 2];
static mut OWNED_ARRAY: [Option<BoxKey>; 3] = [None, None, None];

impl HKey for BoxKey {
    #[inline(always)]
    fn lookup(_x: u64, s: usize) -> &'static BoxKey {
        // SAFETY: single-threaded; no `&mut` to the slot lives.
        unsafe { &(*addr_of!(ARRAY))[s] }
    }
    fn prep_lookup(x: u64, s: usize) {
        // SAFETY: single-threaded.
        unsafe { (*addr_of_mut!(ARRAY))[s] = key_array(x) };
    }
    #[inline(always)]
    fn owned(_x: u64, w: usize) -> BoxKey {
        // SAFETY: single-threaded.
        match unsafe { (*addr_of_mut!(OWNED_ARRAY))[w].take() } {
            Some(k) => k,
            None => trap("no prepared key"),
        }
    }
    fn prep_owned(x: u64) {
        for w in [REF, UNC, CHK] {
            // SAFETY: single-threaded.
            unsafe { (*addr_of_mut!(OWNED_ARRAY))[w] = Some(key_array(x)) };
        }
    }
    #[inline(always)]
    fn word(k: &BoxKey) -> u64 {
        u64::from(k[0])
    }
}

/// A test value.
pub trait HVal: Persistent + 'static {
    fn make(v: u64) -> Self;
    /// A word of the value, the same in every map (a boxed value is read through its box).
    fn word(v: &Self) -> u64;
}

impl HVal for u64 {
    #[inline(always)]
    fn make(v: u64) -> u64 {
        v
    }
    #[inline(always)]
    fn word(v: &u64) -> u64 {
        *v
    }
}

impl HVal for Big {
    #[inline(always)]
    fn make(v: u64) -> Big {
        [v as u8; 1024]
    }
    #[inline(always)]
    fn word(v: &Big) -> u64 {
        u64::from(v[0])
    }
}

/// A start or end bound: 0 unbounded, 1 included, 2 excluded.
#[inline(always)]
fn bound<T>(mode: u32, k: T) -> Bound<T> {
    match mode {
        0 => Bound::Unbounded,
        1 => Bound::Included(k),
        _ => Bound::Excluded(k),
    }
}

// ---- the maps and the metered exports ----

type Unc<K, V> = CrateMap<K, V, MapShape<K, V>>;
type Chk<K, V> = CrateMap<K, V, MapShapeChecked<K, V>>;
type UncIter<K, V> = CIter<'static, 'static, K, V, MapShape<K, V>>;
type ChkIter<K, V> = CIter<'static, 'static, K, V, MapShapeChecked<K, V>>;

/// The metered exports of one map.
macro_rules! metered {
    ($K:ty, $V:ty, $map:ident, $it:ident, $way:expr,
     $get:ident, $ins:ident, $rem:ident, $ent:ident, $seek:ident, $open:ident, $next:ident, $scan:ident,
     $span:ident) => {
        #[no_mangle]
        pub extern "C" fn $get(x: u64) -> u64 {
            // SAFETY: the module is single-threaded; no other reference to the map lives.
            let m = unsafe { &*addr_of!($map) };
            match m.get(<$K as HKey>::lookup(x, 0)) {
                Some(v) => <$V as HVal>::word(v),
                None => u64::MAX,
            }
        }

        #[no_mangle]
        pub extern "C" fn $ins(x: u64, v: u64) -> u64 {
            select($way);
            // SAFETY: as above.
            let m = unsafe { &mut *addr_of_mut!($map) };
            match m.insert(<$K as HKey>::owned(x, $way), <$V as HVal>::make(v)) {
                Some(_) => 1,
                None => 0,
            }
        }

        #[no_mangle]
        pub extern "C" fn $rem(x: u64) -> u64 {
            select($way);
            // SAFETY: as above.
            let m = unsafe { &mut *addr_of_mut!($map) };
            match m.remove(<$K as HKey>::lookup(x, 0)) {
                Some(v) => <$V as HVal>::word(&v),
                None => u64::MAX,
            }
        }

        #[no_mangle]
        pub extern "C" fn $ent(x: u64, v: u64) -> u64 {
            select($way);
            // SAFETY: as above.
            let m = unsafe { &mut *addr_of_mut!($map) };
            <$V as HVal>::word(m.entry(<$K as HKey>::lookup(x, 0)).or_insert(<$V as HVal>::make(v)))
        }

        /// A range iterator from a start bound to an end bound, and its first step: a cursor's seek in use.
        #[no_mangle]
        pub extern "C" fn $seek(start: u32, x: u64, end: u32, e: u64) -> u64 {
            // SAFETY: as above.
            let m = unsafe { &*addr_of!($map) };
            let mut it = m.range((bound(start, <$K as HKey>::lookup(x, 0)), bound(end, <$K as HKey>::lookup(e, 1))));
            match it.next() {
                Some((k, v)) => <$K as HKey>::word(k) ^ <$V as HVal>::word(v),
                None => u64::MAX,
            }
        }

        /// Opens a range iterator into the way's slot, for `next` (unmetered).
        #[no_mangle]
        pub extern "C" fn $open(start: u32, x: u64, end: u32, e: u64) -> u64 {
            // SAFETY: as above; the iterator is replaced before the map changes.
            let m = unsafe { &*addr_of!($map) };
            let it = m.range((bound(start, <$K as HKey>::lookup(x, 0)), bound(end, <$K as HKey>::lookup(e, 1))));
            // SAFETY: as above.
            unsafe { addr_of_mut!($it).write(MaybeUninit::new(it)) };
            0
        }

        #[no_mangle]
        pub extern "C" fn $next() -> u64 {
            // SAFETY: as above; `seek` filled the slot and the map has not changed since.
            let it = unsafe { (*addr_of_mut!($it)).assume_init_mut() };
            match it.next() {
                Some((k, v)) => <$K as HKey>::word(k) ^ <$V as HVal>::word(v),
                None => u64::MAX,
            }
        }

        /// Every entry through an `Iter` loop: the count in the high 32 bits, a checksum in the low.
        #[no_mangle]
        pub extern "C" fn $scan() -> u64 {
            // SAFETY: as above.
            let m = unsafe { &*addr_of!($map) };
            let (mut n, mut acc) = (0u64, 0u64);
            for (k, v) in m.iter() {
                n += 1;
                acc = acc.wrapping_add(<$K as HKey>::word(k) ^ <$V as HVal>::word(v));
            }
            n << 32 | (acc & 0xFFFF_FFFF)
        }

        /// The entries in [x, e) through a range loop: the count in the high 32 bits, a checksum in the low.
        #[no_mangle]
        pub extern "C" fn $span(x: u64, e: u64) -> u64 {
            // SAFETY: as above.
            let m = unsafe { &*addr_of!($map) };
            let (mut n, mut acc) = (0u64, 0u64);
            let r = (Bound::Included(<$K as HKey>::lookup(x, 0)), Bound::Excluded(<$K as HKey>::lookup(e, 1)));
            for (k, v) in m.range(r) {
                n += 1;
                acc = acc.wrapping_add(<$K as HKey>::word(k) ^ <$V as HVal>::word(v));
            }
            n << 32 | (acc & 0xFFFF_FFFF)
        }
    };
}

/// The three maps of a shape, their iterator slots and their metered exports.
macro_rules! shape {
    ($t:ident, $K:ty, $V:ty, $r:ident, $u:ident, $c:ident, $ir:ident, $iu:ident, $ic:ident,
     [$($rx:ident),*], [$($ux:ident),*], [$($cx:ident),*]) => {
        static mut $r: RefMap<$K, $V> = RefMap::new();
        static mut $u: Unc<$K, $V> = CrateMap::new();
        static mut $c: Chk<$K, $V> = CrateMap::new();
        static mut $ir: MaybeUninit<Iter<'static, 'static, $K, $V>> = MaybeUninit::uninit();
        static mut $iu: MaybeUninit<UncIter<$K, $V>> = MaybeUninit::uninit();
        static mut $ic: MaybeUninit<ChkIter<$K, $V>> = MaybeUninit::uninit();

        metered!($K, $V, $r, $ir, REF, $($rx),*);
        metered!($K, $V, $u, $iu, UNC, $($ux),*);
        metered!($K, $V, $c, $ic, CHK, $($cx),*);

        struct $t;
        impl Trio for $t {
            type K = $K;
            type V = $V;
            fn roots() -> [u64; 3] {
                // SAFETY: single-threaded.
                unsafe { [(*addr_of!($r)).root(), (*addr_of!($u)).root(), (*addr_of!($c)).root()] }
            }
            fn lens() -> [u64; 3] {
                // SAFETY: single-threaded.
                unsafe { [(*addr_of!($r)).len(), (*addr_of!($u)).len(), (*addr_of!($c)).len()] }
            }
            fn clear() {
                // SAFETY: single-threaded; the arenas are reset with the maps, so nothing is dropped.
                unsafe {
                    core::ptr::write(addr_of_mut!($r), RefMap::new());
                    core::ptr::write(addr_of_mut!($u), CrateMap::new());
                    core::ptr::write(addr_of_mut!($c), CrateMap::new());
                }
            }
            fn insert(x: u64) {
                // SAFETY: single-threaded.
                unsafe {
                    select(REF);
                    let a = (*addr_of_mut!($r)).insert(<$K>::from_x(x), <$V as HVal>::make(x)).is_none();
                    select(UNC);
                    let b = (*addr_of_mut!($u)).insert(<$K>::from_x(x), <$V as HVal>::make(x)).is_none();
                    select(CHK);
                    let c = (*addr_of_mut!($c)).insert(<$K>::from_x(x), <$V as HVal>::make(x)).is_none();
                    if !(a && b && c) {
                        trap("insert of a present key");
                    }
                }
                ctl().add(x);
            }
            fn remove(x: u64) {
                // SAFETY: single-threaded.
                unsafe {
                    select(REF);
                    let a = <$K>::with_lookup(x, |k| (*addr_of_mut!($r)).remove(k).is_some());
                    select(UNC);
                    let b = <$K>::with_lookup(x, |k| (*addr_of_mut!($u)).remove(k).is_some());
                    select(CHK);
                    let c = <$K>::with_lookup(x, |k| (*addr_of_mut!($c)).remove(k).is_some());
                    if !(a && b && c) {
                        trap("remove of an absent key");
                    }
                }
                ctl().del(x);
            }
        }
    };
}

shape!(
    ShapeA,
    u64,
    u64,
    A_REF,
    A_UNC,
    A_CHK,
    A_IT_REF,
    A_IT_UNC,
    A_IT_CHK,
    [a_get_ref, a_ins_ref, a_rem_ref, a_ent_ref, a_seek_ref, a_open_ref, a_next_ref, a_scan_ref, a_span_ref],
    [a_get_unc, a_ins_unc, a_rem_unc, a_ent_unc, a_seek_unc, a_open_unc, a_next_unc, a_scan_unc, a_span_unc],
    [a_get_chk, a_ins_chk, a_rem_chk, a_ent_chk, a_seek_chk, a_open_chk, a_next_chk, a_scan_chk, a_span_chk]
);
shape!(
    ShapeB,
    u64,
    Big,
    B_REF,
    B_UNC,
    B_CHK,
    B_IT_REF,
    B_IT_UNC,
    B_IT_CHK,
    [b_get_ref, b_ins_ref, b_rem_ref, b_ent_ref, b_seek_ref, b_open_ref, b_next_ref, b_scan_ref, b_span_ref],
    [b_get_unc, b_ins_unc, b_rem_unc, b_ent_unc, b_seek_unc, b_open_unc, b_next_unc, b_scan_unc, b_span_unc],
    [b_get_chk, b_ins_chk, b_rem_chk, b_ent_chk, b_seek_chk, b_open_chk, b_next_chk, b_scan_chk, b_span_chk]
);
shape!(
    ShapeC,
    PString,
    u64,
    C_REF,
    C_UNC,
    C_CHK,
    C_IT_REF,
    C_IT_UNC,
    C_IT_CHK,
    [c_get_ref, c_ins_ref, c_rem_ref, c_ent_ref, c_seek_ref, c_open_ref, c_next_ref, c_scan_ref, c_span_ref],
    [c_get_unc, c_ins_unc, c_rem_unc, c_ent_unc, c_seek_unc, c_open_unc, c_next_unc, c_scan_unc, c_span_unc],
    [c_get_chk, c_ins_chk, c_rem_chk, c_ent_chk, c_seek_chk, c_open_chk, c_next_chk, c_scan_chk, c_span_chk]
);
shape!(
    ShapeD,
    BoxKey,
    u64,
    D_REF,
    D_UNC,
    D_CHK,
    D_IT_REF,
    D_IT_UNC,
    D_IT_CHK,
    [d_get_ref, d_ins_ref, d_rem_ref, d_ent_ref, d_seek_ref, d_open_ref, d_next_ref, d_scan_ref, d_span_ref],
    [d_get_unc, d_ins_unc, d_rem_unc, d_ent_unc, d_seek_unc, d_open_unc, d_next_unc, d_scan_unc, d_span_unc],
    [d_get_chk, d_ins_chk, d_rem_chk, d_ent_chk, d_seek_chk, d_open_chk, d_next_chk, d_scan_chk, d_span_chk]
);

/// The call itself, for the runner's baseline.
#[no_mangle]
pub extern "C" fn nop(k: u64) -> u64 {
    k
}

// ---- unmetered: building the trees and planning operations ----

/// Operation categories `plan` prepares. Removal categories name what the removal does: at the leaf only,
/// or a leaf merge followed by a merge or rotation of the leaf's parent.
pub const GET: u32 = 0;
pub const INS_ROOM: u32 = 1;
pub const INS_SPLIT: u32 = 2;
pub const REM_LEAF: u32 = 3;
pub const REM_MERGE_LEAF: u32 = 4;
pub const REM_ROTL_LEAF: u32 = 5;
pub const REM_ROTR_LEAF: u32 = 6;
pub const REM_MERGE_INNER: u32 = 7;
pub const REM_ROTL_INNER: u32 = 8;
pub const REM_ROTR_INNER: u32 = 9;
pub const ENTRY: u32 = 10;
pub const PRESENT: u32 = 11;
pub const ABSENT: u32 = 12;

/// `plan`'s answer when the trees cannot have the category (too shallow for an internal level).
pub const NONE: u64 = u64::MAX;

/// The three maps of one shape, for the unmetered work.
trait Trio {
    type K: HKey;
    type V: HVal;
    fn roots() -> [u64; 3];
    fn lens() -> [u64; 3];
    fn clear();
    fn insert(x: u64);
    fn remove(x: u64);
}

struct Ctl {
    region: Region,
    shape: u32,
    n: u64,
    seed: u64,
    rng: u64,
    keys: Vec<u64>,
    index: HashMap<u64, usize>,
}

static mut CTL: Option<Ctl> = None;

fn ctl() -> &'static mut Ctl {
    // SAFETY: single-threaded; callers never hold the reference across another `ctl()`.
    match unsafe { (*addr_of_mut!(CTL)).as_mut() } {
        Some(c) => c,
        None => trap("setup first"),
    }
}

impl Ctl {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn add(&mut self, k: u64) {
        if self.index.insert(k, self.keys.len()).is_some() {
            trap("key added twice");
        }
        self.keys.push(k);
    }

    fn del(&mut self, k: u64) {
        let Some(i) = self.index.remove(&k) else { trap("key not present") };
        let last = self.keys.pop().unwrap_or_else(|| trap("no keys"));
        if last != k {
            self.keys[i] = last;
            self.index.insert(last, i);
        }
    }

    fn present(&mut self) -> u64 {
        let i = self.next() % self.keys.len() as u64;
        self.keys[i as usize]
    }

    /// A random absent key below 2^63.
    fn absent(&mut self) -> u64 {
        loop {
            let k = self.next() >> 1;
            if !self.index.contains_key(&k) {
                return k;
            }
        }
    }

    fn has(&self, k: u64) -> bool {
        self.index.contains_key(&k)
    }
}

/// Arena bytes for n entries of a shape: nodes at about 60% fill, the boxes of keys and values, and room
/// for the plans' changes.
fn arena_bytes(shape: u32, n: u64) -> u64 {
    let per_entry = match shape {
        0 => 27,
        1 => 27 + 1_040,
        2 => 41 + 48,
        _ => 27 + 1_040,
    };
    (n * per_entry).saturating_mul(3) / 2 + (16 << 20)
}

fn build<T: Trio>() {
    forget_owned();
    let n = {
        let c = ctl();
        for i in [REF, UNC, CHK] {
            c.region.reset(i);
        }
        c.keys.clear();
        c.index.clear();
        c.rng = c.seed | 1;
        c.n
    };
    T::clear();
    while (ctl().keys.len() as u64) < n {
        let k = ctl().absent();
        T::insert(k);
    }
}

macro_rules! by_shape {
    ($f:ident $(, $arg:expr)*) => {
        match ctl().shape {
            0 => $f::<ShapeA>($($arg),*),
            1 => $f::<ShapeB>($($arg),*),
            2 => $f::<ShapeC>($($arg),*),
            _ => $f::<ShapeD>($($arg),*),
        }
    };
}

/// Builds the three maps of `shape` (0 `<u64, u64>`, 1 `<u64, [u8; 1024]>`, 2 `<PString, u64>`,
/// 3 `<[u8; 1013], u64>`) with `n` random keys from `seed`. Returns 1 when the three trees are equal.
#[no_mangle]
pub extern "C" fn setup(shape: u32, n: u64, seed: u64) -> u64 {
    // SAFETY: single-threaded; no map, iterator or prepared key of the old region is used after this.
    unsafe {
        forget_owned();
        drop((*addr_of_mut!(CTL)).take());
        *addr_of_mut!(CTL) = Some(Ctl {
            region: Region::install(arena_bytes(shape, n)),
            shape,
            n,
            seed,
            rng: seed | 1,
            keys: Vec::with_capacity(n as usize + 1024),
            index: HashMap::with_capacity(n as usize + 1024),
        });
    }
    by_shape!(build);
    check()
}

/// Lets go of prepared keys without dropping them: their arenas are about to be reset or freed.
fn forget_owned() {
    // SAFETY: single-threaded.
    unsafe {
        for k in (*addr_of_mut!(OWNED_TEXT)).iter_mut() {
            core::mem::forget(k.take());
        }
    }
}

/// 1 when the three trees of the current shape have equal digests, lengths and live allocations, and the
/// reference tree keeps its invariants.
#[no_mangle]
pub extern "C" fn check() -> u64 {
    fn same<T: Trio>() -> u64 {
        let [r, u, c] = T::roots();
        let d = walk::digest::<T::K, T::V>(r);
        let lens = T::lens();
        let ok = d == walk::digest::<T::K, T::V>(u)
            && d == walk::digest::<T::K, T::V>(c)
            && lens[0] == lens[1]
            && lens[0] == lens[2]
            && lens[0] == ctl().keys.len() as u64
            && live(REF) == live(UNC)
            && live(REF) == live(CHK)
            && walk::check::<T::K, T::V>(r) == lens[0];
        u64::from(ok)
    }
    by_shape!(same)
}

/// Facts about the reference tree: 0 entries, 1 depth.
#[no_mangle]
pub extern "C" fn info(what: u32) -> u64 {
    fn depth<T: Trio>() -> u64 {
        walk::depth::<T::K, T::V>(T::roots()[0])
    }
    match what {
        0 => ctl().keys.len() as u64,
        _ => by_shape!(depth),
    }
}

/// Writes the lookup forms of x and e into the key slots and, when `owned` is 1, x's owned key per way.
#[no_mangle]
pub extern "C" fn prep(x: u64, e: u64, owned: u32) {
    fn go<T: Trio>(x: u64, e: u64, owned: u32) {
        T::K::prep_lookup(x, 0);
        T::K::prep_lookup(e, 1);
        if owned == 1 {
            T::K::prep_owned(x);
        }
    }
    by_shape!(go, x, e, owned)
}

// ---- tree surgery for the plans ----

fn leaf_cap<T: Trio>() -> u64 {
    Node::<T::K, T::V>::LEAF.0
}

fn inner_cap<T: Trio>() -> u64 {
    Node::<T::K, T::V>::INTERNAL.0
}

fn count<T: Trio>(n: u64) -> u64 {
    Node::<T::K, T::V>::count(n)
}

fn child<T: Trio>(n: u64, j: u64) -> u64 {
    Node::<T::K, T::V>::child(n, j)
}

fn is_leaf<T: Trio>(n: u64) -> bool {
    Node::<T::K, T::V>::is_leaf(n)
}

/// The smallest sibling count at which a node one below its minimum rotates instead of merging.
fn rotates(cap: u64) -> u64 {
    cap - cap / 2 + 1
}

/// Removes keys of leaf n other than `keep` until it holds `to` entries.
fn shrink<T: Trio>(n: u64, keep: u64, to: u64) {
    while count::<T>(n) > to {
        let ks = walk::keys::<T::K, T::V>(n);
        let k = if ks[0] != keep { ks[0] } else { ks[ks.len() - 1] };
        T::remove(k);
    }
}

/// An absent key between two adjacent keys of leaf n, other than `skip`.
fn gap<T: Trio>(n: u64, skip: u64) -> Option<u64> {
    let ks = walk::keys::<T::K, T::V>(n);
    ks.windows(2).map(|w| w[0] + (w[1] - w[0]) / 2).find(|&m| m != skip && !ctl().has(m))
}

/// Inserts keys between adjacent keys of leaf n, other than `skip`, until it holds `to` entries.
fn grow<T: Trio>(n: u64, skip: u64, to: u64) -> bool {
    while count::<T>(n) < to {
        match gap::<T>(n, skip) {
            Some(m) => T::insert(m),
            None => return false,
        }
    }
    true
}

/// One more entry in internal node p, whose children are leaves: its first child splits.
fn raise<T: Trio>(p: u64) -> bool {
    let c = child::<T>(p, 0);
    if count::<T>(p) >= inner_cap::<T>() || !grow::<T>(c, NONE, leaf_cap::<T>()) {
        return false;
    }
    match gap::<T>(c, NONE) {
        Some(m) => {
            T::insert(m);
            true
        }
        None => false,
    }
}

/// One entry fewer in internal node p, whose children are leaves: its first two children merge.
fn lower<T: Trio>(p: u64) {
    let min = leaf_cap::<T>() / 2;
    let (c0, c1) = (child::<T>(p, 0), child::<T>(p, 1));
    shrink::<T>(c0, NONE, min);
    shrink::<T>(c1, NONE, min);
    let ks = walk::keys::<T::K, T::V>(c1);
    T::remove(ks[0]);
}

/// Sets internal node p's count to exactly `to` (≥ its minimum, below its capacity).
fn set_inner<T: Trio>(p: u64, to: u64) -> bool {
    while count::<T>(p) > to {
        lower::<T>(p);
    }
    while count::<T>(p) < to {
        if !raise::<T>(p) {
            return false;
        }
    }
    true
}

/// Brings the trees back to between 95% and 105% of n entries with random inserts and removals.
fn resize<T: Trio>() {
    let n = ctl().n;
    let (lo, hi) = (n - n / 20, n + n / 20);
    while (ctl().keys.len() as u64) < lo.max(2) {
        let k = ctl().absent();
        T::insert(k);
    }
    while ctl().keys.len() as u64 > hi {
        let k = ctl().present();
        T::remove(k);
    }
}

fn plan_in<T: Trio>(cat: u32) -> u64 {
    resize::<T>();
    let (lcap, icap) = (leaf_cap::<T>(), inner_cap::<T>());
    let (lmin, imin) = (lcap / 2, icap / 2);
    let deep = walk::depth::<T::K, T::V>(T::roots()[0]) >= 3;
    if matches!(cat, REM_MERGE_INNER | REM_ROTL_INNER | REM_ROTR_INNER) && !deep {
        return NONE;
    }
    for _ in 0..100_000 {
        let root = T::roots()[0];
        match cat {
            GET | PRESENT => return ctl().present(),
            ABSENT => return ctl().absent(),
            INS_ROOM | ENTRY => {
                let k = ctl().absent();
                let loc = walk::locate::<T::K, T::V>(root, k);
                if count::<T>(loc.node) < lcap {
                    return k;
                }
            }
            INS_SPLIT => {
                let k = ctl().absent();
                let loc = walk::locate::<T::K, T::V>(root, k);
                if grow::<T>(loc.node, k, lcap) {
                    return k;
                }
            }
            REM_LEAF => {
                let k = ctl().present();
                let loc = walk::locate::<T::K, T::V>(root, k);
                if is_leaf::<T>(loc.node) && !loc.path.is_empty() && count::<T>(loc.node) > lmin {
                    return k;
                }
            }
            REM_MERGE_LEAF | REM_ROTL_LEAF | REM_ROTR_LEAF => {
                let k = ctl().present();
                let loc = walk::locate::<T::K, T::V>(root, k);
                let Some(&(p, j)) = loc.path.last() else { continue };
                if !is_leaf::<T>(loc.node) {
                    continue;
                }
                match cat {
                    REM_MERGE_LEAF => {
                        // The parent must take the merge without rebalancing in turn.
                        let pc = count::<T>(p);
                        if (loc.path.len() == 1 && pc < 2) || (loc.path.len() > 1 && pc <= imin) {
                            continue;
                        }
                        let s = if j > 0 { child::<T>(p, j - 1) } else { child::<T>(p, 1) };
                        shrink::<T>(loc.node, k, lmin);
                        shrink::<T>(s, k, lmin);
                        return k;
                    }
                    REM_ROTL_LEAF => {
                        if j == 0 {
                            continue;
                        }
                        let s = child::<T>(p, j - 1);
                        if !grow::<T>(s, k, rotates(lcap).max(count::<T>(s))) {
                            continue;
                        }
                        shrink::<T>(loc.node, k, lmin);
                        return k;
                    }
                    _ => {
                        // Rotation from the right: the parent's first child takes from the second.
                        let (l, s) = (child::<T>(p, 0), child::<T>(p, 1));
                        if !grow::<T>(s, NONE, rotates(lcap).max(count::<T>(s))) {
                            continue;
                        }
                        shrink::<T>(l, NONE, lmin);
                        let ks = walk::keys::<T::K, T::V>(l);
                        return ks[ks.len() / 2];
                    }
                }
            }
            _ => {
                // A leaf merge whose parent p, at its minimum, then merges with or rotates from its
                // sibling q under grandparent g, which takes the change without rebalancing.
                let k = ctl().present();
                let loc = walk::locate::<T::K, T::V>(root, k);
                let f = loc.path.len();
                if f < 2 {
                    continue;
                }
                let (g, _) = loc.path[f - 2];
                let gc = count::<T>(g);
                let g_root = f == 2;
                if (g_root && gc < 2) || (!g_root && gc <= imin) {
                    continue;
                }
                let jp = match cat {
                    REM_ROTR_INNER => 0,
                    _ => 1 + ctl().next() % gc,
                };
                let p = child::<T>(g, jp);
                let q = if jp > 0 { child::<T>(g, jp - 1) } else { child::<T>(g, 1) };
                if !is_leaf::<T>(child::<T>(p, 0)) {
                    continue;
                }
                let q_to = if cat == REM_MERGE_INNER { imin } else { rotates(icap).max(count::<T>(q)) };
                if !set_inner::<T>(q, q_to) || !set_inner::<T>(p, imin) {
                    continue;
                }
                let j = 1 + ctl().next() % count::<T>(p);
                let (l, s) = (child::<T>(p, j), child::<T>(p, j - 1));
                shrink::<T>(s, NONE, lmin);
                shrink::<T>(l, NONE, lmin);
                let ks = walk::keys::<T::K, T::V>(l);
                return ks[ks.len() / 2];
            }
        }
    }
    trap("no key found for the category")
}

/// The key of the next operation of category `cat`, after bringing the three trees into the state the
/// category needs; `NONE` when the trees are too shallow for it. The key enters (or leaves) the key list as
/// if the operation had run, so the runner must apply it to all three maps.
#[no_mangle]
pub extern "C" fn plan(cat: u32) -> u64 {
    let k = by_shape!(plan_in, cat);
    if k == NONE {
        return NONE;
    }
    let c = ctl();
    match cat {
        INS_ROOM | INS_SPLIT | ENTRY => c.add(k),
        REM_LEAF..=REM_ROTR_INNER => c.del(k),
        _ => {}
    }
    k
}

/// What the next removal of x does in the reference tree, one octal digit per level from the leaf up:
/// 0 nothing, 1 a merge, 2 a rotation from the left sibling, 3 from the right sibling, 4 the root's
/// collapse. 0o77 when x is absent, 0o76 when it lies in an internal node.
#[no_mangle]
pub extern "C" fn removal_kind(x: u64) -> u64 {
    fn kind<T: Trio>(x: u64) -> u64 {
        let loc = walk::locate::<T::K, T::V>(T::roots()[0], x);
        if !loc.found {
            return 0o77;
        }
        if !is_leaf::<T>(loc.node) {
            return 0o76;
        }
        let mut path = loc.path;
        let (mut n, mut after, mut code, mut shift) = (loc.node, count::<T>(loc.node) - 1, 0u64, 0u64);
        loop {
            let cap = if is_leaf::<T>(n) { leaf_cap::<T>() } else { inner_cap::<T>() };
            let Some((p, j)) = path.pop() else {
                if after == 0 {
                    code |= 4 << shift;
                }
                return code;
            };
            if after >= cap / 2 {
                return code;
            }
            let s = if j > 0 { child::<T>(p, j - 1) } else { child::<T>(p, 1) };
            if after + count::<T>(s) < cap {
                code |= 1 << shift;
                n = p;
                after = count::<T>(p) - 1;
                shift += 3;
            } else {
                return code | (if j > 0 { 2 } else { 3 }) << shift;
            }
        }
    }
    by_shape!(kind, x)
}

/// Where x lies or would go in the reference tree: the internal levels above its node in bits 48 and up,
/// its index in bits 16 to 47, its node's count in bits 0 to 15. For the runner's diagnostics.
#[no_mangle]
pub extern "C" fn position(x: u64) -> u64 {
    fn pos<T: Trio>(x: u64) -> u64 {
        let loc = walk::locate::<T::K, T::V>(T::roots()[0], x);
        (loc.path.len() as u64) << 48 | loc.index << 16 | count::<T>(loc.node)
    }
    by_shape!(pos, x)
}

/// 1 when inserting x would split its leaf in the reference tree.
#[no_mangle]
pub extern "C" fn insert_splits(x: u64) -> u64 {
    fn splits<T: Trio>(x: u64) -> u64 {
        let loc = walk::locate::<T::K, T::V>(T::roots()[0], x);
        u64::from(!loc.found && count::<T>(loc.node) == leaf_cap::<T>())
    }
    by_shape!(splits, x)
}
