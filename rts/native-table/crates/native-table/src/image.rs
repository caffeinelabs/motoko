//! Image storage: images in whole 64 KiB host pages of area 15, named by block entries kept
//! in a direct table (a group image record of at most `INLINE_ENTRIES` blocks) or in a radix array of area-0
//! blocks; the 24-byte image reference; and the image reader to the extent image storage needs it.
//!
//! **Block entry** (u64): the page reference of the area-15 page that holds image block k, for each of the
//! ⌈length / 65,536⌉ blocks, the last one included. The bytes of the last page past the length are never
//! read and never written.
//!
//! **Image reference** (24 bytes): {root u64 @0: when the height is 0, the block entry of the image's only
//! block, else a radix root; length u64 @8, a multiple of 8; height u8 @16; flags u8 @17, bit 0 *believed*;
//! 6 zero bytes}.
//!
//! **Group image record** (80 bytes): {length u64 @0; height u8 @8, `0xFF` for a direct table; direct
//! entries u8 @9; flags u8 @10, bit 0 believed; 5 zero bytes; entries [u64; 8] @16}. A direct table keeps
//! the block entries in the record; a longer image keeps its radix root in entry 0 and its height at @8.

use crate::area::{alloc_block, image_page_alloc};
use crate::consts::{BLOCK, IMAGE_MAX, INLINE_ENTRIES, PAGE};
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::mem::{fill, put, put_u64, put_u8, rd_u64, rd_u8, sub, sub_mut, PageHost, Region};
use crate::radix::{Radix, HEIGHT_MAX};

/// The height byte of a group image record whose block entries are inline.
pub const DIRECT: u8 = 0xFF;
/// Bytes of a group image record.
pub const RECORD_BYTES: u64 = 80;
/// Bytes of an image reference.
pub const REF_BYTES: u64 = 24;
/// Flags bit 0: the image's bytes came from a load.
pub const BELIEVED: u8 = 1;

/// Entries of a direct table.
const INLINE: usize = 8;
const _: () = assert!(INLINE_ENTRIES == INLINE as u64);

/// Where an image's block entries live.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocks {
    /// `count` ≤ 8 block entries in order, the rest zero: a group image record's direct table, or the one
    /// block of an image reference of height 0.
    Direct { count: u8, entries: [u64; INLINE] },
    /// A radix array whose entry k is the block entry of image block k.
    Radix(Radix),
}

/// One image, as a group image record or an image reference names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Image {
    /// Bytes.
    pub length: u64,
    pub blocks: Blocks,
    /// The bytes came from a load (flags bit 0). Kept wherever the record or reference moves.
    pub believed: bool,
}

/// `Corrupt(Runtime, kind)` at `off`: a runtime-written record or reference that breaks its layout.
const fn bad(kind: Corruption, off: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, off)
}

/// A zero-filled area-0 block holding `entries` from entry 0: a radix block of height 1 written whole.
fn radix_block<H: PageHost>(h: &mut H, tbl: u64, entries: &[u64]) -> Result<u64, Fault> {
    let b = alloc_block(h, tbl, 0)?;
    let s = h.get_mut(Region::Block(b))?;
    fill(s, 0, BLOCK, 0)?;
    let mut at = 0u64;
    for &e in entries {
        put_u64(s, at, e)?;
        // At most 9 entries: at ≤ 72.
        at = at.wrapping_add(8);
    }
    Ok(b)
}

impl Image {
    /// An image of length 0 with no block, not believed: the start of an image the crate writes.
    pub const EMPTY: Image =
        Image { length: 0, blocks: Blocks::Direct { count: 0, entries: [0; INLINE] }, believed: false };

    /// ⌈length / 65,536⌉: the image blocks the length covers.
    #[inline]
    pub fn block_count(&self) -> u64 {
        self.length.div_ceil(PAGE)
    }

    /// The block entry of image block k, or 0 when absent.
    pub fn entry<H: PageHost + ?Sized>(&self, h: &H, k: u64) -> Result<u64, Fault> {
        match &self.blocks {
            Blocks::Direct { count, entries } => {
                if k >= u64::from(*count) {
                    return Ok(0);
                }
                Ok(usize::try_from(k).ok().and_then(|i| entries.get(i).copied()).unwrap_or(0))
            }
            Blocks::Radix(r) => r.get(h, k),
        }
    }

    /// The checks every stored form shares: the length within `IMAGE_MAX` and covered by the block list.
    fn check(&self, off: u64) -> Result<(), Fault> {
        if self.length > IMAGE_MAX {
            return Err(bad(Corruption::Header, off));
        }
        let n = self.block_count();
        match self.blocks {
            Blocks::Direct { count, .. } => {
                if u64::from(count) > INLINE_ENTRIES || u64::from(count) < n {
                    return Err(bad(Corruption::Count, off));
                }
            }
            Blocks::Radix(r) => {
                r.check()?;
                if n > r.capacity() {
                    return Err(bad(Corruption::Depth, off));
                }
            }
        }
        Ok(())
    }

    /// The group image record at `off` of `b`. A height other than `DIRECT` or 1..=`HEIGHT_MAX`, a radix
    /// record whose direct-entries byte is not 0, flags other than bit 0, non-zero reserved bytes, or a
    /// block list that does not cover the length is `Corrupt(Runtime, ·)`.
    pub fn read_record(b: &[u8], off: u64) -> Result<Image, Fault> {
        let r = sub(b, off, RECORD_BYTES)?;
        let length = rd_u64(r, 0)?;
        let height = rd_u8(r, 8)?;
        let direct = rd_u8(r, 9)?;
        let flags = rd_u8(r, 10)?;
        if flags & !BELIEVED != 0 {
            return Err(bad(Corruption::Flags, off));
        }
        if sub(r, 11, 5)?.iter().any(|&x| x != 0) {
            return Err(bad(Corruption::Header, off));
        }
        let mut entries = [0u64; INLINE];
        let mut at = 16u64;
        for e in entries.iter_mut() {
            *e = rd_u64(r, at)?;
            // At most 8 entries: at ≤ 80.
            at = at.wrapping_add(8);
        }
        let blocks = if height == DIRECT {
            if u64::from(direct) > INLINE_ENTRIES {
                return Err(bad(Corruption::Count, off));
            }
            // Entries past the count are not part of the table.
            for (i, e) in entries.iter_mut().enumerate() {
                if i >= usize::from(direct) {
                    *e = 0;
                }
            }
            Blocks::Direct { count: direct, entries }
        } else if (1..=HEIGHT_MAX).contains(&height) && direct == 0 {
            Blocks::Radix(Radix { root: entries.first().copied().unwrap_or(0), height })
        } else {
            return Err(bad(Corruption::Header, off));
        };
        let img = Image { length, blocks, believed: flags & BELIEVED != 0 };
        img.check(off)?;
        Ok(img)
    }

    /// Writes this image as an 80-byte group image record at `off` of `b`. A radix of height 0 (one entry) is
    /// written as a direct table of one entry.
    pub fn put_record(&self, b: &mut [u8], off: u64) -> Result<(), Fault> {
        self.check(off)?;
        let r = sub_mut(b, off, RECORD_BYTES)?;
        fill(r, 0, RECORD_BYTES, 0)?;
        put_u64(r, 0, self.length)?;
        put_u8(r, 10, if self.believed { BELIEVED } else { 0 })?;
        match self.blocks {
            Blocks::Radix(x) if x.height > 0 => {
                put_u8(r, 8, x.height)?;
                put_u64(r, 16, x.root)
            }
            Blocks::Radix(x) => {
                put_u8(r, 8, DIRECT)?;
                put_u8(r, 9, if x.root == 0 { 0 } else { 1 })?;
                put_u64(r, 16, x.root)
            }
            Blocks::Direct { count, entries } => {
                put_u8(r, 8, DIRECT)?;
                put_u8(r, 9, count)?;
                let mut at = 16u64;
                for &e in entries.iter().take(usize::from(count)) {
                    put_u64(r, at, e)?;
                    // At most 8 entries: at ≤ 80.
                    at = at.wrapping_add(8);
                }
                Ok(())
            }
        }
    }

    /// The image reference at `off` of `b`. A length that is not a multiple of 8, flags other than bit 0,
    /// non-zero reserved bytes, a height above the radix maximum, or a block list that does not cover the
    /// length is `Corrupt(Runtime, ·)`.
    pub fn read_ref(b: &[u8], off: u64) -> Result<Image, Fault> {
        let r = sub(b, off, REF_BYTES)?;
        let root = rd_u64(r, 0)?;
        let length = rd_u64(r, 8)?;
        let height = rd_u8(r, 16)?;
        let flags = rd_u8(r, 17)?;
        if flags & !BELIEVED != 0 {
            return Err(bad(Corruption::Flags, off));
        }
        if length % 8 != 0 || sub(r, 18, 6)?.iter().any(|&x| x != 0) {
            return Err(bad(Corruption::Header, off));
        }
        let blocks = if height == 0 {
            let mut entries = [0u64; INLINE];
            if let Some(e) = entries.first_mut() {
                *e = root;
            }
            Blocks::Direct { count: if root == 0 { 0 } else { 1 }, entries }
        } else {
            Blocks::Radix(Radix { root, height })
        };
        let img = Image { length, blocks, believed: flags & BELIEVED != 0 };
        img.check(off)?;
        Ok(img)
    }

    /// Writes this image as a 24-byte image reference at `off` of `b`. A direct table of 2 to 8 entries has
    /// no reference form: convert it with `ref_form` first; it, or a length that is not a multiple of 8, is
    /// `Corrupt(Runtime, Layout)`.
    pub fn put_ref(&self, b: &mut [u8], off: u64) -> Result<(), Fault> {
        self.check(off)?;
        if self.length % 8 != 0 {
            return Err(bad(Corruption::Layout, off));
        }
        let (root, height) = match self.blocks {
            Blocks::Radix(x) => (x.root, x.height),
            Blocks::Direct { count, entries } if count <= 1 => (entries.first().copied().unwrap_or(0), 0),
            Blocks::Direct { .. } => return Err(bad(Corruption::Layout, off)),
        };
        let r = sub_mut(b, off, REF_BYTES)?;
        fill(r, 0, REF_BYTES, 0)?;
        put_u64(r, 0, root)?;
        put_u64(r, 8, self.length)?;
        put_u8(r, 16, height)?;
        put_u8(r, 17, if self.believed { BELIEVED } else { 0 })
    }

    /// The form of this image a 24-byte reference can hold: 1 block, its block entry at
    /// height 0; 2 to 8 blocks, one new area-0 radix block (`alloc_block(0)`, all 512 entries written: the
    /// block entries at 0 to n − 1, zero above) at height 1; a radix, unchanged. One block allocation and one
    /// 4 KiB write at most. The believed flag is kept.
    pub fn ref_form<H: PageHost>(self, h: &mut H, tbl: u64) -> Result<Image, Fault> {
        match self.blocks {
            Blocks::Direct { count, entries } if count >= 2 => {
                let list = entries.get(..usize::from(count)).ok_or(bad(Corruption::Count, tbl))?;
                let root = radix_block(h, tbl, list)?;
                Ok(Image { blocks: Blocks::Radix(Radix { root, height: 1 }), ..self })
            }
            _ => Ok(self),
        }
    }

    /// Appends `bytes` at the image's end: each image block's page is allocated from area 15 when its first
    /// byte is written, and its entry added to the direct table, which becomes a radix of height 1 at the
    /// ninth block, or to the radix, which grows as radix arrays grow. Bytes are written one 4 KiB block of the
    /// page at a time, so a write changes only the blocks it covers.
    pub fn append<H: PageHost>(&mut self, h: &mut H, tbl: u64, bytes: &[u8]) -> Result<(), Fault> {
        let mut rest = bytes;
        while !rest.is_empty() {
            let o = self.length % PAGE;
            let k = self.length / PAGE;
            if o == 0 {
                self.add_block(h, tbl, k)?;
            }
            // The rest of the 4 KiB block that holds offset o of the page.
            let room = BLOCK.wrapping_sub(o % BLOCK);
            let n = room.min(u64::try_from(rest.len()).map_err(|_| bad(Corruption::Range, self.length))?);
            let end = self.length.checked_add(n).filter(|&e| e <= IMAGE_MAX);
            let end = end.ok_or(bad(Corruption::Range, self.length))?;
            let cut = usize::try_from(n).map_err(|_| bad(Corruption::Range, self.length))?;
            let (now, later) = rest.split_at_checked(cut).ok_or(bad(Corruption::Range, self.length))?;
            let p = self.entry(h, k)?;
            if p == 0 {
                return Err(bad(Corruption::Link, k));
            }
            // o < 65,536, so the block index is below 16.
            let j = u32::try_from(o / BLOCK).map_err(|_| bad(Corruption::Range, o))?;
            let b = h.block_of(p, j);
            put(h.get_mut(Region::Block(b))?, o % BLOCK, now)?;
            self.length = end;
            rest = later;
        }
        Ok(())
    }

    /// Appends `len` bytes that `f` writes straight into the image's pages: `f(at, dst)` fills `dst` with the
    /// bytes [at, at + dst.len()) of the appended run, in order. A piece is a whole lent page when it covers
    /// a whole image block, else the covered part of one 4 KiB block, so a piece never crosses an image block
    /// and a write changes only the blocks it covers. This is how a value from the message's argument is
    /// copied once, by the host, into the page that keeps it.
    pub fn append_with<H, F>(&mut self, h: &mut H, tbl: u64, len: u64, mut f: F) -> Result<(), Fault>
    where
        H: PageHost,
        F: FnMut(u64, &mut [u8]) -> Result<(), Fault>,
    {
        let end = self.length.checked_add(len).filter(|&e| e <= IMAGE_MAX);
        let end = end.ok_or(bad(Corruption::Range, self.length))?;
        let mut done = 0u64;
        while self.length < end {
            let o = self.length % PAGE;
            let k = self.length / PAGE;
            if o == 0 {
                self.add_block(h, tbl, k)?;
            }
            let p = self.entry(h, k)?;
            if p == 0 {
                return Err(bad(Corruption::Link, k));
            }
            let left = end.wrapping_sub(self.length);
            let (region, at, n) = if o == 0 && left >= PAGE {
                (Region::Page(p), 0, PAGE)
            } else {
                // o < 65,536, so the block index is below 16.
                let j = u32::try_from(o / BLOCK).map_err(|_| bad(Corruption::Range, o))?;
                let room = BLOCK.wrapping_sub(o % BLOCK);
                (Region::Block(h.block_of(p, j)), o % BLOCK, room.min(left))
            };
            let dst = sub_mut(h.get_mut(region)?, at, n)?;
            f(done, dst)?;
            // n ≤ left = end − length, so neither sum passes `end`.
            done = done.wrapping_add(n);
            self.length = self.length.wrapping_add(n);
        }
        Ok(())
    }

    /// Allocates the page of image block k = the number of blocks held, and records its block entry.
    fn add_block<H: PageHost>(&mut self, h: &mut H, tbl: u64, k: u64) -> Result<(), Fault> {
        match self.blocks {
            Blocks::Direct { count, mut entries } => {
                if u64::from(count) != k {
                    return Err(bad(Corruption::Count, k));
                }
                let p = image_page_alloc(h, tbl)?;
                if let Some(e) = entries.get_mut(usize::from(count)) {
                    *e = p;
                    // count < 8.
                    self.blocks = Blocks::Direct { count: count.wrapping_add(1), entries };
                } else {
                    // The direct table is full: the ninth block moves the image to a radix of height 1.
                    let mut all = [0u64; INLINE + 1];
                    for (d, s) in all.iter_mut().zip(entries.iter()) {
                        *d = *s;
                    }
                    if let Some(last) = all.last_mut() {
                        *last = p;
                    }
                    let root = radix_block(h, tbl, &all)?;
                    self.blocks = Blocks::Radix(Radix { root, height: 1 });
                }
            }
            Blocks::Radix(mut r) => {
                let p = image_page_alloc(h, tbl)?;
                r.set(h, tbl, k, p)?;
                self.blocks = Blocks::Radix(r);
            }
        }
        Ok(())
    }
}

/// Reads one image's bytes, to the extent image storage needs: a read of image offset o resolves block k = o >> 16
/// through the record or reference and checks o + len ≤ length, and o + len inside the block, before it
/// forms the range. Each failure is `Corrupt { image: id, kind: Bounds, offset: o }`, a missing
/// block entry included.
pub struct ImageReader<'h, H: PageHost + ?Sized> {
    host: &'h H,
    id: ImageId,
    img: Image,
}

impl<'h, H: PageHost + ?Sized> ImageReader<'h, H> {
    pub fn new(host: &'h H, id: ImageId, img: Image) -> Self {
        ImageReader { host, id, img }
    }

    pub fn length(&self) -> u64 {
        self.img.length
    }

    /// [off, off + len) when it lies inside the image and inside one 64 KiB image block.
    pub fn bytes(&self, off: u64, len: u64) -> Result<&'h [u8], Fault> {
        let bounds = corrupt(self.id, Corruption::Bounds, off);
        let end = off.checked_add(len).ok_or(bounds)?;
        if end > self.img.length {
            return Err(bounds);
        }
        let k = off / PAGE;
        let o = off % PAGE;
        if o.checked_add(len).ok_or(bounds)? > PAGE {
            return Err(bounds);
        }
        if len == 0 {
            return Ok(&[]);
        }
        let p = self.img.entry(self.host, k)?;
        if p == 0 {
            return Err(bounds);
        }
        let host: &'h H = self.host;
        let page = host.get(Region::Page(p))?;
        sub(page, o, len).map_err(|_| bounds)
    }

    /// The little-endian u64 at `off`.
    pub fn u64_at(&self, off: u64) -> Result<u64, Fault> {
        rd_u64(self.bytes(off, 8)?, 0)
    }
}
