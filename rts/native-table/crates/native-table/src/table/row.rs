//! Row images: {header 8 B: schema version u32, flags u16 (bit 0 the row has big integers, bit 1 it has long
//! items), zero u16; null bitmap ⌈R/8⌉ (1 = null); big bitmap ⌈R/8⌉ (1 = this Nat or Int is a variable item);
//! zero padding to 8; the fixed region, each fixed-width leaf at its offset in ordinal order (null cells
//! zero); the item table, a u32 offset from the image's start per variable leaf in ordinal order (0 for a
//! null value, and for a Nat or Int that is not big); the items, each a tag u8 (0 inline, 1 long) then
//! ULEB128 length ‖ bytes, or 7 zero bytes and a long value's image reference}.
//!
//! A write builds a row image in two passes over the host's cells: the first checks every cell and sizes the
//! image (`plan`), the second writes it into a buffer of its size (`write::build`), which then goes into a buffer
//! block or into a long row image.

use crate::bytes::uleb_len;
use crate::consts::{HUGE_VALUE, LONG_VALUE, PAGE, ROW_INLINE_MAX};
use crate::fault::{rt, Corruption, Fault, ImageId};
use crate::image::{Image, ImageReader};
use crate::mem::{copy_bytes, rd_u8, sub, PageHost, Region};

use super::buffer::RowLoc;
use super::cell::{self, Val};
use super::layout::{LeafInfo, RowHead, Shape, Span, Ty};
use super::long::{self, Meta};
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
/// The smaller stack buffers a shorter inline row image is built in: a stage is zeroed at every write.
const SHORT_STAGE: usize = 256;
const SMALL_STAGE: usize = 512;
const MID_STAGE: usize = 1_024;

/// Whether a variable value of `n` bytes is a long value: past `HUGE_VALUE`.
#[inline]
pub fn is_long(n: u64) -> bool {
    n > HUGE_VALUE
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
    /// Inline bytes: the image's bytes other than medium items and long items.
    pub inline: u64,
    /// Bytes of medium values (longer than `LONG_VALUE`, kept inline until the seal packs them).
    pub medium: u64,
    /// Medium and long values.
    pub large: u64,
}

#[inline(always)]
pub fn cell_of<'a, R: RowSource + ?Sized>(src: &'a mut R, l: &LeafInfo) -> Result<Val<'a>, Fault> {
    cell::check::<true>(l, src.cell(l.id)?)
}

/// `cell_of` for a cell this write has checked already: a Text is not checked as UTF-8 again.
pub fn checked_cell<'a, R: RowSource + ?Sized>(src: &'a mut R, l: &LeafInfo) -> Result<Val<'a>, Fault> {
    cell::check::<false>(l, src.cell(l.id)?)
}

/// The first pass: checks every cell (a Text's bytes as UTF-8 only when `UTF8`), encodes the key leaves into
/// `key` when given, and sizes the image. Kept out of line: inlined into an upsert, it makes the upsert dearer.
#[inline(never)]
pub fn plan<const UTF8: bool, H: PageHost + ?Sized, R: RowSource + ?Sized, K: crate::key::Sink>(
    shape: &Shape,
    h: &H,
    src: &mut R,
    mut key: Option<&mut K>,
) -> Result<Plan, Fault> {
    let mut p = Plan::default();
    let mut items = 0u64;
    let mut out_of_inline = 0u64;
    for o in 0..shape.leaves {
        let l = shape.leaf(h, o)?;
        let item = {
            let v = if UTF8 { cell_of(src, &l)? } else { checked_cell(src, &l)? };
            if let (true, Some(key)) = (l.key, key.as_deref_mut()) {
                cell::enc_key(key, &l, &v)?;
            }
            has_item(&l, &v).then(|| cell::var_len(&v, l.ty))
        };
        if let Some(n) = item {
            let long = is_long(n);
            let b = item_bytes(n, long);
            items = items.checked_add(b).ok_or(rt(Corruption::Range, b))?;
            if long {
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

/// Runs `f` with `len` bytes for a row image: stack bytes when an inline row image of `len` fits them (the
/// smallest of the stages that holds it), else the first `len` bytes of `big`.
pub fn staged<T>(len: u64, big: &mut [u8], f: impl FnOnce(&mut [u8]) -> Result<T, Fault>) -> Result<T, Fault> {
    let n = usize::try_from(len).map_err(|_| rt(Corruption::Range, len))?;
    let (mut short, mut small, mut mid, mut whole);
    let buf: &mut [u8] = if n <= SHORT_STAGE {
        short = [0u8; SHORT_STAGE];
        &mut short
    } else if n <= SMALL_STAGE {
        small = [0u8; SMALL_STAGE];
        &mut small
    } else if n <= MID_STAGE {
        mid = [0u8; MID_STAGE];
        &mut mid
    } else if n <= STAGE {
        whole = [0u8; STAGE];
        &mut whole
    } else {
        big
    };
    f(buf.get_mut(..n).ok_or(Fault::Scratch { need: len })?)
}

/// A new long value holding leaf `l`'s cell of `n` bytes; returns it and the bytes the host copied from the
/// argument.
#[allow(clippy::too_many_arguments)]
pub fn long_value<H: PageHost, R: RowSource + ?Sized>(
    shape: &Shape,
    h: &mut H,
    tbl: u64,
    uid: u64,
    src: &mut R,
    l: &LeafInfo,
    n: u64,
) -> Result<(Image, u64), Fault> {
    let meta = Meta { uid, schema_hash: shape.hash, content: long::CONTENT_VALUE, leaf: u32::from(l.ordinal) };
    let b = match checked_cell(src, l)? {
        Val::Bytes(b) | Val::Big(b) => b,
        Val::Arg(_) => return long::from_arg(h, tbl, &meta, src, l.id, n, matches!(l.ty, Ty::Text)),
        _ => return Err(rt(Corruption::Encoding, u64::from(l.ordinal))),
    };
    Ok((long::from_bytes(h, tbl, &meta, b)?, 0))
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
                Ok(Some(ImageReader::new(h, ImageId::Runtime, &img).bytes(at, len)?))
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

    /// The header flags (`ROW_BIG`, `ROW_LONG`).
    pub fn flags(&self) -> Result<u16, Fault> {
        let mut b = [0u8; 2];
        self.read(4, &mut b)?;
        Ok(u16::from_le_bytes(b))
    }

    pub fn version(&self) -> Result<u32, Fault> {
        Ok(self.version)
    }

    pub fn head(&self, shape: &Shape) -> Result<RowHead, Fault> {
        shape.head_for(self.h, self.version()?)
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
                let img = Image::read_ref(&r, 0)?;
                Ok(Item::Long { img, len: long::content_len(self.h, img)? })
            }
            _ => Err(rt(Corruption::Encoding, at)),
        }
    }

    /// Variable leaf `vi`'s item in a row of layout `head`: its offset, its bytes, and whether it is a long item;
    /// (0, 0, false) when it has none.
    pub fn item_span(&self, head: &RowHead, vi: u64) -> Result<(u64, u64, bool), Fault> {
        let mut b = [0u8; 4];
        self.read(head.items_table_at().wrapping_add(vi.wrapping_mul(4)), &mut b)?;
        let at = u64::from(u32::from_le_bytes(b));
        if at == 0 {
            return Ok((0, 0, false));
        }
        if self.byte(at)? == ITEM_LONG {
            return Ok((at, LONG_ITEM, true));
        }
        let mut hd = [0u8; 10];
        let n = self.len.saturating_sub(at.wrapping_add(1)).min(10);
        self.read(at.wrapping_add(1), hd.get_mut(..usize::try_from(n).unwrap_or(0)).unwrap_or(&mut []))?;
        let (len, k) = crate::bytes::rd_uleb(&hd, 0, ImageId::Runtime)?;
        Ok((at, len.checked_add(k.wrapping_add(1)).ok_or(rt(Corruption::Range, at))?, false))
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
}

/// Walks a stored row's leaves in ordinal order, keeping each leaf's fixed offset, variable index and bitmap
/// bit in the layout of one schema version; a leaf that version lacks takes no place, and reading it is
/// `Corrupt`.
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
        let (fixed, vi, bit) = (self.fixed, self.var, self.bit);
        self.skip(l);
        stored_at(&self.head, row, l, fixed, vi, bit)
    }

    /// Moves past leaf `l` without reading it.
    pub fn skip(&mut self, l: &LeafInfo) {
        self.fixed = self.fixed.wrapping_add(l.ty.fixed_width());
        if l.ty.variable() {
            self.var = self.var.wrapping_add(1);
        }
        self.bit = self.bit.wrapping_add(1);
    }

    /// Moves past a leaf of span `s`.
    pub fn pass(&mut self, s: Span) {
        self.fixed = self.fixed.wrapping_add(s.width);
        self.var = self.var.wrapping_add(u64::from(s.variable));
        self.bit = self.bit.wrapping_add(1);
    }

    /// Moves to leaf `o`, past the leaves before it, read in place (`Shape::spans`).
    pub fn skip_to<H: PageHost + ?Sized>(&mut self, shape: &Shape, h: &H, o: u64) -> Result<(), Fault> {
        shape.spans(h, self.bit, o, |_, s| {
            self.pass(s);
            Ok(())
        })
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
