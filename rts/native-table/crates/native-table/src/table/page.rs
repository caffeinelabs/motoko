//! The group page (area 0): a sealed group's one mutable block. {magic `NTGP` u32 @0; `FORMAT_TABLE` u16 @4;
//! flags u16 @6 (bit 0 compacted image; bit 1 compaction queued; bit 2 patches present; bit 3 huge values
//! present; bit 4 believed; bit 5 quarantined); group number u64 @8; group image record 80 B @16; positions
//! p u16 @96; live rows u16 @98; dead positions still stored u16 @100; zero u16 @102; schema version u32
//! @104; long-value list entries u32 @108; patch directory u64 @112; long-value list u64 @120; live bitmap
//! 512 B @128; post-snapshot patch directory u64 @640; rewrite state block u64 @648; extremes block u64 @656
//! (0: none); bytes 664–4,095 unused}.
//!
//! **Long-value list** (area 0, zero-filled when allocated): {magic `NTLL` u32 @0; `FORMAT_TABLE` u16 @4; zero
//! u16 @6; next block u64 @8; first list index u32 @16; entries in this block u32 @20; zero u64 @24; 168
//! image references from @32}.

use crate::consts::{magic, BLOCK, FORMAT_TABLE};
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::image::{Image, REF_BYTES};
use crate::mem::{put_u16, put_u32, put_u64, rd_u32, rd_u64, rd_u8, Owner, PageHost, Region};

pub const P_FLAGS: u64 = 6;
pub const P_GROUP: u64 = 8;
pub const P_IMAGE: u64 = 16;
pub const P_POSITIONS: u64 = 96;
pub const P_LIVE: u64 = 98;
pub const P_DEAD: u64 = 100;
pub const P_VERSION: u64 = 104;
pub const P_LV_COUNT: u64 = 108;
pub const P_PATCH_DIR: u64 = 112;
pub const P_LV_LIST: u64 = 120;
pub const P_BITMAP: u64 = 128;
pub const P_POST_PATCH: u64 = 640;
pub const P_REWRITE: u64 = 648;
/// The group's extremes block (`extremes`), 0 when it has none.
pub const P_EXT: u64 = 656;
/// The end of the page's fields before its unused bytes.
pub const P_FIELDS: u64 = 664;

pub const PF_COMPACTED: u16 = 1;
pub const PF_QUEUED: u16 = 2;
pub const PF_PATCHES: u16 = 4;
pub const PF_HUGE: u16 = 8;

/// Long-value list blocks.
pub const LV_NEXT: u64 = 8;
pub const LV_FIRST: u64 = 16;
pub const LV_ENTRIES: u64 = 20;
pub const LV_REFS: u64 = 32;
pub const LV_PER_BLOCK: u64 = 168;

const _: () = assert!(LV_REFS + LV_PER_BLOCK * REF_BYTES <= BLOCK);

fn rt(kind: Corruption, at: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, at)
}

/// Whether position p is live on group page `page`.
#[inline]
pub fn live<H: PageHost + ?Sized>(h: &H, page: u64, p: u64) -> Result<bool, Fault> {
    live_bit(h.get(Region::Block(page))?, p)
}

/// Whether position p is live in `b`, a group page's first block.
#[inline]
pub fn live_bit(b: &[u8], p: u64) -> Result<bool, Fault> {
    Ok(rd_u8(b, P_BITMAP.wrapping_add(p / 8))? >> (p % 8) & 1 != 0)
}

/// Clears position p's live bit.
pub fn clear_live<H: PageHost>(h: &mut H, page: u64, p: u64) -> Result<(), Fault> {
    let b = h.get_mut(Region::Block(page), Owner(0))?;
    let at = P_BITMAP.wrapping_add(p / 8);
    let v = rd_u8(b, at)?;
    crate::mem::put_u8(b, at, v & !(1u8 << (p % 8)))
}

/// The block of the long-value list holding entry i, and the entry's offset in it.
pub fn lv_entry<H: PageHost + ?Sized>(h: &H, page: u64, i: u64) -> Result<(u64, u64), Fault> {
    let pb = h.get(Region::Block(page))?;
    let count = u64::from(rd_u32(pb, P_LV_COUNT)?);
    let g = rd_u64(pb, P_GROUP)?;
    let link = corrupt(ImageId::Group(g), Corruption::Link, i);
    if i >= count {
        return Err(link);
    }
    let mut b = rd_u64(pb, P_LV_LIST)?;
    loop {
        if b == 0 {
            return Err(link);
        }
        let s = h.get(Region::Block(b))?;
        let first = u64::from(rd_u32(s, LV_FIRST)?);
        let n = u64::from(rd_u32(s, LV_ENTRIES)?);
        if i >= first && i < first.saturating_add(n).min(first.saturating_add(LV_PER_BLOCK)) {
            return Ok((b, LV_REFS.wrapping_add(i.wrapping_sub(first).wrapping_mul(REF_BYTES))));
        }
        b = rd_u64(s, LV_NEXT)?;
    }
}

/// The long value entry i names, or `None` when it was freed (zero).
pub fn lv_get<H: PageHost + ?Sized>(h: &H, page: u64, i: u64) -> Result<Option<Image>, Fault> {
    let (b, off) = lv_entry(h, page, i)?;
    let s = h.get(Region::Block(b))?;
    if crate::mem::sub(s, off, REF_BYTES)?.iter().all(|&x| x == 0) {
        return Ok(None);
    }
    Image::read_ref(s, off).map(Some)
}

/// A long-value list being built by a seal: its blocks in order, each zero-filled when allocated.
pub struct LvBuilder {
    pub head: u64,
    tail: u64,
    pub count: u64,
}

impl LvBuilder {
    pub const fn new() -> Self {
        LvBuilder { head: 0, tail: 0, count: 0 }
    }

    /// A list being built, from its saved head, last block and count.
    pub const fn resume(head: u64, tail: u64, count: u64) -> Self {
        LvBuilder { head, tail, count }
    }

    /// The list's last block.
    pub fn tail(&self) -> u64 {
        self.tail
    }

    /// Appends one image reference; returns its index.
    pub fn push<H: PageHost>(&mut self, h: &mut H, tbl: u64, img: &Image) -> Result<u64, Fault> {
        let i = self.count;
        if i % LV_PER_BLOCK == 0 {
            let b = crate::area::alloc_zeroed(h, tbl, 0)?;
            {
                let s = h.get_mut(Region::Block(b), Owner(0))?;
                crate::mem::put_u32(s, 0, magic::NTLL)?;
                put_u16(s, 4, FORMAT_TABLE)?;
                put_u32(s, LV_FIRST, u32::try_from(i).map_err(|_| rt(Corruption::Range, i))?)?;
            }
            if self.tail != 0 {
                put_u64(h.get_mut(Region::Block(self.tail), Owner(0))?, LV_NEXT, b)?;
            } else {
                self.head = b;
            }
            self.tail = b;
        }
        let k = i % LV_PER_BLOCK;
        let s = h.get_mut(Region::Block(self.tail), Owner(0))?;
        img.put_ref(s, LV_REFS.wrapping_add(k.wrapping_mul(REF_BYTES)))?;
        put_u32(s, LV_ENTRIES, u32::try_from(k.wrapping_add(1)).map_err(|_| rt(Corruption::Range, k))?)?;
        self.count = i.wrapping_add(1);
        Ok(i)
    }
}

impl Default for LvBuilder {
    fn default() -> Self {
        Self::new()
    }
}
