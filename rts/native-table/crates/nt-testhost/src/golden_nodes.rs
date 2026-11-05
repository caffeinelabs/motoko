//! The golden typed nodes: the bytes of `PBTreeMap`'s nodes, stale copies past `count` included, after
//! chosen steps of fixed scripts, with the log of the nodes each step allocates and hands back. This file
//! holds the scripts, their keys and values, the normalization of a node image and the text form of a
//! script's images and log.
//!
//! One definition serves both sides. The crate's side (`nt-testhost`, test `golden_nodes`) runs the scripts
//! on the crate's tree over the typed test host; the reference side (`nt-eopref`, its own workspace) includes
//! this file by path and runs them on today's `PBTreeMap` in the host mode of Rust EOP (a Rust library for
//! upgrades that keep the heap in place). Each side observes its
//! own run (raw node bytes, which bytes were written, which node references were allocated and handed back)
//! and hands the observations to [`Writer`], so equal observations give byte-identical text. This file
//! depends on `std` only.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// Bytes of a node.
pub const NODE: usize = 4_080;
/// Bytes of a node image line.
pub const LINE: usize = 32;

// ---------------------------------------------------------------------------------------------------------
// Shapes, keys and values.

/// The map types of the scripts: `<u64, u64>`, `<[u8; 16], u64>` and `<[u8; 1012], u64>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    U64,
    A16,
    A1012,
}

/// Capacity and offsets of a node: `PBTreeMap`'s node rules (voff = align_up(8 + c·ks, 8),
/// coff = align_up(voff + 8c, 8); a leaf holds the largest c with voff + 8c ≤ 4,080, an internal node the
/// largest c with coff + 8(c + 1) ≤ 4,080), recomputed here for u64 values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    pub cap: usize,
    pub voff: usize,
    pub coff: usize,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::U64, Kind::A16, Kind::A1012];

    /// Bytes of a key part.
    pub fn ks(self) -> usize {
        match self {
            Kind::U64 => 8,
            Kind::A16 => 16,
            Kind::A1012 => 1_012,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::U64 => "<u64, u64>",
            Kind::A16 => "<[u8; 16], u64>",
            Kind::A1012 => "<[u8; 1012], u64>",
        }
    }

    pub fn geometry(self, leaf: bool) -> Geometry {
        let ks = self.ks();
        let up8 = |a: usize| (a + 7) & !7;
        let offsets = |c: usize| {
            let voff = up8(8 + c * ks);
            (voff, up8(voff + 8 * c))
        };
        let fits = |c: usize| {
            let (voff, coff) = offsets(c);
            if leaf {
                voff + 8 * c <= NODE
            } else {
                coff + 8 * (c + 1) <= NODE
            }
        };
        let mut c = 0;
        while fits(c + 1) {
            c += 1;
        }
        let (voff, coff) = offsets(c);
        Geometry { cap: c, voff, coff }
    }
}

/// Digit d < 254 as a key or value byte: the bytes 0x01..=0xFF but 0xA5, in increasing order. No key or
/// value byte is 0x00, which a zeroed node holds where nothing was written, or 0xA5, a fill pattern and Rust
/// EOP's host-mode poison.
pub fn byte_of(d: u64) -> u8 {
    let b = (d % 254) + 1;
    if b >= 0xA5 {
        (b + 1) as u8
    } else {
        b as u8
    }
}

/// The base-254 digit i of `label`.
fn digit(label: u64, i: u32) -> u64 {
    (label / 254u64.pow(i)) % 254
}

/// The u64 key of `label`: its base-254 digits as bytes, least significant first. Increasing in `label`.
pub fn key_u64(label: u64) -> u64 {
    let mut b = [0u8; 8];
    for (i, x) in b.iter_mut().enumerate() {
        *x = byte_of(digit(label, i as u32));
    }
    u64::from_le_bytes(b)
}

/// The array key of `label`: its eight base-254 digits as bytes, most significant first (so byte order is
/// label order), then bytes made from the label and the position, so that every key byte is checked.
pub fn key_array(label: u64, n: usize) -> Vec<u8> {
    let mut b = vec![0u8; n];
    for (i, x) in b.iter_mut().take(8).enumerate() {
        *x = byte_of(digit(label, 7 - i as u32));
    }
    for (j, x) in b.iter_mut().enumerate().skip(8) {
        *x = byte_of(label.wrapping_mul(131).wrapping_add((j as u64).wrapping_mul(37)));
    }
    b
}

/// The value of `label`: eight bytes of a mix of the label, none 0x00 or 0xA5.
pub fn value(label: u64) -> u64 {
    let mut z = label.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    let mut b = [0u8; 8];
    for (i, x) in b.iter_mut().enumerate() {
        *x = byte_of((z >> (8 * i)) & 0xFF);
    }
    u64::from_le_bytes(b)
}

// ---------------------------------------------------------------------------------------------------------
// Scripts.

/// One operation of a script, on a key named by its label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// Insert the label's key with the label's value; the key is absent.
    Insert(u64),
    /// Remove the label's key; the key is present.
    Remove(u64),
}

/// A step: an operation and, for an imaged step, the cases it exercises (the crate's side checks them).
#[derive(Clone, Debug)]
pub struct Step {
    pub op: Op,
    pub cases: Option<&'static str>,
}

/// A script: an empty map of `kind`, an insert phase, then a remove phase, so that it makes all its
/// allocations before its first hand-back and each node reference names exactly one allocation.
#[derive(Clone, Debug)]
pub struct Script {
    pub name: &'static str,
    pub kind: Kind,
    pub steps: Vec<Step>,
}

impl Script {
    fn new(name: &'static str, kind: Kind) -> Self {
        Script { name, kind, steps: Vec::new() }
    }
    fn ins(&mut self, label: u64) {
        self.steps.push(Step { op: Op::Insert(label), cases: None });
    }
    fn rem(&mut self, label: u64) {
        self.steps.push(Step { op: Op::Remove(label), cases: None });
    }
    /// Makes the step of `op` an imaged step that exercises `cases`.
    fn mark(&mut self, op: Op, cases: &'static str) {
        let mut it = self.steps.iter_mut().filter(|s| s.op == op);
        let st = it.next().unwrap_or_else(|| panic!("{}: no step {op:?}", self.name));
        assert!(it.next().is_none(), "{}: two steps {op:?}", self.name);
        st.cases = Some(cases);
    }

    /// Removes every key present, in ascending order.
    fn rem_all_ascending(&mut self) {
        for l in self.present_after_all() {
            self.rem(l);
        }
    }

    /// The labels present after the last step (the script's own model); panics on an insert of a present
    /// label or a removal of an absent one.
    pub fn present_after_all(&self) -> BTreeSet<u64> {
        let mut s = BTreeSet::new();
        for st in &self.steps {
            match st.op {
                Op::Insert(l) => assert!(s.insert(l), "{}: label {l} inserted twice", self.name),
                Op::Remove(l) => assert!(s.remove(&l), "{}: label {l} removed while absent", self.name),
            }
        }
        s
    }
}

/// The scripts of the golden typed nodes.
pub fn scripts() -> Vec<Script> {
    vec![u64_depth3(), a16_depth2(), c3_depth3()]
}

/// Label unit of the scripts of wide nodes: the gaps between multiples of `G` take later keys.
const G: u64 = 1 << 30;
/// Spacing of keys put into a gap, leaving gaps of their own.
const S: u64 = 1 << 16;

/// The label halfway between `l` and `l + 1` units.
const fn half(l: u64) -> u64 {
    l * G + G / 2
}

/// `<u64, u64>`: leaves of capacity 254 (h = 127, minimum 127), internal nodes of capacity 169 (h = 85,
/// minimum 84); depth 3.
fn u64_depth3() -> Script {
    let mut s = Script::new("u64_depth3", Kind::U64);
    // A leaf fills and splits at i > h; the root splits.
    for l in 1..=255 {
        s.ins(l * G);
    }
    // The right leaf fills from inside, then a key at its front splits it at i < h.
    for t in 1..=127 {
        s.ins(129 * G + t * S);
    }
    s.ins(half(128));
    // The rightmost leaf fills, then a key at its position 127 splits it at i = h.
    for l in 256..=382 {
        s.ins(l * G);
    }
    s.ins(half(255));
    // Ascending keys: a leaf split every 128 keys until the root, full, splits at i > h: depth 3.
    for l in 383..=21_758 {
        s.ins(l * G);
    }
    // Keys after the first leaf: a leaf split every 128 keys under the left internal node (85 keys), which
    // fills and splits at i = 84 < h.
    let g = |t: u64| 127 * G + t * S;
    for t in 1..=10_880 {
        s.ins(g(t));
    }
    // Keys after its second leaf: it fills again and splits at i = 85 = h.
    let hk = |u: u64| g(128) + u;
    for u in 1..=10_880 {
        s.ins(hk(u));
    }
    // The root now has four internal children, of 85, 84, 84 and 84 keys. Two leaf splits under the second
    // and two under the third give them 86 keys, for the rotations at the internal level; one more key gives
    // the third one's last leaf 128 entries, so that removing the root key above it moves a predecessor
    // without an underflow.
    for v in 1..=256 {
        s.ins(g(10_880) + v);
    }
    for v in 1..=257 {
        s.ins(10_878 * G + v);
    }
    // Leaves of 128 entries: beside a leaf that rotates from the left, beside one that rotates from the
    // right, and one that loses an entry without an underflow.
    s.ins(half(21_504));
    s.ins(half(200));
    s.ins(half(11_200));
    // The remove phase: each case once, then emptying the tree in ascending order.
    s.rem(half(11_200));
    s.rem(10_879 * G);
    s.rem(G);
    s.rem(hk(1_100));
    s.rem(10_880 * G);
    s.rem(hk(2_000));
    s.rem(21_700 * G);
    s.rem(half(128));
    s.rem_all_ascending();

    s.mark(Op::Insert(G), "first insert");
    s.mark(Op::Insert(2 * G), "leaf room");
    s.mark(Op::Insert(255 * G), "leaf split i>h, root split");
    s.mark(Op::Insert(half(128)), "leaf split i<h");
    s.mark(Op::Insert(half(255)), "leaf split i=h");
    s.mark(Op::Insert(21_758 * G), "leaf split i>h, internal split i>h, root split");
    s.mark(Op::Insert(g(10_880)), "leaf split i>h, internal split i<h");
    s.mark(Op::Insert(hk(10_880)), "leaf split i>h, internal split i=h");
    s.mark(Op::Remove(half(11_200)), "leaf remove");
    s.mark(Op::Remove(10_879 * G), "predecessor");
    s.mark(Op::Remove(G), "leaf merge");
    s.mark(Op::Remove(hk(1_100)), "leaf merge, internal rotation from the right");
    s.mark(Op::Remove(10_880 * G), "leaf merge, internal rotation from the left");
    s.mark(Op::Remove(hk(2_000)), "leaf merge, internal merge");
    s.mark(Op::Remove(21_700 * G), "leaf rotation from the left");
    s.mark(Op::Remove(half(128)), "leaf rotation from the right");
    s.mark(Op::Remove(130 * G), "leaf merge, internal merge, root collapse (internal)");
    s.mark(Op::Remove(21_504 * G), "leaf merge, root collapse (internal)");
    s.mark(Op::Remove(21_758 * G), "root collapse (leaf)");
    s
}

/// `<[u8; 16], u64>`: leaves of capacity 169 (h = 85, minimum 84) under a root of capacity 127; depth 2.
fn a16_depth2() -> Script {
    let mut s = Script::new("a16_depth2", Kind::A16);
    // A leaf fills and splits at i > h; the root splits.
    for l in 1..=170 {
        s.ins(l * G);
    }
    // The right leaf fills from inside, then a key at its front splits it at i < h.
    let g = |t: u64| 87 * G + t * S;
    for t in 1..=85 {
        s.ins(g(t));
    }
    s.ins(half(86));
    // The rightmost leaf fills, then a key at its position 85 splits it at i = h.
    for l in 171..=255 {
        s.ins(l * G);
    }
    s.ins(half(171));
    // Leaves of 86 and 87 entries beside the leaves that rotate.
    s.ins(half(100));
    s.ins(g(1) + 1);
    s.ins(g(1) + 2);
    // The remove phase: each case once, then emptying the tree in ascending order.
    s.rem(G);
    s.rem(g(84));
    s.rem(200 * G);
    s.rem(2 * G);
    s.rem(201 * G);
    s.rem_all_ascending();

    s.mark(Op::Insert(G), "first insert");
    s.mark(Op::Insert(2 * G), "leaf room");
    s.mark(Op::Insert(170 * G), "leaf split i>h, root split");
    s.mark(Op::Insert(half(86)), "leaf split i<h");
    s.mark(Op::Insert(half(171)), "leaf split i=h");
    s.mark(Op::Remove(G), "leaf remove");
    s.mark(Op::Remove(g(84)), "predecessor");
    s.mark(Op::Remove(200 * G), "leaf rotation from the left");
    s.mark(Op::Remove(2 * G), "leaf rotation from the right");
    s.mark(Op::Remove(201 * G), "leaf merge");
    s.mark(Op::Remove(g(83)), "leaf merge, root collapse (internal)");
    s.mark(Op::Remove(255 * G), "root collapse (leaf)");
    s
}

/// Labels in thousandths, so that later keys fit between earlier ones.
const fn k(x: u64) -> u64 {
    x * 1_000
}

/// `<[u8; 1012], u64>`: leaves and internal nodes of capacity 3 (h = 2, minimum 1); depth 3.
fn c3_depth3() -> Script {
    let mut s = Script::new("c3_depth3", Kind::A1012);
    let groups: [&[u64]; 4] = [
        // Depth 1, then 2, then the root full with three keys.
        &[100, 200, 300, 400, 150, 50, 250, 275, 260],
        // A split leaf under the full root: the root splits at i = h, depth 3.
        &[280, 290, 295],
        // Internal splits at i < h and at i > h.
        &[285, 287, 75, 60, 500, 600, 700, 800, 900, 1_000, 1_100, 1_200, 1_300],
        // Full siblings for the rotations of the remove phase.
        &[55, 450, 850, 870, 288, 289],
    ];
    for x in groups.into_iter().flatten() {
        s.ins(k(*x));
    }
    for l in [289_500, 289_600, 289_700, 289_800] {
        s.ins(l);
    }
    for x in [1_100, 100, 295, 1_300, 75, 50, 200, 250, 289] {
        s.rem(k(x));
    }
    s.rem_all_ascending();

    s.mark(Op::Insert(k(100)), "first insert");
    s.mark(Op::Insert(k(200)), "leaf room");
    s.mark(Op::Insert(k(400)), "leaf split i>h, root split");
    s.mark(Op::Insert(k(50)), "leaf split i<h");
    s.mark(Op::Insert(k(260)), "leaf split i=h");
    s.mark(Op::Insert(k(295)), "leaf split i>h, internal split i=h, root split");
    s.mark(Op::Insert(k(60)), "leaf split i<h, internal split i<h");
    s.mark(Op::Insert(k(1300)), "leaf split i>h, internal split i>h");
    s.mark(Op::Remove(k(1100)), "leaf remove");
    s.mark(Op::Remove(k(100)), "leaf rotation from the left");
    s.mark(Op::Remove(k(295)), "leaf rotation from the right");
    s.mark(Op::Remove(k(1300)), "leaf merge, internal rotation from the left");
    s.mark(Op::Remove(k(75)), "leaf merge");
    s.mark(Op::Remove(k(250)), "leaf merge, internal rotation from the right");
    s.mark(Op::Remove(k(289)), "predecessor");
    s.mark(Op::Remove(k(150)), "leaf merge, internal merge");
    s.mark(Op::Remove(k(500)), "leaf merge, internal merge, root collapse (internal)");
    s.mark(Op::Remove(k(870)), "leaf merge, root collapse (internal)");
    s.mark(Op::Remove(k(1200)), "root collapse (leaf)");
    s
}

// ---------------------------------------------------------------------------------------------------------
// Observations, normalization and the text form.

/// A node image after normalization: 4,080 bytes and, per byte, whether it was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub ord: u64,
    pub bytes: Vec<u8>,
    pub written: Vec<bool>,
}

impl Image {
    pub fn leaf(&self) -> bool {
        self.bytes[2] == 1
    }
    pub fn count(&self) -> u64 {
        u64::from(u16::from_le_bytes([self.bytes[0], self.bytes[1]]))
    }
}

/// What one side observed of one step.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Observed {
    /// The nodes the step allocated, by allocation ordinal (1-based position among the script's `alloc`
    /// results), in allocation order, and whether each is a leaf.
    pub allocs: Vec<(u64, bool)>,
    /// The ordinals of the nodes the step handed back, as a set.
    pub hand_backs: BTreeSet<u64>,
    /// The root's ordinal, 0 for none, and the map's length.
    pub root: u64,
    pub len: u64,
    /// For an imaged step: every node reachable from the root, in pre-order, normalized.
    pub images: Option<Vec<Image>>,
}

/// Normalizes the raw bytes of a node, removing only what differs between hosts by construction. The header
/// must be written and its layout word equal the registered leaf or internal layout number; it becomes 1 or
/// 2. In an internal node every child slot, j = 0 … nI, must be written whole or not at all, and every
/// written non-zero child word becomes the allocation ordinal of the node it names (`ord_of`); a word that
/// names no allocation fails. Unwritten bytes become 0 in `bytes`, so that images compare by value; they
/// print as unwritten.
pub fn normalize(
    kind: Kind,
    raw: &[u8],
    written: &[bool],
    layouts: (u32, u32),
    ord: u64,
    ord_of: &dyn Fn(u64) -> Option<u64>,
) -> Result<Image, String> {
    if raw.len() != NODE || written.len() != NODE {
        return Err(format!("node {ord}: an image of {} bytes", raw.len()));
    }
    if !written[..8].iter().all(|&w| w) {
        return Err(format!("node {ord}: header bytes unwritten"));
    }
    let flags = u16::from_le_bytes([raw[2], raw[3]]);
    if flags > 1 {
        return Err(format!("node {ord}: flags {flags}"));
    }
    let leaf = flags == 1;
    let layout = u32::from_le_bytes(raw[4..8].try_into().unwrap());
    let want = if leaf { layouts.0 } else { layouts.1 };
    if layout != want {
        return Err(format!("node {ord}: layout {layout}, the registered number is {want}"));
    }
    let mut bytes = raw.to_vec();
    bytes[4..8].copy_from_slice(&(if leaf { 1u32 } else { 2u32 }).to_le_bytes());
    if !leaf {
        let g = kind.geometry(false);
        for j in 0..=g.cap {
            let at = g.coff + 8 * j;
            let slot = &written[at..at + 8];
            if !slot.iter().any(|&x| x) {
                continue;
            }
            if !slot.iter().all(|&x| x) {
                return Err(format!("node {ord}: child slot {j} partly written"));
            }
            let word = u64::from_le_bytes(raw[at..at + 8].try_into().unwrap());
            if word != 0 {
                let o = ord_of(word).ok_or_else(|| format!("node {ord}: child slot {j} holds {word:#x}, no alloc"))?;
                bytes[at..at + 8].copy_from_slice(&o.to_le_bytes());
            }
        }
    }
    // Unwritten bytes compare equal whatever they hold.
    for (b, &x) in bytes.iter_mut().zip(written) {
        if !x {
            *b = 0;
        }
    }
    Ok(Image { ord, bytes, written: written.to_vec() })
}

/// One image line: `oooo` (the offset, hex), a space, then two hex digits per written byte and `..` per
/// unwritten byte.
fn line(img_bytes: &[u8], written: &[bool], at: usize) -> String {
    let end = (at + LINE).min(NODE);
    let mut s = format!("{at:04x} ");
    for k in at..end {
        if written[k] {
            let _ = write!(s, "{:02x}", img_bytes[k]);
        } else {
            s.push_str("..");
        }
    }
    s
}

/// Builds a script's text: the header, then one block per step that allocated, handed back or was imaged.
pub struct Writer {
    out: String,
    last: BTreeMap<u64, (Vec<u8>, Vec<bool>)>,
}

impl Writer {
    pub fn new(script: &Script) -> Self {
        let mut out = String::new();
        let k = script.kind;
        let (l, i) = (k.geometry(true), k.geometry(false));
        let _ = writeln!(out, "# golden typed nodes, normalized; format: README.md");
        let _ = writeln!(out, "script {}", script.name);
        let _ = writeln!(
            out,
            "map {} ks {} vs 8 leaf cap {} voff {} internal cap {} voff {} coff {}",
            k.name(),
            k.ks(),
            l.cap,
            l.voff,
            i.cap,
            i.voff,
            i.coff
        );
        let _ = writeln!(out, "steps {}", script.steps.len());
        Writer { out, last: BTreeMap::new() }
    }

    /// Records step `index` (1-based).
    pub fn step(&mut self, index: usize, step: &Step, o: &Observed) {
        if o.allocs.is_empty() && o.hand_backs.is_empty() && o.images.is_none() {
            return;
        }
        let op = match step.op {
            Op::Insert(l) => format!("insert {l}"),
            Op::Remove(l) => format!("remove {l}"),
        };
        let _ = write!(self.out, "\nstep {index} {op}");
        if let Some(c) = step.cases {
            let _ = write!(self.out, " [{c}]");
        }
        self.out.push('\n');
        for &(n, leaf) in &o.allocs {
            let _ = writeln!(self.out, "alloc {n} {}", if leaf { "leaf" } else { "internal" });
        }
        for n in &o.hand_backs {
            let _ = writeln!(self.out, "hand_back {n}");
        }
        let _ = writeln!(self.out, "root {} len {}", o.root, o.len);
        let Some(images) = &o.images else { return };
        let _ = writeln!(self.out, "image {}", images.len());
        for img in images {
            let kind = if img.leaf() { "leaf" } else { "internal" };
            let head = format!("node {} {kind} count {}", img.ord, img.count());
            match self.last.get(&img.ord) {
                None => {
                    let _ = writeln!(self.out, "{head} full");
                    for at in (0..NODE).step_by(LINE) {
                        let end = (at + LINE).min(NODE);
                        if img.written[at..end].iter().any(|&x| x) {
                            let _ = writeln!(self.out, "{}", line(&img.bytes, &img.written, at));
                        }
                    }
                }
                Some((pb, pw)) if *pb == img.bytes && *pw == img.written => {
                    let _ = writeln!(self.out, "{head} same");
                }
                Some((pb, pw)) => {
                    let _ = writeln!(self.out, "{head} delta");
                    for at in (0..NODE).step_by(LINE) {
                        let end = (at + LINE).min(NODE);
                        if pb[at..end] != img.bytes[at..end] || pw[at..end] != img.written[at..end] {
                            let _ = writeln!(self.out, "{}", line(&img.bytes, &img.written, at));
                        }
                    }
                }
            }
            self.last.insert(img.ord, (img.bytes.clone(), img.written.clone()));
        }
    }

    pub fn finish(self) -> String {
        self.out
    }
}

/// A step's block read back from the text: its index, the observations, with the images reconstructed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parsed {
    pub index: usize,
    pub observed: Observed,
}

/// Reads a script's text back, reconstructing every image from `full`, `delta` and `same` blocks.
pub fn parse(text: &str) -> Result<Vec<Parsed>, String> {
    let mut out: Vec<Parsed> = Vec::new();
    let mut last: BTreeMap<u64, (Vec<u8>, Vec<bool>)> = BTreeMap::new();
    let mut cur: Option<(u64, Vec<u8>, Vec<bool>)> = None;
    let flush = |cur: &mut Option<(u64, Vec<u8>, Vec<bool>)>,
                 out: &mut Vec<Parsed>,
                 last: &mut BTreeMap<u64, (Vec<u8>, Vec<bool>)>| {
        if let Some((ord, b, w)) = cur.take() {
            last.insert(ord, (b.clone(), w.clone()));
            if let Some(p) = out.last_mut() {
                p.observed.images.get_or_insert_with(Vec::new).push(Image { ord, bytes: b, written: w });
            }
        }
    };
    for (no, l) in text.lines().enumerate() {
        let bad = || format!("line {}: {l:?}", no + 1);
        let mut f = l.split(' ');
        match f.next() {
            Some("step") => {
                flush(&mut cur, &mut out, &mut last);
                let index = f.next().and_then(|x| x.parse().ok()).ok_or_else(bad)?;
                out.push(Parsed { index, observed: Observed::default() });
            }
            Some("alloc") => {
                let p = out.last_mut().ok_or_else(bad)?;
                let n = f.next().and_then(|x| x.parse().ok()).ok_or_else(bad)?;
                let leaf = match f.next() {
                    Some("leaf") => true,
                    Some("internal") => false,
                    _ => return Err(bad()),
                };
                p.observed.allocs.push((n, leaf));
            }
            Some("hand_back") => {
                let p = out.last_mut().ok_or_else(bad)?;
                p.observed.hand_backs.insert(f.next().and_then(|x| x.parse().ok()).ok_or_else(bad)?);
            }
            Some("root") => {
                let p = out.last_mut().ok_or_else(bad)?;
                p.observed.root = f.next().and_then(|x| x.parse().ok()).ok_or_else(bad)?;
                f.next();
                p.observed.len = f.next().and_then(|x| x.parse().ok()).ok_or_else(bad)?;
            }
            Some("image") => {
                out.last_mut().ok_or_else(bad)?.observed.images = Some(Vec::new());
            }
            Some("node") => {
                flush(&mut cur, &mut out, &mut last);
                let ord: u64 = f.next().and_then(|x| x.parse().ok()).ok_or_else(bad)?;
                let mode = l.rsplit(' ').next().ok_or_else(bad)?;
                cur = Some(match mode {
                    "full" => (ord, vec![0u8; NODE], vec![false; NODE]),
                    "delta" | "same" => {
                        let (b, w) = last.get(&ord).cloned().ok_or_else(bad)?;
                        (ord, b, w)
                    }
                    _ => return Err(bad()),
                });
            }
            Some(h) if h.len() == 4 && cur.is_some() => {
                let at = usize::from_str_radix(h, 16).map_err(|_| bad())?;
                let data = f.next().ok_or_else(bad)?;
                let (_, b, w) = cur.as_mut().ok_or_else(bad)?;
                if at % LINE != 0 || data.len() != 2 * ((at + LINE).min(NODE) - at) {
                    return Err(bad());
                }
                for (k, pair) in data.as_bytes().chunks(2).enumerate() {
                    if pair == b".." {
                        b[at + k] = 0;
                        w[at + k] = false;
                    } else {
                        let s = std::str::from_utf8(pair).map_err(|_| bad())?;
                        b[at + k] = u8::from_str_radix(s, 16).map_err(|_| bad())?;
                        w[at + k] = true;
                    }
                }
            }
            _ => {}
        }
    }
    flush(&mut cur, &mut out, &mut last);
    Ok(out)
}
