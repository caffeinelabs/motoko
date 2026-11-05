//! Keys made from a `u64`, and tree walks over typed nodes for tests and the cost module: a digest of a
//! tree's structure and contents that does not depend on node or buffer addresses, its depth, and where a
//! key lies.

use crate::map::{search, Node};
use crate::part::{PKey, PString, Part, Persistent};

/// A key made from a `u64` x: keys order as their x values do, and read back to them.
pub trait TestKey: PKey {
    /// The key of x; a string allocates its buffer in the selected arena.
    fn from_x(x: u64) -> Self;
    /// The x the key was made from.
    fn x(&self) -> u64 {
        Self::lookup_x(self.as_lookup())
    }
    /// The x a lookup form was made from.
    fn lookup_x(k: Self::Lookup<'_>) -> u64;
    /// Calls `f` with the lookup form of x's key.
    fn with_lookup<R>(x: u64, f: impl FnOnce(Self::Lookup<'_>) -> R) -> R;
}

impl TestKey for u64 {
    fn from_x(x: u64) -> Self {
        x
    }
    fn lookup_x(k: u64) -> u64 {
        k
    }
    fn with_lookup<R>(x: u64, f: impl FnOnce(u64) -> R) -> R {
        f(x)
    }
}

/// The text of x's string key: "k" and 16 lowercase hex digits, so the texts order as the numbers do.
pub fn key_text(x: u64) -> [u8; 17] {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut t = [b'k'; 17];
    for (i, d) in t[1..].iter_mut().enumerate() {
        *d = HEX[((x >> (60 - 4 * i)) & 15) as usize];
    }
    t
}

impl TestKey for PString {
    fn from_x(x: u64) -> Self {
        let t = key_text(x);
        PString::from_str(core::str::from_utf8(&t).unwrap_or(""))
    }
    fn lookup_x(k: &str) -> u64 {
        k.as_bytes().iter().skip(1).fold(0u64, |a, &d| a << 4 | u64::from((d as char).to_digit(16).unwrap_or(0)))
    }
    fn with_lookup<R>(x: u64, f: impl FnOnce(&str) -> R) -> R {
        let t = key_text(x);
        f(core::str::from_utf8(&t).unwrap_or(""))
    }
}

/// The bytes of x's array key: x big-endian, then a fixed fill, so the arrays order as the numbers do.
pub fn key_array<const N: usize>(x: u64) -> [u8; N] {
    let mut a = [0x5Au8; N];
    a[..8].copy_from_slice(&x.to_be_bytes());
    a
}

impl<const N: usize> TestKey for [u8; N] {
    fn from_x(x: u64) -> Self {
        key_array(x)
    }
    fn lookup_x(k: &[u8; N]) -> u64 {
        u64::from_be_bytes(k[..8].try_into().unwrap_or([0; 8]))
    }
    fn with_lookup<R>(x: u64, f: impl FnOnce(&[u8; N]) -> R) -> R {
        let a = key_array::<N>(x);
        f(&a)
    }
}

/// Folds a word into a running digest (splitmix64's finalizer).
#[inline]
pub fn mix(h: u64, w: u64) -> u64 {
    let mut z = h ^ w;
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The first 8 bytes of value i of node n (through its box when boxed), zero-padded.
fn value_word<K: Persistent, V: Persistent>(n: u64, i: u64) -> u64 {
    // SAFETY: a live entry of a live node; the value is read only.
    let p = unsafe { Part::<V>::value_ptr(Node::<K, V>::val(n, i)) }.cast::<u8>();
    let len = (V::SIZE as usize).min(8);
    let mut b = [0u8; 8];
    // SAFETY: the value holds at least `len` bytes.
    unsafe { core::ptr::copy_nonoverlapping(p, b.as_mut_ptr(), len) };
    u64::from_le_bytes(b)
}

/// The x of key i of node n.
fn key_x<K: TestKey, V: Persistent>(n: u64, i: u64) -> u64 {
    // SAFETY: a live entry.
    unsafe { Node::<K, V>::key_ref(n, i) }.x()
}

/// A digest of the tree at `root`: in pre-order, every node's count and leaf bit, then its keys' x values
/// and the first 8 bytes of its values. Equal digests mean the same shape and contents.
pub fn digest<K: TestKey, V: Persistent>(root: u64) -> u64 {
    let mut h = mix(0, root.min(1));
    if root == 0 {
        return h;
    }
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        let c = Node::<K, V>::count(n);
        let leaf = Node::<K, V>::is_leaf(n);
        h = mix(h, c << 1 | u64::from(leaf));
        for i in 0..c {
            h = mix(h, key_x::<K, V>(n, i));
            h = mix(h, value_word::<K, V>(n, i));
        }
        if !leaf {
            for j in (0..=c).rev() {
                stack.push(Node::<K, V>::child(n, j));
            }
        }
    }
    h
}

/// The number of levels of the tree at `root`.
pub fn depth<K: Persistent, V: Persistent>(mut n: u64) -> u64 {
    let mut d = 0;
    while n != 0 {
        d += 1;
        if Node::<K, V>::is_leaf(n) {
            break;
        }
        n = Node::<K, V>::child(n, 0);
    }
    d
}

/// Where a key lies or would go: the internal frames passed (node, child index), the final node, the
/// position in it and whether the key is there.
#[derive(Clone, Debug)]
pub struct Loc {
    pub path: Vec<(u64, u64)>,
    pub node: u64,
    pub index: u64,
    pub found: bool,
}

/// The descent to x's key in the non-empty tree at `root`.
pub fn locate<K: TestKey, V: Persistent>(root: u64, x: u64) -> Loc {
    K::with_lookup(x, |k| {
        let mut path = Vec::new();
        let mut n = root;
        loop {
            match search::<K, V>(n, k) {
                Ok(i) => return Loc { path, node: n, index: i, found: true },
                Err(j) => {
                    if Node::<K, V>::is_leaf(n) {
                        return Loc { path, node: n, index: j, found: false };
                    }
                    path.push((n, j));
                    n = Node::<K, V>::child(n, j);
                }
            }
        }
    })
}

/// The x values of node n's keys.
pub fn keys<K: TestKey, V: Persistent>(n: u64) -> Vec<u64> {
    (0..Node::<K, V>::count(n)).map(|i| key_x::<K, V>(n, i)).collect()
}

/// Checks the tree's invariants (order, fill, equal leaf depths) and returns its entry count.
pub fn check<K: TestKey, V: Persistent>(root: u64) -> u64 {
    fn walk<K: TestKey, V: Persistent>(
        n: u64,
        depth: u32,
        root: bool,
        leaf_depth: &mut Option<u32>,
        prev: &mut Option<u64>,
    ) -> u64 {
        let c = Node::<K, V>::count(n);
        let cap = Node::<K, V>::cap(n);
        let leaf = Node::<K, V>::is_leaf(n);
        assert!(c <= cap, "count above capacity");
        assert!(if root { c >= 1 } else { c >= cap / 2 }, "node below its minimum");
        let mut total = 0;
        for i in 0..=c {
            if !leaf {
                total += walk::<K, V>(Node::<K, V>::child(n, i), depth + 1, false, leaf_depth, prev);
            }
            if i < c {
                let k = key_x::<K, V>(n, i);
                assert!(prev.map_or(true, |p| p < k), "keys out of order");
                *prev = Some(k);
                total += 1;
            }
        }
        if leaf {
            assert_eq!(*leaf_depth.get_or_insert(depth), depth, "leaves at different depths");
        }
        total
    }
    if root == 0 {
        return 0;
    }
    walk::<K, V>(root, 1, true, &mut None, &mut None)
}
