//! A run's image: its header, the places of its parts, its overflow records and its filter, and the checks
//! of its header.
//!
//! A run is a read-only image of kind 2 holding one index-form tree (its *main tree*), node slot k at image
//! offset 4,096·k, slot 0 holding this header. Every byte is defined; gaps are zero; the length is a multiple
//! of 4,096. In order:
//! - slot 0: the header below, with the first and the last entry of the main tree as fences, then the run's
//!   shared prefix;
//! - slots 1 … nodes: the main tree in pre-order; then, when present, the count tree's slots;
//! - the overflow directory at OA = round_up(4,096·(slots + 1), 65,536), slots counting both trees: one
//!   24-byte record {payload u64, key offset u64, key length u64} per entry whose key is longer than W,
//!   record x at OA + ⌊x/2,730⌋·65,536 + (x mod 2,730)·24, so a record never crosses a 64 KiB block and the
//!   last 16 bytes of each directory block are zero. The payload is what the entry's field would hold
//!   without overflow (a rid, the last rid of a chunk, or a count's 62-bit pattern);
//! - the key bytes at OK = OA + ⌈v/2,730⌉·65,536: the shared prefix's bytes beyond the header's room, then
//!   each overflowed key's bytes beyond its first W, packed in record order (a key may cross image blocks and
//!   is read in pieces);
//! - the filter, when the main tree has more than one node, from the first 4 KiB block after the key bytes:
//!   m lines of 64 bytes for d distinct keys, 64 lines per 4 KiB block.
//!
//! **The shared prefix.** A run stores the bytes all its keys share once, and each entry, fence and overflow
//! record holds only the rest of its key (its *stored key*): the key area holds the stored key's first W
//! bytes, an overflow record its length and its bytes beyond W. The prefix's length p is the whole words of
//! the bytes the run's first and last keys share, taken short of the shorter key's end (so every stored key
//! holds at least one byte), and 0 in a run of an index with key runs; runs never change, so it is fixed.
//! The header holds p and the prefix's first min(p, 1,824) bytes; a longer prefix keeps the rest at the start
//! of the key bytes (OK), before the overflowed keys' bytes. Stored keys are ordered as the keys are, since
//! all keys share the prefix; readers compare a key's first p bytes with the prefix once and its rest with
//! the stored keys, and re-attach the prefix to every key they hand out. A run with a prefix carries image
//! version 2; one without, version 1.
//!
//! **The filter.** One insertion per distinct key, the filter key being the key's full encoding:
//! h = `nt_hash64(key)`; line = ⌊h·m / 2^64⌋; g = mix(h ^ 0x5851_F42D_4C95_7F2D); the bits (g >> 9i) & 511,
//! i = 0 … 6, of that line are set, bit b of a line being bit b mod 8 of its byte ⌊b/8⌋.

use core::cmp::Ordering;
use core::ops::Bound::{self, Excluded, Included, Unbounded};

use crate::consts::{
    magic, BLOCK, FILTER_BITS, FILTER_K, FILTER_LINES_PER_BLOCK, FORMAT_IMAGE, FORMAT_KEY, IMAGE_V1, PAGE, PATH_STACK,
    V_CHOICES, WALK_MAX,
};
use crate::fault::{corrupt, Corruption, Fault, ImageId, Refusal, RefusalCode};
use crate::hash::{mix, nt_hash64};
use crate::index::chunk::{self, header as chunk_header, key_field, Chunk, KEY_RUN};
use crate::index::nodes::{EntryRef, Run, RunImage, RunNodes};
use crate::key::probe::{cmp_entry, is_count, is_ovf, is_tomb, payload, KeyProbe, Kind};
use crate::mem::{copy_bytes, put, put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u64, rd_u8};
use crate::tree::node::child;
use crate::tree::{Cursor, Dir, Direction, Fwd, IndexShape, Path, Probe, Rev, Search};

/// Image kind of a run.
pub const KIND_RUN: u8 = 2;

/// Header flag bit 0: rids are relative to the run's rid base.
pub const F_RELATIVE: u8 = 1;
/// Header flag bit 1: the run carries a filter.
pub const F_FILTER: u8 = 2;
/// Header flag bit 2: the run belongs to the primary index.
pub const F_PRIMARY: u8 = 4;
/// Header flag bit 3: the run holds key runs.
pub const F_KEY_RUNS: u8 = 8;
/// Header flag bit 4: the run's keys rise in rid order.
pub const F_KEYS_RISE: u8 = 16;
/// Header flag bit 5: the run carries a count tree.
pub const F_COUNT_TREE: u8 = 32;

/// Header field offsets.
pub mod at {
    pub const MAGIC: u64 = 0;
    pub const VERSION: u64 = 4;
    pub const KIND: u64 = 6;
    pub const FLAGS: u64 = 7;
    pub const BYTES: u64 = 8;
    pub const TABLE_UID: u64 = 16;
    pub const SCHEMA_HASH: u64 = 24;
    pub const INDEX: u64 = 32;
    pub const W: u64 = 34;
    pub const V: u64 = 36;
    pub const FORMAT_KEY: u64 = 38;
    pub const N: u64 = 40;
    pub const NODES: u64 = 48;
    pub const HEIGHT: u64 = 56;
    pub const ROOT: u64 = 64;
    pub const OVF_RECORDS: u64 = 72;
    pub const OA: u64 = 80;
    pub const OK: u64 = 88;
    pub const FILTER: u64 = 96;
    pub const FILTER_BLOCKS: u64 = 104;
    pub const FILTER_LINES: u64 = 112;
    pub const RID_LO: u64 = 120;
    pub const RID_HI: u64 = 128;
    pub const KEY_SPEC_HASH: u64 = 136;
    pub const ROWS: u64 = 144;
    pub const TOTAL: u64 = 152;
    pub const DISTINCT: u64 = 160;
    pub const KEY_RUN_L: u64 = 168;
    pub const TOMBS: u64 = 176;
    pub const CT_ROOT: u64 = 184;
    pub const FENCE_FIRST: u64 = 192;
    pub const FENCE_LAST: u64 = 1_216;
    pub const CT_NODES: u64 = 2_240;
    pub const CT_HEIGHT: u64 = 2_248;
    pub const CT_ENTRIES: u64 = 2_256;
    /// The shared prefix's length p.
    pub const PREFIX_LEN: u64 = 2_264;
    /// The header's fields end here, where the prefix's first min(p, 1,824) bytes start; the rest of slot 0 is
    /// zero.
    pub const END: u64 = 2_272;
    pub const PREFIX: u64 = 2_272;
}

/// Bytes of the shared prefix the header holds; a longer prefix keeps the rest at the start of the key bytes.
pub const PREFIX_HEAD: u64 = BLOCK - at::PREFIX;

/// Bytes of a fence: a key part and a value part, zero-padded.
pub const FENCE_BYTES: u64 = 1_024;
/// Overflow records per 64 KiB directory block.
pub const RECORDS_PER_BLOCK: u64 = 2_730;
/// Bytes of an overflow record.
pub const RECORD_BYTES: u64 = 24;
/// The constant g is derived with.
pub const FILTER_SALT: u64 = 0x5851_F42D_4C95_7F2D;
/// Bytes of a filter line.
pub const LINE_BYTES: u64 = 64;

/// A run header's fields (the fences stay in the image).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RunHeader {
    pub version: u16,
    pub flags: u8,
    pub bytes: u64,
    pub table_uid: u64,
    pub schema_hash: u64,
    pub index: u8,
    pub w: u64,
    pub v: u64,
    pub format_key: u16,
    pub n: u64,
    pub nodes: u64,
    pub height: u64,
    pub root: u64,
    pub records: u64,
    pub oa: u64,
    pub ok: u64,
    pub filter: u64,
    pub filter_blocks: u64,
    pub lines: u64,
    pub rid_lo: u64,
    pub rid_hi: u64,
    pub key_spec_hash: u64,
    pub rows: u64,
    pub total: i64,
    pub d: u64,
    pub key_run_l: u8,
    pub tombs: u64,
    pub ct_root: u64,
    pub ct_nodes: u64,
    pub ct_height: u64,
    pub ct_entries: u64,
    /// The shared prefix's length p (its bytes are written apart).
    pub prefix: u64,
}

fn rt(off: u64) -> Fault {
    corrupt(ImageId::Runtime, Corruption::Count, off)
}

/// Filter lines for d distinct keys: max(64, ⌈10·d / 512⌉), rounded up to a multiple of 64.
pub fn filter_lines(d: u64) -> Result<u64, Fault> {
    let bitsum = d.checked_mul(FILTER_BITS).ok_or(rt(d))?;
    let lines = bitsum.div_ceil(512).max(FILTER_LINES_PER_BLOCK);
    lines.div_ceil(FILTER_LINES_PER_BLOCK).checked_mul(FILTER_LINES_PER_BLOCK).ok_or(rt(d))
}

/// OA for an image of `slots` node slots: round_up(4,096·(slots + 1), 65,536).
pub fn overflow_area(slots: u64) -> Result<u64, Fault> {
    slots
        .checked_add(1)
        .and_then(|s| s.checked_mul(BLOCK))
        .and_then(|b| b.checked_next_multiple_of(PAGE))
        .ok_or(rt(slots))
}

/// OK for v records from OA: OA + ⌈v/2,730⌉·65,536.
pub fn overflow_keys(oa: u64, v: u64) -> Result<u64, Fault> {
    v.div_ceil(RECORDS_PER_BLOCK).checked_mul(PAGE).and_then(|b| b.checked_add(oa)).ok_or(rt(v))
}

/// The image offset of overflow record x.
pub fn record_offset(oa: u64, x: u64) -> Option<u64> {
    let block = (x / RECORDS_PER_BLOCK).checked_mul(PAGE)?;
    // x mod 2,730 < 2,730, so the product stays below 65,520.
    let within = (x % RECORDS_PER_BLOCK).wrapping_mul(RECORD_BYTES);
    oa.checked_add(block)?.checked_add(within)
}

/// The places of a run's parts, from its trees' node counts, its overflow records and key bytes, and its
/// distinct keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RunLayout {
    pub slots: u64,
    pub records: u64,
    pub key_bytes: u64,
    pub oa: u64,
    pub ok: u64,
    /// 0: no filter.
    pub filter: u64,
    pub filter_blocks: u64,
    pub lines: u64,
    /// The image's length.
    pub len: u64,
}

impl RunLayout {
    pub fn new(nodes: u64, ct_nodes: u64, records: u64, key_bytes: u64, d: u64) -> Result<RunLayout, Fault> {
        let slots = nodes.checked_add(ct_nodes).ok_or(rt(nodes))?;
        let oa = overflow_area(slots)?;
        let ok = overflow_keys(oa, records)?;
        let kend = ok.checked_add(key_bytes).ok_or(rt(ok))?;
        let after = kend.checked_next_multiple_of(BLOCK).ok_or(rt(kend))?;
        let (filter, filter_blocks, lines, len) = if nodes > 1 {
            let lines = filter_lines(d)?;
            let blocks = lines / 64;
            let len = blocks.checked_mul(BLOCK).and_then(|b| b.checked_add(after)).ok_or(rt(after))?;
            (after, blocks, lines, len)
        } else {
            (0, 0, 0, after)
        };
        Ok(RunLayout { slots, records, key_bytes, oa, ok, filter, filter_blocks, lines, len })
    }
}

// ---------------------------------------------------------------- the filter

/// The line and the seven bit positions of a filter key in a filter of m lines.
pub fn filter_bits(key: &[u8], lines: u64) -> (u64, [u16; 7]) {
    let (h, b) = filter_hash(key);
    (filter_line(h, lines), b)
}

/// The line of a filter key whose hash is h, in a filter of m lines: ⌊h·m / 2^64⌋, from 64-bit products (two
/// of them for m below 2^32, as every filter a probe record names has).
#[inline]
pub fn filter_line(h: u64, lines: u64) -> u64 {
    mul_hi(h, lines)
}

/// The high 64 bits of the 128-bit product a·b, from 32-bit halves; for b < 2^32, from two products.
#[inline]
fn mul_hi(a: u64, b: u64) -> u64 {
    if b >> 32 == 0 {
        // a·b = a₁·b·2^32 + a₀·b with a₁·b and a₀·b below 2^64, and a₁·b + ⌊a₀·b / 2^32⌋ below 2^64.
        let hi = (a >> 32).wrapping_mul(b);
        let lo = (a & 0xFFFF_FFFF).wrapping_mul(b);
        return hi.wrapping_add(lo >> 32) >> 32;
    }
    let (a0, a1) = (a & 0xFFFF_FFFF, a >> 32);
    let (b0, b1) = (b & 0xFFFF_FFFF, b >> 32);
    // Each product of two 32-bit halves is below 2^64; the sums below are of 32-bit parts and carries.
    let p00 = a0.wrapping_mul(b0);
    let p01 = a0.wrapping_mul(b1);
    let p10 = a1.wrapping_mul(b0);
    let p11 = a1.wrapping_mul(b1);
    let mid = (p00 >> 32).wrapping_add(p01 & 0xFFFF_FFFF).wrapping_add(p10 & 0xFFFF_FFFF);
    p11.wrapping_add(p01 >> 32).wrapping_add(p10 >> 32).wrapping_add(mid >> 32)
}

/// A filter key's hash and its seven bit positions, which do not depend on the filter's size.
pub fn filter_hash(key: &[u8]) -> (u64, [u16; 7]) {
    filter_hash_of(nt_hash64(key))
}

/// `filter_hash` from the key's `nt_hash64` h.
pub fn filter_hash_of(h: u64) -> (u64, [u16; 7]) {
    let g = mix(h ^ FILTER_SALT);
    let mut b = [0u16; 7];
    for (i, slot) in b.iter_mut().enumerate().take(usize::try_from(FILTER_K).unwrap_or(7)) {
        let sh = u32::try_from(i).unwrap_or(0).wrapping_mul(9);
        // A 9-bit value.
        #[allow(clippy::cast_possible_truncation)]
        let bit = (g.wrapping_shr(sh) & 511) as u16;
        *slot = bit;
    }
    (h, b)
}

/// Sets a key's bits in `filter`, the filter's m · 64 bytes.
pub fn filter_set(filter: &mut [u8], lines: u64, key: &[u8]) -> Result<(), Fault> {
    let (line, b) = filter_bits(key, lines);
    let base = line.checked_mul(LINE_BYTES).ok_or(rt(line))?;
    for bit in b {
        let at = base.checked_add(u64::from(bit / 8)).ok_or(rt(line))?;
        let byte = crate::mem::rd_u8(filter, at)?;
        crate::mem::put_u8(filter, at, byte | 1u8.wrapping_shl(u32::from(bit % 8)))?;
    }
    Ok(())
}

/// Whether a filter line (64 bytes) has all seven of a key's bits set.
pub fn filter_line_admits(line: &[u8], bits7: &[u16; 7]) -> bool {
    bits7
        .iter()
        .all(|&bit| line.get(usize::from(bit / 8)).is_some_and(|&x| x & 1u8.wrapping_shl(u32::from(bit % 8)) != 0))
}

// ---------------------------------------------------------------- the header

impl RunHeader {
    /// Writes the header fields into `out`, slot 0's first `at::END` bytes or more; fences are written
    /// apart. Every byte between fields is left as `out` holds it (zero when the caller zeroed it).
    pub fn write(&self, out: &mut [u8]) -> Result<(), Fault> {
        put_u32(out, at::MAGIC, magic::NTIM)?;
        put_u16(out, at::VERSION, self.version)?;
        put_u8(out, at::KIND, KIND_RUN)?;
        put_u8(out, at::FLAGS, self.flags)?;
        put_u64(out, at::BYTES, self.bytes)?;
        put_u64(out, at::TABLE_UID, self.table_uid)?;
        put_u64(out, at::SCHEMA_HASH, self.schema_hash)?;
        put_u8(out, at::INDEX, self.index)?;
        put_u16(out, at::W, u16::try_from(self.w).map_err(|_| rt(at::W))?)?;
        put_u16(out, at::V, u16::try_from(self.v).map_err(|_| rt(at::V))?)?;
        put_u16(out, at::FORMAT_KEY, self.format_key)?;
        put_u64(out, at::N, self.n)?;
        put_u64(out, at::NODES, self.nodes)?;
        put_u8(out, at::HEIGHT, u8::try_from(self.height).map_err(|_| rt(at::HEIGHT))?)?;
        put_u64(out, at::ROOT, self.root)?;
        put_u64(out, at::OVF_RECORDS, self.records)?;
        put_u64(out, at::OA, self.oa)?;
        put_u64(out, at::OK, self.ok)?;
        put_u64(out, at::FILTER, self.filter)?;
        put_u64(out, at::FILTER_BLOCKS, self.filter_blocks)?;
        put_u64(out, at::FILTER_LINES, self.lines)?;
        put_u64(out, at::RID_LO, self.rid_lo)?;
        put_u64(out, at::RID_HI, self.rid_hi)?;
        put_u64(out, at::KEY_SPEC_HASH, self.key_spec_hash)?;
        put_u64(out, at::ROWS, self.rows)?;
        put(out, at::TOTAL, &self.total.to_le_bytes())?;
        put_u64(out, at::DISTINCT, self.d)?;
        put_u8(out, at::KEY_RUN_L, self.key_run_l)?;
        put_u64(out, at::TOMBS, self.tombs)?;
        put_u64(out, at::CT_ROOT, self.ct_root)?;
        put_u64(out, at::CT_NODES, self.ct_nodes)?;
        put_u8(out, at::CT_HEIGHT, u8::try_from(self.ct_height).map_err(|_| rt(at::CT_HEIGHT))?)?;
        put_u64(out, at::CT_ENTRIES, self.ct_entries)?;
        put_u64(out, at::PREFIX_LEN, self.prefix)
    }

    /// Reads the fields from slot 0's bytes; only the slice's length is checked.
    pub fn read(head: &[u8]) -> Result<RunHeader, Fault> {
        let prefix = rd_u64(head, at::PREFIX_LEN)?;
        Ok(RunHeader {
            version: rd_u16(head, at::VERSION)?,
            flags: rd_u8(head, at::FLAGS)?,
            bytes: rd_u64(head, at::BYTES)?,
            table_uid: rd_u64(head, at::TABLE_UID)?,
            schema_hash: rd_u64(head, at::SCHEMA_HASH)?,
            index: rd_u8(head, at::INDEX)?,
            w: u64::from(rd_u16(head, at::W)?),
            v: u64::from(rd_u16(head, at::V)?),
            format_key: rd_u16(head, at::FORMAT_KEY)?,
            n: rd_u64(head, at::N)?,
            nodes: rd_u64(head, at::NODES)?,
            height: u64::from(rd_u8(head, at::HEIGHT)?),
            root: rd_u64(head, at::ROOT)?,
            records: rd_u64(head, at::OVF_RECORDS)?,
            oa: rd_u64(head, at::OA)?,
            ok: rd_u64(head, at::OK)?,
            filter: rd_u64(head, at::FILTER)?,
            filter_blocks: rd_u64(head, at::FILTER_BLOCKS)?,
            lines: rd_u64(head, at::FILTER_LINES)?,
            rid_lo: rd_u64(head, at::RID_LO)?,
            rid_hi: rd_u64(head, at::RID_HI)?,
            key_spec_hash: rd_u64(head, at::KEY_SPEC_HASH)?,
            rows: rd_u64(head, at::ROWS)?,
            total: i64::from_le_bytes(crate::mem::rd_array::<8>(head, at::TOTAL)?),
            d: rd_u64(head, at::DISTINCT)?,
            key_run_l: rd_u8(head, at::KEY_RUN_L)?,
            tombs: rd_u64(head, at::TOMBS)?,
            ct_root: rd_u64(head, at::CT_ROOT)?,
            ct_nodes: rd_u64(head, at::CT_NODES)?,
            ct_height: u64::from(rd_u8(head, at::CT_HEIGHT)?),
            ct_entries: rd_u64(head, at::CT_ENTRIES)?,
            prefix,
        })
    }

    /// The main tree's shape; `None` unless (W, V) is one.
    pub fn shape(&self) -> Option<IndexShape> {
        IndexShape::new(self.w, self.v)
    }

    pub fn has_count_tree(&self) -> bool {
        self.flags & F_COUNT_TREE != 0
    }

    pub fn has_filter(&self) -> bool {
        self.flags & F_FILTER != 0
    }

    /// The prefix's bytes beyond the header's room, at the start of the key bytes.
    pub fn prefix_tail(&self) -> u64 {
        self.prefix.saturating_sub(PREFIX_HEAD)
    }

    /// The image version a run of this header carries: 2 when it stores a prefix.
    pub fn version_needed(&self) -> u16 {
        if self.prefix > 0 {
            FORMAT_IMAGE
        } else {
            IMAGE_V1
        }
    }

    /// The node slots of both trees.
    pub fn slots(&self) -> u64 {
        self.nodes.saturating_add(if self.has_count_tree() { self.ct_nodes } else { 0 })
    }

    /// The geometry rules a reader re-checks when it opens a run, against the image's own length:
    /// returns the offset of the first field that breaks one.
    /// - the image is a run of a version this crate reads, its bytes field equal to the image's length;
    /// - (W, V) is an index shape; n ≥ 1; nodes ≥ 1; 1 ≤ height ≤ 64; the root at 4,096; v ≤ n plus the count
    ///   tree's entries; n ≤ nodes · nL;
    /// - with the count-tree flag: its root at 4,096·(nodes + 1), nodes ≥ 1, 1 ≤ height ≤ 64, entries =
    ///   d ≤ its nodes · nL(W, 0); without it, those fields zero;
    /// - OA and OK where the overflow formulas put them for both trees' slots, OK inside the image;
    /// - the filter flag set exactly when nodes > 1; with a filter, its lines from d, its blocks = lines / 64,
    ///   its offset a multiple of 4,096 at or after OK and its end inside the image; without, those zero;
    /// - a prefix only at version 2, a multiple of 8, none with the key-run flag, and its bytes beyond the
    ///   header's room inside the image and before the filter.
    pub fn geometry(&self, image_len: u64) -> Result<(), u64> {
        if self.version == 0 || self.version > FORMAT_IMAGE {
            return Err(at::VERSION);
        }
        if self.bytes != image_len || self.bytes % 8 != 0 {
            return Err(at::BYTES);
        }
        let shape = self.shape().ok_or(at::W)?;
        if self.n == 0 || self.nodes == 0 || self.height == 0 || self.height > PATH_STACK || self.root != BLOCK {
            return Err(at::N);
        }
        let ct_entries = if self.has_count_tree() { self.ct_entries } else { 0 };
        if self.records > self.n.saturating_add(ct_entries) || self.n > self.nodes.saturating_mul(shape.leaf.cap) {
            return Err(at::N);
        }
        if self.has_count_tree() {
            let cs = IndexShape::new(self.w, 0).ok_or(at::CT_ROOT)?;
            let root = self.nodes.checked_add(1).and_then(|s| s.checked_mul(BLOCK)).ok_or(at::CT_ROOT)?;
            if self.ct_root != root
                || self.ct_nodes == 0
                || self.ct_height == 0
                || self.ct_height > PATH_STACK
                || self.ct_entries != self.d
                || self.ct_entries == 0
                || self.ct_entries > self.ct_nodes.saturating_mul(cs.leaf.cap)
            {
                return Err(at::CT_ROOT);
            }
        } else if self.ct_root != 0 || self.ct_nodes != 0 || self.ct_height != 0 || self.ct_entries != 0 {
            return Err(at::CT_ROOT);
        }
        let oa = overflow_area(self.slots()).map_err(|_| at::OA)?;
        let ok = overflow_keys(oa, self.records).map_err(|_| at::OA)?;
        if self.oa != oa || self.ok != ok || ok > image_len {
            return Err(at::OA);
        }
        if self.has_filter() != (self.nodes > 1) {
            return Err(at::FILTER);
        }
        if self.has_filter() {
            let lines = filter_lines(self.d).map_err(|_| at::FILTER)?;
            let end = self.filter_blocks.checked_mul(BLOCK).and_then(|b| b.checked_add(self.filter));
            if self.lines != lines
                || self.filter_blocks != lines / 64
                || self.filter % BLOCK != 0
                || self.filter < ok
                || end.is_none_or(|e| e > image_len)
            {
                return Err(at::FILTER);
            }
        } else if self.filter != 0 || self.filter_blocks != 0 || self.lines != 0 {
            return Err(at::FILTER);
        }
        if self.prefix != 0 {
            let room = if self.has_filter() { self.filter } else { image_len };
            if self.version < FORMAT_IMAGE
                || self.prefix % 8 != 0
                || self.flags & F_KEY_RUNS != 0
                || ok.checked_add(self.prefix_tail()).is_none_or(|e| e > room)
            {
                return Err(at::PREFIX_LEN);
            }
        }
        Ok(())
    }
}

/// The first 8 bytes of a prefix of `len` bytes held from `at::PREFIX` of slot 0 `head`, big-endian,
/// zero-padded (0 without a prefix).
pub fn prefix_head(head: &[u8], len: u64) -> Result<u64, Fault> {
    if len == 0 {
        return Ok(0);
    }
    let mut b = [0u8; 8];
    let n = usize::try_from(len.min(8)).unwrap_or(8);
    copy_bytes(b.get_mut(..n).unwrap_or(&mut []), crate::mem::sub(head, at::PREFIX, n as u64)?);
    Ok(u64::from_be_bytes(b))
}

/// The shared prefix a run stores for its first and last keys, which share `shared` bytes and the shorter of
/// which is `shorter` bytes long: the whole words of the shared bytes, short of the shorter key's end.
pub fn shared_prefix(shared: u64, shorter: u64) -> u64 {
    shared.min(shorter.saturating_sub(1)) & !7
}

/// Whether the run's prefix is the one its fences give: the stored keys of its first and last entries (their
/// key areas, from the header in `head`) differ in their first 8 bytes, so they share no further word; or
/// they are one key whose stored part ends within its first word (neither overflows, and the areas are equal
/// and zero past their first 8 bytes). A run of an index with key runs stores no prefix and is not checked.
pub fn prefix_matches_fences(head: &[u8], h: &RunHeader) -> bool {
    if h.flags & F_KEY_RUNS != 0 {
        return true;
    }
    let (Some(wu), Some(first), Some(last)) =
        (usize::try_from(h.w).ok(), usize::try_from(at::FENCE_FIRST).ok(), usize::try_from(at::FENCE_LAST).ok())
    else {
        return false;
    };
    let (Some(a), Some(b)) = (head.get(first..first.saturating_add(wu)), head.get(last..last.saturating_add(wu)))
    else {
        return false;
    };
    let field = |at: usize| head.get(at.saturating_add(wu)).map_or(0x80, |&x| x & 0x80);
    if a.get(..8) != b.get(..8) {
        return true;
    }
    field(first) == 0 && field(last) == 0 && a == b && a.get(8..).is_none_or(|z| z.iter().all(|&x| x == 0))
}

/// What the load session expects of a run of index x.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RunExpect {
    pub table_uid: u64,
    pub schema_hash: u64,
    pub index: u8,
    pub key_spec_hash: u64,
    /// The index's key-area width.
    pub w: u64,
    /// The index's key-run field length; 0 when it allows no key runs.
    pub key_run_l: u8,
}

/// The header check of an uploaded run on the first bytes of its image block 0 (zero-filled to 4,096):
/// the common image rules, then the run's. Each refusal names its code and the field's offset.
pub fn check_header(head: &[u8], image: u64, x: &RunExpect) -> Result<RunHeader, Refusal> {
    let refuse = |code: RefusalCode, field: u64| Refusal { code, image, field };
    let bytes = rd_u64(head, at::BYTES).map_err(|_| refuse(RefusalCode::BadSizes, at::BYTES))?;
    if bytes % 8 != 0 || !(1_024..=crate::consts::IMAGE_MAX).contains(&bytes) {
        return Err(refuse(RefusalCode::BadSizes, at::BYTES));
    }
    if rd_u32(head, at::MAGIC).ok() != Some(magic::NTIM) {
        return Err(refuse(RefusalCode::BadMagic, at::MAGIC));
    }
    let h = RunHeader::read(head).map_err(|_| refuse(RefusalCode::BadSizes, at::BYTES))?;
    if h.version == 0 || h.version > FORMAT_IMAGE {
        return Err(refuse(RefusalCode::BadVersion, at::VERSION));
    }
    if rd_u8(head, at::KIND).ok() != Some(KIND_RUN) {
        return Err(refuse(RefusalCode::BadKind, at::KIND));
    }
    if h.table_uid != x.table_uid {
        return Err(refuse(RefusalCode::WrongTable, at::TABLE_UID));
    }
    if h.schema_hash != x.schema_hash {
        return Err(refuse(RefusalCode::SchemaChanged, at::SCHEMA_HASH));
    }
    let primary = h.flags & F_PRIMARY != 0;
    let key_runs = h.flags & F_KEY_RUNS != 0;
    let v_ok = V_CHOICES.contains(&h.v) && h.shape().is_some();
    let runs_ok = if key_runs { x.key_run_l != 0 && h.key_run_l == x.key_run_l } else { h.key_run_l == 0 };
    if h.index != x.index
        || h.key_spec_hash != x.key_spec_hash
        || h.w != x.w
        || !v_ok
        || h.format_key == 0
        || h.format_key > FORMAT_KEY
        || h.flags & F_RELATIVE == 0
        || primary != (x.index == 0)
        || !runs_ok
    {
        return Err(refuse(RefusalCode::WrongIndex, at::INDEX));
    }
    match h.geometry(bytes) {
        Ok(()) if prefix_matches_fences(head, &h) => Ok(h),
        Ok(()) => Err(refuse(RefusalCode::BadSizes, at::PREFIX_LEN)),
        Err(f) if f == at::PREFIX_LEN => Err(refuse(RefusalCode::BadSizes, at::PREFIX_LEN)),
        Err(f) if f < at::OA => Err(refuse(RefusalCode::BadSizes, at::N)),
        Err(f) if f < at::FILTER => Err(refuse(RefusalCode::BadSizes, at::OA)),
        Err(f) if f == at::CT_ROOT => Err(refuse(RefusalCode::BadSizes, at::N)),
        Err(_) => Err(refuse(RefusalCode::BadSizes, at::FILTER)),
    }
}

/// Whether a rid read from an entry, made absolute, lies in the run's runtime-recorded range [lo, hi].
pub fn contains(abs_rid: u64, lo: u64, hi: u64) -> bool {
    abs_rid >= lo && abs_rid <= hi
}

// ---------------------------------------------------------------- reading: counts, prefix sums, ranks

/// Bytes of a key compared or copied at once.
const PIECE: usize = 64;

/// Copies bytes [from, from + out.len()) of an entry's stored key into `out`: the key area below W, the
/// overflow record's bytes from W on; returns how many (fewer at the key's end).
fn entry_key_bytes<I: RunImage>(
    t: &RunNodes<'_, I>,
    e: &EntryRef<'_>,
    from: u64,
    out: &mut [u8],
) -> Result<usize, Fault> {
    let w = t.shape().w;
    let want = u64::try_from(out.len()).unwrap_or(u64::MAX);
    let len = if is_ovf(e.field) { t.run().record(e.field)?.len } else { w };
    let mut done = 0u64;
    let mut at = from;
    while done < want && at < len {
        let dst = out.get_mut(usize::try_from(done).unwrap_or(usize::MAX)..).unwrap_or(&mut []);
        let got = if at < w {
            let a = usize::try_from(at).unwrap_or(usize::MAX);
            let src = e.area.get(a..).unwrap_or(&[]);
            let n = src.len().min(dst.len());
            copy_bytes(dst.get_mut(..n).unwrap_or(&mut []), src.get(..n).unwrap_or(&[]));
            n
        } else {
            t.run().key_piece(e.field, at, dst)?
        };
        if got == 0 {
            break;
        }
        let g = u64::try_from(got).unwrap_or(0);
        done = done.saturating_add(g);
        at = at.saturating_add(g);
    }
    Ok(usize::try_from(done).unwrap_or(0))
}

/// Copies an entry's stored key (its W-byte key area when the key fits W, else its full key) into `out`;
/// returns its length. `Scratch` when `out` is too short.
pub fn stored_key<I: RunImage>(t: &RunNodes<'_, I>, e: &EntryRef<'_>, out: &mut [u8]) -> Result<usize, Fault> {
    let len = if is_ovf(e.field) { t.run().record(e.field)?.len } else { t.shape().w };
    let lu = usize::try_from(len).map_err(|_| Fault::Scratch { need: len })?;
    let dst = out.get_mut(..lu).ok_or(Fault::Scratch { need: len })?;
    let got = entry_key_bytes(t, e, 0, dst)?;
    if got != lu {
        return Err(corrupt(t.run().id(), Corruption::Bounds, e.node));
    }
    Ok(lu)
}

/// Whether bytes [0, n) of the entry's stored key equal `key[..n]`.
fn same_prefix<I: RunImage>(t: &RunNodes<'_, I>, e: &EntryRef<'_>, key: &[u8], n: u64) -> Result<bool, Fault> {
    let mut at = 0u64;
    let mut buf = [0u8; PIECE];
    while at < n {
        let want = usize::try_from(n.wrapping_sub(at)).unwrap_or(PIECE).min(PIECE);
        let got = entry_key_bytes(t, e, at, buf.get_mut(..want).unwrap_or(&mut []))?;
        if got == 0 {
            return Ok(false);
        }
        let a = usize::try_from(at).unwrap_or(usize::MAX);
        let theirs = key.get(a..a.saturating_add(got));
        if theirs != buf.get(..got) {
            return Ok(false);
        }
        at = at.saturating_add(u64::try_from(got).unwrap_or(0));
    }
    Ok(true)
}

/// A key run's header (L, n − 1) when entry e is a key run.
fn key_run_of<I: RunImage>(t: &RunNodes<'_, I>, e: &EntryRef<'_>) -> Result<Option<(u8, u32)>, Fault> {
    if t.shape().v < 8 || is_count(e.field) || is_tomb(e.field) || e.value.first() != Some(&KEY_RUN) {
        return Ok(None);
    }
    match chunk_header(e.value, t.run().id(), e.node)? {
        Chunk::KeyRun { l, n_minus_1 } => Ok(Some((l, n_minus_1))),
        Chunk::Posting { .. } => Ok(None),
    }
}

/// The length of a key-run entry's keys: its record's when it overflows, else `klen`, which must fit W;
/// `Corrupt(Encoding)` when L does not fit it.
fn run_keys_len<I: RunImage>(t: &RunNodes<'_, I>, e: &EntryRef<'_>, l: u8, klen: u64) -> Result<u64, Fault> {
    let len = if is_ovf(e.field) { t.run().record(e.field)?.len } else { klen };
    if len < u64::from(l) || (!is_ovf(e.field) && len > t.shape().w) {
        return Err(corrupt(t.run().id(), Corruption::Encoding, e.node));
    }
    Ok(len)
}

/// The key field of the entry's key of length `len`: its bytes [len − L, len).
fn entry_field<I: RunImage>(t: &RunNodes<'_, I>, e: &EntryRef<'_>, l: u8, len: u64) -> Result<u64, Fault> {
    let mut b = [0u8; 8];
    let lu = usize::from(l);
    let got = entry_key_bytes(t, e, len.saturating_sub(u64::from(l)), b.get_mut(..lu).unwrap_or(&mut []))?;
    if got != lu {
        return Err(corrupt(t.run().id(), Corruption::Bounds, e.node));
    }
    key_field(b.get(..lu).unwrap_or(&[]), l).ok_or(corrupt(t.run().id(), Corruption::Encoding, e.node))
}

/// When entry e is a key run and `key` (its full encoding or its stored form, of true length `klen`) is one
/// of its items: the item's index k.
fn key_run_index<I: RunImage>(
    t: &RunNodes<'_, I>,
    e: &EntryRef<'_>,
    key: &[u8],
    klen: u64,
) -> Result<Option<u32>, Fault> {
    let Some((l, nm1)) = key_run_of(t, e)? else { return Ok(None) };
    let w = t.shape().w;
    if klen < u64::from(l) || is_ovf(e.field) != (klen > w) {
        return Ok(None);
    }
    let len = run_keys_len(t, e, l, klen)?;
    if len != klen {
        return Ok(None);
    }
    let head = klen.wrapping_sub(u64::from(l));
    if !same_prefix(t, e, key, head)? {
        return Ok(None);
    }
    if !is_ovf(e.field) {
        // Past the key, both the area and a stored form are zero padding.
        let k0 = usize::try_from(klen).unwrap_or(usize::MAX);
        let pad = |b: &[u8]| b.get(k0..).is_some_and(|z| z.iter().any(|&x| x != 0));
        if pad(e.area) || pad(key) {
            return Ok(None);
        }
    }
    let ku = usize::try_from(klen).unwrap_or(usize::MAX);
    let fk = key.get(..ku).and_then(|k| key_field(k, l)).ok_or(corrupt(t.run().id(), Corruption::Encoding, e.node))?;
    item_of(t, e, l, nm1, klen, fk)
}

/// The index of the item whose key field is `fk` in the key run e (L = l, n − 1 = nm1, keys of `klen` bytes),
/// when it holds one.
fn item_of<I: RunImage>(
    t: &RunNodes<'_, I>,
    e: &EntryRef<'_>,
    l: u8,
    nm1: u32,
    klen: u64,
    fk: u64,
) -> Result<Option<u32>, Fault> {
    let fe = entry_field(t, e, l, klen)?;
    let Some(back) = fe.checked_sub(fk) else { return Ok(None) };
    if back > u64::from(nm1) {
        return Ok(None);
    }
    // back ≤ n − 1 < 2^32.
    Ok(Some(nm1.wrapping_sub(u32::try_from(back).unwrap_or(0))))
}

/// Writes the stored key of the key-run item whose field is f into `out`: the entry's stored key with bytes
/// [len − L, len) set to f.
fn key_run_member<I: RunImage>(
    t: &RunNodes<'_, I>,
    e: &EntryRef<'_>,
    l: u8,
    len: u64,
    f: u64,
    out: &mut [u8],
) -> Result<usize, Fault> {
    let n = stored_key(t, e, out)?;
    let mut at = len;
    let mut x = f;
    let mut k = 0u8;
    while k < l {
        at = at.wrapping_sub(1);
        // The low byte of x.
        #[allow(clippy::cast_possible_truncation)]
        let byte = (x & 0xFF) as u8;
        put_u8(out, at, byte)?;
        x = x.wrapping_shr(8);
        k = k.wrapping_add(1);
    }
    Ok(n)
}

/// The position of a rank: the entry whose contributions span it, and the sum of everything before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RankPos {
    pub node: u64,
    pub index: u64,
    pub before: i64,
}

/// The path of the last rank selection: per node from the root, its range of cumulative counts [lo, hi).
#[derive(Clone, Copy, Debug)]
pub struct Finger {
    len: usize,
    frames: [(u64, i64, i64); 64],
}

impl Default for Finger {
    fn default() -> Self {
        Finger { len: 0, frames: [(0, 0, 0); 64] }
    }
}

impl Finger {
    fn last(&self) -> Option<(u64, i64, i64)> {
        self.frames.get(self.len.checked_sub(1)?).copied()
    }
    fn pop(&mut self) {
        self.len = self.len.saturating_sub(1);
    }
    fn push(&mut self, f: (u64, i64, i64)) -> bool {
        match self.frames.get_mut(self.len) {
            Some(slot) => {
                *slot = f;
                self.len = self.len.wrapping_add(1);
                true
            }
            None => false,
        }
    }
    pub fn clear(&mut self) {
        self.len = 0;
    }
}

/// The cursor's next entry from `start`, unbounded at the end.
/// The entry after position j of `leaf`, reached by `path` (the internal nodes passed and the child taken in
/// each): entry j of the leaf when the leaf has it, else the separator of the lowest ancestor whose child
/// taken is not its last; `None` past the tree's last entry.
fn after_position<I: RunImage>(
    t: &RunNodes<'_, I>,
    path: &Path,
    leaf: u64,
    j: u64,
) -> Result<Option<(u64, u64)>, Fault> {
    if j < t.node_info(leaf)?.1 {
        return Ok(Some((leaf, j)));
    }
    for &(n, i) in path.frames().iter().rev() {
        if i < t.node_info(n)?.1 {
            return Ok(Some((n, i)));
        }
    }
    Ok(None)
}

fn first_from<I: RunImage, D: Direction>(
    t: &RunNodes<'_, I>,
    start: Bound<&KeyProbe<'_>>,
) -> Result<Option<(u64, u64)>, Fault> {
    let mut c: Cursor<D> = t.seek(start)?;
    t.next(&mut c, Unbounded)
}

impl<I: RunImage> Run<I> {
    fn count_err(&self, off: u64) -> Fault {
        corrupt(self.id(), Corruption::Count, off)
    }

    /// Whether the filter admits `key` (its full encoding); true without a filter.
    pub fn filter_admits(&self, key: &[u8]) -> Result<bool, Fault> {
        if !self.hdr.has_filter() {
            return Ok(true);
        }
        self.filter_admits_hash(nt_hash64(key))
    }

    /// Whether the filter admits the key whose `nt_hash64` is h; true without a filter.
    fn filter_admits_hash(&self, h: u64) -> Result<bool, Fault> {
        if !self.hdr.has_filter() {
            return Ok(true);
        }
        let (h, b) = filter_hash_of(h);
        let line = filter_line(h, self.hdr.lines);
        let off = line.checked_mul(LINE_BYTES).and_then(|o| o.checked_add(self.hdr.filter));
        let off = off.ok_or(corrupt(self.id(), Corruption::Bounds, self.hdr.filter))?;
        Ok(filter_line_admits(self.img.read(off, LINE_BYTES)?, &b))
    }

    /// The sum of the contributions of every main-tree entry before (key, Low): the rows of keys below `key`
    /// less their tombstones. One descent and at most one leaf's contributions (none for a UNIT leaf).
    pub fn prefix(&self, key: &[u8]) -> Result<i64, Fault> {
        let key = match self.strip(key)? {
            Ok(rest) => rest,
            Err(Ordering::Greater) => return Ok(self.hdr.total),
            Err(_) => return Ok(0),
        };
        let t = self.main()?;
        let p = KeyProbe { key, kind: Kind::Low };
        let mut path = Path::new();
        let (leaf, j) = match t.descend(&p, &mut path)? {
            Search::Vacant(n, j) => (n, j),
            Search::Found(n, _) => return Err(corrupt(self.id(), Corruption::Order, n)),
        };
        let (cum, unit) = t.trailer(leaf)?;
        let (_, count) = t.node_info(leaf)?;
        let j = j.min(count);
        if unit {
            let jj = i64::try_from(j).map_err(|_| self.count_err(leaf))?;
            return cum.checked_add(jj).ok_or(self.count_err(leaf));
        }
        let mut acc = cum;
        let mut i = 0u64;
        while i < j {
            let c = t.contribution(&t.entry(leaf, i)?)?;
            acc = acc.checked_add(c).ok_or(self.count_err(leaf))?;
            i = i.wrapping_add(1);
        }
        Ok(acc)
    }

    /// The main-tree entry whose contributions span `rank` (0 ≤ rank < the run's total), from the finger of
    /// the previous call: climb until a node's range holds the rank, then descend, searching the children's
    /// cumulative counts. `None` outside [0, total).
    pub fn select_rank(&self, rank: i64, finger: &mut Finger) -> Result<Option<RankPos>, Fault> {
        let t = self.main()?;
        while let Some((_, lo, hi)) = finger.last() {
            if lo <= rank && rank < hi {
                break;
            }
            finger.pop();
        }
        if finger.len == 0 {
            if rank < 0 || rank >= self.hdr.total {
                return Ok(None);
            }
            finger.push((t.root(), 0, self.hdr.total));
        }
        let s = t.shape();
        loop {
            let (n, lo, hi) = finger.last().ok_or(self.count_err(0))?;
            let (leaf, count) = t.node_info(n)?;
            if leaf {
                let (_, unit) = t.trailer(n)?;
                if unit {
                    let off = rank.checked_sub(lo).and_then(|x| u64::try_from(x).ok()).ok_or(self.count_err(n))?;
                    if off >= count {
                        return Err(self.count_err(n));
                    }
                    return Ok(Some(RankPos { node: n, index: off, before: rank }));
                }
                let mut acc = lo;
                let mut i = 0u64;
                while i < count {
                    let c = t.contribution(&t.entry(n, i)?)?;
                    let next = acc.checked_add(c).ok_or(self.count_err(n))?;
                    if acc <= rank && rank < next {
                        return Ok(Some(RankPos { node: n, index: i, before: acc }));
                    }
                    acc = next;
                    i = i.wrapping_add(1);
                }
                return Err(self.count_err(n));
            }
            // The last child whose cumulative count is at most the rank.
            let cum = |j: u64| -> Result<i64, Fault> { t.trailer(child(s, &t, n, j)?).map(|x| x.0) };
            let (mut a, mut b) = (0u64, count);
            while a < b {
                let mid = a.wrapping_add(b.wrapping_sub(a).wrapping_add(1) / 2);
                if cum(mid)? <= rank {
                    a = mid;
                } else {
                    b = mid.wrapping_sub(1);
                }
            }
            let child_lo = cum(a)?;
            let child_hi = if a < count {
                let sep = t.contribution(&t.entry(n, a)?)?;
                cum(a.wrapping_add(1))?.checked_sub(sep).ok_or(self.count_err(n))?
            } else {
                hi
            };
            if a < count && rank >= child_hi {
                return Ok(Some(RankPos { node: n, index: a, before: child_hi }));
            }
            let c = child(s, &t, n, a)?;
            if !finger.push((c, child_lo, child_hi)) {
                return Err(corrupt(self.id(), Corruption::Depth, c));
            }
        }
    }

    /// The total of `key` in the main tree: its count entry, else the sum of its at most 7 entries before
    /// the position (a key-run entry counts its one item of this key); `successor` also counts the key when
    /// it is an inner item of the key run that follows the position.
    fn main_total(&self, t: &RunNodes<'_, I>, key: &[u8], klen: u64, successor: bool) -> Result<i64, Fault> {
        let w = usize::try_from(t.shape().w).map_err(|_| self.count_err(0))?;
        let lo = KeyProbe { key, kind: Kind::Low };
        let hi = KeyProbe { key, kind: Kind::Count };
        let vs_lo = |e: &EntryRef<'_>| Ok(cmp_entry(t, w, e.area, e.field, &lo, 0)?.0);
        let item = |e: &EntryRef<'_>| Ok(key_run_index(t, e, key, klen)?.is_some());
        self.main_total_by(t, &lo, &hi, vs_lo, item, successor)
    }

    /// `main_total` through the key's probes `lo` and `hi`, its order against an entry (`vs_lo`, the entry
    /// on the left) and whether it is an item of a key-run entry (`item`).
    fn main_total_by<'t, P: Probe<RunNodes<'t, I>>>(
        &self,
        t: &RunNodes<'t, I>,
        lo: &P,
        hi: &P,
        vs_lo: impl Fn(&EntryRef<'_>) -> Result<Ordering, Fault>,
        item: impl Fn(&EntryRef<'_>) -> Result<bool, Fault>,
        successor: bool,
    ) -> Result<i64, Fault> {
        let (leaf, j) = match t.locate(hi)? {
            Search::Found(n, i) => return t.total(&t.entry(n, i)?),
            Search::Vacant(n, j) => (n, j),
        };
        // Most keys' entries and the entry after them lie in the leaf the search ends in: walk back there
        // first, with no path. Only a walk that reaches the leaf's first entry, or a position past its last
        // entry whose successor is wanted, needs the path, and takes the descent below.
        let count = t.node_info(leaf)?.1;
        if !(successor && j >= count) {
            let mut sum = 0i64;
            let mut i = j.min(count);
            let mut seen = 0u64;
            let done = loop {
                let Some(k) = i.checked_sub(1) else { break false };
                let e = t.entry(leaf, k)?;
                if vs_lo(&e)? != Ordering::Greater {
                    break true;
                }
                seen = seen.wrapping_add(1);
                if seen >= WALK_MAX {
                    return Err(self.count_err(leaf));
                }
                let x = if item(&e)? { 1 } else { t.contribution(&e)? };
                sum = sum.checked_add(x).ok_or(self.count_err(leaf))?;
                i = k;
            };
            if done {
                if successor && item(&t.entry(leaf, j)?)? {
                    sum = sum.checked_add(1).ok_or(self.count_err(leaf))?;
                }
                return Ok(sum);
            }
        }
        let mut path = Path::new();
        let (leaf, j) = match t.descend(hi, &mut path)? {
            Search::Found(n, i) => return t.total(&t.entry(n, i)?),
            Search::Vacant(n, j) => (n, j),
        };
        // The entry after the position, read off the path before the reverse walk takes it, so that the
        // path is never copied: entry j of the leaf, else the separator of the lowest ancestor whose child
        // taken is not its last.
        let after = if successor { after_position(t, &path, leaf, j)? } else { None };
        // With the leaf's frame (leaf, j), a reverse cursor has entry j − 1 next, as a seek from above the
        // probe would leave it.
        t.depth(path.push(leaf, j))?;
        let mut c: Cursor<Rev> = Cursor::from_path(path);
        let mut sum = 0i64;
        let mut seen = 0u64;
        while let Some((n, i)) = t.depth(c.next(t.shape(), t, Excluded(lo)))? {
            seen = seen.wrapping_add(1);
            if seen >= WALK_MAX {
                return Err(self.count_err(n));
            }
            let e = t.entry(n, i)?;
            let x = if item(&e)? { 1 } else { t.contribution(&e)? };
            sum = sum.checked_add(x).ok_or(self.count_err(n))?;
        }
        if let Some((n, i)) = after {
            if item(&t.entry(n, i)?)? {
                sum = sum.checked_add(1).ok_or(self.count_err(n))?;
            }
        }
        Ok(sum)
    }

    /// The count of `key` (its full encoding) in this run: 0 when the fences or the filter exclude it; else
    /// its count-tree entry when the run has a count tree; else its count entry, or the sum of its at most 7
    /// entries, an item of a key run counting 1.
    pub fn count_key(&self, key: &[u8]) -> Result<i64, Fault> {
        let full = key;
        // The key's first p bytes against the shared prefix, then its rest against the stored keys.
        let Ok(key) = self.strip(full)? else { return Ok(0) };
        let t = self.main()?;
        let w = usize::try_from(self.hdr.w).map_err(|_| corrupt(self.id(), Corruption::Header, at::W))?;
        let lo = KeyProbe { key, kind: Kind::Low };
        let hi = KeyProbe { key, kind: Kind::Count };
        let (area, field, value) = self.fence(false)?;
        // A key run's first items lie below its stored key, so such a fence bounds nothing below.
        let run_first = self.hdr.v >= 8 && !is_count(field) && value.first() == Some(&KEY_RUN);
        if !run_first && cmp_entry(&t, w, area, field, &hi, 0)?.0 == Ordering::Greater {
            return Ok(0);
        }
        let (area, field, _) = self.fence(true)?;
        if cmp_entry(&t, w, area, field, &lo, 0)?.0 == Ordering::Less {
            return Ok(0);
        }
        if !self.filter_admits(full)? {
            return Ok(0);
        }
        if let Some(ct) = self.count_tree()? {
            return match ct.find(&hi)? {
                Some((n, i)) => ct.total(&ct.entry(n, i)?),
                None => Ok(0),
            };
        }
        let klen = u64::try_from(key.len()).unwrap_or(u64::MAX);
        self.main_total(&t, key, klen, true)
    }

    /// The next distinct key after `after` (a key as this reader hands them out; from the first when `None`)
    /// in direction `dir`, written into `out` with the shared prefix re-attached (then the stored key: its
    /// W-byte key area when it fits W, else in full), with its total. `key_run_len` is the length of the keys
    /// of the index's key runs, needed only when those keys fit W (their length is not stored then). `None`
    /// past the last key.
    pub fn key_step(
        &self,
        after: Option<&[u8]>,
        dir: Dir,
        key_run_len: u64,
        out: &mut [u8],
    ) -> Result<Option<(usize, i64)>, Fault> {
        // `after` past the prefix; a key outside the run's prefix starts the step from an end, or ends it.
        let after = match after.map(|a| self.strip(a)).transpose()? {
            None => None,
            Some(Ok(rest)) => Some(rest),
            Some(Err(o)) => match (o, dir) {
                (Ordering::Less, Dir::Fwd) | (Ordering::Greater, Dir::Rev) => None,
                _ => return Ok(None),
            },
        };
        let pl = self.prefix_into(out)?;
        let out = out.get_mut(pl..).unwrap_or(&mut []);
        // `Scratch` names the bytes `out` needs, the prefix's included.
        let step = self.stored_step(after, dir, key_run_len, out).map_err(|f| match f {
            Fault::Scratch { need } => Fault::Scratch { need: need.saturating_add(pl as u64) },
            f => f,
        })?;
        Ok(step.map(|(n, t)| (n.saturating_add(pl), t)))
    }

    /// `key_step` among the stored keys: `after` and the key written are stored keys.
    fn stored_step(
        &self,
        after: Option<&[u8]>,
        dir: Dir,
        key_run_len: u64,
        out: &mut [u8],
    ) -> Result<Option<(usize, i64)>, Fault> {
        let probe = after.map(|a| KeyProbe { key: a, kind: if dir == Dir::Fwd { Kind::Count } else { Kind::Low } });
        let start = match probe.as_ref() {
            Some(p) => Excluded(p),
            None => Unbounded,
        };
        if let Some(ct) = self.count_tree()? {
            let found = match dir {
                Dir::Fwd => first_from::<I, Fwd>(&ct, start)?,
                Dir::Rev => first_from::<I, Rev>(&ct, start)?,
            };
            let Some((n, i)) = found else { return Ok(None) };
            let e = ct.entry(n, i)?;
            if !is_count(e.field) {
                return Err(self.count_err(n));
            }
            let len = stored_key(&ct, &e, out)?;
            return Ok(Some((len, ct.total(&e)?)));
        }
        let t = self.main()?;
        if let (Dir::Rev, Some(a)) = (dir, after) {
            // `after` may be an inner item of the key run that follows it: the step is the item before.
            let p = KeyProbe { key: a, kind: Kind::Low };
            if let Some((n, i)) = first_from::<I, Fwd>(&t, Included(&p))? {
                let e = t.entry(n, i)?;
                if let Some((l, nm1)) = key_run_of(&t, &e)? {
                    let len = run_keys_len(&t, &e, l, key_run_len)?;
                    if let Some(k) = key_run_index(&t, &e, a, len)? {
                        if k > 0 {
                            let fe = entry_field(&t, &e, l, len)?;
                            let f = fe.wrapping_sub(u64::from(nm1.wrapping_sub(k))).wrapping_sub(1);
                            return Ok(Some((key_run_member(&t, &e, l, len, f, out)?, 1)));
                        }
                    }
                }
            }
        }
        let found = match dir {
            Dir::Fwd => first_from::<I, Fwd>(&t, start)?,
            Dir::Rev => first_from::<I, Rev>(&t, start)?,
        };
        let Some((n, i)) = found else { return Ok(None) };
        let e = t.entry(n, i)?;
        if let Some((l, nm1)) = key_run_of(&t, &e)? {
            let len = run_keys_len(&t, &e, l, key_run_len)?;
            let fe = entry_field(&t, &e, l, len)?;
            let f = match dir {
                Dir::Rev => fe,
                Dir::Fwd => {
                    let first = fe.checked_sub(u64::from(nm1)).ok_or(corrupt(self.id(), Corruption::Range, n))?;
                    match after {
                        Some(a) => match key_run_index(&t, &e, a, len)? {
                            Some(k) if k < nm1 => first.wrapping_add(u64::from(k)).wrapping_add(1),
                            Some(_) => return Err(corrupt(self.id(), Corruption::Order, n)),
                            None => first,
                        },
                        None => first,
                    }
                }
            };
            return Ok(Some((key_run_member(&t, &e, l, len, f, out)?, 1)));
        }
        let len = stored_key(&t, &e, out)?;
        let key = out.get(..len).unwrap_or(&[]);
        let total = self.main_total(&t, key, u64::try_from(len).unwrap_or(0), false)?;
        Ok(Some((len, total)))
    }

    /// The first (with `largest`, the last) key whose total is positive and whose encoding does not start
    /// with the null tag 00, as `key_step` writes it, at the start of `scratch`, with its total; `scratch` is
    /// split in two halves for the walk. Each step must move strictly on, else `Corrupt(Order)`.
    pub fn extreme_key(
        &self,
        largest: bool,
        key_run_len: u64,
        scratch: &mut [u8],
    ) -> Result<Option<(usize, i64)>, Fault> {
        let half = scratch.len() / 2;
        let (a, b) = scratch.split_at_mut_checked(half).ok_or(Fault::Scratch { need: 0 })?;
        let dir = if largest { Dir::Rev } else { Dir::Fwd };
        let mut have: Option<usize> = None;
        loop {
            let prev = have.map(|n| a.get(..n).unwrap_or(&[]));
            let Some((n, total)) = self.key_step(prev, dir, key_run_len, b)? else { return Ok(None) };
            let next = b.get(..n).unwrap_or(&[]);
            if let Some(p) = prev {
                let moved = if largest { next < p } else { next > p };
                if !moved {
                    return Err(corrupt(self.id(), Corruption::Order, 0));
                }
            }
            let need = u64::try_from(n).unwrap_or(0).saturating_mul(2);
            copy_bytes(a.get_mut(..n).ok_or(Fault::Scratch { need })?, next);
            have = Some(n);
            if total > 0 && next.first() != Some(&0) {
                return Ok(Some((n, total)));
            }
        }
    }

    /// The smallest absolute rid above `after` (the smallest of all when `None`) among the run's rows of `key`
    /// (its full encoding): the row entries of the key from that rid on, a posting chunk decoded from its first
    /// rid, or the key's item of a key run. `None` when the run holds no row of the key above `after`. A run of
    /// an uploaded segment holds rows only: a tombstone of the key is `Corrupt(Encoding)`.
    pub fn row_after(&self, key: &[u8], after: Option<u64>) -> Result<Option<u64>, Fault> {
        let Ok(key) = self.strip(key)? else { return Ok(None) };
        let t = self.main()?;
        let w = usize::try_from(self.hdr.w).map_err(|_| corrupt(self.id(), Corruption::Header, at::W))?;
        let klen = u64::try_from(key.len()).unwrap_or(u64::MAX);
        // A row entry's position is its key and its rid, a posting chunk's its last rid: the first entry past
        // (key, after) is the first that may hold a rid above it.
        let p = KeyProbe { key, kind: after.map_or(Kind::Low, Kind::Row) };
        let start = if after.is_some() { Excluded(&p) } else { Included(&p) };
        let end = KeyProbe { key, kind: Kind::Count };
        let mut c: Cursor<Fwd> = t.seek(start)?;
        while let Some((n, i)) = t.next(&mut c, Unbounded)? {
            let e = t.entry(n, i)?;
            match cmp_entry(&t, w, e.area, e.field, &end, 0)?.0 {
                Ordering::Less if is_tomb(e.field) => return Err(corrupt(self.id(), Corruption::Encoding, n)),
                Ordering::Less => {
                    if let Some(r) = self.entry_row_after(&t, &e, key, klen, after)? {
                        return Ok(Some(r));
                    }
                }
                // The key's count entry follows its rows.
                Ordering::Equal => return Ok(None),
                // Past the key's entries: the key may be an inner item of the key run this entry ends.
                Ordering::Greater if key_run_of(&t, &e)?.is_some() => {
                    return self.entry_row_after(&t, &e, key, klen, after)
                }
                Ordering::Greater => return Ok(None),
            }
        }
        Ok(None)
    }

    /// The smallest absolute rid above `after` among the rows of `key` (its stored form, `klen` bytes) that
    /// entry e holds: its rid, its posting chunk's rids, or the key's item when it ends a key run holding it.
    fn entry_row_after(
        &self,
        t: &RunNodes<'_, I>,
        e: &EntryRef<'_>,
        key: &[u8],
        klen: u64,
        after: Option<u64>,
    ) -> Result<Option<u64>, Fault> {
        let id = self.id();
        let stored = if is_ovf(e.field) { self.record(e.field)?.payload } else { payload(e.field) };
        let abs = |r: u64| -> Result<u64, Fault> {
            let a = chunk::abs_rid(r, self.rid_base, id, e.node)?;
            if contains(a, self.rid_lo, self.rid_hi) {
                Ok(a)
            } else {
                Err(corrupt(id, Corruption::Range, e.node))
            }
        };
        let above = |r: u64| after.is_none_or(|a| r > a);
        if let Some((_, nm1)) = key_run_of(t, e)? {
            let Some(k) = key_run_index(t, e, key, klen)? else { return Ok(None) };
            let r = abs(stored.checked_sub(u64::from(nm1.wrapping_sub(k))).ok_or(self.count_err(e.node))?)?;
            return Ok(Some(r).filter(|&r| above(r)));
        }
        if is_count(e.field) {
            return Ok(None);
        }
        if t.shape().v == 0 {
            return Ok(Some(abs(stored)?).filter(|&r| above(r)));
        }
        let Chunk::Posting { m, b } = chunk_header(e.value, id, e.node)? else { return Ok(None) };
        let mut r = chunk::first(e.value, m, b, stored, id, e.node)?;
        let mut k = 0u8;
        loop {
            let a = abs(r)?;
            if above(a) {
                return Ok(Some(a));
            }
            if k == m {
                return Ok(None);
            }
            k = k.wrapping_add(1);
            let gap = chunk::code(e.value, m, b, k, id, e.node)?;
            r = r.checked_add(gap).ok_or(corrupt(id, Corruption::Range, e.node))?;
        }
    }
}
