//! Freeing. A call frees at once what it lets go of: an image (`free_image`), a radix array of blocks
//! (`free_list`), a group page with everything it names (`free_group_page`), and a sealed group's buffer
//! descriptor with its buffer blocks and its long row images (`free_descriptor`). The blocks a seal lets go of
//! (its work space and the sealed group's buffer) do not count toward a sweep (`area::free_block`).

use crate::area::{free_block, image_page_free, push_free};
use crate::consts::BUFFER_BLOCKS_MAX;
use crate::fault::Fault;
use crate::image::{Blocks, Image, REF_BYTES};
use crate::mem::{rd_u32, rd_u64, PageHost, Region};
use crate::radix::Radix;

use super::buffer::{self, SLOT_DEAD};
use super::page;

// ---------------------------------------------------------------- frees at once

/// Frees a radix array listing blocks of area `area`: the blocks, then the array's own blocks.
pub fn free_list<H: PageHost>(h: &mut H, tbl: u64, r: Radix, area: u32) -> Result<(), Fault> {
    r.release(h, tbl, |h, b| push_free(h, tbl, area, b))
}

/// Frees an image: its pages, then its radix blocks.
pub fn free_image<H: PageHost>(h: &mut H, tbl: u64, img: &Image) -> Result<(), Fault> {
    match img.blocks {
        Blocks::Direct { count, entries } => {
            for &e in entries.iter().take(usize::from(count)) {
                if e != 0 {
                    image_page_free(h, tbl, e)?;
                }
            }
            Ok(())
        }
        Blocks::Radix(r) => r.release(h, tbl, |h, p| image_page_free(h, tbl, p)),
    }
}

/// Frees a long-value list: each entry's long value, then each list block.
fn free_lv_list<H: PageHost>(h: &mut H, tbl: u64, first: u64) -> Result<(), Fault> {
    let mut b = first;
    while b != 0 {
        let (n, next) = {
            let s = h.get(Region::Block(b))?;
            (u64::from(rd_u32(s, page::LV_ENTRIES)?).min(page::LV_PER_BLOCK), rd_u64(s, page::LV_NEXT)?)
        };
        for i in 0..n {
            let at = page::LV_REFS.wrapping_add(i.wrapping_mul(REF_BYTES));
            let img = {
                let s = h.get(Region::Block(b))?;
                if crate::mem::sub(s, at, REF_BYTES)?.iter().all(|&x| x == 0) {
                    continue;
                }
                Image::read_ref(s, at)?
            };
            free_image(h, tbl, &img)?;
        }
        free_block(h, tbl, 0, b)?;
        b = next;
    }
    Ok(())
}

/// Frees a group page and everything it names: its image, its long-value list with the long values it lists,
/// its patch directory with the values its entries hold, and its extremes block.
pub fn free_group_page<H: PageHost>(h: &mut H, tbl: u64, pg: u64) -> Result<(), Fault> {
    let (img, list, dir, ext) = {
        let b = h.get(Region::Block(pg))?;
        (
            Image::read_record(b, page::P_IMAGE)?,
            rd_u64(b, page::P_LV_LIST)?,
            rd_u64(b, page::P_PATCH_DIR)?,
            rd_u64(b, page::P_EXT)?,
        )
    };
    free_image(h, tbl, &img)?;
    free_lv_list(h, tbl, list)?;
    super::patch::free_dir(h, tbl, dir)?;
    if ext != 0 {
        free_block(h, tbl, 0, ext)?;
    }
    free_block(h, tbl, 0, pg)
}

/// Frees a sealed group's buffer descriptor: its live rows' long row images (none is read when the descriptor
/// says no row of the group held a long value, `buffer::LONG_ROWS`; the seal took every long item's value into
/// the group), its buffer blocks, an extremes block it still names, and the descriptor.
pub fn free_descriptor<H: PageHost>(h: &mut H, tbl: u64, d: u64) -> Result<(), Fault> {
    let blocks = buffer::u16_of(&*h, d, buffer::D_BLOCKS)?.min(BUFFER_BLOCKS_MAX);
    if buffer::u16_of(&*h, d, buffer::D_FLAGS)? & u64::from(buffer::LONG_ROWS) != 0 {
        for k in 0..blocks {
            let b = buffer::u64_of(&*h, d, buffer::D_RECORDS.wrapping_add(k.wrapping_mul(16)))?;
            for s in 0..buffer::u16_of(&*h, b, buffer::B_SLOTS)? {
                let slot = buffer::read_slot(h.get(Region::Block(b))?, s)?;
                if slot.flags & SLOT_DEAD != 0 {
                    continue;
                }
                if let buffer::RowLoc::Long(img) = buffer::stub_row(h.get(Region::Block(b))?, b, &slot)? {
                    free_image(h, tbl, &img)?;
                }
            }
        }
    }
    for k in 0..blocks {
        let b = buffer::u64_of(&*h, d, buffer::D_RECORDS.wrapping_add(k.wrapping_mul(16)))?;
        push_free(h, tbl, 0, b)?;
    }
    let ext = buffer::u64_of(&*h, d, buffer::D_EXT)?;
    if ext != 0 {
        push_free(h, tbl, 0, ext)?;
    }
    push_free(h, tbl, 0, d)
}
