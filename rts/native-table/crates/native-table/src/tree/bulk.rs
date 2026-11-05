//! The canonical bottom-up build of one index-form tree inside a run image, with value parts and node
//! trailers.
//!
//! A run image is laid out in 4,096-byte slots: slot 0 holds the run header, slot k ≥ 1 holds one node (4,080
//! bytes) followed by its 16-byte trailer. `BulkState` builds one tree from n ≥ 1 entries given in strictly
//! increasing order, n known in advance, writing every byte of every slot it owns exactly once through a
//! `RunSink`. A run's main tree starts at slot 1; its count tree, a second `BulkState` over the same sink,
//! starts at the slot after the main tree's last. Slots are assigned in pre-order, so a tree's root is its
//! first slot. The bytes are a function of the entries and the shape only.
//!
//! **The plan.** The leaves are L0 = ⌈(n + 1)/(nL + 1)⌉ nodes; when L0 = 1 one leaf holding all n entries is
//! the root. Otherwise the leaves hold n − (L0 − 1) entries (the other L0 − 1 are separators one level up):
//! every leaf nL except the last, which gets the rest r; when r < ⌊nL/2⌋ the last two leaves share nL + r
//! instead, ⌈(nL + r)/2⌉ then ⌊(nL + r)/2⌋. A level over C children has ⌈C/(nI + 1)⌉ nodes: one node is the
//! root with C − 1 entries; otherwise the level holds C − L entries filled the same way with nI, and node j
//! has its count + 1 consecutive children. Every non-root node then holds at least half its capacity
//! (rounded down) and all leaves lie at one depth.
//!
//! **Streaming.** Entering a node assigns it the next slot, writes its header (its planned count, the
//! flags, the layout word), writes its trailer's cumulative count (the sum of the contributions of every
//! entry pushed before it, all of which lie before its subtree), and writes its reference into its parent's
//! next child word. A push places the entry in the current leaf while it has room; otherwise it completes
//! the leaf, climbs to the first level whose node still has an entry to place, places the entry there as a
//! separator and enters the next node of every level below. Completing a node zeroes its unused bytes and
//! writes its trailer's flags word (bit 0, UNIT: every entry of the node contributes exactly 1).
//!
//! Overflow records are written by the run builder, not here: a pushed entry whose key is longer than W
//! carries `F_OVF` with its record number, and only its first W key bytes reach the node.

use crate::consts::{BLOCK, FLAG_INDEX, FLAG_LEAF, NODE_SIZE, PATH_STACK};
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::key::probe::{is_ovf, payload, PART};
use crate::mem::{copy_bytes, put, put_u16, put_u32};
use crate::tree::shape::{IndexShape, NodeShape, Shape};

// PATH_STACK = 64.
#[allow(clippy::cast_possible_truncation)]
const FRAMES: usize = PATH_STACK as usize;

/// Offset of a node's trailer in its slot: the cumulative count (i64), then the flags word (u64).
pub const TRAILER: u64 = NODE_SIZE;
/// Trailer flags bit 0: every entry of the node contributes exactly 1.
pub const TRAILER_UNIT: u64 = 1;

/// Where a bulk build's bytes go: an output image through a page host, or the builder's memory. The context
/// is passed per call, so a sink never holds a borrow of the host while the caller reads its sources.
pub trait RunSink {
    type Ctx: ?Sized;
    /// Writes `bytes` at image offset `offset`.
    fn put(&mut self, cx: &mut Self::Ctx, offset: u64, bytes: &[u8]) -> Result<(), Fault>;
    /// Writes `len` zero bytes at image offset `offset`.
    fn zeros(&mut self, cx: &mut Self::Ctx, offset: u64, len: u64) -> Result<(), Fault>;
    /// Writes one entry into a node slot: at `koff` its key part, the key's first W bytes (`key`, zero-padded
    /// to `w` bytes) then its field's 8 big-endian bytes `fbe`; at `voff` its value part, `value` zero-padded to
    /// `v` bytes (none when v = 0). Both lie inside one 4,096-byte slot. By default, one `put` of each part.
    #[allow(clippy::too_many_arguments)]
    fn put_entry(
        &mut self,
        cx: &mut Self::Ctx,
        koff: u64,
        key: &[u8],
        w: u64,
        fbe: [u8; 8],
        voff: u64,
        value: &[u8],
        v: u64,
    ) -> Result<(), Fault> {
        let wu = usize::try_from(w).map_err(|_| rt(Corruption::Layout, w))?;
        let ksu = wu.checked_add(8).ok_or(rt(Corruption::Layout, w))?;
        let take = key.len().min(wu);
        // The key part, in a 64-byte buffer when W + 8 is at most 64.
        let mut small = [0u8; 64];
        let mut big;
        let kpart: &mut [u8] = if ksu <= 64 {
            &mut small
        } else {
            big = [0u8; PART];
            &mut big
        };
        copy_bytes(kpart.get_mut(..take).unwrap_or(&mut []), key.get(..take).unwrap_or(&[]));
        put(kpart, w, &fbe)?;
        self.put(cx, koff, kpart.get(..ksu).unwrap_or(&[]))?;
        if v > 0 {
            let vu = usize::try_from(v).map_err(|_| rt(Corruption::Layout, v))?;
            if value.len() == vu {
                // A full-width value goes to the sink as it is.
                self.put(cx, voff, value)?;
            } else {
                let mut vpart = [0u8; 256];
                copy_bytes(vpart.get_mut(..value.len()).unwrap_or(&mut []), value);
                self.put(cx, voff, vpart.get(..vu).unwrap_or(&[]))?;
            }
        }
        Ok(())
    }
}

/// A sink into one contiguous image in memory: the context is the image's bytes.
#[derive(Clone, Copy, Debug, Default)]
pub struct MemRunSink;

impl RunSink for MemRunSink {
    type Ctx = [u8];
    fn put(&mut self, cx: &mut [u8], offset: u64, bytes: &[u8]) -> Result<(), Fault> {
        put(cx, offset, bytes)
    }
    fn zeros(&mut self, cx: &mut [u8], offset: u64, len: u64) -> Result<(), Fault> {
        crate::mem::fill(cx, offset, len, 0)
    }
}

/// Where one tree lies in its image: its first node slot, the overflow records pushed before it, and the
/// image's node slots and overflow records over all its trees.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct RunGeometry {
    pub first_slot: u64,
    pub first_overflow: u64,
    pub slots: u64,
    pub v: u64,
}

/// One level's open node: its slot, its ordinal j at its level, the entries and children placed so far,
/// its planned entry count, and whether every entry placed so far contributed exactly 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Level {
    pub slot: u64,
    pub ordinal: u64,
    pub entries: u16,
    pub children: u16,
    pub target: u16,
    pub unit: bool,
}

/// One level of a plan: its node count, its entries in all, the capacity c of its nodes, the count of its
/// last node before the last-two rule, and whether the last two nodes share c + last.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PlanLevel {
    pub nodes: u64,
    pub entries: u64,
    pub cap: u64,
    pub last: u64,
    pub last_two: bool,
}

/// The node plan of a tree of n entries with leaf capacity nL and internal capacity nI. Level 0 is the
/// leaves, level `height − 1` the root. It holds at most 64 levels, a path's frames; a tree that needs more
/// is `PathOverflow`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    pub n: u64,
    pub height: u64,
    pub levels: [PlanLevel; FRAMES],
}

fn rt(kind: Corruption, off: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, off)
}

fn add(a: u64, b: u64) -> Result<u64, Fault> {
    a.checked_add(b).ok_or(rt(Corruption::Count, a))
}

fn sub(a: u64, b: u64) -> Result<u64, Fault> {
    a.checked_sub(b).ok_or(rt(Corruption::Count, a))
}

fn mul(a: u64, b: u64) -> Result<u64, Fault> {
    a.checked_mul(b).ok_or(rt(Corruption::Count, a))
}

/// ⌈a / b⌉ for b ≥ 1.
fn ceil_div(a: u64, b: u64) -> Result<u64, Fault> {
    let q = a.checked_div(b).ok_or(rt(Corruption::Count, a))?;
    let r = a.checked_rem(b).ok_or(rt(Corruption::Count, a))?;
    if r == 0 {
        Ok(q)
    } else {
        add(q, 1)
    }
}

/// A non-root level of `entries` entries over `nodes` nodes of capacity `cap`.
fn fill_level(nodes: u64, entries: u64, cap: u64) -> Result<PlanLevel, Fault> {
    let full = mul(sub(nodes, 1)?, cap)?;
    let last = sub(entries, full)?;
    if last > cap {
        return Err(rt(Corruption::Count, entries));
    }
    Ok(PlanLevel { nodes, entries, cap, last, last_two: last < cap / 2 })
}

impl Plan {
    /// The plan of n ≥ 1 entries; `PathOverflow` when it needs more than 64 levels.
    pub fn new(n: u64, nl: u64, ni: u64) -> Result<Plan, Fault> {
        if n == 0 || nl < 2 || ni < 2 {
            return Err(rt(Corruption::Count, n));
        }
        let mut p = Plan { n, height: 0, levels: [PlanLevel::default(); FRAMES] };
        let l0 = ceil_div(add(n, 1)?, add(nl, 1)?)?;
        if l0 == 1 {
            *p.levels.get_mut(0).ok_or(Fault::PathOverflow)? =
                PlanLevel { nodes: 1, entries: n, cap: nl, last: n, last_two: false };
            p.height = 1;
            return Ok(p);
        }
        let leaves = fill_level(l0, sub(n, sub(l0, 1)?)?, nl)?;
        *p.levels.get_mut(0).ok_or(Fault::PathOverflow)? = leaves;
        let mut c = l0;
        let mut level = 1u64;
        loop {
            let slot = usize::try_from(level).ok().and_then(|i| p.levels.get_mut(i)).ok_or(Fault::PathOverflow)?;
            let nodes = ceil_div(c, add(ni, 1)?)?;
            if nodes == 1 {
                let e = sub(c, 1)?;
                *slot = PlanLevel { nodes: 1, entries: e, cap: ni, last: e, last_two: false };
                p.height = add(level, 1)?;
                return Ok(p);
            }
            *slot = fill_level(nodes, sub(c, nodes)?, ni)?;
            c = nodes;
            level = add(level, 1)?;
        }
    }

    /// The level record of level ℓ.
    pub fn level(&self, level: u64) -> Option<&PlanLevel> {
        usize::try_from(level).ok().and_then(|i| self.levels.get(i))
    }

    /// The planned entry count of node j of level ℓ; 0 outside the plan.
    pub fn count(&self, level: u64, j: u64) -> u64 {
        let Some(l) = self.level(level) else { return 0 };
        if j >= l.nodes {
            return 0;
        }
        if l.nodes == 1 {
            return l.entries;
        }
        // nodes ≥ 2 here, and cap and last are at most 509.
        let last_j = l.nodes.wrapping_sub(1);
        if l.last_two {
            let both = l.cap.wrapping_add(l.last);
            if j == last_j {
                both / 2
            } else if j == last_j.wrapping_sub(1) {
                both.div_ceil(2)
            } else {
                l.cap
            }
        } else if j == last_j {
            l.last
        } else {
            l.cap
        }
    }

    /// Nodes over all levels.
    pub fn nodes(&self) -> u64 {
        let mut t = 0u64;
        for l in self.levels.iter().take(usize::try_from(self.height).unwrap_or(0)) {
            t = t.saturating_add(l.nodes);
        }
        t
    }
}

/// The node count and height of the plan of n ≥ 1 entries, level by level as `Plan::new` makes them but
/// without its level records.
pub fn plan_shape(n: u64, nl: u64, ni: u64) -> Result<(u64, u64), Fault> {
    if n == 0 || nl < 2 || ni < 2 {
        return Err(rt(Corruption::Count, n));
    }
    let mut c = ceil_div(add(n, 1)?, add(nl, 1)?)?;
    let mut nodes = c;
    let mut height = 1u64;
    while c > 1 {
        if height >= PATH_STACK {
            return Err(Fault::PathOverflow);
        }
        c = ceil_div(c, add(ni, 1)?)?;
        nodes = add(nodes, c)?;
        height = add(height, 1)?;
    }
    Ok((nodes, height))
}

/// The node count of the plan of n entries; 0 for n = 0.
pub fn plan_nodes(n: u64, nl: u64, ni: u64) -> Result<u64, Fault> {
    if n == 0 {
        return Ok(0);
    }
    Ok(plan_shape(n, nl, ni)?.0)
}

/// The state of one tree's bulk build. It holds at most 64 levels, a path's frames.
#[derive(Clone)]
pub struct BulkState {
    pub shape: IndexShape,
    pub n: u64,
    pub geo: RunGeometry,
    pub pushed: u64,
    pub overflowed: u64,
    pub next_slot: u64,
    pub height: u8,
    pub cumulative: i64,
    pub levels: [Level; FRAMES],
    plan: Plan,
    /// Where the last entry pushed since the state was made lies: its node's slot (0: none), its
    /// index in the node, and whether the node is a leaf.
    pub last: (u64, u16, bool),
}

/// The image offset of slot k.
fn slot_off(slot: u64) -> Result<u64, Fault> {
    mul(slot, BLOCK)
}

impl BulkState {
    /// Refuses n = 0, first_slot = 0, slots < first_slot − 1 + this tree's planned nodes, and
    /// first_overflow > v; `PathOverflow` when the tree needs more than 64 levels.
    pub fn new(shape: IndexShape, n: u64, geo: RunGeometry) -> Result<BulkState, Fault> {
        let plan = Plan::new(n, shape.leaf.cap, shape.internal.cap)?;
        if geo.first_slot == 0 || geo.first_overflow > geo.v {
            return Err(rt(Corruption::Header, geo.first_slot));
        }
        let need = add(sub(geo.first_slot, 1)?, plan.nodes())?;
        if geo.slots < need {
            return Err(rt(Corruption::Header, geo.slots));
        }
        let height = u8::try_from(plan.height).map_err(|_| Fault::PathOverflow)?;
        Ok(BulkState {
            shape,
            n,
            geo,
            pushed: 0,
            overflowed: 0,
            next_slot: geo.first_slot,
            height,
            cumulative: 0,
            levels: [Level::default(); FRAMES],
            plan,
            last: (0, 0, false),
        })
    }

    /// The plan this build follows.
    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    fn level(&self, l: u64) -> Result<Level, Fault> {
        usize::try_from(l).ok().and_then(|i| self.levels.get(i)).copied().ok_or(rt(Corruption::Depth, l))
    }

    fn level_mut(&mut self, l: u64) -> Result<&mut Level, Fault> {
        usize::try_from(l).ok().and_then(|i| self.levels.get_mut(i)).ok_or(rt(Corruption::Depth, l))
    }

    fn node_shape(&self, l: u64) -> NodeShape {
        self.shape.shape(l == 0)
    }

    /// Enters node `ordinal` of level ℓ.
    fn enter<K: RunSink + ?Sized>(&mut self, sink: &mut K, cx: &mut K::Ctx, l: u64, ordinal: u64) -> Result<(), Fault> {
        let slot = self.next_slot;
        self.next_slot = add(slot, 1)?;
        let target = self.plan.count(l, ordinal);
        let target16 = u16::try_from(target).map_err(|_| rt(Corruption::Count, target))?;
        *self.level_mut(l)? = Level { slot, ordinal, entries: 0, children: 0, target: target16, unit: true };
        let base = slot_off(slot)?;
        let mut head = [0u8; 8];
        put_u16(&mut head, 0, target16)?;
        put_u16(&mut head, 2, if l == 0 { FLAG_INDEX | FLAG_LEAF } else { FLAG_INDEX })?;
        put_u32(&mut head, 4, self.shape.layout())?;
        sink.put(cx, base, &head)?;
        sink.put(cx, add(base, TRAILER)?, &self.cumulative.to_le_bytes())?;
        let top = u64::from(self.height).saturating_sub(1);
        if l < top {
            let pl = add(l, 1)?;
            let parent = self.level(pl)?;
            let coff = self.shape.internal.coff;
            let at = add(add(slot_off(parent.slot)?, coff)?, mul(u64::from(parent.children), 8)?)?;
            if u64::from(parent.children) > u64::from(parent.target) {
                return Err(rt(Corruption::Count, at));
            }
            sink.put(cx, at, &base.to_le_bytes())?;
            let p = self.level_mut(pl)?;
            p.children = p.children.checked_add(1).ok_or(rt(Corruption::Count, at))?;
        }
        Ok(())
    }

    /// Writes entry `x` as the next entry of the open node of level ℓ.
    #[allow(clippy::too_many_arguments)]
    fn place<K: RunSink + ?Sized>(
        &mut self,
        sink: &mut K,
        cx: &mut K::Ctx,
        l: u64,
        key: &[u8],
        field: u64,
        value: &[u8],
        contribution: i64,
    ) -> Result<(), Fault> {
        self.place_bytes(sink, cx, l, key, field.to_be_bytes(), value, contribution)
    }

    /// `place` of an entry whose field is given as its 8 big-endian bytes.
    #[allow(clippy::too_many_arguments)]
    fn place_bytes<K: RunSink + ?Sized>(
        &mut self,
        sink: &mut K,
        cx: &mut K::Ctx,
        l: u64,
        key: &[u8],
        fbe: [u8; 8],
        value: &[u8],
        contribution: i64,
    ) -> Result<(), Fault> {
        let node = self.level(l)?;
        let sh = self.node_shape(l);
        let w = self.shape.w;
        let ks = add(w, 8)?;
        let v = self.shape.v;
        let i = u64::from(node.entries);
        let base = slot_off(node.slot)?;
        if u64::try_from(value.len()).map_err(|_| rt(Corruption::Layout, v))? > v {
            return Err(rt(Corruption::Layout, v));
        }
        // The key part: the key area (the key's first W bytes, zero-padded), then the field, big-endian; then
        // the value part, zero-padded. i < 65,536 entries, W + 8 ≤ 1,032 and V ≤ 256: the products are below
        // 2^27.
        let koff = add(add(base, 8)?, i.wrapping_mul(ks))?;
        let voff = add(add(base, sh.voff)?, i.wrapping_mul(v))?;
        sink.put_entry(cx, koff, key, w, fbe, voff, value, v)?;
        self.cumulative = self.cumulative.checked_add(contribution).ok_or(rt(Corruption::Count, base))?;
        self.last = (node.slot, node.entries, l == 0);
        let n = self.level_mut(l)?;
        n.entries = n.entries.checked_add(1).ok_or(rt(Corruption::Count, base))?;
        if contribution != 1 {
            n.unit = false;
        }
        Ok(())
    }

    /// Completes the open node of level ℓ: zeroes its unused bytes and writes its trailer's flags.
    fn complete<K: RunSink + ?Sized>(&mut self, sink: &mut K, cx: &mut K::Ctx, l: u64) -> Result<(), Fault> {
        let node = self.level(l)?;
        let base = slot_off(node.slot)?;
        if node.entries != node.target || (l > 0 && u64::from(node.children) != add(u64::from(node.entries), 1)?) {
            return Err(rt(Corruption::Count, base));
        }
        let sh = self.node_shape(l);
        let count = u64::from(node.entries);
        let kend = add(8, mul(count, add(self.shape.w, 8)?)?)?;
        let vend = add(sh.voff, mul(count, self.shape.v)?)?;
        let gap = |sink: &mut K, cx: &mut K::Ctx, from: u64, to: u64| -> Result<(), Fault> {
            if to > from {
                sink.zeros(cx, add(base, from)?, to.wrapping_sub(from))?;
            }
            Ok(())
        };
        gap(sink, cx, kend, sh.voff)?;
        if l == 0 {
            gap(sink, cx, vend, NODE_SIZE)?;
        } else {
            gap(sink, cx, vend, sh.coff)?;
            gap(sink, cx, add(sh.coff, mul(add(count, 1)?, 8)?)?, NODE_SIZE)?;
        }
        let flags = if node.unit { TRAILER_UNIT } else { 0 };
        sink.put(cx, add(base, add(TRAILER, 8)?)?, &flags.to_le_bytes())
    }

    /// Pushes the next entry. `field`: the entry's field as the run stores it (`F_OVF` | its record number
    /// for a key longer than W); `key`: its full encoding, or its W-byte key area when it fits W; `value`:
    /// its value part (at most V bytes, zero-padded); `contribution`: what the entry adds to counts. The
    /// caller has already checked that the entry is greater than the previous one.
    #[allow(clippy::too_many_arguments)]
    pub fn push<K: RunSink + ?Sized>(
        &mut self,
        sink: &mut K,
        cx: &mut K::Ctx,
        key: &[u8],
        field: u64,
        value: &[u8],
        contribution: i64,
    ) -> Result<(), Fault> {
        if self.pushed >= self.n {
            return Err(rt(Corruption::Count, self.pushed));
        }
        let long = u64::try_from(key.len()).map_err(|_| rt(Corruption::Encoding, 0))? > self.shape.w;
        if long != is_ovf(field) {
            return Err(rt(Corruption::Encoding, self.pushed));
        }
        if long && payload(field) != add(self.geo.first_overflow, self.overflowed)? {
            return Err(rt(Corruption::Encoding, self.pushed));
        }
        if self.pushed == 0 {
            // Start: enter the root, then child 0 of every level down to leaf 0.
            let mut l = u64::from(self.height);
            while l > 0 {
                l = l.wrapping_sub(1);
                self.enter(sink, cx, l, 0)?;
            }
        }
        let leaf = self.level(0)?;
        if leaf.entries < leaf.target {
            self.place(sink, cx, 0, key, field, value, contribution)?;
        } else {
            self.complete(sink, cx, 0)?;
            let mut l = 1u64;
            loop {
                if l >= u64::from(self.height) {
                    return Err(rt(Corruption::Count, self.pushed));
                }
                let node = self.level(l)?;
                if node.entries < node.target {
                    self.place(sink, cx, l, key, field, value, contribution)?;
                    let mut k = l;
                    while k > 0 {
                        k = k.wrapping_sub(1);
                        let next = add(self.level(k)?.ordinal, 1)?;
                        self.enter(sink, cx, k, next)?;
                    }
                    break;
                }
                self.complete(sink, cx, l)?;
                l = add(l, 1)?;
            }
        }
        self.pushed = add(self.pushed, 1)?;
        if long {
            self.overflowed = add(self.overflowed, 1)?;
        }
        Ok(())
    }

    /// Completes every open node bottom-up once all n entries are pushed; returns the root's image offset,
    /// 4,096 · first_slot.
    pub fn finish<K: RunSink + ?Sized>(&mut self, sink: &mut K, cx: &mut K::Ctx) -> Result<u64, Fault> {
        if self.pushed != self.n {
            return Err(rt(Corruption::Count, self.pushed));
        }
        let mut l = 0u64;
        while l < u64::from(self.height) {
            self.complete(sink, cx, l)?;
            l = add(l, 1)?;
        }
        slot_off(self.geo.first_slot)
    }
}
