//! Node shapes: capacities and offsets of a tree's leaves and internal nodes, for the
//! typed form (`PBTreeMap`'s parts) and the index form (a W + 8-byte key part and a V-byte value part).

use crate::consts::{FLAG_INDEX, FLAG_LEAF, NODE_SIZE, PART_MAX, W_MAX, W_MIN};

/// A node's capacity c, the offset of its value parts and, for an internal node, of its children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeShape {
    pub cap: u64,
    pub voff: u64,
    pub coff: u64,
}

const fn align_up(x: u64, a: u64) -> u64 {
    // a is a power of two of at most 4,096 and x below 2^32 here.
    let m = a.wrapping_sub(1);
    x.wrapping_add(m) & !m
}

/// (voff, coff) of a node of capacity c.
const fn node_offsets(c: u64, ks: u64, va: u64, vs: u64) -> (u64, u64) {
    let voff = align_up(8u64.wrapping_add(c.wrapping_mul(ks)), va);
    (voff, align_up(voff.wrapping_add(c.wrapping_mul(vs)), 8))
}

/// Transcription of `PBTreeMap`'s `node_shape`: count c upward from 0; a leaf fits while
/// voff + c·vs ≤ 4,080, an internal node while coff + 8(c + 1) ≤ 4,080; the result is the largest fitting c.
pub const fn node_shape(ks: u64, va: u64, vs: u64, internal: bool) -> NodeShape {
    let mut c: u64 = 0;
    while c < NODE_SIZE {
        let (voff, coff) = node_offsets(c.wrapping_add(1), ks, va, vs);
        let end = if internal {
            coff.wrapping_add(8u64.wrapping_mul(c.wrapping_add(2)))
        } else {
            voff.wrapping_add(c.wrapping_add(1).wrapping_mul(vs))
        };
        if end > NODE_SIZE {
            break;
        }
        c = c.wrapping_add(1);
    }
    let (voff, coff) = node_offsets(c, ks, va, vs);
    NodeShape { cap: c, voff, coff }
}

/// The leaf shape of a typed map, refused at compile time below two entries, with `PBTreeMap`'s text.
pub const fn leaf_checked(ks: u64, va: u64, vs: u64) -> NodeShape {
    let s = node_shape(ks, va, vs, false);
    assert!(s.cap >= 2, "a leaf holds at least two entries");
    s
}

/// The internal shape of a typed map, refused at compile time below two entries.
pub const fn internal_checked(ks: u64, va: u64, vs: u64) -> NodeShape {
    let s = node_shape(ks, va, vs, true);
    assert!(s.cap >= 2, "an internal node holds at least two entries");
    s
}

/// Offsets and capacities of one tree's nodes.
pub trait Shape: Copy {
    /// Typed form (`PBTreeMap`): `find` keeps no depth counter, as today.
    const TYPED: bool;
    /// Clamp counts and in-node indices to the node's capacity: true for every index shape; for a typed
    /// shape, `TypedShape::CHECKED`, false by default, so typed maps keep today's unchecked arithmetic.
    const CHECKED: bool;
    /// Key-part bytes; index form: W + 8.
    fn ks(self) -> u64;
    /// Value-part bytes; index form: V.
    fn vs(self) -> u64;
    /// (nL, voff, coff) or (nI, voff, coff).
    fn shape(self, leaf: bool) -> NodeShape;
    /// The flags word of a fresh node: typed 1 / 0; index 3 / 2.
    fn flags(self, leaf: bool) -> u16;
}

/// Typed form: implemented by the typed front end of Rust EOP (a Rust library for upgrades that keep the heap
/// in place) for a zero-sized marker per (K, V).
pub trait TypedShape {
    /// `Part::<K>::SIZE`.
    const KS: u64;
    /// `Part::<V>::ALIGN`.
    const VA: u64;
    /// `Part::<V>::SIZE`.
    const VS: u64;
    /// Typed maps are unchecked by default.
    const CHECKED: bool = false;
    const LEAF: NodeShape = leaf_checked(Self::KS, Self::VA, Self::VS);
    const INTERNAL: NodeShape = internal_checked(Self::KS, Self::VA, Self::VS);
}

/// The typed shape of marker T.
pub struct Typed<T>(core::marker::PhantomData<fn() -> T>);

impl<T> Clone for Typed<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Typed<T> {}

impl<T> Typed<T> {
    pub const NEW: Self = Typed(core::marker::PhantomData);
}

impl<T: TypedShape> Shape for Typed<T> {
    const TYPED: bool = true;
    const CHECKED: bool = T::CHECKED;
    #[inline(always)]
    fn ks(self) -> u64 {
        T::KS
    }
    #[inline(always)]
    fn vs(self) -> u64 {
        T::VS
    }
    #[inline(always)]
    fn shape(self, leaf: bool) -> NodeShape {
        if leaf {
            T::LEAF
        } else {
            T::INTERNAL
        }
    }
    #[inline(always)]
    fn flags(self, leaf: bool) -> u16 {
        if leaf {
            FLAG_LEAF
        } else {
            0
        }
    }
}

/// `node_shape(ks, 8, v, internal)` in closed form for an index node, whose key part ks and value part v are
/// multiples of 8, so no offset needs alignment: a leaf fits c entries while 8 + c·(ks + v) ≤ 4,080, an
/// internal node while 8 + c·(ks + v) + 8·(c + 1) ≤ 4,080. So nL = ⌊4,072 / (ks + v)⌋ and
/// nI = ⌊4,064 / (ks + v + 8)⌋, with voff = 8 + c·ks and coff = voff + c·v.
pub const fn index_node_shape(ks: u64, v: u64, internal: bool) -> NodeShape {
    // ks ≤ 1,024 and v ≤ 256: every product below stays under 2^20.
    let per = if internal { ks.wrapping_add(v).wrapping_add(8) } else { ks.wrapping_add(v) };
    let room = if internal { NODE_SIZE.wrapping_sub(16) } else { NODE_SIZE.wrapping_sub(8) };
    let cap = match room.checked_div(per) {
        Some(c) => c,
        None => 0,
    };
    let voff = 8u64.wrapping_add(cap.wrapping_mul(ks));
    NodeShape { cap, voff, coff: voff.wrapping_add(cap.wrapping_mul(v)) }
}

/// Index form: key part W + 8 (key area and entry field), value part V, value alignment 8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexShape {
    pub w: u64,
    pub v: u64,
    pub leaf: NodeShape,
    pub internal: NodeShape,
}

impl IndexShape {
    /// W a multiple of 8 in 8..=1,016, V in `V_CHOICES`, W + 8 + V ≤ `PART_MAX`; nL and nI from
    /// `node_shape(W + 8, 8, V, ·)`, so voff = 8 + c·(W + 8) and coff = voff + c·V.
    pub const fn new(w: u64, v: u64) -> Option<IndexShape> {
        if w < W_MIN || w > W_MAX || w % 8 != 0 || !crate::consts::v_choice(v) {
            return None;
        }
        // w ≤ 1,016 and v ≤ 256.
        let ks = w.wrapping_add(8);
        if ks.wrapping_add(v) > PART_MAX {
            return None;
        }
        Some(IndexShape { w, v, leaf: index_node_shape(ks, v, false), internal: index_node_shape(ks, v, true) })
    }

    /// The layout word of this shape's nodes: `form_code` of its (W, V), which `new` checked, made without
    /// checking them again.
    #[inline(always)]
    pub const fn layout(&self) -> u32 {
        // W + 8 ≤ 1,024 and V ≤ 256 in a shape `new` made; masked to their fields anyway.
        #[allow(clippy::cast_possible_truncation)]
        let code = 0x8000_0000
            | ((crate::consts::FORMAT_NODE as u32) << 24)
            | (((self.v & 0xFFF) as u32) << 12)
            | ((self.w.wrapping_add(8) & 0xFFF) as u32);
        code
    }
}

impl Shape for IndexShape {
    const TYPED: bool = false;
    const CHECKED: bool = true;
    #[inline]
    fn ks(self) -> u64 {
        self.w.wrapping_add(8)
    }
    #[inline]
    fn vs(self) -> u64 {
        self.v
    }
    #[inline]
    fn shape(self, leaf: bool) -> NodeShape {
        if leaf {
            self.leaf
        } else {
            self.internal
        }
    }
    #[inline]
    fn flags(self, leaf: bool) -> u16 {
        if leaf {
            FLAG_INDEX | FLAG_LEAF
        } else {
            FLAG_INDEX
        }
    }
}
