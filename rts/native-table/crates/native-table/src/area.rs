//! Areas over host pages: block allocation, free stacks and image-page accounting.
//!
//! A table owns 16 areas. Area 0 holds control structures and rows (the table root, buffers, group pages and
//! every radix block), area 1 every index tree and key log, areas 2–14 are reserved and their records stay
//! zero, and area 15 is image storage: each 64 KiB image block is a whole host page. Each area has a
//! 48-byte record in the table root (at offset 160 + 48a).
//!
//! Allocation does not zero a block in general; every structure writes its header and every byte it later
//! reads. `alloc_zeroed` serves the structures that must be zero-filled when allocated (every radix block
//! among them), so none of their unwritten words can hold the bytes of a recycled image page.

use crate::consts::{AREAS, BLOCK, BLOCKS_PER_PAGE};
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::mem::{fill, put_u64, put_u8, rd_u64, rd_u8, Owner, PageHost, Region};
use crate::radix::{Radix, HEIGHT_MAX};

/// Table root offset of the area records.
pub const AREA_RECORDS: u64 = 160;
/// Bytes of one area record.
pub const AREA_RECORD: u64 = 48;
/// The image-storage area.
pub const IMAGE_AREA: u32 = 15;

const _: () = assert!(AREA_RECORDS + AREAS * AREA_RECORD == 928);

/// One area record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AreaRecord {
    /// @0, @33: the radix array of the area's page references, in allocation order (empty for area 15).
    pub page_list: Radix,
    /// @8: the newest page (0: none).
    pub newest: u64,
    /// @16: the top block of the area's free-block stack (0: empty).
    pub free_head: u64,
    /// @24: pages the area owns (area 15: image pages allocated and not yet freed).
    pub pages: u64,
    /// @32: the next never-used block index in the newest page, 0..=16.
    pub bump: u8,
}

/// The table root offset of area a's record, or `Corrupt(Runtime, Range)` for a ≥ 16.
fn record_offset(a: u32) -> Result<u64, Fault> {
    if u64::from(a) >= AREAS {
        return Err(corrupt(ImageId::Runtime, Corruption::Range, u64::from(a)));
    }
    // a < 16, so the offset is at most 160 + 15 · 48 = 880.
    Ok(AREA_RECORDS.wrapping_add(u64::from(a).wrapping_mul(AREA_RECORD)))
}

/// `Corrupt(Runtime, Range)` unless a may allocate blocks: `alloc_block(a)` and `free_block(a, ·)` take a ≤ 1;
/// a reserved area or the image area is refused.
fn block_area(a: u32) -> Result<(), Fault> {
    if a > 1 {
        return Err(corrupt(ImageId::Runtime, Corruption::Range, u64::from(a)));
    }
    Ok(())
}

impl AreaRecord {
    /// Area a's record from the table root's bytes. A bump above 16 or a page-list height above the radix
    /// maximum is `Corrupt(Runtime, Header)`.
    pub fn take(root: &[u8], a: u32) -> Result<AreaRecord, Fault> {
        let off = record_offset(a)?;
        // off ≤ 880, so every field offset below is at most 914.
        let rec = AreaRecord {
            page_list: Radix { root: rd_u64(root, off)?, height: rd_u8(root, off.wrapping_add(33))? },
            newest: rd_u64(root, off.wrapping_add(8))?,
            free_head: rd_u64(root, off.wrapping_add(16))?,
            pages: rd_u64(root, off.wrapping_add(24))?,
            bump: rd_u8(root, off.wrapping_add(32))?,
        };
        if u64::from(rec.bump) > BLOCKS_PER_PAGE || rec.page_list.height > HEIGHT_MAX {
            return Err(corrupt(ImageId::Runtime, Corruption::Header, off));
        }
        Ok(rec)
    }

    /// Writes area a's record, all 48 bytes, into the table root's bytes.
    pub fn put(&self, root: &mut [u8], a: u32) -> Result<(), Fault> {
        let off = record_offset(a)?;
        // off ≤ 880, so every field offset below is at most 914.
        fill(root, off, AREA_RECORD, 0)?;
        put_u64(root, off, self.page_list.root)?;
        put_u64(root, off.wrapping_add(8), self.newest)?;
        put_u64(root, off.wrapping_add(16), self.free_head)?;
        put_u64(root, off.wrapping_add(24), self.pages)?;
        put_u8(root, off.wrapping_add(32), self.bump)?;
        put_u8(root, off.wrapping_add(33), self.page_list.height)
    }

    /// Area a's record in the table whose root block is `tbl`.
    pub fn read<H: PageHost>(h: &H, tbl: u64, a: u32) -> Result<AreaRecord, Fault> {
        AreaRecord::take(h.get(Region::Block(tbl))?, a)
    }

    fn write<H: PageHost>(&self, h: &mut H, tbl: u64, a: u32) -> Result<(), Fault> {
        self.put(h.get_mut(Region::Block(tbl), Owner(0))?, a)
    }
}

/// The root block of a new table: block 0 of area 0's first page, the first block area 0 allocates.
/// Writes the 16 area records (768 bytes from offset 160) and nothing else of the root; area 0 then owns
/// that page, which is its newest with bump 1 and its page list of height 0.
pub fn new_root<H: PageHost>(h: &mut H) -> Result<u64, Fault> {
    let p = h.page_alloc(Owner(0))?;
    let tbl = h.block_of(p, 0);
    let b = h.get_mut(Region::Block(tbl), Owner(0))?;
    fill(b, AREA_RECORDS, AREAS.wrapping_mul(AREA_RECORD), 0)?;
    let rec = AreaRecord { page_list: Radix { root: p, height: 0 }, newest: p, free_head: 0, pages: 1, bump: 1 };
    rec.put(b, 0)?;
    Ok(tbl)
}

/// alloc_block(a), a ≤ 1: the top of the free stack; else the next never-used block of the newest
/// page; else block 0 of a new page, appended to the area's page list. The block's contents are unspecified,
/// except in test mode, where a block taken from the free stack has its stale link overwritten with the
/// host's fill pattern.
pub fn alloc_block<H: PageHost>(h: &mut H, tbl: u64, a: u32) -> Result<u64, Fault> {
    block_area(a)?;
    let mut rec = AreaRecord::read(h, tbl, a)?;
    if rec.free_head != 0 {
        let b = rec.free_head;
        rec.free_head = rd_u64(h.get(Region::Block(b))?, 0)?;
        rec.write(h, tbl, a)?;
        if let Some(p) = h.test_fill() {
            fill(h.get_mut(Region::Block(b), Owner(a))?, 0, 8, p)?;
        }
        return Ok(b);
    }
    if rec.newest != 0 && u64::from(rec.bump) < BLOCKS_PER_PAGE {
        let b = h.block_of(rec.newest, u32::from(rec.bump));
        // bump < 16.
        rec.bump = rec.bump.wrapping_add(1);
        rec.write(h, tbl, a)?;
        return Ok(b);
    }
    let p = h.page_alloc(Owner(a))?;
    let index = rec.pages;
    rec.pages = rec.pages.checked_add(1).ok_or(corrupt(ImageId::Runtime, Corruption::Count, tbl))?;
    // The page is the newest, with its block 0 taken, before the page-list append, so the radix blocks the
    // append takes for area 0's own list come from this page.
    rec.newest = p;
    rec.bump = 1;
    rec.write(h, tbl, a)?;
    let mut list = rec.page_list;
    list.set(h, tbl, index, p)?;
    let mut rec = AreaRecord::read(h, tbl, a)?;
    rec.page_list = list;
    rec.write(h, tbl, a)?;
    Ok(h.block_of(p, 0))
}

/// alloc_block(a) followed by writing all 4,096 bytes zero: for the structures that are zero-filled when they
/// are allocated (every radix block, image-table and run-descriptor blocks, long-value list blocks, job,
/// queue, drain and totals blocks).
pub fn alloc_zeroed<H: PageHost>(h: &mut H, tbl: u64, a: u32) -> Result<u64, Fault> {
    let b = alloc_block(h, tbl, a)?;
    fill(h.get_mut(Region::Block(b), Owner(a))?, 0, BLOCK, 0)?;
    Ok(b)
}

/// free_block(a, b), a ≤ 1: writes the free head into b's bytes 0..8 and makes b the head.
pub fn free_block<H: PageHost>(h: &mut H, tbl: u64, a: u32, b: u64) -> Result<(), Fault> {
    block_area(a)?;
    let mut rec = AreaRecord::read(h, tbl, a)?;
    put_u64(h.get_mut(Region::Block(b), Owner(a))?, 0, rec.free_head)?;
    rec.free_head = b;
    rec.write(h, tbl, a)
}

/// A page for image storage: `page_alloc(Owner(15))`, counted in area 15's record.
pub fn image_page_alloc<H: PageHost>(h: &mut H, tbl: u64) -> Result<u64, Fault> {
    let mut rec = AreaRecord::read(h, tbl, IMAGE_AREA)?;
    let p = h.page_alloc(Owner(IMAGE_AREA))?;
    rec.pages = rec.pages.checked_add(1).ok_or(corrupt(ImageId::Runtime, Corruption::Count, tbl))?;
    rec.write(h, tbl, IMAGE_AREA)?;
    Ok(p)
}

/// Returns an image page to the host at once, and takes it off area 15's count.
pub fn image_page_free<H: PageHost>(h: &mut H, tbl: u64, p: u64) -> Result<(), Fault> {
    let mut rec = AreaRecord::read(h, tbl, IMAGE_AREA)?;
    rec.pages = rec.pages.checked_sub(1).ok_or(corrupt(ImageId::Runtime, Corruption::Count, tbl))?;
    h.page_free(p, Owner(IMAGE_AREA))?;
    rec.write(h, tbl, IMAGE_AREA)
}
