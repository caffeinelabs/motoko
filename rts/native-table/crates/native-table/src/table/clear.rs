//! Emptying a table: `Table::clear` gives the table new, empty structures at once and parks the old ones; the
//! table's maintenance gives their memory back in bounded steps (`step`).
//!
//! The new structures are an empty table's, on the same root block: area 0 keeps only the root block's page,
//! all of whose blocks count as used, and area 1 holds nothing. The old ones are named by a **park block**
//! `NTPK` (area 0 after the clear): {magic u32 @0; `FORMAT_TABLE` u16 @4; the park block of a later clear u64 @8
//! (0: none); the table root's bytes 16–159 as they were, the group count @40, the open group @48, the tail
//! block @56 and the group directory @84 and @88 among them; the old area 0 and area 1 records at the root's
//! offsets 160 and 208; the next group number or page-list index u64 @256; the phase u8 @264; the first page put
//! aside u64 @272}. The root names the first park block and the last (`root::PARK_FIRST`, `root::PARK_LAST`), and
//! its freeing byte (`root::FREEING`) is set while one is named.
//!
//! A step works on the first park block, in phases:
//! 0. the groups, one a step: a sealed group's images (its rows, its long values, its patches' values) go back,
//!    as a compaction's drop frees them, and the open group's long values; their area-0 blocks are freed onto the
//!    old area 0's free stack, the root holding the old record for the step;
//! 1. area 1's pages, `PAGES` a step (in phases 1 and 2, a zero entry is a page a sweep gave back, passed over);
//! 2. area 0's pages but the root block's page, `PAGES` a step. A page whose blocks may hold the page list's own
//!    radix blocks (index 1 and the multiples of 512: the list's blocks are taken from the page whose append
//!    needs them) is put aside on a list linked through its block 0, which the list never holds;
//! 3. the pages put aside, `PAGES` a step. Then the park block is done: the next one becomes the first, its own
//!    block going back with the pages of the structures that one parks; or, when it is the last, the root
//!    block's page's other blocks and the park block go onto area 0's free stack, and the freeing byte is cleared.

use crate::area::{alloc_block, free_block, AreaRecord};
use crate::consts::{magic, BLOCKS_PER_PAGE, FORMAT_TABLE, RADIX_FAN};
use crate::fault::{rt, Corruption, Fault};
use crate::host::FREEING;
use crate::mem::{copy_bytes, put_u16, put_u32, put_u64, put_u8, rd_u64, rd_u8, sub, sub_mut, PageHost, Region};
use crate::radix::Radix;

use super::buffer::{self, DESC_BIT};
use super::layout::Shape;
use super::{drain, root, write};

/// Park block offsets.
const NEXT: u64 = 8;
const AT: u64 = 256;
const PHASE: u64 = 264;
const ASIDE: u64 = 272;
/// The root's bytes a park block keeps.
const WORDS: u64 = 160;
/// Pages a step gives back.
const PAGES: u64 = 16;

/// The structures a clear parked, and the park blocks the root named before it.
pub struct Parked {
    park: u64,
    first: u64,
    last: u64,
}

/// Parks the table's structures: a park block, taken from the new area 0, holds the root's words and the old
/// area 0 and 1 records; the root then holds the new records. Its other words are left for `Table::init`.
pub fn park<H: PageHost>(h: &mut H, tbl: u64) -> Result<Parked, Fault> {
    let first = root::u64_at(&*h, tbl, root::PARK_FIRST)?;
    let last = root::u64_at(&*h, tbl, root::PARK_LAST)?;
    let old = [AreaRecord::read(&*h, tbl, 0)?, AreaRecord::read(&*h, tbl, 1)?];
    let p0 = old[0].page_list.get(&*h, 0)?;
    let full = u8::try_from(BLOCKS_PER_PAGE).map_err(|_| rt(Corruption::Range, BLOCKS_PER_PAGE))?;
    AreaRecord { page_list: Radix { root: p0, height: 0 }, newest: p0, free_head: 0, pages: 1, bump: full }
        .write(h, tbl, 0)?;
    AreaRecord::default().write(h, tbl, 1)?;
    let park = alloc_block(h, tbl, 0)?;
    let (r, b) = h.pair_mut(Region::Block(tbl), Region::Block(park))?;
    copy_bytes(sub_mut(b, 0, WORDS)?, sub(r, 0, WORDS)?);
    put_u32(b, 0, magic::NTPK)?;
    put_u16(b, 4, FORMAT_TABLE)?;
    put_u64(b, NEXT, 0)?;
    old[0].put(b, 0)?;
    old[1].put(b, 1)?;
    put_u64(b, AT, 0)?;
    put_u8(b, PHASE, 0)?;
    put_u64(b, ASIDE, 0)?;
    Ok(Parked { park, first, last })
}

/// Links the park block after the ones the root named, and sets the freeing byte.
pub fn link<H: PageHost>(h: &mut H, tbl: u64, p: Parked) -> Result<(), Fault> {
    if p.last != 0 {
        put_u64(h.get_mut(Region::Block(p.last))?, NEXT, p.park)?;
    }
    root::set_u64(h, tbl, root::PARK_FIRST, if p.first != 0 { p.first } else { p.park })?;
    root::set_u64(h, tbl, root::PARK_LAST, p.park)?;
    root::set_u8(h, tbl, root::FREEING, FREEING)
}

/// One step of giving back the first park block's structures.
pub fn step<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape) -> Result<(), Fault> {
    let park = root::u64_at(&*h, tbl, root::PARK_FIRST)?;
    let (at, phase) = {
        let b = h.get(Region::Block(park))?;
        (rd_u64(b, AT)?, rd_u8(b, PHASE)?)
    };
    let next = match phase {
        0 if at < root::u64_at(&*h, park, root::NEXT_GROUP)? => {
            group(h, tbl, shape, park, at)?;
            Some(at.wrapping_add(1))
        }
        0 => None,
        1 | 2 => pages(h, park, u32::from(2u8.wrapping_sub(phase)), at)?,
        3 => return aside(h, tbl, park),
        _ => return Err(rt(Corruption::Header, park)),
    };
    let b = h.get_mut(Region::Block(park))?;
    match next {
        Some(at) => put_u64(b, AT, at),
        None => {
            put_u8(b, PHASE, phase.wrapping_add(1))?;
            put_u64(b, AT, 0)
        }
    }
}

/// Gives back group g's images and long values, as the park block's directory names it.
fn group<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, park: u64, g: u64) -> Result<(), Fault> {
    let e = root::directory(&*h, park)?.get(&*h, g)?;
    if e == 0 {
        return Ok(());
    }
    let now = AreaRecord::read(&*h, tbl, 0)?;
    AreaRecord::read(&*h, park, 0)?.write(h, tbl, 0)?;
    if e & DESC_BIT == 0 {
        drain::free_group_page(h, tbl, e)?;
    } else {
        // The open group, unsealed: its rows' long values, when one of them held one.
        let tail = root::u64_at(&*h, park, root::TAIL)?;
        if buffer::u64_of(&*h, tail, buffer::H_FLAGS)? & buffer::OPEN_LONG != 0 {
            let d = e & !DESC_BIT;
            for k in 0..buffer::u16_of(&*h, d, buffer::D_BLOCKS)? {
                let b = buffer::u64_of(&*h, d, buffer::D_RECORDS.wrapping_add(k.wrapping_mul(16)))?;
                for s in 0..buffer::u16_of(&*h, b, buffer::B_SLOTS)? {
                    if let Some(loc) = buffer::slot_row(&*h, b, s)? {
                        write::free_longs(h, tbl, shape, loc)?;
                    }
                }
            }
        }
    }
    now.write(h, tbl, 0)
}

/// Gives back up to `PAGES` pages of area a's old page list from index `at`, the root block's page excepted and
/// the pages that may hold area 0's list's own blocks put aside; the index to go on from, `None` at the end.
fn pages<H: PageHost>(h: &mut H, park: u64, a: u32, at: u64) -> Result<Option<u64>, Fault> {
    let rec = AreaRecord::read(&*h, park, a)?;
    let end = rec.pages.min(at.saturating_add(PAGES));
    for i in at..end {
        let p = rec.page_list.get(&*h, i)?;
        if p == 0 {
            // Given back by a sweep.
        } else if a == 1 {
            h.page_free(p)?;
        } else if i == 1 || (i != 0 && i % RADIX_FAN == 0) {
            let aside = rd_u64(h.get(Region::Block(park))?, ASIDE)?;
            put_u64(h.get_mut(Region::Block(h.block_of(p, 0)))?, 0, aside)?;
            put_u64(h.get_mut(Region::Block(park))?, ASIDE, p)?;
        } else if i != 0 {
            h.page_free(p)?;
        }
    }
    Ok((end < rec.pages).then_some(end))
}

/// Gives back up to `PAGES` of the pages put aside; once none is left, ends the park block.
fn aside<H: PageHost>(h: &mut H, tbl: u64, park: u64) -> Result<(), Fault> {
    let mut p = rd_u64(h.get(Region::Block(park))?, ASIDE)?;
    for _ in 0..PAGES {
        if p == 0 {
            break;
        }
        let next = rd_u64(h.get(Region::Block(h.block_of(p, 0)))?, 0)?;
        h.page_free(p)?;
        p = next;
    }
    put_u64(h.get_mut(Region::Block(park))?, ASIDE, p)?;
    if p != 0 {
        return Ok(());
    }
    let next = rd_u64(h.get(Region::Block(park))?, NEXT)?;
    if next != 0 {
        return root::set_u64(h, tbl, root::PARK_FIRST, next);
    }
    let p0 = AreaRecord::read(&*h, tbl, 0)?.page_list.get(&*h, 0)?;
    if h.block_of(p0, 0) != tbl {
        return Err(rt(Corruption::Link, tbl));
    }
    for i in 1..u32::try_from(BLOCKS_PER_PAGE).map_err(|_| rt(Corruption::Range, BLOCKS_PER_PAGE))? {
        free_block(h, tbl, 0, h.block_of(p0, i))?;
    }
    free_block(h, tbl, 0, park)?;
    root::set_u64(h, tbl, root::PARK_FIRST, 0)?;
    root::set_u64(h, tbl, root::PARK_LAST, 0)?;
    root::set_u8(h, tbl, root::FREEING, root::u8_at(&*h, tbl, root::FREEING)? & !FREEING)
}
