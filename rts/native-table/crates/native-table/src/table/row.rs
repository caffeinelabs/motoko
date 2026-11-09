//! Row images: {header 8 B: schema version u32, flags u16 (bit 0 the row has big integers, bit 1 it has long
//! items), zero u16; null bitmap ⌈R/8⌉ (1 = null); big bitmap ⌈R/8⌉ (1 = this Nat or Int is a variable item);
//! zero padding to 8; the fixed region, each fixed-width leaf at its offset in ordinal order (null cells
//! zero); the item table, a u32 offset from the image's start per variable leaf in ordinal order (0 for a
//! null value, and for a Nat or Int that is not big); the items, each a tag u8 (0 inline, 1 long) then
//! ULEB128 length ‖ bytes, or 7 zero bytes and a long value's image reference}.
//!
//! A write builds a row image in two passes over the host's cells: the first checks every cell and sizes the
//! image, the second writes it in order into the tail block's staging buffer or into a long row image.

use crate::bytes::uleb_len;
use crate::consts::{HUGE_VALUE, LONG_VALUE, PAGE, ROW_INLINE_MAX};
use crate::fault::{rt, Corruption, Fault, ImageId};
use crate::image::{Image, ImageReader};
use crate::mem::{copy_bytes, rd_u16, rd_u32, rd_u8, sub, PageHost, Region};

use super::buffer::RowLoc;
use super::cell::{self, Val};
use super::layout::{LeafInfo, RowHead, Shape, Ty};
use super::long::{self, LongOut, Meta};
use super::RowSource;

/// Header flags.
pub const ROW_BIG: u16 = 1;
pub const ROW_LONG: u16 = 2;
/// Item tags.
pub const ITEM_INLINE: u8 = 0;
pub const ITEM_LONG: u8 = 1;
/// Bytes of a long item: its tag, 7 zero bytes and an image reference.
pub const LONG_ITEM: u64 = 32;
/// The stack buffer an inline row image is built in.
pub const STAGE: usize = 3_072;
const _: () = assert!(STAGE as u64 == ROW_INLINE_MAX);

/// Whether a variable value of `n` bytes is a long value: past `HUGE_VALUE`, or past `LONG_VALUE` in a row
/// of a frozen group.
#[inline]
pub fn is_long(n: u64, frozen: bool) -> bool {
    n > HUGE_VALUE || (frozen && n > LONG_VALUE)
}

/// Bytes of an item of `n` value bytes.
#[inline]
pub fn item_bytes(n: u64, long: bool) -> u64 {
    if long {
        LONG_ITEM
    } else {
        // n is a slice length or a host length below 2^63.
        1u64.wrapping_add(uleb_len(n)).wrapping_add(n)
    }
}

pub fn has_item(l: &LeafInfo, v: &Val<'_>) -> bool {
    l.ty.variable() && !v.is_null() && (matches!(l.ty, Ty::Text | Ty::Blob) || cell::is_big(v))
}

/// What the first pass learns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// Image bytes.
    pub len: u64,
    pub big: bool,
    pub long: bool,
    /// Inline bytes: the image's bytes other than medium items and long items.
    pub inline: u64,
    /// Bytes of medium values (longer than `LONG_VALUE`, kept inline until the seal packs them).
    pub medium: u64,
    /// Medium and long values.
    pub large: u64,
}

pub fn cell_of<'a, R: RowSource + ?Sized>(src: &'a mut R, l: &LeafInfo) -> Result<Val<'a>, Fault> {
    cell::check(l, src.cell(l.id)?)
}

/// The first pass: checks every cell, encodes the key leaves into `key`, and sizes the image.
pub fn plan<H: PageHost + ?Sized, R: RowSource + ?Sized, K: crate::key::Sink>(
    shape: &Shape,
    h: &H,
    src: &mut R,
    frozen: bool,
    key: &mut K,
) -> Result<Plan, Fault> {
    let mut p = Plan::default();
    let mut items = 0u64;
    let mut out_of_inline = 0u64;
    for o in 0..shape.leaves {
        let l = shape.leaf(h, o)?;
        if !l.live() {
            continue;
        }
        let (big, item) = {
            let v = cell_of(src, &l)?;
            if l.key {
                cell::enc_key(key, &l, &v)?;
            }
            (cell::is_big(&v), has_item(&l, &v).then(|| cell::var_len(&v, l.ty)))
        };
        if big {
            p.big = true;
        }
        if let Some(n) = item {
            let long = is_long(n, frozen);
            let b = item_bytes(n, long);
            items = items.checked_add(b).ok_or(rt(Corruption::Range, b))?;
            if long {
                p.long = true;
                p.large = p.large.wrapping_add(1);
                out_of_inline = out_of_inline.wrapping_add(b);
            } else if n > LONG_VALUE {
                p.large = p.large.wrapping_add(1);
                p.medium = p.medium.wrapping_add(n);
                out_of_inline = out_of_inline.wrapping_add(b);
            }
        }
    }
    p.len = shape.items_at().checked_add(items).ok_or(rt(Corruption::Range, items))?;
    p.inline = p.len.saturating_sub(out_of_inline);
    Ok(p)
}

/// Where the second pass writes.
pub trait Out<H: PageHost> {
    fn put(&mut self, h: &mut H, b: &[u8]) -> Result<(), Fault>;
    fn fill<R: RowSource + ?Sized>(
        &mut self,
        h: &mut H,
        src: &mut R,
        leaf: u32,
        len: u64,
        text: bool,
    ) -> Result<(), Fault>;
}

/// An inline row image staged on the stack.
pub struct StackOut {
    pub buf: [u8; STAGE],
    pub len: usize,
    /// Bytes the host copied from the argument.
    pub filled: u64,
}

impl StackOut {
    pub fn new() -> Self {
        StackOut { buf: [0; STAGE], len: 0, filled: 0 }
    }

    pub fn bytes(&self) -> &[u8] {
        self.buf.get(..self.len).unwrap_or(&[])
    }

    fn room(&mut self, n: usize) -> Result<&mut [u8], Fault> {
        let end = self.len.checked_add(n).ok_or(rt(Corruption::Range, 0))?;
        let d = self.buf.get_mut(self.len..end).ok_or(rt(Corruption::Count, end as u64))?;
        self.len = end;
        Ok(d)
    }
}

impl Default for StackOut {
    fn default() -> Self {
        Self::new()
    }
}

impl<H: PageHost> Out<H> for StackOut {
    fn put(&mut self, _h: &mut H, b: &[u8]) -> Result<(), Fault> {
        copy_bytes(self.room(b.len())?, b);
        Ok(())
    }

    fn fill<R: RowSource + ?Sized>(
        &mut self,
        _h: &mut H,
        src: &mut R,
        leaf: u32,
        len: u64,
        text: bool,
    ) -> Result<(), Fault> {
        let n = usize::try_from(len).map_err(|_| rt(Corruption::Range, len))?;
        let d = self.room(n)?;
        src.fill(leaf, 0, d)?;
        if text && core::str::from_utf8(d).is_err() {
            return Err(Fault::BadValue { leaf });
        }
        self.filled = self.filled.saturating_add(len);
        Ok(())
    }
}

impl<H: PageHost> Out<H> for LongOut {
    fn put(&mut self, h: &mut H, b: &[u8]) -> Result<(), Fault> {
        LongOut::put(self, h, b)
    }

    fn fill<R: RowSource + ?Sized>(
        &mut self,
        h: &mut H,
        src: &mut R,
        leaf: u32,
        len: u64,
        text: bool,
    ) -> Result<(), Fault> {
        LongOut::fill(self, h, src, leaf, len, text)
    }
}

/// The second pass: writes the row image planned by `p` into `out`. Each item that is a long value is
/// created here, or, when `reuse` names the row's previous image and that image holds the same value as a
/// long item, its reference moves over. Returns the bytes the host copied from the argument into long values.
#[allow(clippy::too_many_arguments)]
pub fn write<H: PageHost, R: RowSource + ?Sized, O: Out<H>>(
    shape: &Shape,
    h: &mut H,
    tbl: u64,
    uid: u64,
    src: &mut R,
    p: &Plan,
    frozen: bool,
    out: &mut O,
    reuse: Option<RowLoc>,
) -> Result<u64, Fault> {
    let mut filled = 0u64;
    let mut head = [0u8; 8];
    crate::mem::put_u32(&mut head, 0, shape.version)?;
    let flags = (if p.big { ROW_BIG } else { 0 }) | (if p.long { ROW_LONG } else { 0 });
    crate::mem::put_u16(&mut head, 4, flags)?;
    out.put(h, &head)?;
    // The two bitmaps over the schema version's leaves, eight per byte.
    for big in [false, true] {
        let mut byte = 0u8;
        let mut bit = 0u64;
        for o in 0..shape.leaves {
            let l = shape.leaf(&*h, o)?;
            if !l.live() {
                continue;
            }
            let set = {
                let v = cell_of(src, &l)?;
                if big {
                    cell::is_big(&v)
                } else {
                    v.is_null()
                }
            };
            byte |= u8::from(set) << (bit % 8);
            bit = bit.wrapping_add(1);
            if bit % 8 == 0 {
                out.put(h, &[byte])?;
                byte = 0;
            }
        }
        if bit % 8 != 0 {
            out.put(h, &[byte])?;
        }
    }
    let pad = shape.fixed_at.wrapping_sub(8u64.wrapping_add(shape.bitmap.wrapping_mul(2)));
    out.put(h, long::ZEROS.get(..usize::try_from(pad).unwrap_or(0)).unwrap_or(&[]))?;
    // The fixed region.
    for o in 0..shape.leaves {
        let l = shape.leaf(&*h, o)?;
        let w = l.ty.fixed_width();
        if w == 0 || !l.live() {
            continue;
        }
        let v = cell_of(src, &l)?;
        if let (Ty::Fixed(_), Val::Bytes(b)) = (l.ty, v) {
            if w > 30 {
                out.put(h, b)?;
                continue;
            }
        }
        if w > 30 {
            // A null `[u8; N]` wider than a staging cell: zeros.
            let mut left = w;
            while left > 0 {
                let n = left.min(long::ZEROS.len() as u64);
                out.put(h, long::ZEROS.get(..usize::try_from(n).unwrap_or(0)).unwrap_or(&[]))?;
                left = left.wrapping_sub(n);
            }
            continue;
        }
        let mut c = [0u8; 30];
        let n = cell::fixed_cell(&v, l.ty, &mut c);
        out.put(h, c.get(..usize::try_from(n).unwrap_or(0)).unwrap_or(&[]))?;
    }
    // The item table.
    let mut at = shape.items_at();
    for o in 0..shape.leaves {
        let l = shape.leaf(&*h, o)?;
        if !l.ty.variable() || !l.live() {
            continue;
        }
        let item = {
            let v = cell_of(src, &l)?;
            has_item(&l, &v).then(|| cell::var_len(&v, l.ty))
        };
        let mut off = 0u32;
        if let Some(n) = item {
            off = u32::try_from(at).map_err(|_| rt(Corruption::Range, at))?;
            at = at.checked_add(item_bytes(n, is_long(n, frozen))).ok_or(rt(Corruption::Range, at))?;
        }
        out.put(h, &off.to_le_bytes())?;
    }
    if at != p.len {
        return Err(rt(Corruption::Count, at));
    }
    // The items.
    let mut vi = 0u64;
    for o in 0..shape.leaves {
        let l = shape.leaf(&*h, o)?;
        if !l.ty.variable() || !l.live() {
            continue;
        }
        let this = vi;
        vi = vi.wrapping_add(1);
        let (n, arg, text) = {
            let v = cell_of(src, &l)?;
            if !has_item(&l, &v) {
                continue;
            }
            (cell::var_len(&v, l.ty), matches!(v, Val::Arg(_)), matches!(l.ty, Ty::Text))
        };
        if is_long(n, frozen) {
            let old = match reuse {
                Some(loc) => reusable(shape, &*h, loc, &l, this, src)?,
                None => None,
            };
            let img = match old {
                Some(img) => img,
                None => {
                    let meta =
                        Meta { uid, schema_hash: shape.hash, content: long::CONTENT_VALUE, leaf: u32::from(l.ordinal) };
                    if arg {
                        let (img, f) = long::from_arg(h, tbl, &meta, src, l.id, n, text)?;
                        filled = filled.saturating_add(f);
                        img
                    } else {
                        let v = cell_of(src, &l)?;
                        let b = match v {
                            Val::Bytes(b) | Val::Big(b) => b,
                            _ => return Err(rt(Corruption::Encoding, o)),
                        };
                        long::from_bytes(h, tbl, &meta, b)?
                    }
                }
            };
            let mut item = [0u8; 32];
            crate::mem::put_u8(&mut item, 0, ITEM_LONG)?;
            img.put_ref(&mut item, 8)?;
            out.put(h, &item)?;
        } else {
            let mut hd = [0u8; 11];
            crate::mem::put_u8(&mut hd, 0, ITEM_INLINE)?;
            let k = crate::bytes::put_uleb(&mut hd, 1, n)?;
            out.put(h, hd.get(..usize::try_from(k.wrapping_add(1)).unwrap_or(0)).unwrap_or(&[]))?;
            if arg {
                out.fill(h, src, l.id, n, text)?;
            } else {
                let v = cell_of(src, &l)?;
                match v {
                    Val::Bytes(b) | Val::Big(b) => out.put(h, b)?,
                    Val::U(_) | Val::I(_) | Val::Wide(_) => {
                        let mut leb = [0u8; 19];
                        let k = cell::small_leb(&v, matches!(l.ty, Ty::Int), &mut leb);
                        out.put(h, leb.get(..k).unwrap_or(&[]))?;
                    }
                    _ => return Err(rt(Corruption::Encoding, o)),
                }
            }
        }
    }
    Ok(filled)
}

/// The reference of the long item of variable leaf `vi` in the image at `loc`, when it holds the same value as
/// the source's cell; `None` otherwise.
fn reusable<H: PageHost + ?Sized, R: RowSource + ?Sized>(
    shape: &Shape,
    h: &H,
    loc: RowLoc,
    l: &LeafInfo,
    vi: u64,
    src: &mut R,
) -> Result<Option<Image>, Fault> {
    let row = RowRef::new(h, loc)?;
    // Leaf l's item index in the old image's own layout.
    let v = row.version()?;
    let vi = if v == shape.version {
        vi
    } else {
        match shape.offsets_at(h, v, l)? {
            Some((_, ovi, _)) => ovi,
            None => return Ok(None),
        }
    };
    let Item::Long { img, len } = row.item(shape, vi)? else { return Ok(None) };
    let v = cell_of(src, l)?;
    let b = match v {
        Val::Bytes(b) | Val::Big(b) => b,
        _ => return Ok(None),
    };
    if b.len() as u64 != len {
        return Ok(None);
    }
    let mut piece = [0u8; 1_024];
    let mut at = 0u64;
    for chunk in b.chunks(piece.len()) {
        let dst = piece.get_mut(..chunk.len()).ok_or(rt(Corruption::Range, at))?;
        long::read(h, img, at, dst)?;
        if dst != chunk {
            return Ok(None);
        }
        at = at.wrapping_add(chunk.len() as u64);
    }
    Ok(Some(img))
}

/// An item of a stored row image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    None,
    /// `len` value bytes at row offset `at`.
    Inline {
        at: u64,
        len: u64,
    },
    /// A long value of `len` content bytes; its reference lies at row offset `at`.
    Long {
        img: Image,
        len: u64,
    },
}

/// A stored row image, read through checked accessors. An inline image's bytes are taken from its block
/// once, and its header's version once.
pub struct RowRef<'h, H: PageHost + ?Sized> {
    pub h: &'h H,
    pub loc: RowLoc,
    pub len: u64,
    inline: Option<&'h [u8]>,
    version: u32,
}

impl<'h, H: PageHost + ?Sized> RowRef<'h, H> {
    pub fn new(h: &'h H, loc: RowLoc) -> Result<Self, Fault> {
        let (len, inline) = match loc {
            RowLoc::Inline { block, off, len } => (len, Some(sub(h.get(Region::Block(block))?, off, len)?)),
            RowLoc::Long(img) => (long::content_len(h, img)?, None),
        };
        let mut r = RowRef { h, loc, len, inline, version: 0 };
        let mut b = [0u8; 4];
        r.read(0, &mut b)?;
        r.version = u32::from_le_bytes(b);
        Ok(r)
    }

    /// Copies row bytes [off, off + out.len()) into `out`.
    pub fn read(&self, off: u64, out: &mut [u8]) -> Result<(), Fault> {
        let end = off.checked_add(out.len() as u64).ok_or(rt(Corruption::Bounds, off))?;
        if end > self.len {
            return Err(rt(Corruption::Bounds, off));
        }
        match (self.inline, self.loc) {
            (Some(b), _) => {
                copy_bytes(out, sub(b, off, out.len() as u64)?);
                Ok(())
            }
            (None, RowLoc::Long(img)) => long::read(self.h, img, off, out),
            (None, RowLoc::Inline { block, off: base, .. }) => {
                copy_bytes(out, sub(self.h.get(Region::Block(block))?, base.wrapping_add(off), out.len() as u64)?);
                Ok(())
            }
        }
    }

    /// Row bytes [off, off + len) as one slice when they lie in one piece of table memory.
    #[inline(always)]
    pub fn piece(&self, off: u64, len: u64) -> Result<Option<&'h [u8]>, Fault> {
        let end = off.checked_add(len).ok_or(rt(Corruption::Bounds, off))?;
        if end > self.len {
            return Err(rt(Corruption::Bounds, off));
        }
        let h: &'h H = self.h;
        if let Some(b) = self.inline {
            return Ok(Some(sub(b, off, len)?));
        }
        match self.loc {
            RowLoc::Inline { block, off: base, .. } => {
                Ok(Some(sub(h.get(Region::Block(block))?, base.wrapping_add(off), len)?))
            }
            RowLoc::Long(img) => {
                let at = long::HEAD.wrapping_add(off);
                if (at % PAGE).wrapping_add(len) > PAGE {
                    return Ok(None);
                }
                Ok(Some(ImageReader::new(h, ImageId::Runtime, img).bytes(at, len)?))
            }
        }
    }

    fn byte(&self, off: u64) -> Result<u8, Fault> {
        if let Some(b) = self.inline {
            return rd_u8(b, off);
        }
        let mut b = [0u8; 1];
        self.read(off, &mut b)?;
        Ok(b[0])
    }

    /// Whether the value at bitmap bit `bit` (the leaf's index among the row's leaves) is null.
    pub fn null_bit(&self, bit: u64) -> Result<bool, Fault> {
        Ok(self.byte(8u64.wrapping_add(bit / 8))? >> (bit % 8) & 1 != 0)
    }

    /// Whether the value at bitmap bit `bit` is a big Nat or Int.
    pub fn big_bit(&self, head: &RowHead, bit: u64) -> Result<bool, Fault> {
        Ok(self.byte(8u64.wrapping_add(head.bitmap).wrapping_add(bit / 8))? >> (bit % 8) & 1 != 0)
    }

    pub fn version(&self) -> Result<u32, Fault> {
        Ok(self.version)
    }

    pub fn head(&self, shape: &Shape) -> Result<RowHead, Fault> {
        shape.head_for(self.h, self.version()?)
    }

    /// The schema version and flags of the header.
    pub fn header(&self) -> Result<(u32, u16), Fault> {
        let mut b = [0u8; 8];
        self.read(0, &mut b)?;
        Ok((rd_u32(&b, 0)?, rd_u16(&b, 4)?))
    }

    /// Variable leaf `vi`'s item (vi among the variable leaves of the row's version).
    pub fn item(&self, shape: &Shape, vi: u64) -> Result<Item, Fault> {
        let head = self.head(shape)?;
        self.item_at(&head, vi)
    }

    /// Variable leaf `vi`'s item, in a row of layout `head`.
    pub fn item_at(&self, head: &RowHead, vi: u64) -> Result<Item, Fault> {
        let mut b = [0u8; 4];
        self.read(head.items_table_at().wrapping_add(vi.wrapping_mul(4)), &mut b)?;
        let at = u64::from(u32::from_le_bytes(b));
        if at == 0 {
            return Ok(Item::None);
        }
        let mut hd = [0u8; 11];
        let n = (self.len.saturating_sub(at)).min(11);
        self.read(at, hd.get_mut(..usize::try_from(n).unwrap_or(0)).unwrap_or(&mut []))?;
        match rd_u8(&hd, 0)? {
            ITEM_INLINE => {
                let (len, k) = crate::bytes::rd_uleb(&hd, 1, ImageId::Runtime)?;
                let data = at.wrapping_add(1).wrapping_add(k);
                if data.checked_add(len).is_none_or(|e| e > self.len) {
                    return Err(rt(Corruption::Bounds, at));
                }
                Ok(Item::Inline { at: data, len })
            }
            ITEM_LONG => {
                let mut r = [0u8; 24];
                self.read(at.wrapping_add(8), &mut r)?;
                if r.iter().all(|&x| x == 0) {
                    // Its long value moved to its owner of now (a sealed group's list) or to the drain.
                    return Ok(Item::None);
                }
                let img = Image::read_ref(&r, 0)?;
                Ok(Item::Long { img, len: long::content_len(self.h, img)? })
            }
            _ => Err(rt(Corruption::Encoding, at)),
        }
    }

    /// The row offset of the reference of variable leaf `vi`'s long item in a row of layout `head`, when it has
    /// one.
    pub fn long_ref_in(&self, head: &RowHead, vi: u64) -> Result<Option<u64>, Fault> {
        let mut b = [0u8; 4];
        self.read(head.items_table_at().wrapping_add(vi.wrapping_mul(4)), &mut b)?;
        let at = u64::from(u32::from_le_bytes(b));
        if at == 0 || self.byte(at)? != ITEM_LONG {
            return Ok(None);
        }
        let mut r = [0u8; 24];
        self.read(at.wrapping_add(8), &mut r)?;
        if r.iter().all(|&x| x == 0) {
            return Ok(None);
        }
        Ok(Some(at.wrapping_add(8)))
    }
}

/// A stored value, located in its row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stored<'h> {
    Null,
    /// A value read from its fixed cell (Principal and `[u8; N]` excepted).
    Val(Val<'h>),
    /// Bytes of a Principal or `[u8; N]` at row offset `at`.
    Bytes {
        at: u64,
        len: u64,
    },
    /// A variable value: its item.
    Item(Item),
    Unit,
    /// A leaf the row's schema version lacks: it reads its default.
    Absent,
}

/// Walks a stored row's leaves in ordinal order, keeping each leaf's fixed offset, variable index and bitmap
/// bit in the layout of one schema version; a leaf that version lacks is absent and takes no place.
#[derive(Clone, Copy, Debug)]
pub struct Walk {
    pub fixed: u64,
    pub var: u64,
    pub bit: u64,
    pub head: RowHead,
}

impl Walk {
    /// A walk over one row, in its own schema version's layout.
    pub fn of<H: PageHost + ?Sized>(shape: &Shape, row: &RowRef<'_, H>) -> Result<Self, Fault> {
        let head = row.head(shape)?;
        Ok(Walk { fixed: head.fixed_at, var: 0, bit: 0, head })
    }

    /// Leaf `l`'s stored value, then moves past it.
    pub fn next<'h, H: PageHost + ?Sized>(
        &mut self,
        _shape: &Shape,
        row: &RowRef<'h, H>,
        l: &LeafInfo,
    ) -> Result<Stored<'h>, Fault> {
        if !l.at(self.head.version) {
            return Ok(Stored::Absent);
        }
        let (fixed, vi, bit) = (self.fixed, self.var, self.bit);
        self.skip(l);
        stored_at(&self.head, row, l, fixed, vi, bit)
    }

    /// Moves past leaf `l` without reading it.
    pub fn skip(&mut self, l: &LeafInfo) {
        if !l.at(self.head.version) {
            return;
        }
        self.fixed = self.fixed.wrapping_add(l.ty.fixed_width());
        if l.ty.variable() {
            self.var = self.var.wrapping_add(1);
        }
        self.bit = self.bit.wrapping_add(1);
    }
}

/// Leaf `l`'s stored value in a row of layout `head`.
pub fn stored_at<'h, H: PageHost + ?Sized>(
    head: &RowHead,
    row: &RowRef<'h, H>,
    l: &LeafInfo,
    fixed: u64,
    vi: u64,
    bit: u64,
) -> Result<Stored<'h>, Fault> {
    if row.null_bit(bit)? {
        return Ok(Stored::Null);
    }
    match l.ty {
        Ty::Unit => Ok(Stored::Unit),
        Ty::Text | Ty::Blob => Ok(Stored::Item(row.item_at(head, vi)?)),
        Ty::Nat | Ty::Int if row.big_bit(head, bit)? => Ok(Stored::Item(row.item_at(head, vi)?)),
        Ty::Principal => {
            let n = u64::from(row.byte(fixed.wrapping_add(29))?).min(29);
            Ok(Stored::Bytes { at: fixed, len: n })
        }
        Ty::Fixed(n) => Ok(Stored::Bytes { at: fixed, len: u64::from(n) }),
        t => {
            let w = t.fixed_width();
            let mut c = [0u8; 16];
            let cw = usize::try_from(w).unwrap_or(0).min(16);
            row.read(fixed, c.get_mut(..cw).unwrap_or(&mut []))?;
            Ok(Stored::Val(super::cur::own(cell::from_fixed(t, c.get(..cw).unwrap_or(&[])))))
        }
    }
}
