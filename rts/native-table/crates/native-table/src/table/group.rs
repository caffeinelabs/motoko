//! The row-group image (kind 1): its header, leaf directory and segment table, the packer that lays out the
//! cell area, and the reader of a sealed row's cells.
//!
//! An image is, in order: the 128-byte header; the front metadata (the leaf directory, L × 32 bytes from 128;
//! the rank map when compacted, 640 bytes), its end H rounded up to 8; the
//! cell area from H, per leaf in ordinal order its cell segments, then its `Seq` parameters, then its
//! out-of-line directory, none of which crosses an image block; the inline data region, per
//! variable leaf one run of its inline values in slot order; the out-of-line area, per leaf its medium values
//! and 8-byte huge stubs in slot order; the segment table at an offset ≡ 0 mod 32. Its length is the segment
//! table's end.
//!
//! Positions of segments and out-of-line directories count the image's stored slots: every position of the
//! group, or only the positions live at the seal when the image is compacted.

use crate::consts::{magic, FORMAT_IMAGE, PAGE};
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::image::{Image, ImageReader};
use crate::mem::{rd_u16, rd_u32, rd_u64, rd_u8, PageHost, Region};

use super::page;

// ---- header ----
pub const G_GROUP: u64 = 32;
pub const G_P: u64 = 40;
pub const G_N: u64 = 42;
pub const G_L: u64 = 44;
pub const G_FLAGS: u64 = 46;
pub const G_VERSION: u64 = 48;
pub const G_S: u64 = 52;
pub const G_X: u64 = 56;
pub const G_H: u64 = 60;
pub const G_RANK: u64 = 64;
pub const G_DIR: u64 = 68;
pub const G_OOL_AT: u64 = 88;
pub const G_OOL_BYTES: u64 = 96;
pub const G_V: u64 = 104;
pub const G_SEGS: u64 = 112;
pub const G_FIRST_HUGE: u64 = 120;
pub const HEADER: u64 = 128;
/// Image kind of a group.
pub const KIND_GROUP: u8 = 1;
/// Header flags.
pub const GF_COMPACTED: u16 = 1;
pub const GF_OOL: u16 = 2;
pub const GF_HUGE: u16 = 8;
/// Bytes of the rank map: the kept bitmap, then 64 u16 ranks.
pub const RANK_MAP: u64 = 640;

/// Form codes.
pub mod form {
    pub const BOOL: u8 = 1;
    pub const U8: u8 = 2;
    pub const U16: u8 = 3;
    pub const U32: u8 = 4;
    pub const U64: u8 = 5;
    pub const I8: u8 = 6;
    pub const I16: u8 = 7;
    pub const I32: u8 = 8;
    pub const I64: u8 = 9;
    pub const X16: u8 = 10;
    pub const F32: u8 = 11;
    pub const F64: u8 = 12;
    pub const FIXED: u8 = 13;
    pub const UTF8: u8 = 14;
    pub const BINARY: u8 = 15;
    pub const LEB: u8 = 16;
    pub const ENC: u8 = 19;
    pub const UNIT: u8 = 20;
    pub const SEQ: u8 = 24;
}

// ---- leaf directory entry ----
pub const LD_BYTES: u64 = 32;
/// Flags: validity present, out-of-line values.
pub const LF_VALIDITY: u8 = 1;
pub const LF_OOL: u8 = 2;

// ---- segment table entry ----
pub const SEG_BYTES: u64 = 32;
pub mod seg {
    pub const CELLS: u8 = 0;
    pub const SEQ: u8 = 2;
    pub const OOL: u8 = 3;
    pub const RUN: u8 = 4;
}

/// x rounded up to a multiple of 8.
#[inline]
pub const fn align8(x: u64) -> u64 {
    x.wrapping_add(7) & !7
}

/// Whether a form stores its values in the inline data region.
pub fn variable(f: u8) -> bool {
    matches!(f, form::UTF8 | form::BINARY | form::LEB | form::ENC)
}

/// Whether a form has cell segments (with `validity` for those whose cells are validity alone).
pub fn has_cells(f: u8, validity: bool) -> bool {
    match f {
        form::SEQ => false,
        form::UNIT => validity,
        _ => true,
    }
}

/// The cell bytes of a fixed-width form, else 0.
pub fn cell_width(f: u8, fixed_n: u64) -> u64 {
    match f {
        form::U8 | form::I8 => 1,
        form::U16 | form::I16 => 2,
        form::U32 | form::I32 | form::F32 => 4,
        form::U64 | form::I64 | form::F64 => 8,
        form::X16 => 16,
        form::FIXED => fixed_n,
        _ => 0,
    }
}

/// Bytes of a cell segment of k positions.
pub fn cell_bytes(f: u8, width: u64, validity: bool, k: u64) -> u64 {
    let words = k.div_ceil(64).wrapping_mul(8);
    let val = if validity { words } else { 0 };
    let body = match f {
        form::BOOL => words,
        form::UNIT => 0,
        f if variable(f) => align8(k.wrapping_add(1).wrapping_mul(4)),
        _ => align8(k.wrapping_mul(width)),
    };
    val.wrapping_add(body)
}

/// Bytes of an out-of-line directory over n slots with k values.
pub fn ool_dir_bytes(n: u64, k: u64) -> u64 {
    align8(
        n.div_ceil(64)
            .wrapping_mul(8)
            .wrapping_add(n.div_ceil(4_096).wrapping_mul(4))
            .wrapping_add(k.wrapping_add(1).wrapping_mul(4)),
    )
}

/// The offset a part of `bytes` bytes that must not cross an image block takes from `at`.
pub fn place(at: u64, bytes: u64) -> u64 {
    let o = at % PAGE;
    if o.wrapping_add(bytes) > PAGE && o != 0 {
        at.wrapping_add(PAGE.wrapping_sub(o))
    } else {
        at
    }
}

/// A leaf directory entry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LeafDir {
    pub id: u32,
    pub ordinal: u16,
    pub form: u8,
    pub flags: u8,
    pub positions: u32,
    pub width: u16,
    pub first_seg: u32,
    pub segs: u32,
    pub ool: u32,
}

impl LeafDir {
    pub fn read(b: &[u8]) -> Result<LeafDir, Fault> {
        Ok(LeafDir {
            id: rd_u32(b, 0)?,
            ordinal: rd_u16(b, 4)?,
            form: rd_u8(b, 6)?,
            flags: rd_u8(b, 7)?,
            positions: rd_u32(b, 8)?,
            width: rd_u16(b, 12)?,
            first_seg: rd_u32(b, 16)?,
            segs: rd_u32(b, 20)?,
            ool: rd_u32(b, 24)?,
        })
    }

    pub fn write(&self, b: &mut [u8]) -> Result<(), Fault> {
        use crate::mem::{fill, put_u16, put_u32, put_u8};
        fill(b, 0, LD_BYTES, 0)?;
        put_u32(b, 0, self.id)?;
        put_u16(b, 4, self.ordinal)?;
        put_u8(b, 6, self.form)?;
        put_u8(b, 7, self.flags)?;
        put_u32(b, 8, self.positions)?;
        put_u16(b, 12, self.width)?;
        put_u32(b, 16, self.first_seg)?;
        put_u32(b, 20, self.segs)?;
        put_u32(b, 24, self.ool)
    }

    /// Cell segments: the leaf's segments less its `Seq` part, directory and run.
    pub fn cell_segs(&self) -> u64 {
        let mut s = u64::from(self.segs);
        if self.form == form::SEQ {
            s = s.saturating_sub(1);
        }
        if self.flags & LF_OOL != 0 {
            s = s.saturating_sub(1);
        }
        if variable(self.form) {
            s = s.saturating_sub(1);
        }
        s
    }
}

/// A segment table entry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Seg {
    pub off: u64,
    pub first: u32,
    pub positions: u32,
    pub bytes: u32,
    pub kind: u8,
}

impl Seg {
    pub fn read(b: &[u8]) -> Result<Seg, Fault> {
        Ok(Seg {
            off: rd_u64(b, 0)?,
            first: rd_u32(b, 8)?,
            positions: rd_u32(b, 12)?,
            bytes: rd_u32(b, 16)?,
            kind: rd_u8(b, 20)?,
        })
    }

    pub fn bytes32(&self) -> Result<[u8; 32], Fault> {
        use crate::mem::{put_u32, put_u64, put_u8};
        let mut b = [0u8; 32];
        put_u64(&mut b, 0, self.off)?;
        put_u32(&mut b, 8, self.first)?;
        put_u32(&mut b, 12, self.positions)?;
        put_u32(&mut b, 16, self.bytes)?;
        put_u8(&mut b, 20, self.kind)?;
        Ok(b)
    }
}

/// A value read from a sealed group's image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GCell {
    Null,
    Bool(bool),
    U(u64),
    I(i64),
    X16([u8; 16]),
    F32(u32),
    F64(u64),
    Unit,
    /// `len` bytes at image offset `off`: a fixed cell, an inline value or a medium value.
    Bytes {
        off: u64,
        len: u64,
    },
    /// A huge value: entry `index` of the group's long-value list.
    Huge {
        img: Image,
        len: u64,
        index: u64,
    },
}

/// What a `GroupView` reads from a group page and its image's header, kept by callers that reopen the view
/// across host borrows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupMeta {
    pub g: u64,
    pub page: u64,
    pub img: Image,
    pub p: u64,
    pub n: u64,
    pub l: u64,
    pub flags: u16,
    pub s: u64,
    pub rank_at: u64,
    pub ool_at: u64,
    pub segs_at: u64,
    pub version: u32,
}

/// A sealed group, read through its group page and the checked image reader.
pub struct GroupView<'h, H: PageHost + ?Sized> {
    pub h: &'h H,
    pub g: u64,
    pub page: u64,
    pub img: Image,
    pub p: u64,
    pub n: u64,
    pub l: u64,
    pub flags: u16,
    pub s: u64,
    pub rank_at: u64,
    pub ool_at: u64,
    pub segs_at: u64,
    pub version: u32,
}

impl<'h, H: PageHost + ?Sized> GroupView<'h, H> {
    pub fn open(h: &'h H, page: u64) -> Result<Self, Fault> {
        let pb = h.get(Region::Block(page))?;
        if rd_u32(pb, 0)? != magic::NTGP {
            return Err(corrupt(ImageId::Runtime, Corruption::Header, page));
        }
        let g = rd_u64(pb, page::P_GROUP)?;
        let img = Image::read_record(pb, page::P_IMAGE)?;
        let r = ImageReader::new(h, ImageId::Group(g), img);
        let hd = r.bytes(0, HEADER)?;
        let id = ImageId::Group(g);
        if rd_u32(hd, 0)? != magic::NTIM || rd_u16(hd, 4)? > FORMAT_IMAGE || rd_u8(hd, 6)? != KIND_GROUP {
            return Err(corrupt(id, Corruption::Header, 0));
        }
        Ok(GroupView {
            h,
            g,
            page,
            img,
            p: u64::from(rd_u16(hd, G_P)?),
            n: u64::from(rd_u16(hd, G_N)?),
            l: u64::from(rd_u16(hd, G_L)?),
            flags: rd_u16(hd, G_FLAGS)?,
            s: u64::from(rd_u32(hd, G_S)?),
            rank_at: u64::from(rd_u32(hd, G_RANK)?),
            ool_at: rd_u64(hd, G_OOL_AT)?,
            segs_at: rd_u64(hd, G_SEGS)?,
            version: rd_u32(hd, G_VERSION)?,
        })
    }

    pub fn meta(&self) -> GroupMeta {
        GroupMeta {
            g: self.g,
            page: self.page,
            img: self.img,
            p: self.p,
            n: self.n,
            l: self.l,
            flags: self.flags,
            s: self.s,
            rank_at: self.rank_at,
            ool_at: self.ool_at,
            segs_at: self.segs_at,
            version: self.version,
        }
    }

    pub fn from_meta(h: &'h H, m: &GroupMeta) -> Self {
        GroupView {
            h,
            g: m.g,
            page: m.page,
            img: m.img,
            p: m.p,
            n: m.n,
            l: m.l,
            flags: m.flags,
            s: m.s,
            rank_at: m.rank_at,
            ool_at: m.ool_at,
            segs_at: m.segs_at,
            version: m.version,
        }
    }

    pub fn reader(&self) -> ImageReader<'h, H> {
        ImageReader::new(self.h, ImageId::Group(self.g), self.img)
    }

    fn bad(&self, kind: Corruption, at: u64) -> Fault {
        corrupt(ImageId::Group(self.g), kind, at)
    }

    /// The stored slot of position p: p itself, or through the rank map of a compacted image; `None` for a
    /// position the image does not store.
    pub fn slot(&self, p: u64) -> Result<Option<u64>, Fault> {
        if p >= self.p {
            return Ok(None);
        }
        if self.flags & GF_COMPACTED == 0 {
            return Ok(Some(p));
        }
        let r = self.reader();
        let word = r.u64_at(self.rank_at.wrapping_add((p / 64).wrapping_mul(8)))?;
        if word >> (p % 64) & 1 == 0 {
            return Ok(None);
        }
        let rank =
            u64::from(rd_u16(r.bytes(self.rank_at.wrapping_add(512).wrapping_add((p / 64).wrapping_mul(2)), 2)?, 0)?);
        let below = (word & ((1u64 << (p % 64)).wrapping_sub(1))).count_ones();
        Ok(Some(rank.wrapping_add(u64::from(below))))
    }

    /// The leaf directory entry of leaf `o`: the image holds every leaf the table had when it was packed, in
    /// ordinal order. A leaf added later lies past the directory's end and gets an entry without cells, whose
    /// every value reads as null (an added leaf is always nullable and never of type `()`).
    pub fn leaf(&self, o: u64) -> Result<LeafDir, Fault> {
        if o >= self.l {
            return self.added(o);
        }
        LeafDir::read(self.reader().bytes(HEADER.wrapping_add(o.wrapping_mul(LD_BYTES)), LD_BYTES)?)
    }

    /// The entry of leaf `o`, added after the image was packed: no cells. Kept out of line, so that the
    /// lookups of the image's own leaves compile as they would without it.
    #[cold]
    #[inline(never)]
    fn added(&self, o: u64) -> Result<LeafDir, Fault> {
        let ordinal = u16::try_from(o).map_err(|_| self.bad(Corruption::Leaf, o))?;
        Ok(LeafDir { ordinal, form: form::UNIT, ..LeafDir::default() })
    }

    pub fn seg(&self, i: u64) -> Result<Seg, Fault> {
        if i >= self.s {
            return Err(self.bad(Corruption::Range, i));
        }
        Seg::read(self.reader().bytes(self.segs_at.wrapping_add(i.wrapping_mul(SEG_BYTES)), SEG_BYTES)?)
    }

    /// The leaf's segment of `kind` after its cells (`Seq` part, directory or run).
    fn tail_seg(&self, ld: &LeafDir, kind: u8) -> Result<Seg, Fault> {
        let first = u64::from(ld.first_seg).wrapping_add(ld.cell_segs());
        for i in first..u64::from(ld.first_seg).wrapping_add(u64::from(ld.segs)) {
            let s = self.seg(i)?;
            if s.kind == kind {
                return Ok(s);
            }
        }
        Err(self.bad(Corruption::Link, u64::from(ld.ordinal)))
    }

    /// The cell segment holding slot s, and s's index in it.
    fn cell_seg(&self, ld: &LeafDir, s: u64) -> Result<(Seg, u64), Fault> {
        let (mut lo, mut hi) = (0u64, ld.cell_segs());
        if hi == 0 {
            return Err(self.bad(Corruption::Link, u64::from(ld.ordinal)));
        }
        while hi.wrapping_sub(lo) > 1 {
            let mid = lo.wrapping_add(hi.wrapping_sub(lo) / 2);
            if u64::from(self.seg(u64::from(ld.first_seg).wrapping_add(mid))?.first) <= s {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let sg = self.seg(u64::from(ld.first_seg).wrapping_add(lo))?;
        let i = s.checked_sub(u64::from(sg.first)).ok_or(self.bad(Corruption::Range, s))?;
        if i >= u64::from(sg.positions) {
            return Err(self.bad(Corruption::Range, s));
        }
        Ok((sg, i))
    }

    /// The value of leaf `ld` at stored slot s, position p.
    pub fn cell(&self, ld: &LeafDir, s: u64, p: u64) -> Result<GCell, Fault> {
        let r = self.reader();
        if ld.form == form::SEQ {
            let part = self.tail_seg(ld, seg::SEQ)?;
            let base = r.u64_at(part.off)?;
            let step = r.u64_at(part.off.wrapping_add(8))?;
            // base + step · p in the leaf's 64-bit range, as the seal checked.
            return Ok(GCell::U(base.wrapping_add(step.wrapping_mul(p))));
        }
        let validity = ld.flags & LF_VALIDITY != 0;
        if !has_cells(ld.form, validity) {
            return Ok(GCell::Unit);
        }
        let (sg, i) = self.cell_seg(ld, s)?;
        let words = u64::from(sg.positions).div_ceil(64).wrapping_mul(8);
        let mut body = sg.off;
        if validity {
            let w = r.u64_at(sg.off.wrapping_add((i / 64).wrapping_mul(8)))?;
            if w >> (i % 64) & 1 == 0 {
                return Ok(GCell::Null);
            }
            body = body.wrapping_add(words);
        }
        let width = u64::from(ld.width);
        let at = body.wrapping_add(i.wrapping_mul(width));
        let le = |len: u64| -> Result<u64, Fault> {
            let b = r.bytes(at, len)?;
            let mut a = [0u8; 8];
            crate::mem::copy_bytes(&mut a, b);
            Ok(u64::from_le_bytes(a))
        };
        Ok(match ld.form {
            form::UNIT => GCell::Unit,
            form::BOOL => GCell::Bool(r.u64_at(body.wrapping_add((i / 64).wrapping_mul(8)))? >> (i % 64) & 1 != 0),
            form::U8 | form::U16 | form::U32 | form::U64 => GCell::U(le(width)?),
            form::I8 | form::I16 | form::I32 | form::I64 => {
                let raw = le(width)?;
                let bits = width.wrapping_mul(8);
                let shift = 64u64.saturating_sub(bits);
                // shift < 64.
                #[allow(clippy::cast_possible_truncation)]
                let x = ((raw << shift as u32) as i64) >> shift as u32;
                GCell::I(x)
            }
            form::F32 => GCell::F32(u32::try_from(le(4)?).unwrap_or(0)),
            form::F64 => GCell::F64(le(8)?),
            form::X16 => {
                let mut a = [0u8; 16];
                crate::mem::copy_bytes(&mut a, r.bytes(at, 16)?);
                GCell::X16(a)
            }
            form::FIXED => GCell::Bytes { off: at, len: width },
            f if variable(f) => {
                if ld.flags & LF_OOL != 0 {
                    if let Some(c) = self.ool_cell(ld, s)? {
                        return Ok(c);
                    }
                }
                let o = body.wrapping_add(i.wrapping_mul(4));
                let a = u64::from(rd_u32(r.bytes(o, 4)?, 0)?);
                let b = u64::from(rd_u32(r.bytes(o.wrapping_add(4), 4)?, 0)?);
                let run = self.tail_seg(ld, seg::RUN)?;
                let len = b.checked_sub(a).ok_or(self.bad(Corruption::Order, o))?;
                if b > u64::from(run.bytes) {
                    return Err(self.bad(Corruption::Bounds, o));
                }
                GCell::Bytes { off: run.off.wrapping_add(a), len }
            }
            _ => return Err(self.bad(Corruption::Leaf, u64::from(ld.ordinal))),
        })
    }

    /// The out-of-line value at slot s, when the leaf's directory marks it.
    fn ool_cell(&self, ld: &LeafDir, s: u64) -> Result<Option<GCell>, Fault> {
        let Some((off, len)) = self.ool_entry(ld, s)? else { return Ok(None) };
        if len == 8 {
            let i = self.huge_at(off)?;
            let img = page::lv_get(self.h, self.page, i)?.ok_or(self.bad(Corruption::Link, i))?;
            let len = super::long::content_len(self.h, img)?;
            return Ok(Some(GCell::Huge { img, len, index: i }));
        }
        Ok(Some(GCell::Bytes { off, len }))
    }

    /// A huge value's index at image offset `off`: out-of-line values are packed by bytes, so it may cross an
    /// image block.
    fn huge_at(&self, off: u64) -> Result<u64, Fault> {
        let mut ib = [0u8; 8];
        super::long::read_image(&self.reader(), off, &mut ib)?;
        Ok(u64::from_le_bytes(ib))
    }

    /// The image offset and length of slot s's out-of-line entry, when the leaf's directory marks it.
    fn ool_entry(&self, ld: &LeafDir, s: u64) -> Result<Option<(u64, u64)>, Fault> {
        let r = self.reader();
        let dir = self.tail_seg(ld, seg::OOL)?;
        let n = u64::from(dir.positions);
        let word = r.u64_at(dir.off.wrapping_add((s / 64).wrapping_mul(8)))?;
        if word >> (s % 64) & 1 == 0 {
            return Ok(None);
        }
        let rank_at = dir.off.wrapping_add(n.div_ceil(64).wrapping_mul(8));
        let mut idx = u64::from(rd_u32(r.bytes(rank_at.wrapping_add((s / 4_096).wrapping_mul(4)), 4)?, 0)?);
        let from = (s / 4_096).wrapping_mul(64);
        // The words before s's: read in one piece when they lie in one image block, else a word at a time.
        let start = dir.off.wrapping_add(from.wrapping_mul(8));
        let len = (s / 64).wrapping_sub(from).wrapping_mul(8);
        if (start % PAGE).saturating_add(len) <= PAGE {
            for w in r.bytes(start, len)?.chunks_exact(8) {
                idx = idx
                    .wrapping_add(u64::from(w.first_chunk::<8>().map_or(0, |c| u64::from_le_bytes(*c)).count_ones()));
            }
        } else {
            for w in from..s / 64 {
                idx = idx.wrapping_add(u64::from(r.u64_at(dir.off.wrapping_add(w.wrapping_mul(8)))?.count_ones()));
            }
        }
        idx = idx.wrapping_add(u64::from((word & ((1u64 << (s % 64)).wrapping_sub(1))).count_ones()));
        let offs = rank_at.wrapping_add(n.div_ceil(4_096).wrapping_mul(4));
        let a = u64::from(rd_u32(r.bytes(offs.wrapping_add(idx.wrapping_mul(4)), 4)?, 0)?);
        let b = u64::from(rd_u32(r.bytes(offs.wrapping_add(idx.wrapping_add(1).wrapping_mul(4)), 4)?, 0)?);
        let len = b.checked_sub(a).ok_or(self.bad(Corruption::Order, offs))?;
        Ok(Some((self.ool_at.wrapping_add(a), len)))
    }
}
