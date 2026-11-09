//! Run probes: the facts a count of one key reads of each listed run, written when the run is listed, so that
//! a count reads no run header, re-checks no geometry and copies no structure.
//!
//! **The header is checked once.** A run's image never changes once it is listed. Its header's geometry (the
//! same checks an uploaded run's header check runs: version, length, shape, node counts, overflow area,
//! filter) is checked when its record is written, which is when the run is listed (`RunList::loaded_append`).
//! A run whose header fails gets no record, and every count of it opens it from its header, which faults. A
//! count trusts the record as it trusts the run's descriptor and never re-checks the geometry. Every byte it
//! reads of the image is still range-checked against the image's length, its 64 KiB blocks and its block
//! entries, and every node it reads is checked (a reference inside its tree's slots, flags, layout word, count
//! clamped to the capacity, the depth bound, chunk headers), so damaged memory gives a fault or a wrong count,
//! never a panic or a read outside the image. A header damaged after its run was listed is not read by a count
//! through the record, so it may give a wrong count rather than a fault.
//!
//! **Probe record** (64 bytes, little-endian):
//!
//! | Offset | Size | Field |
//! |---:|---:|---|
//! | 0 | 8 | start sequence of the run it describes |
//! | 8 | 8 | image root of the run it describes |
//! | 16 | 8 | the first fence's key bytes 0..8 as a big-endian number; 0 when the first entry is a key run, whose items lie below its stored key; 2^64 − 1 when the run stores a shared prefix |
//! | 24 | 8 | the last fence's key bytes 0..8 as a big-endian number; when the run stores a shared prefix, the prefix's first 8 bytes as a big-endian number, at most 2^64 − 2 (which then stands for both) |
//! | 32 | 8 | the filter's image offset (0: no filter) |
//! | 40 | 4 | the filter's lines |
//! | 44 | 4 | the length of the run's shared prefix, which its header stores; for a run that stores none, the bytes its fences share (below): 0 when the first entry is a key run |
//! | 48 | 4 | the main tree's nodes |
//! | 52 | 4 | the count tree's nodes (0: no count tree) |
//! | 56 | 8 | the image's length |
//!
//! A record names its run when its start sequence and image root equal the descriptor's; a record that names
//! no run (all zero, or left from another run) sends the count to the run's header, as before records
//! existed. A run whose header fails its checks, or whose node counts or filter lines do not fit 32 bits,
//! gets an all-zero record, and so does one whose shared prefix does not fit 32 bits. The layout is that of
//! `FORMAT_TABLE` 3: version 1 held the lines in bytes 40..48, and a filter of 2^32 lines or more (one an image
//! can hold) set the high half this layout reads at 44; version 2 held there the bytes the run's fences share
//! for every run. Such records are never read: a count refuses a run list of another version with a header
//! fault.
//!
//! **The fences' shared bytes**, of a run that stores no shared prefix: the bytes its first and last keys
//! share in the comparator's byte sequence (the key area, the key zero-padded to W, then an overflowed key's
//! bytes beyond W, read from its overflow record), so every entry shares them; a listing looks at no more than
//! `PREFIX_SCAN` of them (or W). A run whose first entry is a key run records 0: its items lie below its
//! stored key, which then bounds nothing.
//!
//! **Probe store**: a radix array of area-0 blocks, its root and height in the run list's loaded-run record
//! (0: no record yet). Entry 1 + ⌊j / 64⌋ is the block of loaded runs j, record j at offset 64·(j mod 64), as
//! the loaded-run list lays out their descriptors; entry 0 is absent. A missing block sends the count of its
//! runs to their headers.
//!
//! **The count** of a key over the runs, per run: a run whose first 8 key bytes lie outside the fences' counts
//! 0 before the key is hashed; then the filter's one line; then, in a run with a shared prefix, the key's first
//! p bytes against the prefix the header holds (a key without them is in no entry), and everything after with
//! the key's rest against the stored keys (the trees hold the keys past the prefix); then one descent, of the
//! count tree when the run has one, else of the main tree, comparing key areas a word at a time; a key without a count entry sums the
//! entries before its position in the leaf the descent ends in, with the separators on the descent's way
//! standing in for the entries before and after the leaf, and a second descent keeping its path when the
//! key's entries reach into an earlier leaf. A key run counts 1 when the key is one of its items, as when the
//! run is opened. A key that fits W and 64 bytes is compared through its words alone. Any other key (longer
//! than W, or than 64 bytes) reads its further words from its bytes, compares the bytes beyond W of an
//! overflowed entry where its overflow record puts them (the record directory's place and size read from the
//! header once per run, as its listing checked them), and skips the positions a descent's lower and upper
//! bounds already share with it, the run's first and last keys (its fences) being the first bounds: the key
//! is compared with the first fence in full once per run; in a run without a shared prefix, a key that
//! differs from it inside the fences' shared bytes lies outside the run, and the last fence is compared from
//! their end. A key of more than 8 bytes that fits W and 64 bytes skips the
//! first word of every key area when the fences' first 8 bytes are equal (in a run without a prefix), and so
//! the key's. Whatever that cannot answer (a tree deeper than 16 internal levels under a walk across leaves,
//! any check that fails) is counted by opening the run from its header, which gives the same answer or its
//! own fault.

use core::cmp::Ordering;

use crate::area::alloc_zeroed;
use crate::consts::{magic, BLOCK, FLAG_INDEX, FLAG_LEAF, FORMAT_TABLE, PAGE, PATH_STACK, RADIX_FAN, WALK_MAX};
use crate::fault::{rt, Corruption, Fault, ImageId};
use crate::image::ImageReader;
use crate::index::chunk::{key_field, B_MAX, KEY_RUN, POSTING};
use crate::index::nodes::Run;
use crate::index::run::{
    at as rh, filter_hash, filter_line, record_offset, RunHeader, KIND_RUN, LINE_BYTES, RECORDS_PER_BLOCK, RECORD_BYTES,
};
use crate::index::runlist::{LoadedRecord, RunDesc, DESC, LOADED_PER_BLOCK};
use crate::key::probe::{is_count, payload, payload_to_count};
use crate::key::words::{area_cmp, bytes_cmp, bytes_cmp_any, diff_byte, word_below, word_padded, KeyWords, KEY_BYTES};
use crate::mem::{put_u32, put_u64, rd_u16, rd_u32, rd_u64, rd_u8, PageHost, Region};
use crate::radix::Radix;
use crate::tree::shape::index_node_shape;

/// Bytes of a probe record.
pub const PROBE: u64 = 64;

const _: () = assert!(LOADED_PER_BLOCK * PROBE == BLOCK && PROBE == DESC);

/// One probe record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RunProbe {
    pub seq: u64,
    pub root: u64,
    /// The first key's first 8 bytes, big-endian; 0 when they bound nothing.
    pub lo: u64,
    /// The last key's first 8 bytes, big-endian.
    pub hi: u64,
    /// 0: no filter.
    pub filter: u64,
    /// Below 2^32.
    pub lines: u64,
    /// The shared prefix's length, which the header stores; for a run that stores none, the bytes its fences
    /// share (below 2^32).
    pub prefix: u64,
    pub nodes: u32,
    /// 0: no count tree.
    pub ct_nodes: u32,
    pub len: u64,
}

/// The big-endian number of bytes [off, off + 8) of `b`.
fn be_at(b: &[u8], off: u64) -> Result<u64, Fault> {
    crate::mem::rd_array::<8>(b, off).map(u64::from_be_bytes)
}

impl RunProbe {
    /// The record that names no run.
    pub const NONE: RunProbe =
        RunProbe { seq: 0, root: 0, lo: 0, hi: 0, filter: 0, lines: 0, prefix: 0, nodes: 0, ct_nodes: 0, len: 0 };

    /// The record at `off` of `b`.
    pub fn read(b: &[u8], off: u64) -> Result<RunProbe, Fault> {
        let at = |k: u64| off.checked_add(k).ok_or(rt(Corruption::Bounds, off));
        Ok(RunProbe {
            seq: rd_u64(b, at(0)?)?,
            root: rd_u64(b, at(8)?)?,
            lo: rd_u64(b, at(16)?)?,
            hi: rd_u64(b, at(24)?)?,
            filter: rd_u64(b, at(32)?)?,
            lines: u64::from(rd_u32(b, at(40)?)?),
            prefix: u64::from(rd_u32(b, at(44)?)?),
            nodes: rd_u32(b, at(48)?)?,
            ct_nodes: rd_u32(b, at(52)?)?,
            len: rd_u64(b, at(56)?)?,
        })
    }

    /// Writes all 64 bytes of the record at `off` of `b`.
    pub fn write(&self, b: &mut [u8], off: u64) -> Result<(), Fault> {
        let at = |k: u64| off.checked_add(k).ok_or(rt(Corruption::Bounds, off));
        put_u64(b, at(0)?, self.seq)?;
        put_u64(b, at(8)?, self.root)?;
        put_u64(b, at(16)?, self.lo)?;
        put_u64(b, at(24)?, self.hi)?;
        put_u64(b, at(32)?, self.filter)?;
        put_u32(b, at(40)?, u32::try_from(self.lines).map_err(|_| rt(Corruption::Range, self.lines))?)?;
        put_u32(b, at(44)?, u32::try_from(self.prefix).map_err(|_| rt(Corruption::Range, self.prefix))?)?;
        put_u32(b, at(48)?, self.nodes)?;
        put_u32(b, at(52)?, self.ct_nodes)?;
        put_u64(b, at(56)?, self.len)
    }

    /// Whether this record describes the run of descriptor `d`.
    pub fn names(&self, d: &RunDesc) -> bool {
        self.root != 0 && self.root == d.root && self.seq == d.start_seq
    }

    /// The record of the run of descriptor `d`, from its header: magic, kind and geometry checked as opening
    /// the run checks them, and its W and V equal to the descriptor's. `NONE` when any of that fails.
    pub fn of<H: PageHost + ?Sized>(h: &H, d: &RunDesc) -> RunProbe {
        Self::from_header(h, d).unwrap_or(RunProbe::NONE)
    }

    fn from_header<H: PageHost + ?Sized>(h: &H, d: &RunDesc) -> Result<RunProbe, Fault> {
        let bad = rt(Corruption::Header, d.root);
        if d.root == 0 {
            return Err(bad);
        }
        let img = d.image(h)?;
        let head = ImageReader::new(h, ImageId::Runtime, img).bytes(0, BLOCK)?;
        if rd_u32(head, rh::MAGIC)? != magic::NTIM || rd_u8(head, rh::KIND)? != KIND_RUN {
            return Err(bad);
        }
        let hdr = RunHeader::read(head)?;
        hdr.geometry(img.length).map_err(|_| bad)?;
        if hdr.w != u64::from(d.w) || hdr.v != u64::from(d.v) {
            return Err(bad);
        }
        // The first entry: its field after its W-byte key area, its value part after the field.
        let field_at = rh::FENCE_FIRST.checked_add(hdr.w).ok_or(bad)?;
        let field = be_at(head, field_at)?;
        let kind = rd_u8(head, field_at.checked_add(8).ok_or(bad)?)?;
        let run_first = hdr.v >= 8 && !is_count(field) && kind == KEY_RUN;
        // A run that stores a shared prefix holds keys of more than 8 bytes only: a range no key lies in sends
        // every key of at most 8 bytes away at the first check, and a longer one to the prefix, whose first 8
        // bytes `hi` holds.
        let (lo, hi) = if hdr.prefix > 0 {
            (u64::MAX, crate::index::run::prefix_head(head, hdr.prefix)?.min(PREFIX_HEAD_ANY))
        } else {
            (if run_first { 0 } else { be_at(head, rh::FENCE_FIRST)? }, be_at(head, rh::FENCE_LAST)?)
        };
        let (filter, lines) = if hdr.has_filter() { (hdr.filter, hdr.lines) } else { (0, 0) };
        let ct = if hdr.has_count_tree() { hdr.ct_nodes } else { 0 };
        let prefix = if hdr.prefix > 0 {
            hdr.prefix
        } else if run_first {
            0
        } else if lo != hi {
            // The fences' first 8 bytes differ: they share the bytes before the first that differs.
            u64::from((lo ^ hi).leading_zeros() / 8)
        } else {
            fences_shared(&ImageReader::new(h, ImageId::Runtime, img), &hdr, head)?
        };
        if lines > u64::from(u32::MAX) || prefix > u64::from(u32::MAX) {
            return Err(bad);
        }
        Ok(RunProbe {
            seq: d.start_seq,
            root: d.root,
            lo,
            hi,
            filter,
            lines,
            prefix,
            nodes: u32::try_from(hdr.nodes).map_err(|_| bad)?,
            ct_nodes: u32::try_from(ct).map_err(|_| bad)?,
            len: img.length,
        })
    }
}

/// The most bytes of the fences a record's shared bytes are taken over, which bounds the work of a listing.
pub const PREFIX_SCAN: u64 = 4_096;

/// The bytes the run's first and last keys share in the comparator's byte sequence (the W-byte key area,
/// then an overflowed key's bytes beyond W, read from their overflow records), at most `PREFIX_SCAN`. `head`
/// is the image's first 4 KiB, which holds both fences' key parts.
fn fences_shared<H: PageHost + ?Sized>(rd: &ImageReader<'_, H>, hdr: &RunHeader, head: &[u8]) -> Result<u64, Fault> {
    let bad = rt(Corruption::Header, rh::FENCE_FIRST);
    let w = hdr.w;
    let area = |at: u64| -> Result<(&[u8], u64), Fault> {
        let a = crate::mem::sub(head, at, w)?;
        Ok((a, be_at(head, at.checked_add(w).ok_or(bad)?)?))
    };
    let (a, fa) = area(rh::FENCE_FIRST)?;
    let (b, fb) = area(rh::FENCE_LAST)?;
    let (o, n) = bytes_cmp(a, b);
    if o != Ordering::Equal {
        return Ok(n as u64);
    }
    let ovf = crate::consts::F_OVF;
    if fa & ovf == 0 || fb & ovf == 0 {
        return Ok(w);
    }
    // Both keys are longer than W: their bytes beyond W, where their overflow records put them.
    let rec = |field: u64| -> Result<(u64, u64), Fault> {
        let x = payload(field);
        if x >= hdr.records {
            return Err(bad);
        }
        let r = rd.bytes(record_offset(hdr.oa, x).ok_or(bad)?, RECORD_BYTES)?;
        Ok((crate::mem::rd_u64(r, 8)?, crate::mem::rd_u64(r, 16)?))
    };
    let (ka, la) = rec(fa)?;
    let (kb, lb) = rec(fb)?;
    let end = la.min(lb).min(PREFIX_SCAN.max(w));
    let mut at = w;
    while at < end {
        // A piece of each within one image block.
        let (oa, ob) = (ka.wrapping_add(at.wrapping_sub(w)), kb.wrapping_add(at.wrapping_sub(w)));
        let room = |o: u64| PAGE.wrapping_sub(o % PAGE);
        let n = end.wrapping_sub(at).min(room(oa)).min(room(ob));
        let (o, s) = bytes_cmp_any(rd.bytes(oa, n)?, rd.bytes(ob, n)?);
        if o != Ordering::Equal {
            return Ok(at.wrapping_add(s as u64));
        }
        at = at.wrapping_add(n);
    }
    Ok(end)
}

// ---------------------------------------------------------------- the store

/// Writes the record of loaded run j (descriptor `d`) unless the one there already names it, making the
/// store's block for j when absent.
pub fn set_loaded<H: PageHost>(h: &mut H, tbl: u64, store: &mut Radix, j: u64, d: &RunDesc) -> Result<(), Fault> {
    let e = (j / LOADED_PER_BLOCK).checked_add(1).ok_or(rt(Corruption::Range, j))?;
    let mut b = store.get(h, e)?;
    if b == 0 {
        b = alloc_zeroed(h, tbl, 0)?;
        store.set(h, tbl, e, b)?;
    }
    let off = (j % LOADED_PER_BLOCK).wrapping_mul(PROBE);
    let cur = h.get(Region::Block(b))?;
    if d.root != 0 && rd_u64(cur, off.wrapping_add(8))? == d.root && rd_u64(cur, off)? == d.start_seq {
        return Ok(());
    }
    let r = RunProbe::of(&*h, d);
    r.write(h.get_mut(Region::Block(b))?, off)
}

// ---------------------------------------------------------------- the count

/// The high byte of an entry field (stored big-endian): its flag bits.
const TOP_OVF: u8 = 0x80;
const TOP_COUNT: u8 = 0x40;
const TOP_TOMB: u8 = 0x20;

/// A filter line's bytes.
// LINE_BYTES = 64.
#[allow(clippy::cast_possible_truncation)]
const LINE: usize = LINE_BYTES as usize;

/// A node slot's bytes.
// BLOCK = 4,096.
#[allow(clippy::cast_possible_truncation)]
const NODE: usize = BLOCK as usize;
/// Key areas start at 8 + i·(W + 8), a multiple of 8 below the node's 4,080 bytes: masking with this keeps
/// such an offset as it is and bounds every read at it inside the slot.
const KO_MASK: usize = NODE - 8;

/// A node slot, read where it lies.
type Node<'h> = &'h [u8; NODE];

/// A key prepared once for a count over many runs.
struct CountKey<'k> {
    key: &'k [u8],
    klen: usize,
    /// The first 8 bytes, zero-padded, as a big-endian number: the fences' order.
    k8: u64,
    /// The key zero-padded, as little-endian words (meaningful for keys of at most 64 bytes): a key that fits
    /// W and these is compared through them alone; a longer key's further words are read from its bytes.
    words: KeyWords,
    /// The key's filter hash and the byte and mask of each of its seven bits in a line, once `hashed`.
    hash: u64,
    bits: [(u8, u8); 7],
    hashed: bool,
}

impl<'k> CountKey<'k> {
    fn new(key: &'k [u8]) -> CountKey<'k> {
        let words = KeyWords::of(key);
        let k8 = words.get(0).swap_bytes();
        CountKey { key, klen: key.len(), k8, words, hash: 0, bits: [(0, 0); 7], hashed: false }
    }

    /// The key past its first p bytes, `key` (p a whole number of words below the key's length), prepared as
    /// `new` prepares it: its words are this key's from word p/8 on when this key's words hold it whole.
    #[inline(always)]
    fn past(&self, key: &'k [u8], p: usize) -> CountKey<'k> {
        let words = if self.klen <= KEY_BYTES {
            let sh = p / 8;
            let mut w = KeyWords::default();
            for (i, slot) in w.w.iter_mut().enumerate() {
                *slot = self.words.get(i.wrapping_add(sh));
            }
            w
        } else {
            KeyWords::of(key)
        };
        let k8 = words.get(0).swap_bytes();
        CountKey { key, klen: key.len(), k8, words, hash: 0, bits: [(0, 0); 7], hashed: false }
    }

    /// Makes the key's filter hash and bit places, on first use.
    #[inline(always)]
    fn hash(&mut self) {
        if !self.hashed {
            self.hash_now();
        }
    }

    #[inline(never)]
    fn hash_now(&mut self) {
        let (h, bits) = filter_hash(self.key);
        for (slot, b) in self.bits.iter_mut().zip(bits) {
            // b < 512: its byte is below 64.
            #[allow(clippy::cast_possible_truncation)]
            let byte = (b / 8) as u8;
            *slot = (byte, 1u8.wrapping_shl(u32::from(b % 8)));
        }
        self.hash = h;
        self.hashed = true;
    }
}

#[inline(always)]
fn le64(s: &[u8], o: usize) -> Option<u64> {
    s.get(o..)?.first_chunk::<8>().map(|x| u64::from_le_bytes(*x))
}

#[inline(always)]
fn le32(s: &[u8], o: usize) -> Option<u32> {
    s.get(o..)?.first_chunk::<4>().map(|x| u32::from_le_bytes(*x))
}

#[inline(always)]
fn le16(s: &[u8], o: usize) -> Option<u16> {
    s.get(o..)?.first_chunk::<2>().map(|x| u16::from_le_bytes(*x))
}

#[inline(always)]
fn be64(s: &[u8], o: usize) -> Option<u64> {
    s.get(o..)?.first_chunk::<8>().map(|x| u64::from_be_bytes(*x))
}

/// The word at a masked key offset of a node (always inside it).
#[inline(always)]
fn key_word(s: Node<'_>, koff: usize) -> u64 {
    le64(s, koff & KO_MASK).unwrap_or(0)
}

/// A 64-byte descriptor or record read where it lies.
type Slot<'a> = &'a [u8; 64];

#[inline(always)]
fn f64_at(s: Slot<'_>, o: usize) -> u64 {
    le64(s, o).unwrap_or(0)
}

#[inline(always)]
fn f32_at(s: Slot<'_>, o: usize) -> u64 {
    le32(s, o).map_or(0, u64::from)
}

#[inline(always)]
fn f16_at(s: Slot<'_>, o: usize) -> u64 {
    le16(s, o).map_or(0, u64::from)
}

/// The image's bytes through its block entries, the last page read kept.
struct Img<'h, H: PageHost + ?Sized> {
    h: &'h H,
    root: u64,
    height: u8,
    len: u64,
    /// The radix block of an image of height 1, read on first use.
    radix: &'h [u8],
    /// The image block whose page is held; `u64::MAX` none.
    k: u64,
    page: &'h [u8],
}

impl<'h, H: PageHost + ?Sized> Img<'h, H> {
    #[inline(always)]
    fn new(h: &'h H, root: u64, height: u8, len: u64) -> Img<'h, H> {
        Img { h, root, height, len, radix: &[], k: u64::MAX, page: &[] }
    }

    /// The page of image block k.
    #[inline(always)]
    fn block(&mut self, k: u64) -> Option<&'h [u8]> {
        let p = match self.height {
            0 => {
                if k == 0 {
                    self.root
                } else {
                    0
                }
            }
            1 => {
                if self.radix.is_empty() {
                    self.radix = self.h.get(Region::Block(self.root)).ok()?;
                }
                if k >= RADIX_FAN {
                    0
                } else {
                    // k < 512.
                    le64(self.radix, usize::try_from(k).ok()?.wrapping_mul(8))?
                }
            }
            _ => Radix { root: self.root, height: self.height }.get(self.h, k).ok()?,
        };
        if p == 0 {
            return None;
        }
        self.h.get(Region::Page(p)).ok()
    }

    /// Bytes [off, off + n) inside the image's length and one 64 KiB image block.
    #[inline]
    fn read(&mut self, off: u64, n: u64) -> Option<&'h [u8]> {
        let o = off % PAGE;
        if off.checked_add(n)? > self.len || o.checked_add(n)? > PAGE {
            return None;
        }
        let k = off / PAGE;
        if k != self.k {
            self.page = self.block(k)?;
            self.k = k;
        }
        let a = usize::try_from(o).ok()?;
        self.page.get(a..a.checked_add(usize::try_from(n).ok()?)?)
    }

    /// Node slot n (a multiple of 4,096 inside one image block).
    #[inline(always)]
    fn slot(&mut self, n: u64) -> Option<Node<'h>> {
        self.read(n, BLOCK)?.first_chunk::<NODE>()
    }
}

/// An image as the count of a key longer than W or 64 bytes reads it: the nodes through `img`, and overflow
/// records and key bytes through a second page held, so that reading them leaves the nodes' page held.
struct ImgX<'a, 'h, H: PageHost + ?Sized> {
    img: &'a mut Img<'h, H>,
    /// The record directory's block read last, cached for the run's count: the page of 64 KiB directory block
    /// k (from OA), and its first record's number, 2,730·k (`NO_DIR` before the first, which no record's number
    /// lies within 2,730 of).
    dir: &'h [u8],
    dir_lo: u64,
    /// The page held for overflowed keys' bytes.
    keys: Held<'h>,
    /// The header's overflow record count and directory offset, once read; `oa` is `u64::MAX` before.
    recs: u64,
    oa: u64,
}

/// `ImgX::dir_lo` before a directory block is read: every record number (below 2^61) less it wraps past 2^63.
const NO_DIR: u64 = 1 << 63;

/// A page of an image held: its image block (`u64::MAX` none) and bytes.
struct Held<'h> {
    k: u64,
    page: &'h [u8],
}

impl<'h> Held<'h> {
    const NONE: Held<'h> = Held { k: u64::MAX, page: &[] };

    /// As `Img::read`, through this page.
    fn read<H: PageHost + ?Sized>(&mut self, img: &mut Img<'h, H>, off: u64, n: u64) -> Option<&'h [u8]> {
        let o = off % PAGE;
        if off.checked_add(n)? > img.len || o.checked_add(n)? > PAGE {
            return None;
        }
        let k = off / PAGE;
        if k != self.k {
            self.page = img.block(k)?;
            self.k = k;
        }
        let a = usize::try_from(o).ok()?;
        self.page.get(a..a.checked_add(usize::try_from(n).ok()?)?)
    }
}

impl<'a, 'h, H: PageHost + ?Sized> ImgX<'a, 'h, H> {
    fn new(img: &'a mut Img<'h, H>) -> ImgX<'a, 'h, H> {
        ImgX { img, dir: &[], dir_lo: NO_DIR, keys: Held::NONE, recs: 0, oa: u64::MAX }
    }

    /// Bytes of overflowed keys, as `Img::read` reads them.
    fn read_x(&mut self, off: u64, n: u64) -> Option<&'h [u8]> {
        self.keys.read(self.img, off, n)
    }

    /// The overflow record of an overflowed entry's field: its payload, its key bytes' offset and its key's
    /// length. The record directory's place and the record count are the header's, which the run's listing
    /// checked; `None` for a record past the count.
    fn record(&mut self, field: u64) -> Option<(u64, u64, u64)> {
        if self.oa == u64::MAX {
            let b = self.img.read(rh::OVF_RECORDS, 16)?;
            self.recs = le64(b, 0)?;
            self.oa = le64(b, 8)?;
        }
        let x = payload(field);
        if x >= self.recs {
            return None;
        }
        // Record x lies at 24·(x − 2,730·k) of directory block k (whose records the listing checked lie
        // inside the image).
        let mut rel = x.wrapping_sub(self.dir_lo);
        if rel >= RECORDS_PER_BLOCK {
            let k = x / RECORDS_PER_BLOCK;
            self.dir = self.img.block(self.oa.checked_div(PAGE)?.checked_add(k)?)?;
            self.dir_lo = k.wrapping_mul(RECORDS_PER_BLOCK);
            rel = x.wrapping_sub(self.dir_lo);
        }
        let o = usize::try_from(rel.wrapping_mul(RECORD_BYTES)).ok()?;
        let r = self.dir.get(o..o.checked_add(24)?)?;
        Some((le64(r, 0)?, le64(r, 8)?, le64(r, 16)?))
    }
}

/// The geometry of one tree's nodes (every figure below 4,096).
#[derive(Clone, Copy, Default)]
struct Geo {
    w: u16,
    v: u16,
    cap_leaf: u16,
    cap_internal: u16,
    layout: u32,
}

impl Geo {
    /// As the index shape (W, V) gives it; `None` unless (W, V) is one.
    fn new(w: u64, v: u64) -> Option<Geo> {
        let layout = crate::consts::form_code(w, v)?;
        let ks = w.checked_add(8)?;
        Some(Geo {
            w: u16::try_from(w).ok()?,
            v: u16::try_from(v).ok()?,
            cap_leaf: u16::try_from(index_node_shape(ks, v, false).cap).ok()?,
            cap_internal: u16::try_from(index_node_shape(ks, v, true).cap).ok()?,
            layout,
        })
    }

    #[inline(always)]
    fn w(&self) -> usize {
        usize::from(self.w)
    }

    #[inline(always)]
    fn v(&self) -> usize {
        usize::from(self.v)
    }

    #[inline(always)]
    fn wwords(&self) -> usize {
        usize::from(self.w / 8)
    }

    /// Key part: W + 8.
    #[inline(always)]
    fn ks(&self) -> usize {
        self.w().wrapping_add(8)
    }

    /// The key offset of entry i, i below a node's capacity: 8 + i·(W + 8).
    #[inline(always)]
    fn ko(&self, i: usize) -> usize {
        i.wrapping_mul(self.ks()).wrapping_add(8) & KO_MASK
    }

    /// The offset of entry i's field.
    #[inline(always)]
    fn fo(&self, i: usize) -> usize {
        self.ko(i).wrapping_add(self.w())
    }

    /// The value-part offset of entry i of a leaf (`leaf`) or an internal node: voff + i·V, voff = 8 + c·(W + 8)
    /// for the node's capacity c.
    #[inline(always)]
    fn vo(&self, leaf: bool, i: usize) -> usize {
        let cap = usize::from(if leaf { self.cap_leaf } else { self.cap_internal });
        cap.wrapping_mul(self.ks()).wrapping_add(8).wrapping_add(i.wrapping_mul(self.v()))
    }

    /// The offset of child j of an internal node: coff + 8j, coff = voff + c·V.
    #[inline(always)]
    fn co(&self, j: usize) -> usize {
        self.vo(false, usize::from(self.cap_internal)).wrapping_add(j.wrapping_mul(8))
    }
}

/// The geometries of the last main tree and the last count tree read, by (W, V): runs of one index mostly
/// share them.
struct Geos {
    keys: [u64; 2],
    geo: [Geo; 2],
}

impl Geos {
    fn new() -> Geos {
        Geos { keys: [u64::MAX; 2], geo: [Geo::default(); 2] }
    }

    #[inline(always)]
    fn get(&mut self, slot: usize, w: u64, v: u64) -> Option<Geo> {
        let key = w.wrapping_shl(16) | v;
        let k = self.keys.get_mut(slot)?;
        let g = self.geo.get_mut(slot)?;
        if *k != key {
            *g = Geo::new(w, v)?;
            *k = key;
        }
        Some(*g)
    }
}

/// An entry of a node passed on the way down, by its node's bytes and its index.
#[derive(Clone, Copy)]
struct Sep<'h> {
    node: Node<'h>,
    i: usize,
}

/// Where the descent for (key, `Count`) ended.
enum Pos<'h> {
    /// The key's count entry: its count.
    Count(i64),
    /// Position j of a leaf of c entries, and the separators before and after the leaf in key order (the
    /// lowest ancestor's entry left of the child taken, and right of it), when they exist.
    Leaf { s: Node<'h>, j: usize, c: usize, pred: Option<Sep<'h>>, succ: Option<Sep<'h>> },
}

/// The position of (key, `Count`) among the c entries of node `s`: `Ok(i)` when entry i is the key's count
/// entry, else `Err(j)`, j the entries below it. The lower midpoint, as the tree's search. With `SHARED0`,
/// every entry's first word is the key's and is not compared.
#[inline(always)]
fn search<const SHARED0: bool>(s: Node<'_>, c: usize, g: &Geo, k: &CountKey<'_>) -> Option<Result<usize, usize>> {
    let (mut lo, mut hi) = (0usize, c);
    let k0 = k.words.get(0);
    while lo < hi {
        // lo and hi are at most a node's capacity, below 4,096.
        let mid = lo.wrapping_add(hi).wrapping_shr(1);
        let koff = g.ko(mid);
        if !SHARED0 {
            let a = key_word(s, koff);
            if a != k0 {
                if word_below(a, k0) {
                    lo = mid.wrapping_add(1);
                } else {
                    hi = mid;
                }
                continue;
            }
        }
        let ord = if g.w > 8 { area_cmp(s, koff, 1, g.wwords(), &k.words)? } else { Ordering::Equal };
        match ord {
            Ordering::Less => lo = mid.wrapping_add(1),
            Ordering::Greater => hi = mid,
            Ordering::Equal => {
                let top = *s.get(g.fo(mid))?;
                if top & TOP_OVF != 0 {
                    // An overflowed key is longer than the key, which fits W: above it.
                    hi = mid;
                } else if top & TOP_COUNT != 0 {
                    return Some(Ok(mid));
                } else {
                    // A row or tombstone ranks below a count entry.
                    lo = mid.wrapping_add(1);
                }
            }
        }
    }
    Some(Err(lo))
}

/// Node n of the tree whose slots are [first, last] (byte offsets), checked as a run's node reader checks it:
/// its bytes, whether it is a leaf, and its count clamped to its capacity.
#[inline(always)]
fn node<'h, H: PageHost + ?Sized>(
    img: &mut Img<'h, H>,
    g: &Geo,
    n: u64,
    first: u64,
    last: u64,
) -> Option<(Node<'h>, bool, usize)> {
    if n % BLOCK != 0 || n < first || n > last {
        return None;
    }
    let s = img.slot(n)?;
    let fl = le16(s, 2)?;
    if (fl != FLAG_INDEX && fl != FLAG_INDEX | FLAG_LEAF) || le32(s, 4)? != g.layout {
        return None;
    }
    let leaf = fl & FLAG_LEAF != 0;
    let cap = if leaf { g.cap_leaf } else { g.cap_internal };
    Some((s, leaf, usize::from(le16(s, 0)?.min(cap))))
}

/// Child j of internal node `s`, j at most its count: inside the node.
#[inline(always)]
fn child(s: Node<'_>, g: &Geo, j: usize) -> Option<u64> {
    le64(s, g.co(j))
}

/// The descent for (key, `Count`) from `root` through slots [first, last] (byte offsets), every node checked
/// as a run's node reader checks it; with `SHARED0`, every entry's first word is the key's.
fn locate<'h, H: PageHost + ?Sized, const SHARED0: bool>(
    img: &mut Img<'h, H>,
    g: &Geo,
    root: u64,
    first: u64,
    last: u64,
    k: &CountKey<'_>,
) -> Option<Pos<'h>> {
    let mut n = root;
    let mut levels = 0u64;
    let mut pred = None;
    let mut succ = None;
    loop {
        let (s, leaf, c) = node(img, g, n, first, last)?;
        let j = match search::<SHARED0>(s, c, g, k)? {
            // A key that fits W never finds an overflowed entry: its payload is the count.
            Ok(i) => return Some(Pos::Count(payload_to_count(be64(s, g.fo(i))?))),
            Err(j) => j,
        };
        if leaf {
            return Some(Pos::Leaf { s, j, c, pred, succ });
        }
        if j > 0 {
            pred = Some(Sep { node: s, i: j.wrapping_sub(1) });
        }
        if j < c {
            succ = Some(Sep { node: s, i: j });
        }
        levels = levels.wrapping_add(1);
        if levels >= PATH_STACK {
            return None;
        }
        n = child(s, g, j)?;
    }
}

/// Internal levels a walk across leaves keeps; a deeper tree is counted from its opened run.
const FRAMES: usize = 16;

/// A walk back's place: the internal nodes above it, each with the number of its entries the walk has not
/// yet reached (it takes them right to left), and the leaf with its entries not yet reached.
struct Back<'h> {
    frames: [Option<(Node<'h>, usize)>; FRAMES],
    depth: usize,
    leaf: Node<'h>,
    i: usize,
}

impl<'h> Back<'h> {
    /// A place whose one frame is internal node `node` with its entries before entry `i` left: the outermost
    /// node the walk may reach. `enter` gives it its leaf.
    fn at(node: Node<'h>, i: usize) -> Back<'h> {
        let mut frames = [None; FRAMES];
        if let Some(f) = frames.first_mut() {
            *f = Some((node, i));
        }
        Back { frames, depth: 1, leaf: node, i: 0 }
    }

    /// Enters the subtree left of entry e of internal node `s`: the walk goes on from its rightmost leaf's last
    /// entry, every internal node passed kept.
    #[inline(always)]
    fn enter<H: PageHost + ?Sized>(
        &mut self,
        img: &mut Img<'h, H>,
        g: &Geo,
        first: u64,
        last: u64,
        s: Node<'h>,
        e: usize,
    ) -> Option<()> {
        let mut n = child(s, g, e)?;
        loop {
            let (x, is_leaf, c) = node(img, g, n, first, last)?;
            if is_leaf {
                self.leaf = x;
                self.i = c;
                return Some(());
            }
            *self.frames.get_mut(self.depth)? = Some((x, c));
            self.depth = self.depth.wrapping_add(1);
            n = child(x, g, c)?;
        }
    }

    /// The entry before the walk's place, which the walk moves to: in the leaf, or the separator of the
    /// lowest frame with an entry left (with whether it is a leaf's); `None` when every frame's first entry
    /// is passed.
    #[inline(always)]
    fn prev(&mut self) -> Option<(Node<'h>, usize, bool)> {
        if self.i > 0 {
            self.i = self.i.wrapping_sub(1);
            return Some((self.leaf, self.i, true));
        }
        loop {
            let d = self.depth.checked_sub(1)?;
            let f = self.frames.get_mut(d)?.as_mut()?;
            if f.1 > 0 {
                f.1 = f.1.wrapping_sub(1);
                return Some((f.0, f.1, false));
            }
            self.depth = d;
        }
    }
}

/// The sum of the key's entries before its position when they reach past its leaf's first entry: the descent
/// again, keeping its path, then the walk back from the position (`walk_on`). `None` for a check that fails,
/// too deep a tree, or `WALK_MAX` entries.
fn walk_back<H: PageHost + ?Sized>(
    img: &mut Img<'_, H>,
    g: &Geo,
    root: u64,
    last: u64,
    k: &CountKey<'_>,
) -> Option<i64> {
    let mut frames: [Option<(Node<'_>, usize)>; FRAMES] = [None; FRAMES];
    let mut depth = 0usize;
    let mut n = root;
    let (leaf, i) = loop {
        let (s, is_leaf, c) = node(img, g, n, root, last)?;
        let j = search::<false>(s, c, g, k)?.err()?;
        if is_leaf {
            break (s, j);
        }
        *frames.get_mut(depth)? = Some((s, j));
        depth = depth.wrapping_add(1);
        n = child(s, g, j)?;
    };
    let mut b = Back { frames, depth, leaf, i };
    Some(walk_on(img, g, root, last, k, &mut b, 0, 0)?.0)
}

/// The walk back continued from the separator the descent kept: `sep`, the entry before the descent's leaf in key
/// order, which the caller found not below the key, after the key's first `j` entries of `leaf`, all of the key's.
/// Those entries, the separator, then the subtree left of it from its rightmost leaf, then its node's earlier entries
/// and their subtrees: no second descent. When the key's entries reach past the first entry of the separator's node and
/// that node is not the root (the tree's first slot), the walk needs ancestors the descent did not keep, and the full
/// walk back (`walk_back`) answers.
#[inline(never)]
fn walk_kept<'h, H: PageHost + ?Sized>(
    img: &mut Img<'h, H>,
    g: &Geo,
    last: u64,
    k: &CountKey<'_>,
    sep: Sep<'h>,
    leaf: Node<'h>,
    j: usize,
) -> Option<i64> {
    let mut sum = 0i64;
    for p in 0..j {
        sum = sum.checked_add(walked(leaf, g, p, true, k)?)?;
    }
    let seen = j.wrapping_add(1);
    if seen >= WALK {
        return None;
    }
    let sum = sum.checked_add(walked(sep.node, g, sep.i, false, k)?)?;
    let mut b = Back::at(sep.node, sep.i);
    b.enter(img, g, BLOCK, last, sep.node, sep.i)?;
    let (sum, passed_all) = walk_on(img, g, BLOCK, last, k, &mut b, sum, seen)?;
    if passed_all && !core::ptr::eq(sep.node, img.slot(BLOCK)?) {
        return walk_back(img, g, BLOCK, last, k);
    }
    Some(sum)
}

/// From a walk back's place, the entries before it in reverse key order (`Back::prev`; before a separator,
/// the subtree left of it from its rightmost leaf's last entry), while their key area is not below the key,
/// as the opened run's reverse cursor walks them: `sum` plus what each adds, and whether the walk passed the
/// first entry of every frame without meeting an entry below the key. `seen` entries were walked before.
/// `None` for a check that fails, too deep a tree, or `WALK_MAX` entries in all.
#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn walk_on<'h, H: PageHost + ?Sized>(
    img: &mut Img<'h, H>,
    g: &Geo,
    first: u64,
    last: u64,
    k: &CountKey<'_>,
    b: &mut Back<'h>,
    mut sum: i64,
    mut seen: usize,
) -> Option<(i64, bool)> {
    loop {
        let Some((s, e, in_leaf)) = b.prev() else { return Some((sum, true)) };
        if area_cmp(s, g.ko(e), 0, g.wwords(), &k.words)? == Ordering::Less {
            return Some((sum, false));
        }
        seen = seen.wrapping_add(1);
        if seen >= WALK {
            return None;
        }
        sum = sum.checked_add(walked(s, g, e, in_leaf, k)?)?;
        if !in_leaf {
            b.enter(img, g, first, last, s, e)?;
        }
    }
}

/// The contribution of the entry whose field's high byte is `top` and whose value part starts at `vo` of `s`:
/// a count entry 0; at V = 0 ±1; a posting chunk ±(m + 1). `None` for any other chunk or a chunk header that
/// fails its check.
#[inline(always)]
fn contribution(s: &[u8], top: u8, vo: usize, v: usize) -> Option<i64> {
    if top & TOP_COUNT != 0 {
        return Some(0);
    }
    let sign: i64 = if top & TOP_TOMB != 0 { -1 } else { 1 };
    if v == 0 {
        return Some(sign);
    }
    let [kind, m, b] = *s.get(vo..)?.first_chunk::<3>()?;
    // m ≤ 255 and b ≤ 62: 3 + ⌈m·b / 8⌉ ≤ 1,980, computed without overflow.
    let need = u32::from(m).wrapping_mul(u32::from(b)).wrapping_add(7).wrapping_shr(3).wrapping_add(3);
    if kind != POSTING || b > B_MAX || usize::try_from(need).ok()? > v {
        return None;
    }
    Some(sign.wrapping_mul(i64::from(m).wrapping_add(1)))
}

/// Whether the entry is a key run: V ≥ 8, a row entry, value kind 1.
#[inline(always)]
fn is_key_run(top: u8, kind: Option<&u8>, v: usize) -> bool {
    v >= 8 && top & (TOP_COUNT | TOP_TOMB) == 0 && kind == Some(&KEY_RUN)
}

/// For the key run at entry i of node `s`, its value part at `vo`: whether the key is one of its items, and
/// its items n. An item's key has the key's length and differs from the stored key (the last item's) only in
/// its last L bytes, read as a big-endian number at most n − 1 below the stored key's. `None` when the run's
/// header fails its check.
fn run_holds(s: &[u8], g: &Geo, i: usize, vo: usize, k: &CountKey<'_>) -> Option<(bool, i64)> {
    let l = *s.get(vo.wrapping_add(1))?;
    if l == 0 || l > 8 {
        return None;
    }
    let nm1 = u32::from_le_bytes(*s.get(vo.wrapping_add(2)..)?.first_chunk::<4>()?);
    let n = i64::from(nm1).wrapping_add(1);
    let ko = g.ko(i);
    let area = s.get(ko..ko.checked_add(g.w())?)?;
    // The key fits W, so an item of an overflowed entry is never the key.
    let Some(head) = k.klen.checked_sub(usize::from(l)) else { return Some((false, n)) };
    if *s.get(g.fo(i))? & TOP_OVF != 0
        || area.get(..head) != k.key.get(..head)
        || area.get(k.klen..).is_none_or(|z| z.iter().any(|&x| x != 0))
    {
        return Some((false, n));
    }
    let fk = key_field(k.key, l)?;
    let fe = key_field(area.get(..k.klen)?, l)?;
    Some((fe.checked_sub(fk).is_some_and(|back| back <= u64::from(nm1)), n))
}

/// What entry i of a leaf (`leaf`) or an internal node `s` adds to the key's count in a walk back from its
/// position: 1 for a key run holding the key as an item, else its contribution. `None` for a chunk header that
/// fails its check.
#[inline(always)]
fn walked(s: &[u8], g: &Geo, i: usize, leaf: bool, k: &CountKey<'_>) -> Option<i64> {
    let top = *s.get(g.fo(i))?;
    let vo = g.vo(leaf, i);
    if is_key_run(top, s.get(vo), g.v()) {
        let (holds, n) = run_holds(s, g, i, vo, k)?;
        return Some(if holds { 1 } else { n });
    }
    contribution(s, top, vo, g.v())
}

/// Whether a filter line holds all seven of the key's bits.
#[inline(always)]
fn line_admits(line: &[u8], bits: &[(u8, u8); 7]) -> bool {
    let Some(l) = line.first_chunk::<LINE>() else { return false };
    // Each byte index is below 64, the line's length.
    bits.iter().all(|&(i, m)| l.get(usize::from(i & 63)).is_some_and(|&x| x & m != 0))
}

/// The key past a run's shared prefix of `p` bytes, prepared as `CountKey::new` prepares a key, kept for the
/// runs of that prefix length that follow (made for the first run that stores a prefix).
struct Rest<'k> {
    p: u64,
    key: CountKey<'k>,
}

/// The count of the key in one run, from its descriptor `d` and its record `r`; `None` when this path cannot
/// answer. `WIDE`: the key is longer than 8 bytes. Inlined into its one caller, in the loop over the runs.
#[inline(always)]
fn fast<'k, H: PageHost + ?Sized, const WIDE: bool>(
    h: &H,
    d: Slot<'_>,
    r: Slot<'_>,
    k: &mut CountKey<'k>,
    rest: &mut Option<Rest<'k>>,
    geos: &mut Geos,
) -> Option<i64> {
    let (lo, hi) = (f64_at(r, 16), f64_at(r, 24));
    // A run that stores a shared prefix has lo > hi, so that every key of at most 8 bytes, which it holds none
    // of, leaves at the range check; hi is the prefix's first 8 bytes (below 2^64 − 1).
    let prefixed = WIDE && lo > hi;
    if prefixed {
        if hi != PREFIX_HEAD_ANY && k.k8 != hi {
            return Some(0);
        }
    } else if k.k8 < lo || k.k8 > hi {
        return Some(0);
    }
    let mut img = Img::new(h, f64_at(d, 0), *d.get(56)?, f64_at(r, 56));
    let filter = f64_at(r, 32);
    if filter != 0 {
        k.hash();
        // The line is below the filter's lines, whose bytes lie inside the image.
        let off = filter_line(k.hash, f32_at(r, 40)).wrapping_mul(LINE_BYTES).wrapping_add(filter);
        if !line_admits(img.read(off, LINE_BYTES)?, &k.bits) {
            return Some(0);
        }
    }
    let w = f16_at(d, 62);
    let k: &CountKey<'k> = if prefixed {
        match rest_of(&mut img, r, k, rest, hi)? {
            Some(rk) => rk,
            None => return Some(0),
        }
    } else {
        k
    };
    if k.klen > KEY_BYTES || u64::try_from(k.klen).ok()? > w {
        // The bytes every entry shares with the first fence: the fences' (a run without a shared prefix).
        let shared = if prefixed { 0 } else { usize::try_from(f32_at(r, 44)).ok()? };
        return fast_g(&mut ImgX::new(&mut img), d, r, k, geos, w, shared);
    }
    if WIDE && !prefixed && lo == hi {
        // The fences' first 8 bytes are equal, and so the key's and every entry's.
        return fast_lean::<H, true>(&mut img, d, r, k, geos, w);
    }
    fast_lean::<H, false>(&mut img, d, r, k, geos, w)
}

/// A probe record's `hi` for a run whose shared prefix starts with 8 bytes of 0xFF or 8 bytes whose number
/// is 2^64 − 2: the prefix's first 8 bytes are then compared from the header.
const PREFIX_HEAD_ANY: u64 = u64::MAX - 1;

/// The key past the shared prefix of the run of record `r` (its length p in the record, its first 8 bytes `hi`
/// unless `PREFIX_HEAD_ANY`, both compared already in that case): `None` when the key does not start with the
/// prefix, else the key past it, made once for the runs of one prefix length (`rest`).
#[inline(always)]
fn rest_of<'r, 'k, H: PageHost + ?Sized>(
    img: &mut Img<'_, H>,
    r: Slot<'_>,
    k: &CountKey<'k>,
    rest: &'r mut Option<Rest<'k>>,
    hi: u64,
) -> Option<Option<&'r CountKey<'k>>> {
    let p = f32_at(r, 44);
    let pu = usize::try_from(p).ok()?;
    // Every key of the run is longer than its prefix.
    if k.klen <= pu {
        return Some(None);
    }
    // The prefix's first 8 bytes were compared through `hi`, unless it could not say.
    let from = if hi == PREFIX_HEAD_ANY { 0 } else { 8 };
    if from < pu && !prefix_rest_is(img, k, from, pu)? {
        return Some(None);
    }
    if rest.as_ref().is_none_or(|x| x.p != p) {
        let tail = k.key.get(pu..)?;
        *rest = Some(Rest { p, key: k.past(tail, pu) });
    }
    Some(rest.as_ref().map(|x| &x.key))
}

/// Whether bytes [from, p) of the key equal the run's shared prefix's (the header's, then the rest at the
/// start of the key bytes, read in pieces within image blocks); the key is longer than p.
#[inline(never)]
fn prefix_rest_is<H: PageHost + ?Sized>(
    img: &mut Img<'_, H>,
    k: &CountKey<'_>,
    from: usize,
    pu: usize,
) -> Option<bool> {
    let head = usize::try_from(crate::index::run::PREFIX_HEAD).ok()?;
    let hn = pu.min(head);
    if from < hn {
        let mine = k.key.get(from..hn)?;
        let off = rh::PREFIX.checked_add(u64::try_from(from).ok()?)?;
        if bytes_cmp_any(img.read(off, u64::try_from(hn.wrapping_sub(from)).ok()?)?, mine).0 != Ordering::Equal {
            return Some(false);
        }
    }
    if pu > hn {
        let ok = le64(img.read(rh::OK, 8)?, 0)?;
        let mut at = hn.max(from);
        while at < pu {
            let off = ok.checked_add(u64::try_from(at.wrapping_sub(hn)).ok()?)?;
            let room = usize::try_from(PAGE.wrapping_sub(off % PAGE)).ok()?;
            let n = pu.wrapping_sub(at).min(room);
            if bytes_cmp_any(img.read(off, u64::try_from(n).ok()?)?, k.key.get(at..at.checked_add(n)?)?).0
                != Ordering::Equal
            {
                return Some(false);
            }
            at = at.wrapping_add(n);
        }
    }
    Some(true)
}

/// `fast` past the fences' first 8 bytes and the filter, for a key that fits W and 64 bytes; with `SHARED0`,
/// every entry's first word is the key's.
#[inline(always)]
fn fast_lean<H: PageHost + ?Sized, const SHARED0: bool>(
    img: &mut Img<'_, H>,
    d: Slot<'_>,
    r: Slot<'_>,
    k: &CountKey<'_>,
    geos: &mut Geos,
    w: u64,
) -> Option<i64> {
    // The node counts are below 2^32: every slot offset below stays under 2^45.
    let nodes = f32_at(r, 48);
    let ct = f32_at(r, 52);
    if ct != 0 {
        let g = geos.get(1, w, 0)?;
        let first = nodes.wrapping_add(1).wrapping_mul(BLOCK);
        let last = nodes.wrapping_add(ct).wrapping_mul(BLOCK);
        return match locate::<H, SHARED0>(img, &g, first, first, last, k)? {
            Pos::Count(x) => Some(x),
            Pos::Leaf { .. } => Some(0),
        };
    }
    let g = geos.get(0, w, f16_at(d, 60))?;
    let last = nodes.wrapping_mul(BLOCK);
    let (s, j, c, pred, succ) = match locate::<H, SHARED0>(img, &g, BLOCK, BLOCK, last, k)? {
        Pos::Count(x) => return Some(x),
        Pos::Leaf { s, j, c, pred, succ } => (s, j, c, pred, succ),
    };
    // The entries before the position whose key area is not below the key: the key's rows and tombstones.
    let mut sum = 0i64;
    let mut i = j;
    loop {
        let Some(p) = i.checked_sub(1) else {
            // The leaf's first entry is passed: the entry before it is the separator left of the descent.
            // When that is not below the key, the key's entries reach further back.
            if let Some(sep) = pred {
                if area_cmp(sep.node, g.ko(sep.i), 0, g.wwords(), &k.words)? != Ordering::Less {
                    sum = walk_kept(img, &g, last, k, sep, s, j)?;
                }
            }
            break;
        };
        if area_cmp(s, g.ko(p), 0, g.wwords(), &k.words)? == Ordering::Less {
            break;
        }
        // At most `WALK_MAX` − 1 entries of one key before its position, as the opened run allows.
        if j.wrapping_sub(p) >= WALK {
            return None;
        }
        sum = sum.checked_add(walked(s, &g, p, true, k)?)?;
        i = p;
    }
    // The entry after the position: a key run there may hold the key as an inner item.
    let (node, at, in_leaf) = if j < c {
        (s, j, true)
    } else {
        match succ {
            Some(x) => (x.node, x.i, false),
            None => return Some(sum),
        }
    };
    let vo = g.vo(in_leaf, at);
    if is_key_run(*node.get(g.fo(at))?, node.get(vo), g.v()) && run_holds(node, &g, at, vo, k)?.0 {
        sum = sum.checked_add(1)?;
    }
    Some(sum)
}

// ---------------------------------------------------------------- keys longer than W or 64 bytes

/// The positions a descent's lower and upper bounds share with the key, in the comparator's sequence (the key
/// area zero-padded to W, then a longer key's bytes beyond W): every entry between the two bounds shares at
/// least the fewer of them, so a comparison starts there.
#[derive(Clone, Copy, Default)]
struct Shared {
    lo: usize,
    hi: usize,
    /// Whether the last entry a search found below the key's count entry holds the key (a row or tombstone of
    /// it): when a leaf search ends at j > 0, that is entry j − 1.
    lo_key: bool,
}

/// Orders the overflowed stored key of `field` against the key, both longer than W, their first `from`
/// positions (W at least) known equal: their bytes from there in memcmp order, a proper prefix first, the
/// stored key's bytes read where its overflow record puts them. Also returns the positions they share. `None`
/// when the record fails its checks.
fn tail_cmp<H: PageHost + ?Sized>(
    img: &mut ImgX<'_, '_, H>,
    w: usize,
    field: u64,
    k: &CountKey<'_>,
    from: usize,
) -> Option<(Ordering, usize)> {
    let (_, key_at, len) = img.record(field)?;
    let len = usize::try_from(len).ok()?;
    if len <= w {
        return None;
    }
    let mut at = from.max(w);
    loop {
        if at >= len || at >= k.klen {
            return Some((len.cmp(&k.klen), at));
        }
        // The stored key's byte `at` lies at key_at + (at − W); a piece is read within one image block.
        let off = key_at.checked_add(u64::try_from(at.wrapping_sub(w)).ok()?)?;
        let room = usize::try_from(PAGE.wrapping_sub(off % PAGE)).ok()?;
        let n = len.wrapping_sub(at).min(k.klen.wrapping_sub(at)).min(room);
        let a = img.read_x(off, u64::try_from(n).ok()?)?;
        let (o, s) = bytes_cmp_any(a, k.key.get(at..)?);
        if o != Ordering::Equal {
            return Some((o, at.wrapping_add(s)));
        }
        at = at.wrapping_add(n);
    }
}

/// Orders entry i of node `s` against the key by key alone (not by rank), positions before `skip` known
/// equal: the key area word by word against the key zero-padded to W, then, when both keys are longer than
/// W, the stored key's bytes beyond W. Also returns the positions they share.
#[inline]
fn entry_cmp<H: PageHost + ?Sized>(
    img: &mut ImgX<'_, '_, H>,
    s: &[u8],
    g: &Geo,
    i: usize,
    k: &CountKey<'_>,
    skip: usize,
) -> Option<(Ordering, usize)> {
    part_cmp(img, s, g.ko(i), g.w(), k, skip)
}

/// `entry_cmp` for the key part at `ko` of `s` (a W-byte key area, then the field), W = `w`.
#[inline]
fn part_cmp<H: PageHost + ?Sized>(
    img: &mut ImgX<'_, '_, H>,
    s: &[u8],
    ko: usize,
    w: usize,
    k: &CountKey<'_>,
    skip: usize,
) -> Option<(Ordering, usize)> {
    // The key area from `skip`, word by word, against the key zero-padded to W (so an area holding a byte past
    // a shorter key's end is above it).
    let mut i = skip & !7;
    while i < w {
        let a = le64(s, ko.wrapping_add(i))?;
        let b = word_padded(k.key, i);
        if a != b {
            let o = if word_below(a, b) { Ordering::Less } else { Ordering::Greater };
            return Some((o, i.wrapping_add(diff_byte(a, b))));
        }
        i = i.wrapping_add(8);
    }
    let fo = ko.wrapping_add(w);
    match (*s.get(fo)? & TOP_OVF != 0, k.klen > w) {
        (false, false) => Some((Ordering::Equal, w)),
        // Equal key areas, one key longer than W: the overflowed one is the longer.
        (true, false) => Some((Ordering::Greater, w)),
        (false, true) => Some((Ordering::Less, w)),
        (true, true) => tail_cmp(img, w, be64(s, fo)?, k, skip),
    }
}

/// `search` for any key: the position of (key, `Count`) among the c entries of node `s`, each comparison
/// starting at the positions the bounds `sh` share with the key, and `sh` moved to the node's bounds.
fn search_g<H: PageHost + ?Sized>(
    img: &mut ImgX<'_, '_, H>,
    s: Node<'_>,
    c: usize,
    g: &Geo,
    k: &CountKey<'_>,
    sh: &mut Shared,
) -> Option<Result<usize, usize>> {
    let (mut lo, mut hi) = (0usize, c);
    while lo < hi {
        // lo and hi are at most a node's capacity, below 4,096.
        let mid = lo.wrapping_add(hi).wrapping_shr(1);
        let (o, shared) = entry_cmp(img, s, g, mid, k, sh.lo.min(sh.hi))?;
        match o {
            Ordering::Less => {
                lo = mid.wrapping_add(1);
                sh.lo = shared;
                sh.lo_key = false;
            }
            Ordering::Greater => {
                hi = mid;
                sh.hi = shared;
            }
            Ordering::Equal => {
                if *s.get(g.fo(mid))? & TOP_COUNT != 0 {
                    return Some(Ok(mid));
                }
                // A row or tombstone ranks below a count entry.
                lo = mid.wrapping_add(1);
                sh.lo = shared;
                sh.lo_key = true;
            }
        }
    }
    Some(Err(lo))
}

/// The count an entry found as the key's count entry holds: its payload, or its overflow record's.
fn count_of<H: PageHost + ?Sized>(img: &mut ImgX<'_, '_, H>, field: u64) -> Option<i64> {
    if field & crate::consts::F_OVF != 0 {
        return Some(payload_to_count(img.record(field)?.0));
    }
    Some(payload_to_count(field))
}

/// `locate` for any key, from `root` = `first` through slots [first, last].
fn locate_g<'h, H: PageHost + ?Sized>(
    img: &mut ImgX<'_, 'h, H>,
    g: &Geo,
    first: u64,
    last: u64,
    k: &CountKey<'_>,
    sh: &mut Shared,
) -> Option<Pos<'h>> {
    let mut n = first;
    let mut levels = 0u64;
    let mut pred = None;
    let mut succ = None;
    loop {
        let (s, leaf, c) = node(img.img, g, n, first, last)?;
        let j = match search_g(img, s, c, g, k, sh)? {
            Ok(i) => return Some(Pos::Count(count_of(img, be64(s, g.fo(i))?)?)),
            Err(j) => j,
        };
        if leaf {
            return Some(Pos::Leaf { s, j, c, pred, succ });
        }
        if j > 0 {
            pred = Some(Sep { node: s, i: j.wrapping_sub(1) });
        }
        if j < c {
            succ = Some(Sep { node: s, i: j });
        }
        levels = levels.wrapping_add(1);
        if levels >= PATH_STACK {
            return None;
        }
        n = child(s, g, j)?;
    }
}

/// Whether entry i is one of the key's: its key equal to the key (the walk back reaches only entries not
/// above the key), the first `skip` positions known equal.
#[inline]
fn holds_key<H: PageHost + ?Sized>(
    img: &mut ImgX<'_, '_, H>,
    s: &[u8],
    g: &Geo,
    i: usize,
    k: &CountKey<'_>,
    skip: usize,
) -> Option<bool> {
    Some(entry_cmp(img, s, g, i, k, skip)?.0 != Ordering::Less)
}

/// What the run's fences tell a key longer than W or 64 bytes (past the run's shared prefix, as the fences
/// hold the stored keys): `None` when the key lies outside them (it is in no entry), else the positions the
/// first and last keys share with it. Every entry lies between the two, so it shares at least the fewer of
/// them with the key, and comparisons start there. Every entry also shares the fences' first `shared`
/// positions (their shared bytes, of a run without a shared prefix; else 0): a key that differs from the
/// first fence inside them lies outside the run, and the last fence, which shares them with the first, is
/// compared from there. A run whose first entry is a key run bounds nothing below (its items lie below its
/// stored key): its first share is 0. The fences are the header's, whose key parts lie in the run's first
/// block.
fn fences_g<H: PageHost + ?Sized>(
    img: &mut ImgX<'_, '_, H>,
    w: usize,
    v: u64,
    k: &CountKey<'_>,
    shared: usize,
) -> Option<Option<Shared>> {
    let head = img.img.read(0, BLOCK)?;
    // The fences' offsets and W are below 4,096: every key part read here lies inside the block.
    let first = usize::try_from(rh::FENCE_FIRST).ok()?;
    let last = usize::try_from(rh::FENCE_LAST).ok()?;
    let field = be64(head, first.wrapping_add(w))?;
    let run_first = v >= 8 && !is_count(field) && head.get(first.wrapping_add(w).wrapping_add(8)) == Some(&KEY_RUN);
    let (lo, from) = if run_first {
        (0, 0)
    } else {
        let (o, s) = part_cmp(img, head, first, w, k, 0)?;
        if o == Ordering::Greater || s < shared {
            return Some(None);
        }
        (s, shared)
    };
    let (o, hi) = part_cmp(img, head, last, w, k, from)?;
    if o == Ordering::Less {
        return Some(None);
    }
    Some(Some(Shared { lo, hi, lo_key: false }))
}

/// `fast` past the fences' first 8 bytes and the filter, for a key longer than W or 64 bytes.
#[inline(never)]
fn fast_g<H: PageHost + ?Sized>(
    img: &mut ImgX<'_, '_, H>,
    d: Slot<'_>,
    r: Slot<'_>,
    k: &CountKey<'_>,
    geos: &mut Geos,
    w: u64,
    shared: usize,
) -> Option<i64> {
    let v = f16_at(d, 60);
    let Some(sh) = fences_g(img, usize::try_from(w).ok()?, v, k, shared)? else { return Some(0) };
    let nodes = f32_at(r, 48);
    let ct = f32_at(r, 52);
    if ct != 0 {
        let g = geos.get(1, w, 0)?;
        let first = nodes.wrapping_add(1).wrapping_mul(BLOCK);
        let last = nodes.wrapping_add(ct).wrapping_mul(BLOCK);
        return match locate_g(img, &g, first, last, k, &mut { sh })? {
            Pos::Count(x) => Some(x),
            Pos::Leaf { .. } => Some(0),
        };
    }
    let g = geos.get(0, w, v)?;
    main_g(img, &g, nodes.wrapping_mul(BLOCK), k, sh)
}

/// The key's count in a main tree of slots [4,096, `last`] (byte offsets), for a key longer than W or 64
/// bytes: `fast`'s walk, whole keys compared from the positions every entry shares with the key (`sh`, from
/// the fences).
fn main_g<H: PageHost + ?Sized>(
    img: &mut ImgX<'_, '_, H>,
    g: &Geo,
    last: u64,
    k: &CountKey<'_>,
    sh: Shared,
) -> Option<i64> {
    let mut bounds = sh;
    let (s, j, c, pred, succ) = match locate_g(img, g, BLOCK, last, k, &mut bounds)? {
        Pos::Count(x) => return Some(x),
        Pos::Leaf { s, j, c, pred, succ } => (s, j, c, pred, succ),
    };
    let skip = sh.lo.min(sh.hi);
    let mut sum = 0i64;
    let mut i = j;
    loop {
        let Some(p) = i.checked_sub(1) else {
            if let Some(sep) = pred {
                if holds_key(img, sep.node, g, sep.i, k, skip)? {
                    sum = walk_kept_g(img, g, last, k, skip, sep, sum, j)?;
                }
            }
            break;
        };
        // The leaf's search compared entry j − 1 last on the way below the key: it already told whether that
        // entry holds the key.
        let holds = if p.wrapping_add(1) == j { bounds.lo_key } else { holds_key(img, s, g, p, k, skip)? };
        if !holds {
            break;
        }
        if j.wrapping_sub(p) >= WALK {
            return None;
        }
        sum = sum.checked_add(walked_g(img, s, g, p, true, k)?)?;
        i = p;
    }
    let (node, at, in_leaf) = if j < c {
        (s, j, true)
    } else {
        match succ {
            Some(x) => (x.node, x.i, false),
            None => return Some(sum),
        }
    };
    let vo = g.vo(in_leaf, at);
    if is_key_run(*node.get(g.fo(at))?, node.get(vo), g.v()) && run_holds_g(img, node, g, at, vo, k)?.0 {
        sum = sum.checked_add(1)?;
    }
    Some(sum)
}

/// `walk_back` for any key, `sh` from the fences.
fn walk_back_g<H: PageHost + ?Sized>(
    img: &mut ImgX<'_, '_, H>,
    g: &Geo,
    last: u64,
    k: &CountKey<'_>,
    sh: Shared,
) -> Option<i64> {
    let mut frames: [Option<(Node<'_>, usize)>; FRAMES] = [None; FRAMES];
    let mut depth = 0usize;
    let mut n = BLOCK;
    let mut bounds = sh;
    let (leaf, i) = loop {
        let (s, is_leaf, c) = node(img.img, g, n, BLOCK, last)?;
        let j = search_g(img, s, c, g, k, &mut bounds)?.err()?;
        if is_leaf {
            break (s, j);
        }
        *frames.get_mut(depth)? = Some((s, j));
        depth = depth.wrapping_add(1);
        n = child(s, g, j)?;
    };
    let mut b = Back { frames, depth, leaf, i };
    Some(walk_on_g(img, g, last, k, sh.lo.min(sh.hi), &mut b, 0, 0)?.0)
}

/// `walk_kept` for any key, every entry sharing its first `skip` positions with the key.
#[allow(clippy::too_many_arguments)]
#[inline(never)]
fn walk_kept_g<'h, H: PageHost + ?Sized>(
    img: &mut ImgX<'_, 'h, H>,
    g: &Geo,
    last: u64,
    k: &CountKey<'_>,
    skip: usize,
    sep: Sep<'h>,
    sum: i64,
    seen: usize,
) -> Option<i64> {
    let seen = seen.wrapping_add(1);
    if seen >= WALK {
        return None;
    }
    let sum = sum.checked_add(walked_g(img, sep.node, g, sep.i, false, k)?)?;
    let mut b = Back::at(sep.node, sep.i);
    b.enter(img.img, g, BLOCK, last, sep.node, sep.i)?;
    let (sum, passed_all) = walk_on_g(img, g, last, k, skip, &mut b, sum, seen)?;
    if passed_all && !core::ptr::eq(sep.node, img.img.slot(BLOCK)?) {
        return walk_back_g(img, g, last, k, Shared { lo: skip, hi: skip, lo_key: false });
    }
    Some(sum)
}

/// `walk_on` for any key, comparisons from position `skip`, which every entry shares with the key.
#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn walk_on_g<'h, H: PageHost + ?Sized>(
    img: &mut ImgX<'_, 'h, H>,
    g: &Geo,
    last: u64,
    k: &CountKey<'_>,
    skip: usize,
    b: &mut Back<'h>,
    mut sum: i64,
    mut seen: usize,
) -> Option<(i64, bool)> {
    loop {
        let Some((s, e, in_leaf)) = b.prev() else { return Some((sum, true)) };
        if !holds_key(img, s, g, e, k, skip)? {
            return Some((sum, false));
        }
        seen = seen.wrapping_add(1);
        if seen >= WALK {
            return None;
        }
        sum = sum.checked_add(walked_g(img, s, g, e, in_leaf, k)?)?;
        if !in_leaf {
            b.enter(img.img, g, BLOCK, last, s, e)?;
        }
    }
}

/// `walked` for any key.
#[inline]
fn walked_g<H: PageHost + ?Sized>(
    img: &mut ImgX<'_, '_, H>,
    s: &[u8],
    g: &Geo,
    i: usize,
    leaf: bool,
    k: &CountKey<'_>,
) -> Option<i64> {
    let top = *s.get(g.fo(i))?;
    let vo = g.vo(leaf, i);
    if is_key_run(top, s.get(vo), g.v()) {
        let (holds, n) = run_holds_g(img, s, g, i, vo, k)?;
        return Some(if holds { 1 } else { n });
    }
    contribution(s, top, vo, g.v())
}

/// `run_holds` for any key: an item has the stored key's length, so a key longer than W is an item only of an
/// overflowed entry, whose bytes beyond W its overflow record holds.
fn run_holds_g<H: PageHost + ?Sized>(
    img: &mut ImgX<'_, '_, H>,
    s: &[u8],
    g: &Geo,
    i: usize,
    vo: usize,
    k: &CountKey<'_>,
) -> Option<(bool, i64)> {
    let w = g.w();
    let fo = g.fo(i);
    let ovf = *s.get(fo)? & TOP_OVF != 0;
    if !ovf {
        return run_holds(s, g, i, vo, k);
    }
    let l = *s.get(vo.wrapping_add(1))?;
    if l == 0 || l > 8 {
        return None;
    }
    let nm1 = u32::from_le_bytes(*s.get(vo.wrapping_add(2)..)?.first_chunk::<4>()?);
    let n = i64::from(nm1).wrapping_add(1);
    let lu = usize::from(l);
    let (_, key_at, len) = img.record(be64(s, fo)?)?;
    let Some(head) = k.klen.checked_sub(lu) else { return Some((false, n)) };
    if k.klen <= w || usize::try_from(len).ok()? != k.klen {
        return Some((false, n));
    }
    // The stored key: its first W bytes in the key area, the rest at key_at. Bytes [0, head) must equal the
    // key's, and bytes [head, len) are its field.
    let ko = g.ko(i);
    let area = s.get(ko..ko.checked_add(w)?)?;
    let hw = head.min(w);
    if area.get(..hw) != k.key.get(..hw) {
        return Some((false, n));
    }
    let mut at = w;
    while at < head {
        let off = key_at.checked_add(u64::try_from(at.wrapping_sub(w)).ok()?)?;
        let room = usize::try_from(PAGE.wrapping_sub(off % PAGE)).ok()?;
        let m = head.wrapping_sub(at).min(room);
        if img.read_x(off, u64::try_from(m).ok()?)? != k.key.get(at..at.checked_add(m)?)? {
            return Some((false, n));
        }
        at = at.wrapping_add(m);
    }
    let mut fb = [0u8; 8];
    for (j, slot) in fb.iter_mut().enumerate().take(lu) {
        let p = head.wrapping_add(j);
        *slot = match area.get(p) {
            Some(&x) => x,
            None => {
                let off = key_at.checked_add(u64::try_from(p.wrapping_sub(w)).ok()?)?;
                *img.read_x(off, 1)?.first()?
            }
        };
    }
    let fe = key_field(fb.get(..lu)?, l)?;
    let fk = key_field(k.key, l)?;
    Some((fe.checked_sub(fk).is_some_and(|back| back <= u64::from(nm1)), n))
}

// WALK_MAX = 8.
#[allow(clippy::cast_possible_truncation)]
const WALK: usize = WALK_MAX as usize;

/// The count of the key in a run opened from its header.
#[inline(never)]
fn opened<H: PageHost + ?Sized>(h: &H, index: u8, d: &[u8], key: &[u8]) -> Result<i64, Fault> {
    let d = RunDesc::read(d, 0)?;
    let img = d.image(h)?;
    let id = ImageId::Run { index, start_seq: d.start_seq };
    Run::open(ImageReader::new(h, id, img), d.rid_base, d.rid_lo, d.rid_hi)?.count_key(key)
}

/// The count of the key in the run of descriptor `d`, through its record `r` when it names the run; `WIDE`:
/// the key is longer than 8 bytes.
#[inline(always)]
fn one<'k, H: PageHost + ?Sized, const WIDE: bool>(
    h: &H,
    index: u8,
    d: Slot<'_>,
    r: Option<Slot<'_>>,
    k: &mut CountKey<'k>,
    rest: &mut Option<Rest<'k>>,
    geos: &mut Geos,
) -> Result<i64, Fault> {
    let root = f64_at(d, 0);
    if let Some(r) = r {
        if f64_at(r, 8) == root && f64_at(r, 0) == f64_at(d, 40) {
            if let Some(x) = fast::<H, WIDE>(h, d, r, k, rest, geos) {
                return Ok(x);
            }
        }
    }
    opened(h, index, d, k.key)
}

/// The 64 bytes at `off` of a block, when they lie inside it.
#[inline(always)]
fn slot(b: &[u8], off: u64) -> Option<Slot<'_>> {
    b.get(usize::try_from(off).ok()?..)?.first_chunk::<64>()
}

/// The block at entry `e` of the probe store, or nothing when it is absent or unreadable: the runs are then
/// counted from their headers.
fn store_block<H: PageHost + ?Sized>(h: &H, store: Radix, e: u64) -> &[u8] {
    if store.root == 0 {
        return &[];
    }
    match store.get(h, e) {
        Ok(b) if b != 0 => h.get(Region::Block(b)).unwrap_or(&[]),
        _ => &[],
    }
}

/// The count of `key` (its full encoding) over the loaded runs of the run list in block `run_list` of index
/// `index`: the sum of the key's count in each.
pub fn count_key<H: PageHost + ?Sized>(h: &H, run_list: u64, index: u8, key: &[u8]) -> Result<i128, Fault> {
    let b = h.get(Region::Block(run_list))?;
    if rd_u32(b, 0)? != magic::NTRL || rd_u16(b, 4)? != FORMAT_TABLE {
        return Err(rt(Corruption::Header, run_list));
    }
    let loaded = LoadedRecord::read(b)?;
    let mut k = CountKey::new(key);
    if k.klen > 8 {
        count_runs::<H, true>(h, &loaded, index, &mut k)
    } else {
        count_runs::<H, false>(h, &loaded, index, &mut k)
    }
}

/// `count_key` over the loaded runs of `loaded`, for a key longer than 8 bytes (`WIDE`) or not.
#[inline(always)]
fn count_runs<H: PageHost + ?Sized, const WIDE: bool>(
    h: &H,
    loaded: &LoadedRecord,
    index: u8,
    k: &mut CountKey<'_>,
) -> Result<i128, Fault> {
    let mut geos = Geos::new();
    let mut rest = None;
    let mut sum = 0i128;
    let mut j = 0u64;
    while j < loaded.runs {
        let blk = j / LOADED_PER_BLOCK;
        let lb = loaded.list.get(h, blk)?;
        if lb == 0 {
            return Err(rt(Corruption::Link, j));
        }
        let descs = h.get(Region::Block(lb))?;
        let probes = store_block(h, loaded.probes, blk.wrapping_add(1));
        let end = blk.wrapping_add(1).saturating_mul(LOADED_PER_BLOCK).min(loaded.runs);
        while j < end {
            let off = (j % LOADED_PER_BLOCK).wrapping_mul(DESC);
            let d = slot(descs, off).ok_or(rt(Corruption::Bounds, lb))?;
            sum =
                sum.wrapping_add(i128::from(one::<H, WIDE>(h, index, d, slot(probes, off), k, &mut rest, &mut geos)?));
            j = j.wrapping_add(1);
        }
    }
    Ok(sum)
}
