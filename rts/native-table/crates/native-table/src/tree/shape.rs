//! Node shapes: capacities and offsets of a tree's leaves and internal nodes, for the index form (a W + 8-byte
//! key part and a V-byte value part). A *counted* tree's internal nodes also keep the row entries under each
//! child: a u64 per child at toff = coff + 8·(c + 1), after the children.

use crate::consts::{NODE_SIZE, PART_MAX, W_MAX, W_MIN};

/// A node's capacity c, the offset of its value parts and, for an internal node, of its children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeShape {
    pub cap: u64,
    pub voff: u64,
    pub coff: u64,
}

/// Offsets and capacities of one tree's nodes.
pub trait Shape: Copy {
    /// Key-part bytes; index form: W + 8.
    fn ks(self) -> u64;
    /// Value-part bytes; index form: V.
    fn vs(self) -> u64;
    /// (nL, voff, coff) or (nI, voff, coff).
    fn shape(self, leaf: bool) -> NodeShape;
}

/// The shape of an index node, whose key part ks and value part v are multiples of 8, so no offset needs
/// alignment: a leaf fits c entries while 8 + c·(ks + v) ≤ 4,080, an internal node while
/// 8 + c·(ks + v) + b·(c + 1) ≤ 4,080, with b = 8 bytes a child, 16 in a counted tree. So nL = ⌊4,072 / (ks + v)⌋
/// and nI = ⌊(4,072 − b) / (ks + v + b)⌋, with voff = 8 + c·ks and coff = voff + c·v.
pub const fn index_node_shape(ks: u64, v: u64, internal: bool, counted: bool) -> NodeShape {
    // ks ≤ 1,024 and v ≤ 256: every product below stays under 2^20.
    let b = if internal && counted {
        16
    } else if internal {
        8
    } else {
        0
    };
    let per = ks.wrapping_add(v).wrapping_add(b);
    let room = NODE_SIZE.wrapping_sub(8).wrapping_sub(b);
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
    /// `index_node_shape(W + 8, V, ·, false)`, so voff = 8 + c·(W + 8) and coff = voff + c·V. Inlined, so that a
    /// shape read from an index's descriptor is worked out only as far as its reader uses it.
    #[inline(always)]
    pub const fn new(w: u64, v: u64) -> Option<IndexShape> {
        Self::with(w, v, false)
    }

    /// As `new(w, 0)`, for a counted tree, whose internal nodes keep the row entries under each child (its node
    /// host's `NodeMem::COUNTED`): a secondary index's tree.
    #[inline(always)]
    pub const fn counted(w: u64) -> Option<IndexShape> {
        Self::with(w, 0, true)
    }

    #[inline(always)]
    const fn with(w: u64, v: u64, counted: bool) -> Option<IndexShape> {
        if w < W_MIN || w > W_MAX || w % 8 != 0 || !crate::consts::v_choice(v) {
            return None;
        }
        // w ≤ 1,016 and v ≤ 256.
        let ks = w.wrapping_add(8);
        if ks.wrapping_add(v) > PART_MAX {
            return None;
        }
        Some(IndexShape {
            w,
            v,
            leaf: index_node_shape(ks, v, false, false),
            internal: index_node_shape(ks, v, true, counted),
        })
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
}
