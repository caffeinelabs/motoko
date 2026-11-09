//! Freeing. A call frees at once what it lets go of: an image (`free_image`), a radix array of blocks
//! (`free_list`), a group page with everything it names (`free_group_page`), and a sealed group's buffer
//! descriptor with its buffer blocks and the long values its rows still name (`free_descriptor`). A removed
//! secondary index's tree and key log, whose size no call bounds, go to the drain instead: a stack of records
//! that maintenance steps free, within `Units::DRAIN` a step.
//!
//! **Drain block** (area 0, zero-filled when allocated): {magic `NTDR` u32 @0; `FORMAT_TABLE` u16 @4; records
//! u16 @6; block below u64 @8; zero u16 @16; 127 records of 32 bytes from @32}. A record is {kind u8 @0; height
//! u8 @1; depth u8 @2; zero u8 @3; digits [u16; 7] @4, the walk's position; zero @18; root u64 @24}. The table
//! root names the top block at offset 144.
//!
//! Kinds: 18 a removed secondary index's tree (its key width W in digit 0); 19 a secondary index's key log (its
//! blocks, then its radix blocks). A radix walk keeps its position in the digits, base 512, most significant
//! first.

use crate::area::{alloc_zeroed, free_block, image_page_free};
use crate::consts::{magic, BLOCK, BUFFER_BLOCKS_MAX, FORMAT_TABLE, PATH_STACK};
use crate::fault::{rt, Corruption, Fault};
use crate::host::Progress;
use crate::image::{Blocks, Image, REF_BYTES};
use crate::index::area_nodes::{AreaView, KeyLog};
use crate::mem::{put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u64, rd_u8, PageHost, Region};
use crate::radix::{Radix, Units, HEIGHT_MAX};
use crate::tree::{node, IndexShape};

use super::buffer::{self, SLOT_DEAD};
use super::layout::{RowHead, Shape};
use super::page;
use super::root;
use super::row::{Item, RowRef, ROW_LONG};

pub const K_INDEX_TREE: u8 = 18;
pub const K_KEY_LOG: u8 = 19;

const RECORDS: u64 = 32;
const PER_BLOCK: u64 = 127;
const REC: u64 = 32;

// ---------------------------------------------------------------- frees at once

/// Frees a radix array whole: each present entry handed to `leaf`, then each radix block.
fn release_all<H, F>(h: &mut H, tbl: u64, r: Radix, leaf: F) -> Result<(), Fault>
where
    H: PageHost,
    F: FnMut(&mut H, u64) -> Result<(), Fault>,
{
    let mut c = 0u64;
    let mut u = Units { pages: u64::MAX, entries: u64::MAX, blocks: u64::MAX };
    match r.release_step(h, tbl, &mut c, &mut u, leaf)? {
        Progress::Done => Ok(()),
        Progress::More => Err(rt(Corruption::Count, r.root)),
    }
}

/// Frees a radix array listing blocks of area `area`: the blocks, then the array's own blocks.
pub fn free_list<H: PageHost>(h: &mut H, tbl: u64, r: Radix, area: u32) -> Result<(), Fault> {
    release_all(h, tbl, r, |h, b| free_block(h, tbl, area, b))
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
        Blocks::Radix(r) => release_all(h, tbl, r, |h, p| image_page_free(h, tbl, p)),
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

/// The layout of rows of schema version v.
fn head_of<H: PageHost + ?Sized>(h: &H, shape: &Shape, v: u32) -> Result<RowHead, Fault> {
    if v == shape.version {
        return Ok(shape.head);
    }
    let (mut r, mut fixed, mut vars) = (0u64, 0u64, 0u64);
    shape.spans(h, 0, shape.leaves, |_, s| {
        if s.at(v) {
            r = r.wrapping_add(1);
            fixed = fixed.wrapping_add(s.width);
            if s.variable {
                vars = vars.wrapping_add(1);
            }
        }
        Ok(true)
    })?;
    Ok(RowHead::of(v, r, fixed, vars))
}

/// Frees a sealed group's buffer descriptor: the long values its live rows still name (a row whose header says it
/// holds no long item is passed over without reading its items, and none is read when the descriptor says no row
/// of the group held a long value, `buffer::LONG_ROWS`), its buffer blocks, an extremes block it still names,
/// and the descriptor.
pub fn free_descriptor<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, d: u64) -> Result<(), Fault> {
    let blocks = buffer::u16_of(&*h, d, buffer::D_BLOCKS)?.min(BUFFER_BLOCKS_MAX);
    if buffer::u16_of(&*h, d, buffer::D_FLAGS)? & u64::from(buffer::LONG_ROWS) != 0 {
        // The layout of the last schema version other than the table's that a row with long items had.
        let mut head: Option<RowHead> = None;
        for k in 0..blocks {
            let b = buffer::u64_of(&*h, d, buffer::D_RECORDS.wrapping_add(k.wrapping_mul(16)))?;
            for s in 0..buffer::u16_of(&*h, b, buffer::B_SLOTS)? {
                let slot = buffer::read_slot(h.get(Region::Block(b))?, s)?;
                if slot.flags & SLOT_DEAD != 0 {
                    continue;
                }
                let loc = buffer::stub_row(h.get(Region::Block(b))?, b, &slot)?;
                let (v, flags) = RowRef::new(&*h, loc)?.header()?;
                if flags & ROW_LONG != 0 {
                    // The row's own version's items: a row written before a schema change has its own layout.
                    let hd = match head {
                        Some(x) if x.version == v => x,
                        _ => head_of(&*h, shape, v)?,
                    };
                    head = Some(hd);
                    for vi in 0..hd.vars {
                        // A long item whose reference a seal moved to its group's list reads as no item.
                        if let Item::Long { img, .. } = RowRef::new(&*h, loc)?.item_at(&hd, vi)? {
                            free_image(h, tbl, &img)?;
                        }
                    }
                }
                if let buffer::RowLoc::Long(img) = loc {
                    free_image(h, tbl, &img)?;
                }
            }
        }
    }
    for k in 0..blocks {
        let b = buffer::u64_of(&*h, d, buffer::D_RECORDS.wrapping_add(k.wrapping_mul(16)))?;
        free_block(h, tbl, 0, b)?;
    }
    let ext = buffer::u64_of(&*h, d, buffer::D_EXT)?;
    if ext != 0 {
        free_block(h, tbl, 0, ext)?;
    }
    free_block(h, tbl, 0, d)
}

// ---------------------------------------------------------------- the drain

/// One drain record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rec {
    pub kind: u8,
    pub height: u8,
    pub digits: [u16; 7],
    pub root: u64,
}

impl Rec {
    pub fn new(kind: u8, root: u64, height: u8) -> Rec {
        Rec { kind, height, digits: [0; 7], root }
    }

    /// The digits as a cursor: base 512, most significant first.
    fn cursor(&self) -> u64 {
        self.digits.iter().fold(0u64, |c, &d| (c << 9) | u64::from(d & 511))
    }

    fn set_cursor(&mut self, c: u64) {
        for (i, d) in self.digits.iter_mut().enumerate() {
            // i ≤ 6, so the shift is at most 54; the digit is below 512.
            #[allow(clippy::cast_possible_truncation)]
            let v = (c.wrapping_shr(9u32.wrapping_mul(6u32.wrapping_sub(i as u32))) & 511) as u16;
            *d = v;
        }
    }

    fn write(&self, b: &mut [u8], at: u64) -> Result<(), Fault> {
        crate::mem::fill(b, at, REC, 0)?;
        put_u8(b, at, self.kind)?;
        put_u8(b, at.wrapping_add(1), self.height)?;
        for (i, d) in self.digits.iter().enumerate() {
            put_u16(b, at.wrapping_add(4).wrapping_add((i as u64).wrapping_mul(2)), *d)?;
        }
        put_u64(b, at.wrapping_add(24), self.root)
    }

    fn read(b: &[u8], at: u64) -> Result<Rec, Fault> {
        let mut digits = [0u16; 7];
        for (i, d) in digits.iter_mut().enumerate() {
            *d = rd_u16(b, at.wrapping_add(4).wrapping_add((i as u64).wrapping_mul(2)))?;
        }
        Ok(Rec {
            kind: rd_u8(b, at)?,
            height: rd_u8(b, at.wrapping_add(1))?,
            digits,
            root: rd_u64(b, at.wrapping_add(24))?,
        })
    }
}

/// Pushes a record: one changed block, the top drain block (and the root when a new block starts).
pub fn push<H: PageHost>(h: &mut H, tbl: u64, r: &Rec) -> Result<(), Fault> {
    let mut top = root::u64_at(h, tbl, root::DRAIN_TOP)?;
    let n = if top == 0 { PER_BLOCK } else { u64::from(rd_u16(h.get(Region::Block(top))?, 6)?) };
    let n = if n >= PER_BLOCK {
        let b = alloc_zeroed(h, tbl, 0)?;
        {
            let s = h.get_mut(Region::Block(b))?;
            put_u32(s, 0, magic::NTDR)?;
            put_u16(s, 4, FORMAT_TABLE)?;
            put_u64(s, 8, top)?;
        }
        root::set_u64(h, tbl, root::DRAIN_TOP, b)?;
        top = b;
        0
    } else {
        n
    };
    let s = h.get_mut(Region::Block(top))?;
    r.write(s, RECORDS.wrapping_add(n.wrapping_mul(REC)))?;
    // n < 127.
    put_u16(s, 6, u16::try_from(n.wrapping_add(1)).map_err(|_| rt(Corruption::Count, n))?)
}

pub fn pending<H: PageHost + ?Sized>(h: &H, tbl: u64) -> Result<bool, Fault> {
    Ok(root::u64_at(h, tbl, root::DRAIN_TOP)? != 0)
}

/// The top record: its block, its offset there, and the record.
fn top<H: PageHost + ?Sized>(h: &H, tbl: u64) -> Result<Option<(u64, u64, Rec)>, Fault> {
    let b = root::u64_at(h, tbl, root::DRAIN_TOP)?;
    if b == 0 {
        return Ok(None);
    }
    let s = h.get(Region::Block(b))?;
    let n = u64::from(rd_u16(s, 6)?);
    if n == 0 || n > PER_BLOCK {
        return Err(rt(Corruption::Count, b));
    }
    let at = RECORDS.wrapping_add(n.wrapping_sub(1).wrapping_mul(REC));
    Ok(Some((b, at, Rec::read(s, at)?)))
}

/// Pops the top record; `true` when that emptied its drain block, which is freed. A record pushed later takes
/// the popped one's place.
fn pop<H: PageHost>(h: &mut H, tbl: u64, block: u64) -> Result<bool, Fault> {
    let (n, below) = {
        let s = h.get(Region::Block(block))?;
        (rd_u16(s, 6)?, rd_u64(s, 8)?)
    };
    let n = n.saturating_sub(1);
    if n == 0 {
        root::set_u64(h, tbl, root::DRAIN_TOP, below)?;
        free_block(h, tbl, 0, block)?;
        Ok(true)
    } else {
        put_u16(h.get_mut(Region::Block(block))?, 6, n)?;
        Ok(false)
    }
}

/// What one record's work did.
enum Done {
    /// The record's structure is freed: pop it.
    Pop,
    /// It ran out of its allowance: the step ends, the record keeps its new position.
    Stop(Rec),
}

/// One piece of the drain: the top record's work within `units`, spending from it. Returns `Done` when the
/// drain is empty, and whether the piece changed it (a record popped or moved on), so that a caller that runs
/// pieces while the allowance lasts stops when one changes nothing.
pub fn step<H: PageHost>(h: &mut H, tbl: u64, units: &mut Units) -> Result<(Progress, bool), Fault> {
    let Some((block, at, rec)) = top(h, tbl)? else { return Ok((Progress::Done, false)) };
    if units.entries == 0 || units.blocks < u64::from(HEIGHT_MAX) || units.pages == 0 {
        return Ok((Progress::More, false));
    }
    match work(h, tbl, rec, units)? {
        Done::Pop => {
            if pop(h, tbl, block)? {
                // The emptied drain block was freed, and the next one below is written from now on.
                units.blocks = units.blocks.saturating_sub(2);
            }
            Ok((if pending(&*h, tbl)? { Progress::More } else { Progress::Done }, true))
        }
        Done::Stop(r) => {
            let s = h.get_mut(Region::Block(block))?;
            r.write(s, at)?;
            Ok((Progress::More, r != rec))
        }
    }
}

fn radix_walk<H: PageHost, F>(h: &mut H, tbl: u64, mut rec: Rec, units: &mut Units, leaf: F) -> Result<Done, Fault>
where
    F: FnMut(&mut H, u64) -> Result<(), Fault>,
{
    let r = Radix { root: rec.root, height: rec.height };
    let mut c = rec.cursor();
    let p = r.release_step(h, tbl, &mut c, units, leaf)?;
    match p {
        Progress::Done => Ok(Done::Pop),
        Progress::More => {
            rec.set_cursor(c);
            Ok(Done::Stop(rec))
        }
    }
}

fn work<H: PageHost>(h: &mut H, tbl: u64, rec: Rec, units: &mut Units) -> Result<Done, Fault> {
    match rec.kind {
        K_KEY_LOG if rec.height == 0 => {
            // A log of one block: the radix array's root is the block itself.
            free_block(h, tbl, 1, rec.root)?;
            units.blocks = units.blocks.saturating_sub(1);
            Ok(Done::Pop)
        }
        K_KEY_LOG => block_list(h, tbl, rec, units, 1),
        K_INDEX_TREE => index_tree(h, tbl, rec, units),
        _ => Err(rt(Corruption::Encoding, u64::from(rec.kind))),
    }
}

/// A radix array listing blocks of area `area`, each freed with its walk. Each listed entry is a block, which
/// spends from the blocks allowance (the pages one is for pages): the walk gets as many entries as leave room
/// for its own radix blocks.
fn block_list<H: PageHost>(h: &mut H, tbl: u64, rec: Rec, units: &mut Units, area: u32) -> Result<Done, Fault> {
    let room = units.blocks.saturating_sub(u64::from(HEIGHT_MAX));
    let mut u = Units { pages: units.pages.min(room), entries: units.entries, blocks: units.blocks };
    let given = u.pages;
    let r = radix_walk(h, tbl, rec, &mut u, |h, b| free_block(h, tbl, area, b));
    units.entries = u.entries;
    units.blocks = u.blocks.saturating_sub(given.saturating_sub(u.pages));
    r
}

/// A removed secondary index's tree in area 1, freed from its right end: each step descends along the last
/// children to a node that has none left, frees it, and takes it off its parent (the parent's count lowered by
/// one, or, at count 0, its only child cleared). The tree that is left is the walk's position, so the record
/// keeps none. A step reads one node per level, each an entry of the allowance, and changes two blocks, a
/// parent counted once while the steps of one piece keep changing it.
fn index_tree<H: PageHost>(h: &mut H, tbl: u64, rec: Rec, units: &mut Units) -> Result<Done, Fault> {
    let shape = IndexShape::new(u64::from(rec.digits[0]), 0).ok_or(rt(Corruption::Layout, rec.root))?;
    let mut changed = 0u64;
    loop {
        if rec.root == 0 {
            return Ok(Done::Pop);
        }
        if units.blocks < 2 || units.entries == 0 {
            return Ok(Done::Stop(rec));
        }
        let (n, parent, read) = {
            let v = AreaView::new(&*h, shape, KeyLog::default());
            let (mut n, mut parent, mut read) = (rec.root, 0u64, 1u64);
            while !node::is_leaf(&v, n)? {
                let c = node::child(shape, &v, n, node::count(shape, &v, n)?)?;
                if c == 0 {
                    break;
                }
                if read > PATH_STACK {
                    return Err(rt(Corruption::Depth, rec.root));
                }
                (parent, n, read) = (n, c, read.wrapping_add(1));
            }
            (n, parent, read)
        };
        units.entries = units.entries.saturating_sub(read);
        free_block(h, tbl, 1, n)?;
        units.blocks = units.blocks.saturating_sub(1);
        if parent == 0 {
            return Ok(Done::Pop);
        }
        let c = node::count(shape, &AreaView::new(&*h, shape, KeyLog::default()), parent)?;
        let b = h.get_mut(Region::Block(parent))?;
        if c > 0 {
            put_u16(b, 0, u16::try_from(c.wrapping_sub(1)).map_err(|_| rt(Corruption::Count, parent))?)?;
        } else {
            put_u64(b, node::co(shape, 0), 0)?;
        }
        if parent != changed {
            units.blocks = units.blocks.saturating_sub(1);
            changed = parent;
        }
    }
}

const _: () = assert!(RECORDS + PER_BLOCK * REC == BLOCK);
