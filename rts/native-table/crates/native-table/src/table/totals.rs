//! The totals of integer leaves, kept by every write: each integer leaf's running record holds the non-null
//! count and the exact sum of its values over every live row, sealed and buffered, so a sum or an average
//! reads one record at any table size. Float leaves keep no total: a float sum held as it runs would depend
//! on the order of the writes and drift under deletes.
//!
//! **Records** (`stats::running`): `R_INT` for integers of at most 64 bits, `R_NUM` for Nat and Int, `R_WIDE`
//! for u128 and i128. A record lies in the hot state when the rule of room gives it a place there
//! (`Shape::open`), else in the totals space, at the same layout.
//!
//! **Totals space**: one byte space made of bytes 16–4,095 of each totals block (area 0: {magic `NTTS` u32
//! @0; `FORMAT_TABLE` u16 @4; zero u16 @6; zero u64 @8; 4,080 bytes of the space}), in the order of the radix
//! array whose root (u64 @72) and height (u8 @80) the hot state holds; root 0: no space. The space starts with
//! a header {used bytes u32 @0; the records' offset u32 @4}. The records are those of the live integer leaves
//! the hot state's running-record list does not name, in ordinal order (at first `Shape::tot_bytes` of them,
//! right after the header); the rooms of big-form sums follow wherever there is space. A schema change that
//! changes which leaves have records here writes them anew at the space's end and leaves the old ones unused.
//!
//! **Big-form sums**: an `R_NUM` keeps the values within 64 bits (a Nat below 2^64, an Int inside i64) in its
//! i128, and every other value in its big-form sum: two's-complement bytes, little-endian, in a room of the
//! totals space {capacity u32 @0; zero u32 @4; the sum's bytes from 8}. A sum that outgrows its room moves to
//! a new room at the space's end with twice the bytes it needs, and the old room is left unused.

use crate::area::{alloc_block, alloc_zeroed};
use crate::consts::{magic, BIGSUM_INLINE, BLOCK, FORMAT_TABLE};
use crate::fault::{rt, Corruption, Fault};
use crate::mem::{put, put_u16, put_u32, put_u64, put_u8, rd_array, rd_u32, rd_u64, rd_u8, PageHost, Region};
use crate::radix::Radix;
use crate::stats::running::{self, RKind, R_BIG_LEN, R_BIG_ROOM, R_NONNULL, R_SUM};
use crate::stats::I192;

use super::buffer::{H_TOTALS, H_TOTALS_HEIGHT};
use super::cell::{self, Val};
use super::cur::{self, SVal};
use super::group::align8;
use super::hot::Sign;
use super::layout::{LeafInfo, Ty};

/// Bytes before a totals block's share of the space.
pub const BLOCK_HEAD: u64 = 16;
/// Bytes of the space a totals block holds.
pub const BLOCK_ROOM: u64 = BLOCK - BLOCK_HEAD;
/// Bytes of the space's header; the records start after it.
pub const SPACE_HEAD: u64 = 8;
/// Bytes of a room's header; the sum's bytes start after it.
pub const ROOM_HEAD: u64 = 8;
/// The most bytes of a record.
const REC: usize = 32;

/// Where a leaf's running record lies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    /// The leaf has none.
    None,
    /// At this offset of the hot state's block.
    Hot(u64),
    /// At this offset of the totals space's records.
    Space(u64),
}

// ---------------------------------------------------------------- the totals space

/// The totals space named by a hot state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Totals {
    pub blocks: Radix,
    pub used: u64,
    /// The offset of the records.
    pub recs: u64,
    /// The last block found, by its index in the space (the radix is walked once per block).
    last: (u64, u64),
}

impl Totals {
    /// The space the hot state of block `tail` names, `None` when it names none.
    pub fn read<H: PageHost + ?Sized>(h: &H, tail: u64) -> Result<Option<Totals>, Fault> {
        let b = h.get(Region::Block(tail))?;
        let blocks = Radix { root: rd_u64(b, H_TOTALS)?, height: rd_u8(b, H_TOTALS_HEIGHT)? };
        if blocks.root == 0 {
            return Ok(None);
        }
        blocks.check()?;
        let mut t = Totals { blocks, used: SPACE_HEAD, recs: SPACE_HEAD, last: (u64::MAX, 0) };
        t.used = u64::from(t.u32_at(h, 0)?);
        t.recs = u64::from(t.u32_at(h, 4)?);
        if t.used < SPACE_HEAD || t.recs < SPACE_HEAD || t.recs > t.used {
            return Err(rt(Corruption::Header, tail));
        }
        Ok(Some(t))
    }

    /// A new space of `bytes` zero bytes past its header, named by the hot state of block `tail`.
    pub fn create<H: PageHost>(h: &mut H, tbl: u64, tail: u64, bytes: u64) -> Result<Totals, Fault> {
        let mut t = Totals { blocks: Radix::EMPTY, used: 0, recs: SPACE_HEAD, last: (u64::MAX, 0) };
        let end = SPACE_HEAD.checked_add(bytes).ok_or(rt(Corruption::Range, bytes))?;
        t.grow_to(h, tbl, end, true)?;
        t.used = end;
        t.write_used(h)?;
        t.write_hot(h, tail)?;
        Ok(t)
    }

    /// The space of block `tail`'s hot state, made empty when it names none.
    pub(super) fn ensure<H: PageHost>(h: &mut H, tbl: u64, tail: u64) -> Result<Totals, Fault> {
        match Totals::read(&*h, tail)? {
            Some(t) => Ok(t),
            None => Totals::create(h, tbl, tail, 0),
        }
    }

    fn write_hot<H: PageHost>(&self, h: &mut H, tail: u64) -> Result<(), Fault> {
        let b = h.get_mut(Region::Block(tail))?;
        put_u64(b, H_TOTALS, self.blocks.root)?;
        put_u8(b, H_TOTALS_HEIGHT, self.blocks.height)
    }

    fn write_used<H: PageHost>(&mut self, h: &mut H) -> Result<(), Fault> {
        let used = u32::try_from(self.used).map_err(|_| rt(Corruption::Range, self.used))?;
        let recs = u32::try_from(self.recs).map_err(|_| rt(Corruption::Range, self.recs))?;
        self.put_bytes(h, 0, &used.to_le_bytes())?;
        self.put_bytes(h, 4, &recs.to_le_bytes())
    }

    /// Moves the records to space offset `at`, inside the used bytes.
    pub(super) fn set_recs<H: PageHost>(&mut self, h: &mut H, at: u64) -> Result<(), Fault> {
        if at < SPACE_HEAD || at > self.used {
            return Err(rt(Corruption::Range, at));
        }
        self.recs = at;
        self.write_used(h)
    }

    /// The block that holds space byte `off`, the byte's offset in it and the bytes of the space contiguous
    /// from there.
    fn locate<H: PageHost + ?Sized>(&self, h: &H, off: u64) -> Result<(u64, u64, u64), Fault> {
        let k = off / BLOCK_ROOM;
        let o = off % BLOCK_ROOM;
        let b = if self.last.0 == k { self.last.1 } else { self.blocks.get(h, k)? };
        if b == 0 {
            return Err(rt(Corruption::Link, off));
        }
        Ok((b, BLOCK_HEAD.wrapping_add(o), BLOCK_ROOM.wrapping_sub(o)))
    }

    /// `locate`, remembering the block for the next byte.
    fn locate_kept<H: PageHost + ?Sized>(&mut self, h: &H, off: u64) -> Result<(u64, u64, u64), Fault> {
        let r = self.locate(h, off)?;
        self.last = (off / BLOCK_ROOM, r.0);
        Ok(r)
    }

    /// Copies space bytes [off, off + out.len()) into `out`.
    pub fn read_bytes<H: PageHost + ?Sized>(&mut self, h: &H, off: u64, out: &mut [u8]) -> Result<(), Fault> {
        let end = off.checked_add(out.len() as u64).ok_or(rt(Corruption::Range, off))?;
        if end > self.used {
            return Err(rt(Corruption::Bounds, off));
        }
        let mut at = off;
        let mut rest = out;
        while !rest.is_empty() {
            let (b, o, room) = self.locate_kept(h, at)?;
            let n = usize::try_from(room).map_err(|_| rt(Corruption::Range, at))?.min(rest.len());
            let (now, later) = rest.split_at_mut_checked(n).ok_or(rt(Corruption::Range, at))?;
            crate::mem::copy_bytes(now, crate::mem::sub(h.get(Region::Block(b))?, o, n as u64)?);
            at = at.wrapping_add(n as u64);
            rest = later;
        }
        Ok(())
    }

    /// Writes `src` at space offset `off`, inside the used bytes.
    pub fn write_bytes<H: PageHost>(&mut self, h: &mut H, off: u64, src: &[u8]) -> Result<(), Fault> {
        let end = off.checked_add(src.len() as u64).ok_or(rt(Corruption::Range, off))?;
        if end > self.used {
            return Err(rt(Corruption::Bounds, off));
        }
        self.put_bytes(h, off, src)
    }

    fn put_bytes<H: PageHost>(&mut self, h: &mut H, off: u64, src: &[u8]) -> Result<(), Fault> {
        let mut at = off;
        let mut rest = src;
        while !rest.is_empty() {
            let (b, o, room) = self.locate_kept(&*h, at)?;
            let n = usize::try_from(room).map_err(|_| rt(Corruption::Range, at))?.min(rest.len());
            let (now, later) = rest.split_at_checked(n).ok_or(rt(Corruption::Range, at))?;
            put(h.get_mut(Region::Block(b))?, o, now)?;
            at = at.wrapping_add(n as u64);
            rest = later;
        }
        Ok(())
    }

    fn u32_at<H: PageHost + ?Sized>(&mut self, h: &H, off: u64) -> Result<u32, Fault> {
        let mut b = [0u8; 4];
        self.read_bytes(h, off, &mut b)?;
        Ok(u32::from_le_bytes(b))
    }

    fn byte<H: PageHost + ?Sized>(&mut self, h: &H, off: u64) -> Result<u8, Fault> {
        let (b, o, _) = self.locate_kept(h, off)?;
        rd_u8(h.get(Region::Block(b))?, o)
    }

    /// Extends the space's blocks to cover `end` bytes, each new block's header written and, with `zero`, its
    /// room zeroed.
    fn grow_to<H: PageHost>(&mut self, h: &mut H, tbl: u64, end: u64, zero: bool) -> Result<(), Fault> {
        let need = end.div_ceil(BLOCK_ROOM);
        let mut k = self.used.div_ceil(BLOCK_ROOM);
        while k < need {
            let b = if zero { alloc_zeroed(h, tbl, 0)? } else { alloc_block(h, tbl, 0)? };
            {
                let s = h.get_mut(Region::Block(b))?;
                put_u32(s, 0, magic::NTTS)?;
                put_u16(s, 4, FORMAT_TABLE)?;
                put_u16(s, 6, 0)?;
                put_u64(s, 8, 0)?;
            }
            let mut list = self.blocks;
            list.set(h, tbl, k, b)?;
            self.blocks = list;
            // k < need, which a u64 holds.
            k = k.wrapping_add(1);
        }
        Ok(())
    }

    /// Appends `bytes` of room at the space's end and returns its offset; the header and, when the block list
    /// changed, the hot state of block `tail` are written. The room's bytes are unspecified until written.
    pub(super) fn append<H: PageHost>(&mut self, h: &mut H, tbl: u64, tail: u64, bytes: u64) -> Result<u64, Fault> {
        let at = self.used;
        let end = at.checked_add(bytes).filter(|&e| e <= u64::from(u32::MAX)).ok_or(rt(Corruption::Range, at))?;
        let before = self.blocks;
        self.grow_to(h, tbl, end, false)?;
        self.used = end;
        self.write_used(h)?;
        if self.blocks != before {
            self.write_hot(h, tail)?;
        }
        Ok(at)
    }
}

// ---------------------------------------------------------------- values as totals see them

/// An integer leaf's value in the form a write holds it, by reference.
#[derive(Clone, Copy, Debug)]
pub enum TSrc<'a> {
    /// A checked host cell.
    Val(&'a Val<'a>),
    /// A value wherever it lies, as an update or a rewrite holds it.
    Cur(&'a SVal),
}

/// The canonical LEB128 bytes of a Nat or Int beyond 64 bits, where they lie.
#[derive(Clone, Copy, Debug)]
enum Leb<'a> {
    /// In a slice: a host's cell.
    Slice(&'a [u8]),
    /// A value wherever it lies.
    Cur(&'a SVal),
}

impl Leb<'_> {
    fn byte<H: PageHost + ?Sized>(&self, h: &H, j: u64) -> Result<u8, Fault> {
        let mut b = [0u8; 1];
        self.read(h, j, &mut b)?;
        Ok(b[0])
    }

    /// Copies bytes [j, j + out.len()).
    fn read<H: PageHost + ?Sized>(&self, h: &H, j: u64, out: &mut [u8]) -> Result<(), Fault> {
        match self {
            Leb::Slice(s) => {
                crate::mem::copy_bytes(out, crate::mem::sub(s, j, out.len() as u64)?);
                Ok(())
            }
            Leb::Cur(s) => cur::read(h, s, j, out),
        }
    }
}

/// One integer leaf's value as its total takes it.
#[derive(Clone, Copy, Debug)]
enum TVal<'a> {
    /// A null, or a leaf without a total.
    None,
    /// A value within 64 bits.
    Small(i128),
    /// A Nat or Int beyond 64 bits that fits i128.
    Big128(i128),
    /// A Nat or Int beyond i128: its canonical LEB128 form (signed for Int) of this many bytes.
    Leb(Leb<'a>, u64),
    U128(u128),
    I128(i128),
}

/// Whether `x` is within 64 bits for leaf type `t`: below 2^64 for a Nat, inside i64 for an Int.
fn within_64(t: Ty, x: i128) -> bool {
    match t {
        Ty::Nat => u64::try_from(x).is_ok(),
        _ => i64::try_from(x).is_ok(),
    }
}

/// A Nat or Int value that fits i128.
fn num(t: Ty, x: i128) -> TVal<'static> {
    if within_64(t, x) {
        TVal::Small(x)
    } else {
        TVal::Big128(x)
    }
}

/// The value of a checked host cell of leaf `l`.
fn of_val<'a>(l: &LeafInfo, v: &Val<'a>) -> Result<TVal<'a>, Fault> {
    let bad = || rt(Corruption::Encoding, u64::from(l.ordinal));
    Ok(match (l.ty.running(), *v) {
        (None, _) | (_, Val::Null) => TVal::None,
        (Some(RKind::Int), Val::U(x)) => TVal::Small(i128::from(x)),
        (Some(RKind::Int), Val::I(x)) => TVal::Small(i128::from(x)),
        (Some(RKind::Num), Val::U(x)) => num(l.ty, i128::from(x)),
        (Some(RKind::Num), Val::I(x)) => num(l.ty, i128::from(x)),
        (Some(RKind::Num), Val::Wide(x)) => num(l.ty, x),
        (Some(RKind::Num), Val::Big(b)) => TVal::Leb(Leb::Slice(b), b.len() as u64),
        (Some(RKind::Wide), Val::U128(x)) => TVal::U128(x),
        (Some(RKind::Wide), Val::I128(x)) => TVal::I128(x),
        _ => return Err(bad()),
    })
}

/// The value of leaf `l` held as `s`.
fn of_cur<'a>(l: &LeafInfo, s: &'a SVal) -> Result<TVal<'a>, Fault> {
    match s {
        SVal::Null => Ok(TVal::None),
        SVal::V(v) => of_val(l, v),
        SVal::B { len, .. } | SVal::Huge { len, .. } if l.ty.running() == Some(RKind::Num) => {
            Ok(TVal::Leb(Leb::Cur(s), *len))
        }
        _ if l.ty.running().is_none() => Ok(TVal::None),
        _ => Err(rt(Corruption::Encoding, u64::from(l.ordinal))),
    }
}

/// A `TVal::Leb` of at most 19 bytes that fits i128 as the value's class, within 64 bits or `Big128`, so a
/// value is kept in the same part of a sum whatever form it is stored in.
fn decoded<H: PageHost + ?Sized>(h: &H, t: Ty, src: &Leb<'_>, n: u64) -> Result<Option<TVal<'static>>, Fault> {
    if n > 19 {
        return Ok(None);
    }
    let mut b = [0u8; 19];
    let k = usize::try_from(n).map_err(|_| rt(Corruption::Range, n))?;
    let bytes = b.get_mut(..k).ok_or(rt(Corruption::Range, n))?;
    for (j, x) in bytes.iter_mut().enumerate() {
        *x = src.byte(h, j as u64)?;
    }
    Ok(cell::leb_value(bytes, matches!(t, Ty::Int)).map(|x| num(t, x)))
}

// ---------------------------------------------------------------- values as a group's extremes see them

/// Where an integer leaf's value stands among the leaf's values, as its group's extremes (`extremes`) keep
/// it: null; a value within i128 (any value of a u128 leaf) as an unsigned number in the leaf's order, a
/// signed leaf's with its sign bit flipped; or a Nat or Int beyond i128.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rank {
    Null,
    In(u128),
    Big,
}

pub fn signed(t: Ty) -> bool {
    matches!(t, Ty::I(_) | Ty::Int | Ty::I128)
}

/// The rank of value `x` of integer leaf type `t`, within i128.
pub fn rank_of_int(t: Ty, x: i128) -> u128 {
    // The bits of x, its sign bit flipped for a signed type.
    let u = x as u128;
    if signed(t) {
        u ^ (1 << 127)
    } else {
        u
    }
}

fn rank_of(l: &LeafInfo, v: &TVal<'_>) -> Rank {
    match *v {
        TVal::None => Rank::Null,
        TVal::Small(x) | TVal::Big128(x) | TVal::I128(x) => Rank::In(rank_of_int(l.ty, x)),
        TVal::U128(x) => Rank::In(x),
        TVal::Leb(..) => Rank::Big,
    }
}

/// The rank of leaf `l`'s value held as `s`.
pub fn rank_cur<H: PageHost + ?Sized>(h: &H, l: &LeafInfo, s: &SVal) -> Result<Rank, Fault> {
    Ok(match of_cur(l, s)? {
        TVal::Leb(src, n) => match decoded(h, l.ty, &src, n)? {
            Some(x) => rank_of(l, &x),
            None => Rank::Big,
        },
        v => rank_of(l, &v),
    })
}

/// The value of rank `r` of integer leaf type `t`: `Small` when it fits i128, else its bytes written to `buf`
/// as a sum's are.
pub fn of_rank(t: Ty, r: u128, buf: &mut [u8]) -> Result<Total, Fault> {
    if signed(t) {
        return Ok(Total::Small((r ^ (1 << 127)) as i128));
    }
    if let Ok(x) = i128::try_from(r) {
        return Ok(Total::Small(x));
    }
    let mut b = [0u8; 17];
    crate::mem::copy_bytes(b.get_mut(..16).unwrap_or(&mut []), &r.to_le_bytes());
    bytes_total(&b, buf)
}

// ---------------------------------------------------------------- keeping a total

/// The record bytes a leaf's total takes, `None` for a leaf without one.
fn rec_len(l: &LeafInfo) -> Option<usize> {
    l.ty.running().map(|k| usize::try_from(k.bytes()).unwrap_or(REC).min(REC))
}

/// Moves the total of integer leaf `l` (its record at `at`, the hot state in block `tail`) by one value:
/// added with `Sign::Add`, taken away with `Sign::Remove`; the value joins or leaves the extremes of its
/// row's group too, whose extremes block is `ext` (0: none).
#[allow(clippy::too_many_arguments)]
pub fn apply<H: PageHost>(
    h: &mut H,
    tbl: u64,
    tail: u64,
    at: At,
    l: &LeafInfo,
    src: TSrc<'_>,
    sign: Sign,
    ext: u64,
) -> Result<(), Fault> {
    let sub = sign == Sign::Remove;
    // The common case first: a value within 64 bits (any integer of at most 64 bits, a Nat below 2^64, an
    // Int inside i64), read straight from the form it comes in.
    let small = match src {
        TSrc::Val(Val::Null) | TSrc::Cur(SVal::Null) => return Ok(()),
        TSrc::Val(Val::U(x)) | TSrc::Cur(SVal::V(Val::U(x))) => Some(i128::from(*x)),
        TSrc::Val(Val::I(x)) | TSrc::Cur(SVal::V(Val::I(x))) => Some(i128::from(*x)),
        _ => None,
    };
    if let (Some(x), At::Hot(o)) = (small, at) {
        let b = h.get_mut(Region::Block(tail))?;
        running::add_count(b, o.wrapping_add(R_NONNULL), if sub { -1 } else { 1 })?;
        running::add_sum(b, o.wrapping_add(R_SUM), if sub { x.wrapping_neg() } else { x })?;
        return super::extremes::apply(h, ext, l, Rank::In(rank_of_int(l.ty, x)), sign);
    }
    let Some(n) = rec_len(l) else { return Ok(()) };
    let tv = match (small, src) {
        (Some(x), _) => TVal::Small(x),
        (None, TSrc::Val(v)) => of_val(l, v)?,
        (None, TSrc::Cur(s)) => of_cur(l, s)?,
    };
    let v = &tv;
    if matches!(v, TVal::None) {
        return Ok(());
    }
    let small;
    let v = match v {
        TVal::Leb(src, len) => match decoded(&*h, l.ty, src, *len)? {
            Some(x) => {
                small = x;
                &small
            }
            None => v,
        },
        _ => v,
    };
    super::extremes::apply(h, ext, l, rank_of(l, v), sign)?;
    match at {
        At::None => Ok(()),
        At::Hot(o) => {
            let big = move_rec(h.get_mut(Region::Block(tail))?, o, v, sign)?;
            if let Some((len, room)) = big {
                let (len, room) = move_big(h, tbl, tail, len, room, l, v, sign)?;
                let b = h.get_mut(Region::Block(tail))?;
                put_u32(b, o.wrapping_add(R_BIG_LEN), len)?;
                put_u32(b, o.wrapping_add(R_BIG_ROOM), room)?;
            }
            Ok(())
        }
        At::Space(o) => {
            let mut r = [0u8; REC];
            let rec = r.get_mut(..n).ok_or(rt(Corruption::Range, o))?;
            let mut sp = Totals::read(&*h, tail)?.ok_or(rt(Corruption::Link, tail))?;
            let o = sp.recs.wrapping_add(o);
            sp.read_bytes(&*h, o, rec)?;
            if let Some((len, room)) = move_rec(rec, 0, v, sign)? {
                let (len, room) = move_big(h, tbl, tail, len, room, l, v, sign)?;
                put_u32(rec, R_BIG_LEN, len)?;
                put_u32(rec, R_BIG_ROOM, room)?;
                sp = Totals::read(&*h, tail)?.ok_or(rt(Corruption::Link, tail))?;
            }
            sp.write_bytes(h, o, rec)
        }
    }
}

/// Moves the record at `o` of `b` by one non-null value: its count, and its i128 or i192 sum. For a value of
/// the big-form sum, returns the sum's length and room for the caller to move.
fn move_rec(b: &mut [u8], o: u64, v: &TVal<'_>, sign: Sign) -> Result<Option<(u32, u32)>, Fault> {
    let sub = sign == Sign::Remove;
    running::add_count(b, o.wrapping_add(R_NONNULL), if sub { -1 } else { 1 })?;
    let at = o.wrapping_add(R_SUM);
    match *v {
        TVal::None => {}
        TVal::Small(x) => running::add_sum(b, at, if sub { x.wrapping_neg() } else { x })?,
        TVal::U128(x) => {
            let mut s = i192_at(b, at)?;
            s.apply_u128(x, sub);
            put_i192(b, at, &s)?;
        }
        TVal::I128(x) => {
            let mut s = i192_at(b, at)?;
            s.apply_i128(x, sub);
            put_i192(b, at, &s)?;
        }
        TVal::Big128(_) | TVal::Leb(..) => {
            return Ok(Some((rd_u32(b, o.wrapping_add(R_BIG_LEN))?, rd_u32(b, o.wrapping_add(R_BIG_ROOM))?)))
        }
    }
    Ok(None)
}

fn i192_at(b: &[u8], at: u64) -> Result<I192, Fault> {
    let mut s = I192::default();
    for (i, limb) in s.0.iter_mut().enumerate() {
        *limb = rd_u64(b, at.wrapping_add((i as u64).wrapping_mul(8)))?;
    }
    Ok(s)
}

fn put_i192(b: &mut [u8], at: u64, s: &I192) -> Result<(), Fault> {
    for (i, limb) in s.0.iter().enumerate() {
        put_u64(b, at.wrapping_add((i as u64).wrapping_mul(8)), *limb)?;
    }
    Ok(())
}

/// Adds a value to (or takes it from) the big-form sum of `len` bytes in room `room` (0: none yet), moving
/// the sum to a room with twice the bytes it needs when it does not fit. Returns its new length and room.
#[allow(clippy::too_many_arguments)]
fn move_big<H: PageHost>(
    h: &mut H,
    tbl: u64,
    tail: u64,
    len: u32,
    room: u32,
    l: &LeafInfo,
    v: &TVal<'_>,
    sign: Sign,
) -> Result<(u32, u32), Fault> {
    let signed = matches!(l.ty, Ty::Int);
    let mut small = [0u8; 19];
    let (src, n) = match *v {
        TVal::Big128(x) => {
            let k = if signed { cell::put_sleb128(&mut small, x) } else { cell::put_uleb128(&mut small, x as u128) };
            (Leb::Slice(small.get(..k).unwrap_or(&[])), k as u64)
        }
        TVal::Leb(src, n) => (src, n),
        _ => return Err(rt(Corruption::Encoding, u64::from(l.ordinal))),
    };
    if n == 0 {
        return Err(rt(Corruption::Encoding, u64::from(l.ordinal)));
    }
    let mut sp = Totals::ensure(h, tbl, tail)?;
    let len = u64::from(len);
    let mut room = u64::from(room);
    let vbytes = n.checked_mul(7).ok_or(rt(Corruption::Range, n))?.div_ceil(8);
    let need = len.max(vbytes).checked_add(1).ok_or(rt(Corruption::Range, len))?;
    let mut cap = if room == 0 { 0 } else { u64::from(sp.u32_at(&*h, room)?) };
    if need > cap {
        let ncap = align8(need.checked_mul(2).ok_or(rt(Corruption::Range, need))?).max(BIGSUM_INLINE);
        let to = sp.append(h, tbl, tail, ROOM_HEAD.wrapping_add(ncap))?;
        let c32 = u32::try_from(ncap).map_err(|_| rt(Corruption::Range, ncap))?;
        sp.write_bytes(h, to, &c32.to_le_bytes())?;
        sp.write_bytes(h, to.wrapping_add(4), &[0; 4])?;
        let mut piece = [0u8; 64];
        let mut k = 0u64;
        while k < len {
            let m = len.wrapping_sub(k).min(64);
            let pc = piece.get_mut(..usize::try_from(m).unwrap_or(0)).unwrap_or(&mut []);
            sp.read_bytes(&*h, room.wrapping_add(ROOM_HEAD).wrapping_add(k), pc)?;
            sp.write_bytes(h, to.wrapping_add(ROOM_HEAD).wrapping_add(k), pc)?;
            k = k.wrapping_add(m);
        }
        room = to;
        cap = ncap;
    }
    let at = room.wrapping_add(ROOM_HEAD);
    let new = big_add(h, &mut sp, at, len, cap, &src, n, signed, sign == Sign::Remove)?;
    let new = u32::try_from(new).map_err(|_| rt(Corruption::Range, new))?;
    let room = u32::try_from(room).map_err(|_| rt(Corruption::Range, room))?;
    Ok((new, room))
}

/// `stats::big_apply` on a sum in the totals space and a value read through the host: adds (or subtracts)
/// the value whose canonical (signed) LEB128 form is the `leb_len` bytes of `leb` to the sum of `len` bytes
/// at space offset `at`, which holds `cap` bytes. Returns the sum's new length.
#[allow(clippy::too_many_arguments)]
fn big_add<H: PageHost>(
    h: &mut H,
    sp: &mut Totals,
    at: u64,
    len: u64,
    cap: u64,
    leb: &Leb<'_>,
    leb_len: u64,
    signed: bool,
    subtract: bool,
) -> Result<u64, Fault> {
    let vbytes = leb_len.checked_mul(7).ok_or(rt(Corruption::Range, leb_len))?.div_ceil(8);
    let need = len.max(vbytes).checked_add(1).ok_or(rt(Corruption::Range, len))?;
    if need > cap {
        return Err(rt(Corruption::Range, need));
    }
    let neg_v = signed && leb.byte(&*h, leb_len.wrapping_sub(1))? & 0x40 != 0;
    let top = if len == 0 { 0 } else { sp.byte(&*h, at.wrapping_add(len).wrapping_sub(1))? };
    let ext: u8 = if top & 0x80 != 0 { 0xFF } else { 0 };
    let gext: u8 = if neg_v { 0x7F } else { 0 };
    let mut carry: u16 = u16::from(subtract);
    // In pieces of `PIECE` sum bytes: piece p takes the value's 7-bit groups from 8·PIECE·p/7 on, a whole
    // number since PIECE is a multiple of 7, and its bits are the groups' bits in order.
    let mut groups = [0u8; PIECE_GROUPS];
    let mut sum = [0u8; PIECE];
    let mut i = 0u64;
    while i < need {
        let m = need.wrapping_sub(i).min(PIECE as u64);
        let g0 = (i / 7).wrapping_mul(8);
        let gn = leb_len.saturating_sub(g0).min(PIECE_GROUPS as u64);
        let gk = usize::try_from(gn).map_err(|_| rt(Corruption::Range, gn))?;
        let (have, past) = groups.split_at_mut_checked(gk).ok_or(rt(Corruption::Range, gn))?;
        if !have.is_empty() {
            leb.read(&*h, g0, have)?;
            have.iter_mut().for_each(|x| *x &= 0x7F);
        }
        past.fill(gext);
        let sn = len.saturating_sub(i).min(m);
        let mk = usize::try_from(m).map_err(|_| rt(Corruption::Range, m))?;
        let s = sum.get_mut(..mk).ok_or(rt(Corruption::Range, m))?;
        let sk = usize::try_from(sn).map_err(|_| rt(Corruption::Range, sn))?;
        let (held, above) = s.split_at_mut_checked(sk).ok_or(rt(Corruption::Range, sn))?;
        sp.read_bytes(&*h, at.wrapping_add(i), held)?;
        above.fill(ext);
        let (mut acc, mut bits, mut next) = (0u32, 0u32, groups.iter());
        for a in s.iter_mut() {
            while bits < 8 {
                acc |= u32::from(next.next().copied().unwrap_or(gext)) << bits;
                bits = bits.wrapping_add(7);
            }
            let mut x = acc.to_le_bytes()[0];
            acc >>= 8;
            bits = bits.wrapping_sub(8);
            if subtract {
                x = !x;
            }
            let t = u16::from(*a).wrapping_add(u16::from(x)).wrapping_add(carry);
            *a = t.to_le_bytes()[0];
            carry = t >> 8;
        }
        sp.put_bytes(h, at.wrapping_add(i), s)?;
        i = i.wrapping_add(m);
    }
    space_trimmed(h, sp, at, need)
}

/// Sum bytes a big-form sum is moved by at a time (a multiple of 7), and the value's 7-bit groups they take.
const PIECE: usize = 448;
const PIECE_GROUPS: usize = PIECE / 7 * 8;

/// The length of the two's-complement sum of `n` bytes at space offset `at` without its redundant sign bytes
/// (0 for zero).
fn space_trimmed<H: PageHost + ?Sized>(h: &H, sp: &mut Totals, at: u64, need: u64) -> Result<u64, Fault> {
    let mut n = need;
    let mut buf = [0u8; PIECE];
    // The top bytes first, then whole pieces downwards: most sums stop at their top two bytes.
    let mut window = 16u64;
    'down: while n > 1 {
        let k = n.min(window);
        let lo = n.wrapping_sub(k);
        let kk = usize::try_from(k).map_err(|_| rt(Corruption::Range, k))?;
        let w = buf.get_mut(..kk).ok_or(rt(Corruption::Range, k))?;
        sp.read_bytes(h, at.wrapping_add(lo), w)?;
        window = PIECE as u64;
        // Bytes n − 1 and n − 2 while both lie in the window.
        while n > 1 && n.wrapping_sub(2) >= lo {
            let j = usize::try_from(n.wrapping_sub(2).wrapping_sub(lo)).map_err(|_| rt(Corruption::Range, n))?;
            let (below, hi) = match w.get(j..) {
                Some([lower, upper, ..]) => (*lower, *upper),
                _ => return Err(rt(Corruption::Range, n)),
            };
            if (hi == 0 && below & 0x80 == 0) || (hi == 0xFF && below & 0x80 != 0) {
                n = n.wrapping_sub(1);
            } else {
                break 'down;
            }
        }
    }
    if n == 1 && sp.byte(h, at)? == 0 {
        n = 0;
    }
    Ok(n)
}

// ---------------------------------------------------------------- reading a total

/// An integer leaf's sum: `Small` when it fits i128, else `Big(n)`, its n bytes, two's complement and
/// little-endian, written at the start of the caller's buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Total {
    Small(i128),
    Big(usize),
}

/// The sum and the non-null count of integer leaf `l`, its record at `at` (the hot state in block `tail`). A
/// sum past i128 that `buf` cannot hold is `Scratch { need }`.
pub fn total<H: PageHost + ?Sized>(
    h: &H,
    tail: u64,
    at: At,
    l: &LeafInfo,
    buf: &mut [u8],
) -> Result<(Total, u64), Fault> {
    let n = rec_len(l).ok_or(Fault::BadRef)?;
    let mut r = [0u8; REC];
    let rec = r.get_mut(..n).ok_or(rt(Corruption::Range, 0))?;
    let mut sp = Totals::read(h, tail)?;
    match at {
        At::None => return Err(Fault::BadRef),
        At::Hot(o) => crate::mem::copy_bytes(rec, crate::mem::sub(h.get(Region::Block(tail))?, o, n as u64)?),
        At::Space(o) => {
            let sp = sp.as_mut().ok_or(rt(Corruption::Link, tail))?;
            sp.read_bytes(h, sp.recs.wrapping_add(o), rec)?
        }
    }
    let count = rd_u64(rec, R_NONNULL)?;
    let small = i128::from_le_bytes(rd_array::<16>(rec, R_SUM)?);
    let t = match l.ty.running().ok_or(Fault::BadRef)? {
        RKind::Int => Total::Small(small),
        RKind::Wide => {
            let s = i192_at(rec, R_SUM)?;
            let mut b = [0u8; 24];
            put_i192(&mut b, 0, &s)?;
            bytes_total(&b, buf)?
        }
        _ => {
            let len = u64::from(rd_u32(rec, R_BIG_LEN)?);
            if len == 0 {
                return Ok((Total::Small(small), count));
            }
            let room = u64::from(rd_u32(rec, R_BIG_ROOM)?);
            let sp = sp.as_mut().ok_or(rt(Corruption::Link, tail))?;
            // The big-form sum, sign-extended, plus the i128: at most one byte longer than the longer.
            let need = len.max(16).checked_add(1).ok_or(rt(Corruption::Range, len))?;
            let k = usize::try_from(need).map_err(|_| rt(Corruption::Range, need))?;
            let out = buf.get_mut(..k).ok_or(Fault::Scratch { need })?;
            let lu = usize::try_from(len).map_err(|_| rt(Corruption::Range, len))?;
            let (big, rest) = out.split_at_mut_checked(lu).ok_or(rt(Corruption::Range, len))?;
            sp.read_bytes(h, room.wrapping_add(ROOM_HEAD), big)?;
            let ext = if big.last().is_some_and(|&x| x & 0x80 != 0) { 0xFF } else { 0 };
            rest.fill(ext);
            let x = small.to_le_bytes();
            let x_ext: u16 = if small < 0 { 0xFF } else { 0 };
            let mut carry = 0u16;
            for (i, b) in out.iter_mut().enumerate() {
                let y = x.get(i).map_or(x_ext, |&v| u16::from(v));
                let s = u16::from(*b).wrapping_add(y).wrapping_add(carry);
                *b = s.to_le_bytes()[0];
                carry = s >> 8;
            }
            let m = trimmed(out);
            match small_of(out.get(..m).unwrap_or(&[])) {
                Some(x) => Total::Small(x),
                None => Total::Big(m),
            }
        }
    };
    Ok((t, count))
}

/// The length of a two's-complement number without its redundant sign bytes (at least 1 byte).
pub(super) fn trimmed(b: &[u8]) -> usize {
    let mut n = b.len();
    while n > 1 {
        let hi = b.get(n.wrapping_sub(1)).copied().unwrap_or(0);
        let below = b.get(n.wrapping_sub(2)).copied().unwrap_or(0);
        if (hi == 0 && below & 0x80 == 0) || (hi == 0xFF && below & 0x80 != 0) {
            n = n.wrapping_sub(1);
        } else {
            break;
        }
    }
    n
}

/// A two's-complement number of at most 16 bytes as an i128.
pub(super) fn small_of(b: &[u8]) -> Option<i128> {
    if b.len() > 16 {
        return None;
    }
    let ext = if b.last().is_some_and(|&x| x & 0x80 != 0) { 0xFF } else { 0 };
    let mut a = [ext; 16];
    crate::mem::copy_bytes(a.get_mut(..b.len())?, b);
    Some(i128::from_le_bytes(a))
}

/// A two's-complement number as a `Total`: an i128 when it fits, else its bytes copied to `buf`.
pub(super) fn bytes_total(b: &[u8], buf: &mut [u8]) -> Result<Total, Fault> {
    let n = trimmed(b);
    let b = b.get(..n).unwrap_or(&[]);
    if let Some(x) = small_of(b) {
        return Ok(Total::Small(x));
    }
    let out = buf.get_mut(..n).ok_or(Fault::Scratch { need: n as u64 })?;
    crate::mem::copy_bytes(out, b);
    Ok(Total::Big(n))
}
