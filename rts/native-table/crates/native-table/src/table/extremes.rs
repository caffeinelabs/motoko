//! A group's extremes: for each integer leaf (an integer of at most 64 bits, Nat, Int, u128 or i128), the
//! smallest and the largest of its non-null values over the group's live rows, kept by every write of the
//! group's rows in every row state, so that a leaf's minimum or maximum over the table reads one record per
//! group.
//!
//! **Extremes block** (area 0, zero-filled when allocated), made when its group opens and named by the group's
//! buffer descriptor (@48), then by its group page (@656), to which the seal's flip moves it, as a
//! compaction's flip moves it to the group's next page: {magic `NTEX` u32 @0; `FORMAT_TABLE` u16 @4; zero u16
//! @6; group number u64 @8; leaves u32 @16, the ordinals its records cover; zero @20..32; then from @32 a
//! record of 40 bytes per ordinal below the leaves}. A record is {flags u8 @0; zero @1..8; minimum u128 @8;
//! maximum u128 @24}, the values as `totals::Rank` ranks them. Flags: bit 0 the record holds a value; bit 1
//! the minimum is loose; bit 2 the maximum is loose; bit 3 a value beyond i128 joined the group.
//!
//! A value joining the group (an insert, the new side of an update) widens the record, and a minimum or
//! maximum it reaches is exact again. A value leaving the group (a delete, the old side of an update) that
//! equals the minimum or the maximum makes it loose: it still bounds the group's values, but no row may hold
//! it any more. A loose extreme, a group a value beyond i128 joined, and a leaf the block does not cover (one
//! a schema change added after the group opened, or an ordinal from `CAP` on) are read from the group's rows.

use core::cmp::Ordering;

use crate::area::alloc_zeroed;
use crate::consts::{magic, BLOCK, FORMAT_TABLE, RADIX_FAN};
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::mem::{put, put_u16, put_u32, put_u64, put_u8, rd_array, rd_u32, rd_u64, rd_u8, Owner, PageHost, Region};

use super::buffer::{self, Located, DESC_BIT};
use super::cur::{self, SVal};
use super::hot::Sign;
use super::layout::{LeafInfo, Shape, Ty};
use super::overlay;
use super::page;
use super::root;
use super::totals::{self, Rank, Total};

fn rt(kind: Corruption, at: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, at)
}

/// The leaves field.
pub const LEAVES: u64 = 16;
/// The first record.
pub const RECS: u64 = 32;
/// Bytes of a record.
pub const REC: u64 = 40;
/// The most ordinals a block covers.
pub const CAP: u64 = (BLOCK - RECS) / REC;

/// Record flags.
pub const HAS: u8 = 1;
pub const MIN_LOOSE: u8 = 2;
pub const MAX_LOOSE: u8 = 4;
pub const BIG: u8 = 8;

/// One leaf's record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rec {
    pub flags: u8,
    pub min: u128,
    pub max: u128,
}

impl Rec {
    fn read(b: &[u8], at: u64) -> Result<Rec, Fault> {
        Ok(Rec {
            flags: rd_u8(b, at)?,
            min: u128::from_le_bytes(rd_array::<16>(b, at.wrapping_add(8))?),
            max: u128::from_le_bytes(rd_array::<16>(b, at.wrapping_add(24))?),
        })
    }

    fn write(&self, b: &mut [u8], at: u64) -> Result<(), Fault> {
        put_u8(b, at, self.flags)?;
        put(b, at.wrapping_add(8), &self.min.to_le_bytes())?;
        put(b, at.wrapping_add(24), &self.max.to_le_bytes())
    }

    /// A value of rank x joins the group.
    fn join(&mut self, x: u128) {
        if self.flags & HAS == 0 {
            *self = Rec { flags: (self.flags | HAS) & !(MIN_LOOSE | MAX_LOOSE), min: x, max: x };
            return;
        }
        if x <= self.min {
            self.min = x;
            self.flags &= !MIN_LOOSE;
        }
        if x >= self.max {
            self.max = x;
            self.flags &= !MAX_LOOSE;
        }
    }

    /// A value of rank x leaves the group.
    fn leave(&mut self, x: u128) {
        if self.flags & HAS == 0 {
            return;
        }
        if x == self.min {
            self.flags |= MIN_LOOSE;
        }
        if x == self.max {
            self.flags |= MAX_LOOSE;
        }
    }
}

/// A new extremes block for group g, covering the ordinals of `shape` below `CAP`; 0 when none of them is a
/// live integer leaf, as nothing would be kept.
pub fn create<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, g: u64) -> Result<u64, Fault> {
    let n = shape.leaves.min(CAP);
    let mut any = false;
    for o in 0..n {
        let l = shape.leaf(&*h, o)?;
        if l.live() && l.ty.running().is_some() {
            any = true;
            break;
        }
    }
    if !any {
        return Ok(0);
    }
    let blk = alloc_zeroed(h, tbl, 0)?;
    let b = h.get_mut(Region::Block(blk), Owner(0))?;
    put_u32(b, 0, magic::NTEX)?;
    put_u16(b, 4, FORMAT_TABLE)?;
    put_u64(b, 8, g)?;
    put_u32(b, LEAVES, u32::try_from(n).map_err(|_| rt(Corruption::Range, n))?)?;
    Ok(blk)
}

/// The extremes block of the buffered group whose descriptor is `desc`.
pub fn of_desc<H: PageHost + ?Sized>(h: &H, desc: u64) -> Result<u64, Fault> {
    rd_u64(h.get(Region::Block(desc))?, buffer::D_EXT)
}

/// The extremes block of the sealed group whose page is `pg`.
pub fn of_page<H: PageHost + ?Sized>(h: &H, pg: u64) -> Result<u64, Fault> {
    rd_u64(h.get(Region::Block(pg))?, page::P_EXT)
}

/// Leaf `l`'s record in block `blk`; `None` when the block does not cover the leaf.
pub fn record<H: PageHost + ?Sized>(h: &H, blk: u64, l: &LeafInfo) -> Result<Option<Rec>, Fault> {
    let b = h.get(Region::Block(blk))?;
    let o = u64::from(l.ordinal);
    if o >= u64::from(rd_u32(b, LEAVES)?) {
        return Ok(None);
    }
    Rec::read(b, RECS.wrapping_add(o.wrapping_mul(REC))).map(Some)
}

/// A value of leaf `l` of rank `r` joins (`Sign::Add`) or leaves (`Sign::Remove`) the group whose extremes
/// block is `blk` (0: none). The block is written only when the record changes.
pub fn apply<H: PageHost>(h: &mut H, blk: u64, l: &LeafInfo, r: Rank, sign: Sign) -> Result<(), Fault> {
    if blk == 0 || r == Rank::Null {
        return Ok(());
    }
    let Some(old) = record(&*h, blk, l)? else { return Ok(()) };
    let mut new = old;
    match (r, sign) {
        (Rank::In(x), Sign::Add) => new.join(x),
        (Rank::In(x), Sign::Remove) => new.leave(x),
        (Rank::Big, Sign::Add) => new.flags |= BIG,
        _ => {}
    }
    if new != old {
        let at = RECS.wrapping_add(u64::from(l.ordinal).wrapping_mul(REC));
        new.write(h.get_mut(Region::Block(blk), Owner(0))?, at)?;
    }
    Ok(())
}

/// Names the extremes block of a rewrite's source (a frozen group's descriptor, `seal`, or a sealed group's
/// old page) on its new group page, at the flip; `true` when the source has one. The source still names it
/// until `forget`.
pub fn hand_over<H: PageHost>(h: &mut H, source: u64, seal: bool, pending: u64) -> Result<bool, Fault> {
    let at = if seal { buffer::D_EXT } else { page::P_EXT };
    let blk = rd_u64(h.get(Region::Block(source))?, at)?;
    if blk != 0 {
        put_u64(h.get_mut(Region::Block(pending), Owner(0))?, page::P_EXT, blk)?;
    }
    Ok(blk != 0)
}

/// Clears the source's name of the extremes block it handed over, before the source goes to the drain.
pub fn forget<H: PageHost>(h: &mut H, source: u64, seal: bool) -> Result<(), Fault> {
    let at = if seal { buffer::D_EXT } else { page::P_EXT };
    put_u64(h.get_mut(Region::Block(source), Owner(0))?, at, 0)
}

// ---------------------------------------------------------------- reading

/// The best value found so far.
#[derive(Clone, Copy, Debug)]
enum Best {
    None,
    In(u128),
    /// A Nat or Int beyond i128, where it lies.
    Big(SVal),
}

/// Whether a Nat's or Int's LEB128 value beyond i128 is negative.
fn negative<H: PageHost + ?Sized>(h: &H, l: &LeafInfo, s: &SVal) -> Result<bool, Fault> {
    if l.ty != Ty::Int {
        return Ok(false);
    }
    let last = s.len().checked_sub(1).ok_or(rt(Corruption::Encoding, u64::from(l.ordinal)))?;
    Ok(cur::byte(h, s, last)? & 0x40 != 0)
}

/// The order of two canonical LEB128 values beyond i128 of leaf `l`: by sign, then by length (a longer one is
/// further from zero), then by their 7-bit groups from the most significant.
fn order_big<H: PageHost + ?Sized>(h: &H, l: &LeafInfo, a: &SVal, b: &SVal) -> Result<Ordering, Fault> {
    let (na, nb) = (negative(h, l, a)?, negative(h, l, b)?);
    if na != nb {
        return Ok(if na { Ordering::Less } else { Ordering::Greater });
    }
    let (la, lb) = (a.len(), b.len());
    if la != lb {
        let o = la.cmp(&lb);
        return Ok(if na { o.reverse() } else { o });
    }
    let mut j = la;
    while j > 0 {
        j = j.wrapping_sub(1);
        let (x, y) = (cur::byte(h, a, j)? & 0x7F, cur::byte(h, b, j)? & 0x7F);
        if x != y {
            return Ok(x.cmp(&y));
        }
    }
    Ok(Ordering::Equal)
}

/// The order of two values of leaf `l`, neither `Best::None`.
fn order<H: PageHost + ?Sized>(h: &H, l: &LeafInfo, a: &Best, b: &Best) -> Result<Ordering, Fault> {
    Ok(match (a, b) {
        (Best::In(x), Best::In(y)) => x.cmp(y),
        (Best::Big(s), Best::In(_)) => {
            if negative(h, l, s)? {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        (Best::In(_), Best::Big(s)) => {
            if negative(h, l, s)? {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
        (Best::Big(s), Best::Big(t)) => order_big(h, l, s, t)?,
        _ => return Err(rt(Corruption::Range, u64::from(l.ordinal))),
    })
}

impl Best {
    /// Takes `c` when it is better than the value held: smaller, or with `largest` larger.
    fn offer<H: PageHost + ?Sized>(&mut self, h: &H, l: &LeafInfo, c: Best, largest: bool) -> Result<(), Fault> {
        if matches!(c, Best::None) {
            return Ok(());
        }
        let better = match *self {
            Best::None => true,
            held => {
                let o = order(h, l, &c, &held)?;
                if largest {
                    o == Ordering::Greater
                } else {
                    o == Ordering::Less
                }
            }
        };
        if better {
            *self = c;
        }
        Ok(())
    }

    /// Takes leaf `l`'s value `v` when it is better.
    fn offer_value<H: PageHost + ?Sized>(&mut self, h: &H, l: &LeafInfo, v: &SVal, largest: bool) -> Result<(), Fault> {
        match totals::rank_cur(h, l, v)? {
            Rank::Null => Ok(()),
            Rank::In(x) => self.offer(h, l, Best::In(x), largest),
            Rank::Big => self.offer(h, l, Best::Big(*v), largest),
        }
    }
}

/// Leaf `l`'s minimum (`largest`: maximum) over the live rows of group g, whose directory entry is `entry`,
/// read from its rows into `best`.
fn from_rows<H: PageHost + ?Sized>(
    h: &H,
    shape: &Shape,
    l: &LeafInfo,
    g: u64,
    entry: u64,
    largest: bool,
    best: &mut Best,
) -> Result<(), Fault> {
    if entry & DESC_BIT == 0 {
        let s = overlay::sealed(h, entry)?;
        for pos in 0..s.meta.p {
            if overlay::sealed_live(h, &s, pos)? {
                let v = overlay::sealed_value(h, shape, &s, l, pos)?;
                best.offer_value(h, l, &v, largest)?;
            }
        }
        return Ok(());
    }
    let desc = entry & !DESC_BIT;
    let frozen = buffer::u16_of(h, desc, buffer::D_FLAGS)? & u64::from(buffer::FROZEN) != 0;
    buffer::for_each_slot(h, desc, |pos, block, slot| {
        let at = Located { group: g, pos, desc, frozen, block, slot };
        if !overlay::buffered_live(h, &at)? {
            return Ok(());
        }
        let b = overlay::buffered(h, &at)?;
        let v = overlay::buffered_leaf(h, shape, &b, l)?;
        best.offer_value(h, l, &v, largest)
    })
}

/// A Nat's or Int's canonical LEB128 value `s` beyond i128 as a sum's big form: its two's-complement bytes,
/// little-endian, without redundant sign bytes, at the start of `buf` (`Scratch` when `buf` is shorter).
fn big_total<H: PageHost + ?Sized>(h: &H, l: &LeafInfo, s: &SVal, buf: &mut [u8]) -> Result<Total, Fault> {
    let n = s.len();
    let neg = negative(h, l, s)?;
    // Seven bits a LEB128 byte, and a sign byte.
    let need = n.checked_mul(7).ok_or(rt(Corruption::Range, n))?.div_ceil(8).wrapping_add(1);
    let k = usize::try_from(need).map_err(|_| rt(Corruption::Range, need))?;
    let out = buf.get_mut(..k).ok_or(Fault::Scratch { need })?;
    let (mut acc, mut bits, mut w) = (0u32, 0u32, 0usize);
    let mut piece = [0u8; 64];
    let mut j = 0u64;
    while j < n {
        let m = n.wrapping_sub(j).min(64);
        let p = piece.get_mut(..usize::try_from(m).unwrap_or(0)).ok_or(rt(Corruption::Range, m))?;
        cur::read(h, s, j, p)?;
        for &x in p.iter() {
            acc |= u32::from(x & 0x7F) << bits;
            bits = bits.wrapping_add(7);
            if bits >= 8 {
                *out.get_mut(w).ok_or(rt(Corruption::Range, j))? = acc.to_le_bytes()[0];
                acc >>= 8;
                bits = bits.wrapping_sub(8);
                w = w.wrapping_add(1);
            }
        }
        j = j.wrapping_add(m);
    }
    let ext = if neg { 0xFF } else { 0 };
    if bits > 0 {
        let v = if neg { acc | (0xFF << bits) } else { acc };
        *out.get_mut(w).ok_or(rt(Corruption::Range, n))? = v.to_le_bytes()[0];
        w = w.wrapping_add(1);
    }
    for b in out.iter_mut().skip(w) {
        *b = ext;
    }
    Ok(Total::Big(totals::trimmed(out)))
}

/// The smallest (`largest`: the largest) non-null value of integer leaf `l` over the table's live rows, `None`
/// when no live row holds one: each group's from its record, and a group's whose record does not give it read
/// from the group's rows. A value past i128 is written to `buf` as a sum is (`Total::Big`), or is
/// `Scratch { need }` when `buf` is shorter.
///
/// The group directory is read once a call, a directory of one level from its one block. Of each group's
/// record only the flags and the side asked for are read, in one lookup of its block, and two values within
/// i128 are compared in place, by a loop made for the side asked for.
pub fn extreme<H: PageHost + ?Sized>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    l: &LeafInfo,
    largest: bool,
    buf: &mut [u8],
) -> Result<Option<Total>, Fault> {
    if largest {
        extreme_of::<H, true>(h, tbl, shape, l, buf)
    } else {
        extreme_of::<H, false>(h, tbl, shape, l, buf)
    }
}

/// `extreme` for one side: the largest when `LARGEST`, else the smallest.
fn extreme_of<H: PageHost + ?Sized, const LARGEST: bool>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    l: &LeafInfo,
    buf: &mut [u8],
) -> Result<Option<Total>, Fault> {
    let largest = LARGEST;
    if l.ty.running().is_none() || !l.live() {
        return Err(Fault::BadRef);
    }
    let loose = if largest { MAX_LOOSE } else { MIN_LOOSE };
    let open = root::u64_at(h, tbl, root::OPEN_GROUP)?;
    let dir = root::directory(h, tbl)?;
    let one_level = if dir.height == 1 && dir.root != 0 { Some(h.get(Region::Block(dir.root))?) } else { None };
    let o = u64::from(l.ordinal);
    let at = RECS.wrapping_add(o.wrapping_mul(REC));
    let side = at.wrapping_add(if largest { 24 } else { 8 });
    let mut best = Best::None;
    for g in 0..=open {
        let entry = match one_level {
            // g < 512: its entry's offset is at most 4,088.
            Some(d) if g < RADIX_FAN => rd_u64(d, g.wrapping_mul(8))?,
            _ => dir.get(h, g)?,
        };
        if entry == 0 {
            continue;
        }
        let blk = if entry & DESC_BIT != 0 { of_desc(h, entry & !DESC_BIT)? } else { of_page(h, entry)? };
        if blk != 0 {
            let b = h.get(Region::Block(blk))?;
            if o < u64::from(rd_u32(b, LEAVES)?) {
                let flags = rd_u8(b, at)?;
                if flags & (BIG | loose) == 0 {
                    if flags & HAS != 0 {
                        let x = u128::from_le_bytes(rd_array::<16>(b, side)?);
                        match best {
                            Best::None => best = Best::In(x),
                            Best::In(y) => {
                                if (largest && x > y) || (!largest && x < y) {
                                    best = Best::In(x);
                                }
                            }
                            Best::Big(_) => best.offer(h, l, Best::In(x), largest)?,
                        }
                    }
                    continue;
                }
            }
        }
        from_rows(h, shape, l, g, entry, largest, &mut best)?;
    }
    match best {
        Best::None => Ok(None),
        Best::In(x) => totals::of_rank(l.ty, x, buf).map(Some),
        Best::Big(s) => big_total(h, l, &s, buf).map(Some),
    }
}
