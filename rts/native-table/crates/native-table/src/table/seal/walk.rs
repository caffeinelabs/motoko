//! A seal's reads of its group's rows in place. The walk takes the group's stored positions in order, block by
//! block: the block that holds a position is found once per block (the descriptor's binary search, as
//! `buffer::locate_in` makes it), and each of its slots is read from the slot array in turn. A row image is read
//! where it lies, in its buffer block or, for a row stored as a long value, in the long value's page, once its
//! length is checked to cover the header, the null byte, the cell and the item table entry the leaf reads; the
//! leaf's cell and item are then read from it. A visitor takes each position: pass 1's accumulators, or pass 2's
//! cells and runs, held in locals for a walk's positions and written back at its end.
//!
//! Every position the walk cannot read this way is left to the general value path, which reads it with the
//! same checks and faults as before: a slot that is dead or moved; a long row image that is not whole in one
//! page; a row of another schema version; a row or item that does not lie where its slot and item table say; a
//! long item; and a big Nat or Int. A walk stops at such a position, the general path reads it, and a later walk
//! starts after it. So the results are the general path's, value for value.

use crate::bytes::uleb_len;
use crate::consts::{BUFFER_BLOCKS_MAX, LONG_VALUE, PAGE};
use crate::fault::ImageId;
use crate::image::{Image, ImageReader, BELIEVED};
use crate::mem::{PageHost, Region};

use super::super::buffer::{B_FIRST, B_SLOTS, D_BLOCKS, D_RECORDS, SLOT_LONG};
use super::super::group::form;
use super::super::layout::{RowHead, Ty};
use super::super::long;
use super::{Acc, Rw, A_SEQ_FIRST, A_SEQ_OK, A_SEQ_STEP, A_STORED_NULL};

/// A block lent in place.
type Blk<'a> = &'a [u8; 4_096];

/// How a leaf's value lies in its row's fixed region or item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Bool,
    U(u8),
    I(u8),
    Char,
    F32,
    F64,
    Nat,
    Int,
    Bytes,
    Principal,
}

impl Kind {
    /// The kind of a leaf type the walk reads, else `None`.
    pub fn of(t: Ty) -> Option<Kind> {
        Some(match t {
            Ty::Bool => Kind::Bool,
            Ty::U(w) if matches!(w, 1 | 2 | 4 | 8) => Kind::U(w),
            Ty::I(w) if matches!(w, 1 | 2 | 4 | 8) => Kind::I(w),
            Ty::Char => Kind::Char,
            Ty::F32 => Kind::F32,
            Ty::F64 => Kind::F64,
            Ty::Nat => Kind::Nat,
            Ty::Int => Kind::Int,
            Ty::Text | Ty::Blob => Kind::Bytes,
            Ty::Principal => Kind::Principal,
            _ => return None,
        })
    }

    /// The code the walk's visitors are compiled for.
    pub const fn code(self) -> u8 {
        match self {
            Kind::Bool => K_BOOL,
            Kind::U(1) => K_U1,
            Kind::U(2) => K_U2,
            Kind::U(4) => K_U4,
            Kind::U(_) => K_U8,
            Kind::I(1) => K_I1,
            Kind::I(2) => K_I2,
            Kind::I(4) => K_I4,
            Kind::I(_) => K_I8,
            Kind::Char => K_CHAR,
            Kind::F32 => K_F32,
            Kind::F64 => K_F64,
            Kind::Nat => K_NAT,
            Kind::Int => K_INT,
            Kind::Bytes => K_BYTES,
            Kind::Principal => K_PRINCIPAL,
        }
    }
}

/// The codes of the kinds (`Kind::code`): each visitor is compiled once per code it reads, so that its work
/// at a position has no test of the leaf's kind.
pub const K_BOOL: u8 = 0;
pub const K_U1: u8 = 1;
pub const K_U2: u8 = 2;
pub const K_U4: u8 = 3;
pub const K_U8: u8 = 4;
pub const K_I1: u8 = 5;
pub const K_I2: u8 = 6;
pub const K_I4: u8 = 7;
pub const K_I8: u8 = 8;
pub const K_CHAR: u8 = 9;
pub const K_F32: u8 = 10;
pub const K_F64: u8 = 11;
pub const K_NAT: u8 = 12;
pub const K_INT: u8 = 13;
pub const K_BYTES: u8 = 14;
pub const K_PRINCIPAL: u8 = 15;

/// The kind of code c (`Kind::code` undone; a code past the last is `Bytes`).
const fn kind_of(c: u8) -> Kind {
    match c {
        K_BOOL => Kind::Bool,
        K_U1 => Kind::U(1),
        K_U2 => Kind::U(2),
        K_U4 => Kind::U(4),
        K_U8 => Kind::U(8),
        K_I1 => Kind::I(1),
        K_I2 => Kind::I(2),
        K_I4 => Kind::I(4),
        K_I8 => Kind::I(8),
        K_CHAR => Kind::Char,
        K_F32 => Kind::F32,
        K_F64 => Kind::F64,
        K_NAT => Kind::Nat,
        K_INT => Kind::Int,
        K_PRINCIPAL => Kind::Principal,
        _ => Kind::Bytes,
    }
}

/// `$f::<_, code, $extra...>($args)` for the kind code `$c`: one instantiation per code.
macro_rules! by_code {
    ($c:expr, $f:ident, [$($x:expr),*], ($($a:expr),*)) => {
        match $c {
            K_BOOL => $f::<_, K_BOOL $(, $x)*>($($a),*),
            K_U1 => $f::<_, K_U1 $(, $x)*>($($a),*),
            K_U2 => $f::<_, K_U2 $(, $x)*>($($a),*),
            K_U4 => $f::<_, K_U4 $(, $x)*>($($a),*),
            K_U8 => $f::<_, K_U8 $(, $x)*>($($a),*),
            K_I1 => $f::<_, K_I1 $(, $x)*>($($a),*),
            K_I2 => $f::<_, K_I2 $(, $x)*>($($a),*),
            K_I4 => $f::<_, K_I4 $(, $x)*>($($a),*),
            K_I8 => $f::<_, K_I8 $(, $x)*>($($a),*),
            K_CHAR => $f::<_, K_CHAR $(, $x)*>($($a),*),
            K_F32 => $f::<_, K_F32 $(, $x)*>($($a),*),
            K_F64 => $f::<_, K_F64 $(, $x)*>($($a),*),
            K_NAT => $f::<_, K_NAT $(, $x)*>($($a),*),
            K_INT => $f::<_, K_INT $(, $x)*>($($a),*),
            K_PRINCIPAL => $f::<_, K_PRINCIPAL $(, $x)*>($($a),*),
            _ => $f::<_, K_BYTES $(, $x)*>($($a),*),
        }
    };
}

/// A leaf's place in a row, as offsets from the row's start: every row of the table holds them.
#[derive(Clone, Copy, Debug)]
pub struct Cut {
    pub kind: Kind,
    cell: usize,
    item: usize,
    null_at: usize,
    null_mask: u8,
    big_at: usize,
}

impl Cut {
    /// The place of leaf type `t` at offsets (cell, item index, bitmap bit) in a row of layout `head`.
    pub fn new(t: Ty, head: &RowHead, cell: u64, vi: u64, bit: u64) -> Option<Cut> {
        let kind = Kind::of(t)?;
        let page = usize::try_from(PAGE).ok()?;
        let us = |x: u64| usize::try_from(x).ok().filter(|&v| v < page);
        let cell = us(cell)?;
        let item = us(head.items_table_at().checked_add(vi.checked_mul(4)?)?)?;
        let null_at = us(8u64.checked_add(bit / 8)?)?;
        let big_at = us(8u64.checked_add(head.bitmap)?.checked_add(bit / 8)?)?;
        let null_mask = 1u8 << (bit % 8);
        Some(Cut { kind, cell, item, null_at, null_mask, big_at })
    }
}

// ---------------------------------------------------------------- rows

/// A live row's image, read where it lies, at the places of a cut, which every row of the table holds.
pub trait Bytes<'a>: Copy {
    fn len(&self) -> usize;
    /// Byte i (a byte of its block, or 0, past the row's end).
    fn byte(&self, i: usize) -> u8;
    /// The w ∈ {1, 2, 4, 8} bytes at i, little-endian.
    fn le(&self, i: usize, w: usize) -> u64;
    /// Bytes [from, from + n) as one slice, when they lie in the row.
    fn slice(&self, from: usize, n: usize) -> Option<&'a [u8]>;

    #[inline(always)]
    fn null(&self, cut: &Cut) -> bool {
        self.byte(cut.null_at) & cut.null_mask != 0
    }

    /// A Nat's or Int's 16-byte cell as a value of at most 64 bits, or `None` when it is big or wider.
    #[inline(always)]
    fn small(&self, cut: &Cut, signed: bool) -> Option<i128> {
        if self.byte(cut.big_at) & cut.null_mask != 0 {
            return None;
        }
        let lo = self.le(cut.cell, 8);
        let hi = self.le(cut.cell.wrapping_add(8), 8);
        if signed {
            let x = lo as i64;
            (hi == if x < 0 { u64::MAX } else { 0 }).then_some(i128::from(x))
        } else {
            (hi == 0).then_some(i128::from(lo))
        }
    }

    /// The leaf's inline item as (offset of its bytes in the row, length), or `None` when its item is absent,
    /// long, or does not lie in the row.
    #[inline(always)]
    fn item_at(&self, cut: &Cut) -> Option<(usize, usize)> {
        let len = self.len();
        let at = usize::try_from(self.le(cut.item, 4)).ok()?;
        if at == 0 || at >= len || self.byte(at) != 0 {
            return None;
        }
        // The length's ULEB128, ending within the 11 bytes the general reader takes and within the row: one
        // byte for a value shorter than 128 bytes.
        let window = len.wrapping_sub(at).min(11);
        let x = self.byte(at.wrapping_add(1));
        let (n, k) = if x < 0x80 && window > 1 {
            (u64::from(x), 2usize)
        } else {
            let mut n = 0u64;
            let mut k = 1usize;
            let mut shift = 0u32;
            loop {
                if k >= window || k > 9 {
                    return None;
                }
                let x = self.byte(at.wrapping_add(k));
                n |= u64::from(x & 0x7F) << shift;
                shift = shift.wrapping_add(7);
                k = k.wrapping_add(1);
                if x & 0x80 == 0 {
                    break;
                }
            }
            (n, k)
        };
        let data = at.wrapping_add(k);
        let n = usize::try_from(n).ok()?;
        (data.checked_add(n)? <= len).then_some((data, n))
    }

    /// A Principal's 30-byte cell when it is canonical (its length byte, at 29, at most 29 and its bytes from
    /// the length to 29 zero), so that it is the cell `cur::principal_cell` makes of the value's bytes; else
    /// `None`.
    #[inline(always)]
    fn principal(&self, cut: &Cut) -> Option<&'a [u8]> {
        let c = self.slice(cut.cell, 30)?;
        let n = usize::from(*c.get(29)?);
        if n > 29 || c.get(n..29)?.iter().any(|&b| b != 0) {
            return None;
        }
        Some(c)
    }
}

/// A row in its buffer block: offset and length checked to lie in the block, so a byte is read through a
/// masked index, with no further check.
#[derive(Clone, Copy)]
pub struct InBlock<'a> {
    b: Blk<'a>,
    off: usize,
    len: usize,
}

impl<'a> Bytes<'a> for InBlock<'a> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.len
    }
    #[inline(always)]
    fn byte(&self, i: usize) -> u8 {
        self.b.get(self.off.wrapping_add(i) & 4_095).copied().unwrap_or(0)
    }
    /// The callers read below the row's length, which lies in the block: the offset is clamped to the block's
    /// last w bytes (which it never passes), so that no further check remains.
    #[inline(always)]
    fn le(&self, i: usize, w: usize) -> u64 {
        let j = self.off.wrapping_add(i);
        match w {
            8 => self.b.get(j.min(4_088)..).and_then(|t| t.first_chunk::<8>()).map_or(0, |c| u64::from_le_bytes(*c)),
            4 => self
                .b
                .get(j.min(4_092)..)
                .and_then(|t| t.first_chunk::<4>())
                .map_or(0, |c| u64::from(u32::from_le_bytes(*c))),
            2 => self
                .b
                .get(j.min(4_094)..)
                .and_then(|t| t.first_chunk::<2>())
                .map_or(0, |c| u64::from(u16::from_le_bytes(*c))),
            _ => u64::from(self.byte(i)),
        }
    }
    #[inline(always)]
    fn slice(&self, from: usize, n: usize) -> Option<&'a [u8]> {
        if from.checked_add(n)? > self.len {
            return None;
        }
        let a = self.off.wrapping_add(from);
        self.b.get(a..a.wrapping_add(n))
    }
}

/// A row stored as a long value: its content, whole in one page.
#[derive(Clone, Copy)]
pub struct InPage<'a> {
    s: &'a [u8],
}

impl<'a> Bytes<'a> for InPage<'a> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.s.len()
    }
    #[inline(always)]
    fn byte(&self, i: usize) -> u8 {
        self.s.get(i).copied().unwrap_or(0)
    }
    #[inline(always)]
    fn le(&self, i: usize, w: usize) -> u64 {
        let t = self.s.get(i..).unwrap_or(&[]);
        match w {
            8 => t.first_chunk::<8>().map_or(0, |c| u64::from_le_bytes(*c)),
            4 => t.first_chunk::<4>().map_or(0, |c| u64::from(u32::from_le_bytes(*c))),
            2 => t.first_chunk::<2>().map_or(0, |c| u64::from(u16::from_le_bytes(*c))),
            _ => u64::from(self.byte(i)),
        }
    }
    #[inline(always)]
    fn slice(&self, from: usize, n: usize) -> Option<&'a [u8]> {
        self.s.get(from..from.checked_add(n)?)
    }
}

// ---------------------------------------------------------------- the walk

/// What a visitor does with a position.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Taken: on to the next position.
    Next,
    /// Not taken; the piece ends before it.
    Stop,
    /// Not taken; the general path reads it.
    Slow,
}

/// Why a walk ended, and the position it ended at (the first one it did not take).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// No stored position is left.
    Last,
    /// The visitor stopped.
    Stop,
    /// The general path reads the position.
    Slow,
}

/// A walk's visitor: what it does at each stored position.
pub trait Visit<'a> {
    /// The walk's outcome the visitor keeps.
    fn piece(&mut self) -> &mut Piece;
    /// A stored position dead at the snapshot (an image that is not compacted stores it).
    fn dead(&mut self, q: u64) -> Flow;
    /// A live row, `long` when it is stored as a long value.
    fn live<R: Bytes<'a>>(&mut self, q: u64, r: R, long: bool) -> Flow;
}

/// Walks the group's stored positions from q with visitor v, block by block, until the visitor stops, a
/// position the general path must read, or the last one. A bitmap word is read once per 64 positions and a block
/// found once per block; within them, a position costs its liveness bit and its slot entry, read at an offset
/// that falls by 8 from one slot to the next. Returns where and why the walk ended; the positions the visitor
/// took are added to its piece's count.
#[inline(always)]
pub fn walk<'a, H: PageHost + ?Sized, V: Visit<'a>>(h: &'a H, rw: &Rw, mut q: u64, v: &mut V) -> (u64, End) {
    let Some(d) = h.get(Region::Block(rw.source)).ok().and_then(|d| <Blk<'a>>::try_from(d).ok()) else {
        return (q, End::Slow);
    };
    let p = rw.p;
    // The block in work: its record, its first position and the end of its positions (none at first).
    let mut rec: Option<u64> = None;
    let mut b: Blk<'a> = d;
    let (mut first, mut end) = (0u64, 0u64);
    let mut taken = 0u64;
    let r = 'walk: loop {
        if q >= p {
            break (q, End::Last);
        }
        let w = q / 64;
        let Some(&word) = usize::try_from(w).ok().and_then(|i| rw.bits.get(i)) else { break (q, End::Slow) };
        let wend = w.wrapping_add(1).wrapping_mul(64).min(p);
        while q < wend {
            if word >> (q % 64) & 1 == 0 && rw.compacted {
                // Not stored: on to the next live position, in this word or a later one.
                let rest = word & (u64::MAX << (q % 64));
                q = if rest != 0 {
                    w.wrapping_mul(64).wrapping_add(u64::from(rest.trailing_zeros())).min(wend)
                } else {
                    wend
                };
                continue;
            }
            if q >= end {
                // The block that holds q: when the records rise strictly, each first position its block's (the
                // liveness pass checked them), the next record's block when q lies in it, as the binary search
                // would find; else that search.
                let next = match rec {
                    Some(k) if rw.ordered => next_block(h, d, k.wrapping_add(1), q),
                    _ => None,
                };
                let Some((k, nb, f, slots)) = next.or_else(|| locate(h, d, q)) else { break 'walk (q, End::Slow) };
                rec = Some(k);
                b = nb;
                first = f;
                // The block's positions up to its last slot (or the group's last position).
                end = f.saturating_add(slots).min(p);
            }
            // The positions of this word and block the walk takes in turn: those of slots below 512 (past slot
            // 511 the general path reads the position, and faults).
            let lim = wend.min(end).min(first.saturating_add(512));
            if q >= lim {
                break 'walk (q, End::Slow);
            }
            // Slot s's entry, 8 bytes below those of the slots before it.
            let mut sat = 4_088u64.wrapping_sub(q.wrapping_sub(first).wrapping_mul(8));
            while q < lim {
                let f = if word >> (q % 64) & 1 == 0 {
                    if rw.compacted {
                        break;
                    }
                    v.dead(q)
                } else {
                    // The entry lies at or below 4,088: the masked read is the entry's.
                    let i = usize::try_from(sat & 4_088).unwrap_or(0);
                    let e = b.get(i..).and_then(|x| x.first_chunk::<8>()).map_or(0, |c| u64::from_le_bytes(*c));
                    let off = usize::try_from(e & 0xFFFF).unwrap_or(usize::MAX);
                    let len = usize::try_from((e >> 16) & 0xFFFF).unwrap_or(usize::MAX);
                    // Flags at byte 4: a row stored as a long value is read from its page; dead and moved
                    // slots, by the general path.
                    match (e >> 32) & 0xFF {
                        0 if off.wrapping_add(len) <= 4_096 => v.live(q, InBlock { b, off, len }, false),
                        x if x == u64::from(SLOT_LONG) => match long_row(h, b, off) {
                            Some(s) => v.live(q, InPage { s }, true),
                            None => Flow::Slow,
                        },
                        _ => Flow::Slow,
                    }
                };
                match f {
                    Flow::Next => {
                        q = q.wrapping_add(1);
                        sat = sat.wrapping_sub(8);
                        taken = taken.wrapping_add(1);
                    }
                    Flow::Stop => break 'walk (q, End::Stop),
                    Flow::Slow => break 'walk (q, End::Slow),
                }
            }
        }
    };
    let pc = v.piece();
    pc.read = pc.read.wrapping_add(taken);
    r
}

/// A long row image's content, when it lies whole in one page of its long value: the stub's reference at
/// `off` + 8 of block b, the content's length at the image's offset 32 and the content from offset 64, read
/// as `RowRef` reads them. A reference of height 0 names the image's one page: its fields are checked as
/// `Image::read_ref` checks them and the page is read directly; a radix reference is read through the image
/// reader.
#[inline(always)]
fn long_row<'a, H: PageHost + ?Sized>(h: &'a H, b: Blk<'a>, off: usize) -> Option<&'a [u8]> {
    let r = b.get(off.checked_add(8)?..)?.first_chunk::<24>()?;
    let word = |i: usize| r.get(i..).and_then(|s| s.first_chunk::<8>()).map_or(0, |c| u64::from_le_bytes(*c));
    let (root, length) = (word(0), word(8));
    let (height, flags, reserved) = (word(16) & 0xFF, (word(16) >> 8) & 0xFF, word(16) >> 16);
    if height != 0 {
        return long_row_radix(h, b, off);
    }
    // Flags other than bit 0, a length that is not a multiple of 8 or non-zero reserved bytes break the
    // reference; its one block entry must cover the length, which must hold the content's length at 32.
    if flags & !u64::from(BELIEVED) != 0 || length % 8 != 0 || reserved != 0 || root == 0 || length > PAGE {
        return None;
    }
    if length < 40 {
        return None;
    }
    let page = h.get(Region::Page(root)).ok()?;
    let len = u64::from_le_bytes(*page.get(32..)?.first_chunk::<8>()?);
    let end = long::HEAD.checked_add(len)?;
    if end > length {
        return None;
    }
    if len == 0 {
        return Some(&[]);
    }
    page.get(usize::try_from(long::HEAD).ok()?..usize::try_from(end).ok()?)
}

/// A long row image's content through the image reader, for a radix reference.
#[inline(never)]
fn long_row_radix<'a, H: PageHost + ?Sized>(h: &'a H, b: Blk<'a>, off: usize) -> Option<&'a [u8]> {
    let img = Image::read_ref(b, u64::try_from(off).ok()?.checked_add(8)?).ok()?;
    let r = ImageReader::new(h, ImageId::Runtime, img);
    let len = r.u64_at(32).ok()?;
    r.bytes(long::HEAD, len).ok()
}

/// Record k's block, its first position and its slots, when record k exists and q lies in its block's slots.
#[inline(never)]
fn next_block<'a, H: PageHost + ?Sized>(h: &'a H, d: Blk<'a>, k: u64, q: u64) -> Option<(u64, Blk<'a>, u64, u64)> {
    let blocks = rd16(d, D_BLOCKS);
    if blocks > BUFFER_BLOCKS_MAX || k >= blocks {
        return None;
    }
    let rec = usize::try_from(D_RECORDS.wrapping_add(k.wrapping_mul(16))).ok()?;
    let block = u64::from_le_bytes(*d.get(rec..)?.first_chunk::<8>()?);
    let b: Blk<'a> = h.get(Region::Block(block)).ok()?.try_into().ok()?;
    let first = rd16(b, B_FIRST);
    let slots = rd16(b, B_SLOTS);
    if q < first || q.wrapping_sub(first) >= slots {
        return None;
    }
    Some((k, b, first, slots))
}

/// The block whose record holds position q, the record's index, the block's first position and its slots:
/// the last record of descriptor d whose first position is at or below q, by the binary search
/// `buffer::locate_in` makes, then the block's own header. `None` when q lies in no slot of that block.
#[inline(never)]
fn locate<'a, H: PageHost + ?Sized>(h: &'a H, d: Blk<'a>, q: u64) -> Option<(u64, Blk<'a>, u64, u64)> {
    let blocks = rd16(d, D_BLOCKS);
    if blocks == 0 || blocks > BUFFER_BLOCKS_MAX {
        return None;
    }
    let (mut lo, mut hi) = (0u64, blocks);
    while hi.wrapping_sub(lo) > 1 {
        let mid = lo.wrapping_add(hi.wrapping_sub(lo) / 2);
        // Records lie below offset 64 + 240 · 16 = 3,904: the masked read is the record's.
        let first = rd16(d, D_RECORDS.wrapping_add(mid.wrapping_mul(16)).wrapping_add(8));
        if first <= q {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let rec = usize::try_from(D_RECORDS.wrapping_add(lo.wrapping_mul(16))).ok()?;
    let block = u64::from_le_bytes(*d.get(rec..)?.first_chunk::<8>()?);
    let b: Blk<'a> = h.get(Region::Block(block)).ok()?.try_into().ok()?;
    let first = rd16(b, B_FIRST);
    let slots = rd16(b, B_SLOTS);
    if q < first || q.wrapping_sub(first) >= slots {
        return None;
    }
    Some((lo, b, first, slots))
}

/// The u16 at `at` of a block, below 4,095 (masked, so no check remains).
#[inline(always)]
fn rd16(b: Blk<'_>, at: u64) -> u64 {
    let i = usize::try_from(at & 4_094).unwrap_or(0);
    let lo = b.get(i).copied().unwrap_or(0);
    let hi = b.get(i | 1).copied().unwrap_or(0);
    u64::from(u16::from_le_bytes([lo, hi]))
}

/// How a walk ended.
#[derive(Clone, Copy, Debug, Default)]
pub struct Piece {
    /// The general path reads the position the walk stopped at.
    pub slow: bool,
    /// The stage lacks room for the next slot or value: the caller writes it out.
    pub full: bool,
    /// No stored position is left.
    pub end: bool,
    /// Positions the walk read.
    pub read: u64,
}

// ---------------------------------------------------------------- pass 1

/// The `Seq` test in locals: the general rule (`seal::seq_add`) for the first two values and after a gap,
/// and from then on one addition and compare per adjacent position. Exact: the values are of at most 64
/// bits, so the step is below 2^65 in magnitude and no product or sum of the rule leaves i128.
#[derive(Clone, Copy)]
struct Seq {
    ok: bool,
    first: bool,
    has_step: bool,
    q0: u64,
    v0: i128,
    step: i128,
    /// The last position tested and the value the next one must hold, while `run`.
    run: bool,
    last: u64,
    expect: i128,
}

impl Seq {
    fn load(acc: &Acc) -> Seq {
        Seq {
            ok: acc.flags & A_SEQ_OK != 0,
            first: acc.flags & A_SEQ_FIRST != 0,
            has_step: acc.flags & A_SEQ_STEP != 0,
            q0: acc.seq_q0,
            v0: acc.seq_v0,
            step: acc.seq_step,
            run: false,
            last: 0,
            expect: 0,
        }
    }

    fn store(&self, acc: &mut Acc) {
        let mut f = acc.flags & !(A_SEQ_OK | A_SEQ_FIRST | A_SEQ_STEP);
        if self.ok {
            f |= A_SEQ_OK;
        }
        if self.first {
            f |= A_SEQ_FIRST;
        }
        if self.has_step {
            f |= A_SEQ_STEP;
        }
        acc.flags = f;
        acc.seq_q0 = self.q0;
        acc.seq_v0 = self.v0;
        acc.seq_step = self.step;
    }

    #[inline(always)]
    fn add(&mut self, q: u64, v: i128) {
        if !self.ok {
            return;
        }
        if self.run && q == self.last.wrapping_add(1) {
            if v != self.expect {
                self.ok = false;
            }
            self.expect = self.expect.wrapping_add(self.step);
            self.last = q;
            return;
        }
        self.general(q, v);
    }

    #[inline(always)]
    fn general(&mut self, q: u64, v: i128) {
        let qi = i128::from(q);
        if !self.first {
            self.first = true;
            self.q0 = q;
            self.v0 = v;
        } else if !self.has_step {
            let dq = qi.wrapping_sub(i128::from(self.q0));
            let dv = v.wrapping_sub(self.v0);
            if dq <= 0 || dv.checked_rem(dq) != Some(0) {
                self.ok = false;
            } else if let Some(s) = dv.checked_div(dq) {
                self.step = s;
                self.has_step = true;
            } else {
                self.ok = false;
            }
        } else {
            let want = self.step.checked_mul(qi.wrapping_sub(i128::from(self.q0))).and_then(|x| x.checked_add(self.v0));
            if want != Some(v) {
                self.ok = false;
            }
        }
        self.run = self.ok && self.has_step;
        self.last = q;
        self.expect = v.wrapping_add(self.step);
    }
}

/// Pass 1's per-value range of an integer leaf in locals: its largest value (unsigned) and its smallest and
/// largest (signed); and whether a stored position is dead or null.
#[derive(Clone, Copy)]
struct Range {
    stored_null: bool,
    umax: u64,
    imin: i64,
    imax: i64,
}

impl Range {
    fn load(acc: &Acc) -> Range {
        Range { stored_null: false, umax: acc.umax, imin: acc.imin, imax: acc.imax }
    }

    fn store(&self, acc: &mut Acc) {
        if self.stored_null {
            acc.flags |= A_STORED_NULL;
        }
        acc.umax = self.umax;
        acc.imin = self.imin;
        acc.imax = self.imax;
    }

    #[inline(always)]
    fn unsigned(&mut self, x: u64) {
        self.umax = self.umax.max(x);
    }

    #[inline(always)]
    fn signed(&mut self, x: i64) {
        self.imin = self.imin.min(x);
        self.imax = self.imax.max(x);
    }
}

/// The shift that puts a w-byte value's bytes at a word's top.
#[inline(always)]
fn sh_of(w: u8) -> u32 {
    64u32.wrapping_sub(u32::from(w).wrapping_mul(8)) & 63
}

/// SLEB128 length of a value of at most 64 bits (`uleb_len` gives the ULEB128 one).
#[inline(always)]
fn sleb_len(x: i64) -> u64 {
    // The bits of the magnitude plus the sign bit.
    let m = if x < 0 { !(x as u64) } else { x as u64 };
    let bits = 65u64.wrapping_sub(u64::from(m.leading_zeros()));
    bits.wrapping_add(6) / 7
}

/// Pass 1's visitor: one leaf's accumulators in locals, for a leaf of kind code `K`.
struct Scan<const K: u8> {
    cut: Cut,
    pc: Piece,
    n: Range,
    seq: Seq,
    sizes0: u64,
    sizes1: u64,
    medium: u64,
    medium_bytes: u64,
}

impl<'a, const K: u8> Visit<'a> for Scan<K> {
    #[inline(always)]
    fn piece(&mut self) -> &mut Piece {
        &mut self.pc
    }

    #[inline(always)]
    fn dead(&mut self, _q: u64) -> Flow {
        self.n.stored_null = true;
        Flow::Next
    }

    #[inline(always)]
    fn live<R: Bytes<'a>>(&mut self, q: u64, r: R, _long: bool) -> Flow {
        let cut = &self.cut;
        if r.null(cut) {
            self.n.stored_null = true;
            self.seq.ok = false;
            return Flow::Next;
        }
        match kind_of(K) {
            Kind::U(wd) => {
                let x = r.le(cut.cell, usize::from(wd));
                self.n.unsigned(x);
                self.seq.add(q, i128::from(x));
            }
            Kind::I(wd) => {
                let sh = sh_of(wd);
                let x = ((r.le(cut.cell, usize::from(wd)) << sh) as i64) >> sh;
                self.n.signed(x);
                self.seq.add(q, i128::from(x));
            }
            Kind::Nat | Kind::Int => {
                let signed = K == K_INT;
                let Some(x) = r.small(cut, signed) else { return Flow::Slow };
                // Values of at most 64 bits: the cast keeps them whole.
                #[allow(clippy::cast_possible_truncation)]
                if signed {
                    let y = x as i64;
                    self.n.signed(y);
                    self.sizes1 = self.sizes1.wrapping_add(sleb_len(y));
                } else {
                    let y = x as u64;
                    self.n.unsigned(y);
                    self.sizes1 = self.sizes1.wrapping_add(uleb_len(y));
                }
                self.seq.add(q, x);
            }
            Kind::Bytes => {
                let Some((_, nb)) = r.item_at(cut) else { return Flow::Slow };
                let len = nb as u64;
                if len > LONG_VALUE {
                    self.medium = self.medium.wrapping_add(1);
                    self.medium_bytes = self.medium_bytes.wrapping_add(len);
                } else {
                    self.sizes0 = self.sizes0.wrapping_add(len);
                }
            }
            Kind::Bool | Kind::Char | Kind::F32 | Kind::F64 | Kind::Principal => {}
        }
        Flow::Next
    }
}

/// Pass 1 over leaf `cut` from position q: each stored position's value read in place and fed to the leaf's
/// accumulators, until no stored position is left or a position the general path must read (the walk stops
/// at it). Returns how the walk ended and its position.
pub fn scan<H: PageHost + ?Sized>(h: &H, rw: &Rw, cut: &Cut, acc: &mut Acc, q: u64) -> (Piece, u64) {
    by_code!(cut.kind.code(), scan_k, [], (h, rw, cut, acc, q))
}

fn scan_k<H: PageHost + ?Sized, const K: u8>(h: &H, rw: &Rw, cut: &Cut, acc: &mut Acc, q: u64) -> (Piece, u64) {
    let mut v = Scan::<K> {
        cut: *cut,
        pc: Piece::default(),
        n: Range::load(acc),
        seq: Seq::load(acc),
        sizes0: 0,
        sizes1: 0,
        medium: 0,
        medium_bytes: 0,
    };
    let (q, end) = walk(h, rw, q, &mut v);
    let Scan { mut pc, n, seq, sizes0, sizes1, medium, medium_bytes, .. } = v;
    pc.slow = end == End::Slow;
    pc.end = end == End::Last;
    n.store(acc);
    seq.store(acc);
    for (i, d) in [(0usize, sizes0), (1, sizes1), (2, medium), (4, medium_bytes)] {
        if let Some(s) = acc.sizes.get_mut(i) {
            *s = s.wrapping_add(d);
        }
    }
    (pc, q)
}

// ---------------------------------------------------------------- pass 2

/// A value as pass 2 writes it: null; a number's or a float's bits (its row cell, sign-extended); a Bool; a
/// byte string's inline item.
#[derive(Clone, Copy)]
enum V<'a> {
    Null,
    Bits(u64),
    Bool(bool),
    Bytes(&'a [u8]),
}

/// The value at a live row of a leaf of kind code `K`, read in place; `None` when the general path reads it.
#[inline(always)]
fn value<'a, R: Bytes<'a>, const K: u8>(r: &R, cut: &Cut) -> Option<V<'a>> {
    if r.null(cut) {
        return Some(V::Null);
    }
    Some(match kind_of(K) {
        Kind::Bool => V::Bool(r.byte(cut.cell) != 0),
        Kind::U(w) => V::Bits(r.le(cut.cell, usize::from(w))),
        Kind::I(w) => {
            let sh = 64u32.wrapping_sub(u32::from(w).wrapping_mul(8)) & 63;
            V::Bits((((r.le(cut.cell, usize::from(w)) << sh) as i64) >> sh) as u64)
        }
        Kind::Char | Kind::F32 => V::Bits(r.le(cut.cell, 4)),
        Kind::F64 => V::Bits(r.le(cut.cell, 8)),
        // Values of at most 64 bits: the cast keeps their bits.
        #[allow(clippy::cast_possible_truncation)]
        Kind::Nat => V::Bits(r.small(cut, false)? as u64),
        #[allow(clippy::cast_possible_truncation)]
        Kind::Int => V::Bits(r.small(cut, true)? as u64),
        Kind::Bytes => {
            let (data, n) = r.item_at(cut)?;
            V::Bytes(r.slice(data, n)?)
        }
        Kind::Principal => V::Bytes(r.principal(cut)?),
    })
}

/// Whether a live row of a leaf of kind code `K` holds a value (not null), with the checks `value` makes;
/// `None` when the general path reads it.
#[inline(always)]
fn present<'a, R: Bytes<'a>, const K: u8>(r: &R, cut: &Cut) -> Option<bool> {
    if r.null(cut) {
        return Some(false);
    }
    match kind_of(K) {
        Kind::Nat => {
            r.small(cut, false)?;
        }
        Kind::Int => {
            r.small(cut, true)?;
        }
        Kind::Bytes => {
            let (data, n) = r.item_at(cut)?;
            r.slice(data, n)?;
        }
        _ => {}
    }
    Some(true)
}

/// The stage pass 2 writes the image through, and the bytes in it.
pub struct Stage<'s> {
    pub buf: &'s mut [u8; 4_096],
    pub at: usize,
}

impl Stage<'_> {
    #[inline(always)]
    fn room(&self) -> usize {
        4_096usize.wrapping_sub(self.at)
    }

    /// Writes `b`, which the caller checked fits, with one bulk copy.
    #[inline(always)]
    fn put(&mut self, b: &[u8]) {
        let end = self.at.wrapping_add(b.len());
        if let Some(d) = self.buf.get_mut(self.at..end) {
            crate::mem::copy_bytes(d, b);
            self.at = end;
        }
    }

    /// Writes the 8 bytes of x, which the caller checked fit (the offset clamped to the stage's last 8 bytes,
    /// which it never passes, so that no further check remains), with one store.
    #[inline(always)]
    fn put8(&mut self, x: u64) {
        if let Some(d) = self.buf.get_mut(self.at.min(4_088)..).and_then(|d| d.first_chunk_mut::<8>()) {
            *d = x.to_le_bytes();
            self.at = self.at.wrapping_add(8);
        }
    }

    /// Writes the 4 bytes of x, which the caller checked fit, as `put8` does.
    #[inline(always)]
    fn put4(&mut self, x: u32) {
        if let Some(d) = self.buf.get_mut(self.at.min(4_092)..).and_then(|d| d.first_chunk_mut::<4>()) {
            *d = x.to_le_bytes();
            self.at = self.at.wrapping_add(4);
        }
    }

    /// Writes the low w ≤ 8 bytes of x.
    #[inline(always)]
    fn put_le(&mut self, x: u64, w: usize) {
        match w {
            8 => self.put8(x),
            // The low bytes of x: the casts keep what the cell holds.
            #[allow(clippy::cast_possible_truncation)]
            4 => self.put4(x as u32),
            2 => {
                #[allow(clippy::cast_possible_truncation)]
                let v = (x as u16).to_le_bytes();
                if let Some(d) = self.buf.get_mut(self.at.min(4_094)..).and_then(|d| d.first_chunk_mut::<2>()) {
                    *d = v;
                    self.at = self.at.wrapping_add(2);
                }
            }
            _ => {
                if let Some(d) = self.buf.get_mut(self.at & 4_095) {
                    *d = x.to_le_bytes()[0];
                    self.at = self.at.wrapping_add(1);
                }
            }
        }
    }
}

/// What pass 2 writes of a leaf's segment.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// Validity words: a bit per stored slot, 1 for a value.
    Validity,
    /// Cells of a fixed-width form of `width` bytes, or a `Bool` form's bits.
    Cells,
    /// A variable form's offsets: the running offset per slot, and the end.
    Offsets,
    /// The out-of-line directory's bitmap: a bit per slot, 1 for a value out of line.
    OolBits,
}

/// A pass-2 cursor over one leaf's slots: the slot in the segment (or the directory), the position after the
/// last one read, the word under construction, and the running offset.
#[derive(Clone, Copy)]
pub struct Cur {
    pub slot: u64,
    pub q: u64,
    pub word: u64,
    pub acc: u64,
}

/// A null's cell of a `FIXED` form of at most 32 bytes.
static ZEROS: [u8; 32] = [0; 32];

/// The codes of the parts (`Part`), which pass 2's visitor is compiled for.
const P_VALIDITY: u8 = 0;
const P_CELLS: u8 = 1;
const P_OFFSETS: u8 = 2;
const P_OOLBITS: u8 = 3;

/// Pass 2's visitor writing part `P` of a segment's slots up to k − 1 into the stage, for a leaf of kind code
/// `K`.
struct Write<'b, const K: u8, const P: u8> {
    cut: Cut,
    form: u8,
    width: usize,
    k: u64,
    cur: Cur,
    st: Stage<'b>,
    pc: Piece,
}

impl<const K: u8, const P: u8> Write<'_, K, P> {
    /// Writes slot `cur.slot`'s validity bit, 1 for a value.
    #[inline(always)]
    fn bit(&mut self, present: bool) -> Flow {
        let i = self.cur.slot;
        self.cur.word |= u64::from(present) << (i % 64);
        if i % 64 == 63 || i.wrapping_add(1) == self.k {
            self.st.put8(self.cur.word);
            self.cur.word = 0;
        }
        self.cur.slot = i.wrapping_add(1);
        Flow::Next
    }

    /// Writes slot `cur.slot`'s value: `Next`, or `Slow` when the general path writes it.
    #[inline(always)]
    fn put(&mut self, v: V<'_>) -> Flow {
        let i = self.cur.slot;
        let k = self.k;
        let last = i % 64 == 63 || i.wrapping_add(1) == k;
        match P {
            P_VALIDITY => return self.bit(!matches!(v, V::Null)),
            P_OOLBITS => {
                let ool = match v {
                    V::Bytes(b) => b.len() as u64 > LONG_VALUE,
                    _ => false,
                };
                self.cur.word |= u64::from(ool) << (i % 64);
                if last {
                    self.st.put8(self.cur.word);
                    self.cur.word = 0;
                }
            }
            P_OFFSETS => {
                let Ok(o) = u32::try_from(self.cur.acc) else { return Flow::Slow };
                let mut next = self.cur.acc;
                if let V::Bytes(b) = v {
                    if b.len() as u64 <= LONG_VALUE {
                        next = next.wrapping_add(b.len() as u64);
                    }
                }
                let end = if i.wrapping_add(1) == k {
                    let Ok(e) = u32::try_from(next) else { return Flow::Slow };
                    Some(e)
                } else {
                    None
                };
                self.st.put4(o);
                if let Some(e) = end {
                    self.st.put4(e);
                }
                self.cur.acc = next;
            }
            _ => match self.form {
                form::FIXED => match v {
                    V::Bytes(b) if b.len() == self.width => self.st.put(b),
                    V::Null => self.st.put(ZEROS.get(..self.width).unwrap_or(&[])),
                    _ => return Flow::Slow,
                },
                form::BOOL => {
                    self.cur.acc |= u64::from(matches!(v, V::Bool(true))) << (i % 64);
                    if last {
                        self.st.put8(self.cur.acc);
                        self.cur.acc = 0;
                    }
                }
                _ => {
                    let x = match v {
                        V::Bits(x) => x,
                        _ => 0,
                    };
                    self.st.put_le(x, self.width);
                }
            },
        }
        self.cur.slot = self.cur.slot.wrapping_add(1);
        Flow::Next
    }

    /// Whether the walk ends before the next slot.
    #[inline(always)]
    fn stop(&mut self) -> bool {
        if self.cur.slot >= self.k {
            return true;
        }
        // A slot writes at most 30 bytes (a Principal's cell), or 8 and, the last one of an offsets part, 4
        // more.
        if self.st.room() < 32 {
            self.pc.full = true;
            return true;
        }
        false
    }
}

impl<'a, const K: u8, const P: u8> Visit<'a> for Write<'_, K, P> {
    #[inline(always)]
    fn piece(&mut self) -> &mut Piece {
        &mut self.pc
    }

    #[inline(always)]
    fn dead(&mut self, _q: u64) -> Flow {
        if self.stop() {
            return Flow::Stop;
        }
        self.put(V::Null)
    }

    #[inline(always)]
    fn live<R: Bytes<'a>>(&mut self, _q: u64, r: R, _long: bool) -> Flow {
        if self.stop() {
            return Flow::Stop;
        }
        if P == P_VALIDITY {
            let Some(p) = present::<R, K>(&r, &self.cut) else { return Flow::Slow };
            return self.bit(p);
        }
        let Some(v) = value::<R, K>(&r, &self.cut) else { return Flow::Slow };
        self.put(v)
    }
}

/// Pass 2 over a segment's slots `cur.slot` to k − 1: what `part` writes for each, into the stage, until the
/// stage lacks room for one more slot or a position the general path must read.
///
/// The visitor is compiled per part and per way the leaf's value lies in its row: a validity part reads only
/// whether a value is present (with the checks of a Nat's, an Int's or a byte string's), a cell part its
/// kind's cell (a `Char` or `F32` as a 4-byte unsigned cell, an `F64` as an 8-byte one), an offsets or
/// out-of-line part a byte string. Any other pairing ends the walk at once, for the general path.
#[allow(clippy::too_many_arguments)]
pub fn write<H: PageHost + ?Sized>(
    h: &H,
    rw: &Rw,
    cut: &Cut,
    part: Part,
    f: u8,
    width: usize,
    cur: &mut Cur,
    k: u64,
    st: &mut Stage<'_>,
) -> Piece {
    let bytes = cut.kind == Kind::Bytes;
    let a = (h, rw, cut, f, width, cur, k, st);
    match part {
        Part::Validity => match cut.kind {
            Kind::Nat => write_k::<_, K_NAT, P_VALIDITY>(a),
            Kind::Int => write_k::<_, K_INT, P_VALIDITY>(a),
            Kind::Bytes => write_k::<_, K_BYTES, P_VALIDITY>(a),
            _ => write_k::<_, K_BOOL, P_VALIDITY>(a),
        },
        Part::Cells if !bytes => {
            let c = match cut.kind.code() {
                K_CHAR | K_F32 => K_U4,
                K_F64 => K_U8,
                c => c,
            };
            match c {
                K_BOOL => write_k::<_, K_BOOL, P_CELLS>(a),
                K_U1 => write_k::<_, K_U1, P_CELLS>(a),
                K_U2 => write_k::<_, K_U2, P_CELLS>(a),
                K_U4 => write_k::<_, K_U4, P_CELLS>(a),
                K_U8 => write_k::<_, K_U8, P_CELLS>(a),
                K_I1 => write_k::<_, K_I1, P_CELLS>(a),
                K_I2 => write_k::<_, K_I2, P_CELLS>(a),
                K_I4 => write_k::<_, K_I4, P_CELLS>(a),
                K_I8 => write_k::<_, K_I8, P_CELLS>(a),
                K_NAT => write_k::<_, K_NAT, P_CELLS>(a),
                K_PRINCIPAL => write_k::<_, K_PRINCIPAL, P_CELLS>(a),
                _ => write_k::<_, K_INT, P_CELLS>(a),
            }
        }
        Part::Offsets if bytes => write_k::<_, K_BYTES, P_OFFSETS>(a),
        Part::OolBits if bytes => write_k::<_, K_BYTES, P_OOLBITS>(a),
        _ => Piece { slow: true, ..Piece::default() },
    }
}

/// The arguments of `write` past its part.
type WriteArgs<'x, 'b, H> = (&'x H, &'x Rw, &'x Cut, u8, usize, &'x mut Cur, u64, &'x mut Stage<'b>);

fn write_k<H: PageHost + ?Sized, const K: u8, const P: u8>(a: WriteArgs<'_, '_, H>) -> Piece {
    let (h, rw, cut, f, width, cur, k, st) = a;
    let mut v = Write::<K, P> {
        cut: *cut,
        form: f,
        width,
        k,
        cur: *cur,
        st: Stage { buf: &mut *st.buf, at: st.at },
        pc: Piece::default(),
    };
    let (q, end) = walk(h, rw, cur.q, &mut v);
    let mut pc = v.pc;
    // A segment's slot past the stored positions is the general path's fault to report.
    pc.slow = end == End::Slow || (end == End::Last && v.cur.slot < k);
    let at = v.st.at;
    *cur = Cur { q, ..v.cur };
    st.at = at;
    pc
}

/// Positions a visitor passes, stopping at a value out of line.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    /// Slots of the out-of-line directory's offsets, up to `k`: inline values write nothing.
    OolOffsets,
    /// Positions of the out-of-line area: inline values write nothing.
    OolArea,
}

/// A visitor that passes positions, counting slots for `Pass::OolOffsets`.
struct Skip {
    cut: Cut,
    pass: Pass,
    k: u64,
    slot: u64,
    pc: Piece,
}

impl<'a> Visit<'a> for Skip {
    #[inline(always)]
    fn piece(&mut self) -> &mut Piece {
        &mut self.pc
    }

    #[inline(always)]
    fn dead(&mut self, _q: u64) -> Flow {
        if self.pass == Pass::OolOffsets && self.slot >= self.k {
            return Flow::Stop;
        }
        self.slot = self.slot.wrapping_add(1);
        Flow::Next
    }

    #[inline(always)]
    fn live<R: Bytes<'a>>(&mut self, _q: u64, r: R, _long: bool) -> Flow {
        if self.pass == Pass::OolOffsets && self.slot >= self.k {
            return Flow::Stop;
        }
        match value::<R, K_BYTES>(&r, &self.cut) {
            Some(V::Bytes(b)) if b.len() as u64 > LONG_VALUE => return Flow::Slow,
            Some(_) => {}
            None => return Flow::Slow,
        }
        self.slot = self.slot.wrapping_add(1);
        Flow::Next
    }
}

fn skip<H: PageHost + ?Sized>(h: &H, rw: &Rw, cut: &Cut, pass: Pass, cur: &mut Cur, k: u64) -> Piece {
    // The callers pass byte string leaves only; any other is the general path's.
    if cut.kind != Kind::Bytes {
        return Piece { slow: true, ..Piece::default() };
    }
    let mut v = Skip { cut: *cut, pass, k, slot: cur.slot, pc: Piece::default() };
    let (q, end) = walk(h, rw, cur.q, &mut v);
    let mut pc = v.pc;
    pc.slow = end == End::Slow || (pass == Pass::OolOffsets && end == End::Last && v.slot < k);
    pc.end = end == End::Last;
    cur.slot = v.slot;
    cur.q = q;
    pc
}

/// The out-of-line directory's offsets: the slots whose values lie inline write nothing; a slot out of line
/// stops the walk (the general path writes its offset).
pub fn ool_offsets<H: PageHost + ?Sized>(h: &H, rw: &Rw, cut: &Cut, cur: &mut Cur, n: u64) -> Piece {
    skip(h, rw, cut, Pass::OolOffsets, cur, n)
}

/// Pass 2's visitor copying a leaf's inline values into its run in the inline data region.
struct Run<'b> {
    cut: Cut,
    st: Stage<'b>,
    pc: Piece,
}

impl<'a> Visit<'a> for Run<'_> {
    #[inline(always)]
    fn piece(&mut self) -> &mut Piece {
        &mut self.pc
    }

    #[inline(always)]
    fn dead(&mut self, _q: u64) -> Flow {
        Flow::Next
    }

    #[inline(always)]
    fn live<R: Bytes<'a>>(&mut self, _q: u64, r: R, _long: bool) -> Flow {
        let Some(v) = value::<R, K_BYTES>(&r, &self.cut) else { return Flow::Slow };
        if let V::Bytes(b) = v {
            if b.len() as u64 <= LONG_VALUE {
                if b.len() > self.st.room() {
                    self.pc.full = true;
                    return Flow::Stop;
                }
                self.st.put(b);
            }
        }
        Flow::Next
    }
}

/// A leaf's values in the inline data region (`ool` false) or the out-of-line area from position `cur.q`: each
/// stored live value of at most `LONG_VALUE` bytes is copied whole into the stage, or passed in the out-of-line
/// area; a value out of line stops the out-of-line walk (the general path copies it). The walk ends when the
/// stage lacks room for the next value, at a position the general path must read, or after the last stored
/// position (`end`).
pub fn run<H: PageHost + ?Sized>(h: &H, rw: &Rw, cut: &Cut, cur: &mut Cur, st: &mut Stage<'_>, ool: bool) -> Piece {
    if ool {
        return skip(h, rw, cut, Pass::OolArea, cur, 0);
    }
    // The callers pass byte string leaves only; any other is the general path's.
    if cut.kind != Kind::Bytes {
        return Piece { slow: true, ..Piece::default() };
    }
    let mut v = Run { cut: *cut, st: Stage { buf: &mut *st.buf, at: st.at }, pc: Piece::default() };
    let (q, end) = walk(h, rw, cur.q, &mut v);
    let mut pc = v.pc;
    pc.slow = end == End::Slow;
    pc.end = end == End::Last;
    let at = v.st.at;
    cur.q = q;
    st.at = at;
    pc
}
