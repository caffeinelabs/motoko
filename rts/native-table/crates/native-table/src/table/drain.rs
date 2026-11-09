//! The drain: a stack of records naming structures to free, freed in bounded units by maintenance.
//!
//! **Drain block** (area 0, zero-filled when allocated): {magic `NTDR` u32 @0; `FORMAT_TABLE` u16 @4; records
//! u16 @6; block below u64 @8; zero u16 @16; 127 records of 32 bytes from @32}. A record is {kind u8 @0; height
//! u8 @1; depth u8 @2; zero u8 @3; digits [u16; 7] @4, the walk's position; zero @18; root u64 @24}. The table
//! root names the top block at offset 144.
//!
//! Kinds this table writes: 2 an image (its pages, then its radix blocks); 5 a group page (its image, its
//! long-value list as kind 16, its patch directories as kind 9, its extremes block as kind 13, the page); 6 a
//! buffer descriptor (the long values its live rows still name, its buffer blocks, an extremes block it still
//! names as kind 13, the descriptor; one such walk at a time, in the order the descriptors reach the drain); 7 a block chain (height 0: the next block at @0, a patch value chain;
//! height 2: at @4,088, a patch directory's blocks); 9 a patch directory (each unit's patch blocks as kind 9
//! of height 1, whose walk hands each live entry's chain or long value to the drain and frees the blocks;
//! then the directory's blocks as kind 7); 13 one block; 16 a long-value list (each entry's image, each list
//! block; height 1: the blocks alone, their entries owned elsewhere); 17 a rewrite's work space block list,
//! or a table's totals block list; 18 a removed secondary index's tree (its key width W in digit 0); 19 a
//! secondary index's key log (its blocks, then its radix blocks). A radix walk keeps its position in the
//! digits, base 512, most significant first; the other walks keep their phase and counters in the first
//! digits.

use crate::area::{alloc_zeroed, free_block};
use crate::consts::PATH_STACK;
use crate::consts::{magic, BLOCK, BUFFER_BLOCKS_MAX, FORMAT_TABLE};
use crate::fault::{rt, Corruption, Fault};
use crate::host::Progress;
use crate::image::{Blocks, Image};
use crate::index::area_nodes::{AreaView, KeyLog};
use crate::mem::{put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u64, rd_u8, PageHost, Region};
use crate::radix::{Radix, Units, HEIGHT_MAX};
use crate::tree::{node, IndexShape};

use super::buffer::{self, RowLoc, SLOT_DEAD};
use super::layout::{RowHead, Shape};
use super::page;
use super::root;
use super::row::{Item, RowRef, ROW_LONG};

pub const K_IMAGE: u8 = 2;
pub const K_GROUP_PAGE: u8 = 5;
pub const K_DESCRIPTOR: u8 = 6;
pub const K_CHAIN: u8 = 7;
pub const K_PATCH_DIR: u8 = 9;
pub const K_BLOCK: u8 = 13;
pub const K_LV_LIST: u8 = 16;
pub const K_BLOCK_LIST: u8 = 17;
pub const K_INDEX_TREE: u8 = 18;
pub const K_KEY_LOG: u8 = 19;

const RECORDS: u64 = 32;
/// The radix entries a descriptor walk spends per slot (reading its row's header costs about as much), per
/// item of a row's item table, and per piece of a row layout's walk over `HEAD_PIECE` leaves.
pub const ROW_ENTRIES: u64 = 8;
const ITEM_ENTRIES: u64 = 4;
const HEAD_ENTRIES: u64 = 24;
const HEAD_PIECE: u64 = 32;
const PER_BLOCK: u64 = 127;
const REC: u64 = 32;
/// A descriptor on the drain is read by nothing but its walk, so the walk keeps the row layout of one older
/// schema version in its spare bytes, built in pieces: {version u32 @3,904; leaves walked u32 @3,908; leaves R
/// u32 @3,912; fixed bytes u32 @3,916; variable leaves u32 @3,920}. Those bytes are zero until the walk first
/// writes them, which its record's digit 4 says (`HEAD_SET`).
const D_HEAD: u64 = buffer::D_SPARE;
const HEAD_SET: usize = 4;

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

    /// The record of an image, or `None` for an image with no block.
    pub fn image(img: &Image) -> Option<Rec> {
        match img.blocks {
            Blocks::Radix(r) if r.root != 0 => Some(Rec::new(K_IMAGE, r.root, r.height)),
            Blocks::Direct { count: 1, entries } => entries.first().map(|&e| Rec::new(K_IMAGE, e, 0)),
            _ => None,
        }
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

/// Descriptor walks run one at a time, in the order their descriptors reach the drain: a row updated while
/// its group was frozen may have moved to the tail block of a later group (a moved stub), so a group's buffer
/// blocks are freed only after the walks of the groups before it read their rows. Root job slot 7 holds the
/// descriptor whose walk is under way u64 @0 and the last one waiting u64 @8; the waiting descriptors are
/// chained through their next-frozen field (`buffer::D_NEXT`), which nothing else reads once a descriptor is
/// on the drain.
const WALKS: u64 = root::JOB_SLOTS + 7 * 32;
const WALKS_LAST: u64 = WALKS + 8;

/// Enters buffer descriptor `d` in the walks' order: `true` when its walk is now the one under way (its
/// record is the caller's to place), `false` when it waits after the last one. Changes `d`'s block, the last
/// waiting descriptor's block and the root.
fn enter_walk<H: PageHost>(h: &mut H, tbl: u64, d: u64) -> Result<bool, Fault> {
    put_u64(h.get_mut(Region::Block(d))?, buffer::D_NEXT, 0)?;
    if root::u64_at(&*h, tbl, WALKS)? == 0 {
        root::set_u64(h, tbl, WALKS, d)?;
        root::set_u64(h, tbl, WALKS_LAST, d)?;
        return Ok(true);
    }
    let last = root::u64_at(&*h, tbl, WALKS_LAST)?;
    put_u64(h.get_mut(Region::Block(last))?, buffer::D_NEXT, d)?;
    root::set_u64(h, tbl, WALKS_LAST, d)?;
    Ok(false)
}

/// Hands buffer descriptor `d` to the drain: its walk's record now, or its turn after the walks before it.
pub fn push_descriptor<H: PageHost>(h: &mut H, tbl: u64, d: u64) -> Result<(), Fault> {
    if enter_walk(h, tbl, d)? {
        push(h, tbl, &Rec::new(K_DESCRIPTOR, d, 0))?;
    }
    Ok(())
}

/// Pushes an image's record when it has a block.
pub fn push_image<H: PageHost>(h: &mut H, tbl: u64, img: &Image) -> Result<(), Fault> {
    match Rec::image(img) {
        Some(r) => push(h, tbl, &r),
        None => Ok(()),
    }
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
    /// It ran out of its allowance: the unit ends, the record keeps its new position.
    Stop(Rec),
    /// It saved its position and pushed a record above it: the unit ends.
    Pushed,
}

/// One piece of the drain: the top record's work within `units`, spending from it. Returns `Done` when the
/// drain is empty, and whether the piece changed it (a record popped, pushed or moved on), so that a caller
/// that gives it fresh allowances piece after piece stops when one changes nothing.
pub fn step<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, units: &mut Units) -> Result<(Progress, bool), Fault> {
    let Some((block, at, rec)) = top(h, tbl)? else { return Ok((Progress::Done, false)) };
    if units.entries == 0 || units.blocks < u64::from(HEIGHT_MAX) || units.pages == 0 {
        return Ok((Progress::More, false));
    }
    match work(h, tbl, shape, rec, units)? {
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
        Done::Pushed => Ok((Progress::More, true)),
    }
}

/// The most radix entries one step of a radix walk examines: each costs a few hundred instructions, and a
/// step stays a small piece between two readings of a host's meter.
const RADIX_PIECE: u64 = 12;

fn radix_walk<H: PageHost, F>(h: &mut H, tbl: u64, mut rec: Rec, units: &mut Units, leaf: F) -> Result<Done, Fault>
where
    F: FnMut(&mut H, u64) -> Result<(), Fault>,
{
    let r = Radix { root: rec.root, height: rec.height };
    let mut c = rec.cursor();
    let given = units.entries.min(RADIX_PIECE);
    let mut u = Units { entries: given, ..*units };
    let p = r.release_step(h, tbl, &mut c, &mut u, leaf)?;
    units.entries = units.entries.saturating_sub(given.saturating_sub(u.entries));
    units.pages = u.pages;
    units.blocks = u.blocks;
    match p {
        Progress::Done => Ok(Done::Pop),
        Progress::More => {
            rec.set_cursor(c);
            Ok(Done::Stop(rec))
        }
    }
}

fn work<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, rec: Rec, units: &mut Units) -> Result<Done, Fault> {
    match rec.kind {
        K_IMAGE => {
            if rec.height == 0 {
                crate::area::image_page_free(h, tbl, rec.root)?;
                units.pages = units.pages.saturating_sub(1);
                return Ok(Done::Pop);
            }
            radix_walk(h, tbl, rec, units, |h, p| crate::area::image_page_free(h, tbl, p))
        }
        K_BLOCK_LIST => block_list(h, tbl, rec, units, 0),
        K_KEY_LOG if rec.height == 0 => {
            // A log of one block: the radix array's root is the block itself.
            free_block(h, tbl, 1, rec.root)?;
            units.blocks = units.blocks.saturating_sub(1);
            Ok(Done::Pop)
        }
        K_KEY_LOG => block_list(h, tbl, rec, units, 1),
        K_INDEX_TREE => index_tree(h, tbl, rec, units),
        K_BLOCK => {
            free_block(h, tbl, 0, rec.root)?;
            units.blocks = units.blocks.saturating_sub(1);
            Ok(Done::Pop)
        }
        K_GROUP_PAGE => group_page(h, tbl, rec, units),
        K_DESCRIPTOR => descriptor(h, tbl, shape, rec, units),
        K_LV_LIST if rec.height == 1 => chain(h, tbl, rec, units, page::LV_NEXT),
        K_LV_LIST => lv_list(h, tbl, rec, units),
        K_CHAIN => chain(h, tbl, rec, units, chain_next(rec.height)),
        K_PATCH_DIR if rec.height == 1 => patch_blocks(h, tbl, rec, units),
        K_PATCH_DIR => patch_dir(h, tbl, rec, units),
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

fn group_page<H: PageHost>(h: &mut H, tbl: u64, mut rec: Rec, units: &mut Units) -> Result<Done, Fault> {
    let pg = rec.root;
    loop {
        match rec.digits[0] {
            0 => {
                rec.digits[0] = 1;
                let img = Image::read_record(h.get(Region::Block(pg))?, page::P_IMAGE)?;
                match img.blocks {
                    Blocks::Direct { count, entries } => {
                        for &e in entries.iter().take(usize::from(count)) {
                            if e != 0 {
                                crate::area::image_page_free(h, tbl, e)?;
                            }
                        }
                        units.pages = units.pages.saturating_sub(u64::from(count));
                    }
                    Blocks::Radix(r) => {
                        if r.root != 0 {
                            return push_child(h, tbl, rec, &Rec::new(K_IMAGE, r.root, r.height));
                        }
                    }
                }
            }
            1 => {
                rec.digits[0] = 2;
                let list = buffer::u64_of(h, pg, page::P_LV_LIST)?;
                if list != 0 {
                    return push_child(h, tbl, rec, &Rec::new(K_LV_LIST, list, 0));
                }
            }
            2 => {
                rec.digits[0] = 3;
                let dir = buffer::u64_of(h, pg, page::P_PATCH_DIR)?;
                if dir != 0 {
                    return push_child(h, tbl, rec, &Rec::new(K_PATCH_DIR, dir, 0));
                }
            }
            3 => {
                rec.digits[0] = 4;
                let dir = buffer::u64_of(h, pg, page::P_POST_PATCH)?;
                if dir != 0 {
                    return push_child(h, tbl, rec, &Rec::new(K_PATCH_DIR, dir, 0));
                }
            }
            4 => {
                rec.digits[0] = 5;
                let ext = buffer::u64_of(h, pg, page::P_EXT)?;
                if ext != 0 {
                    return push_child(h, tbl, rec, &Rec::new(K_BLOCK, ext, 0));
                }
            }
            _ => {
                free_block(h, tbl, 0, pg)?;
                units.blocks = units.blocks.saturating_sub(1);
                return Ok(Done::Pop);
            }
        }
    }
}

/// Saves `rec` as the top record, then pushes `child` above it; the unit ends.
fn push_child<H: PageHost>(h: &mut H, tbl: u64, rec: Rec, child: &Rec) -> Result<Done, Fault> {
    let top_now = root::u64_at(h, tbl, root::DRAIN_TOP)?;
    let at = {
        let s = h.get(Region::Block(top_now))?;
        RECORDS.wrapping_add(u64::from(rd_u16(s, 6)?).saturating_sub(1).wrapping_mul(REC))
    };
    rec.write(h.get_mut(Region::Block(top_now))?, at)?;
    push(h, tbl, child)?;
    Ok(Done::Pushed)
}

fn lv_list<H: PageHost>(h: &mut H, tbl: u64, mut rec: Rec, units: &mut Units) -> Result<Done, Fault> {
    loop {
        let b = rec.root;
        if b == 0 {
            return Ok(Done::Pop);
        }
        let (n, next) = {
            let s = h.get(Region::Block(b))?;
            (u64::from(crate::mem::rd_u32(s, page::LV_ENTRIES)?).min(page::LV_PER_BLOCK), rd_u64(s, page::LV_NEXT)?)
        };
        let mut i = u64::from(rec.digits[0]);
        while i < n {
            if units.entries == 0 {
                rec.digits[0] = u16::try_from(i).unwrap_or(0);
                return Ok(Done::Stop(rec));
            }
            units.entries = units.entries.saturating_sub(1);
            let at = page::LV_REFS.wrapping_add(i.wrapping_mul(crate::image::REF_BYTES));
            let s = h.get(Region::Block(b))?;
            let zero = crate::mem::sub(s, at, crate::image::REF_BYTES)?.iter().all(|&x| x == 0);
            i = i.wrapping_add(1);
            if !zero {
                let img = Image::read_ref(s, at)?;
                rec.digits[0] = u16::try_from(i).unwrap_or(0);
                if let Some(child) = Rec::image(&img) {
                    return push_child(h, tbl, rec, &child);
                }
            }
        }
        free_block(h, tbl, 0, b)?;
        units.blocks = units.blocks.saturating_sub(1);
        rec.root = next;
        rec.digits[0] = 0;
        if units.blocks < u64::from(HEIGHT_MAX) {
            return Ok(Done::Stop(rec));
        }
    }
}

/// The layout of rows of schema version v for a descriptor's walk: the table's own, or one built in the
/// descriptor's dead bytes (`D_HEAD`), a piece of `HEAD_PIECE` leaves per call; `None` when this call's piece
/// did not finish it.
fn desc_head<H: PageHost>(
    h: &mut H,
    shape: &Shape,
    rec: &mut Rec,
    v: u32,
    units: &mut Units,
) -> Result<Option<RowHead>, Fault> {
    if v == shape.version {
        return Ok(Some(shape.head));
    }
    let d = rec.root;
    let set = rec.digits.get(HEAD_SET).copied().unwrap_or(0) != 0;
    let (mut walked, mut r, mut fixed, mut vars) = {
        let b = h.get(Region::Block(d))?;
        if set && rd_u32(b, D_HEAD)? == v {
            let f = |o: u64| -> Result<u64, Fault> { Ok(u64::from(rd_u32(b, D_HEAD.wrapping_add(o))?)) };
            (f(4)?, f(8)?, f(12)?, f(16)?)
        } else {
            (0, 0, 0, 0)
        }
    };
    if walked < shape.leaves {
        if units.entries < HEAD_ENTRIES {
            return Ok(None);
        }
        units.entries = units.entries.saturating_sub(HEAD_ENTRIES);
        let to = walked.saturating_add(HEAD_PIECE).min(shape.leaves);
        shape.spans(&*h, walked, to, |_, s| {
            if s.at(v) {
                r = r.wrapping_add(1);
                fixed = fixed.wrapping_add(s.width);
                if s.variable {
                    vars = vars.wrapping_add(1);
                }
            }
            Ok(true)
        })?;
        walked = to;
        // The descriptor is a changed block of the unit: counted once per piece.
        units.blocks = units.blocks.saturating_sub(1);
        let w = |x: u64| u32::try_from(x).map_err(|_| rt(Corruption::Range, x));
        let b = h.get_mut(Region::Block(d))?;
        put_u32(b, D_HEAD, v)?;
        if let Some(s) = rec.digits.get_mut(HEAD_SET) {
            *s = 1;
        }
        put_u32(b, D_HEAD.wrapping_add(4), w(walked)?)?;
        put_u32(b, D_HEAD.wrapping_add(8), w(r)?)?;
        put_u32(b, D_HEAD.wrapping_add(12), w(fixed)?)?;
        put_u32(b, D_HEAD.wrapping_add(16), w(vars)?)?;
        if walked < shape.leaves {
            return Ok(None);
        }
    }
    Ok(Some(RowHead::of(v, r, fixed, vars)))
}

/// Phases of a descriptor's walk, in digit 0: 0 its rows' long values, 1 its buffer blocks, 2 an extremes
/// block it still names, 3 the descriptor. Digits 1, 2 and 3 hold the block record, the slot and the item, and
/// digit 4 whether the walk wrote a row layout (`HEAD_SET`). A row whose header says it has no long item is passed
/// over without reading its item table, and a frozen group whose descriptor says no row held a long value
/// (`buffer::LONG_ROWS`) skips the first phase.
fn descriptor<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, mut rec: Rec, units: &mut Units) -> Result<Done, Fault> {
    let d = rec.root;
    let blocks = buffer::u16_of(h, d, buffer::D_BLOCKS)?.min(BUFFER_BLOCKS_MAX);
    // A frozen group none of whose rows ever held a long value or was a long row image hands nothing on: its
    // rows are not read.
    let flags = buffer::u16_of(h, d, buffer::D_FLAGS)?;
    let rows = flags & u64::from(buffer::FROZEN) == 0 || flags & u64::from(buffer::LONG_ROWS) != 0;
    loop {
        match rec.digits[0] {
            0 if !rows => {
                rec.digits[0] = 1;
                rec.digits[1] = 0;
            }
            0 => {
                let k = u64::from(rec.digits[1]);
                if k >= blocks {
                    rec.digits[0] = 1;
                    rec.digits[1] = 0;
                    continue;
                }
                let b = buffer::u64_of(h, d, buffer::D_RECORDS.wrapping_add(k.wrapping_mul(16)))?;
                let slots = buffer::u16_of(h, b, buffer::B_SLOTS)?;
                let s = u64::from(rec.digits[2]);
                if s >= slots {
                    rec.digits[1] = rec.digits[1].wrapping_add(1);
                    rec.digits[2] = 0;
                    rec.digits[3] = 0;
                    continue;
                }
                if units.entries < ROW_ENTRIES {
                    return Ok(Done::Stop(rec));
                }
                units.entries = units.entries.saturating_sub(ROW_ENTRIES);
                let slot = buffer::read_slot(h.get(Region::Block(b))?, s)?;
                if slot.flags & SLOT_DEAD != 0 {
                    rec.digits[2] = rec.digits[2].wrapping_add(1);
                    rec.digits[3] = 0;
                    continue;
                }
                let loc = buffer::stub_row(h.get(Region::Block(b))?, b, &slot)?;
                // Long items first, from digit 3 on; then, for a long row image, the image itself.
                let (v, flags) = RowRef::new(&*h, loc)?.header()?;
                let mut child = None;
                if flags & ROW_LONG != 0 {
                    // The row's own version's items: a row written before a schema change has its own layout.
                    let Some(head) = desc_head(h, shape, &mut rec, v, units)? else { return Ok(Done::Stop(rec)) };
                    let row = RowRef::new(&*h, loc)?;
                    let mut vi = u64::from(rec.digits[3]);
                    while vi < head.vars {
                        if units.entries < ITEM_ENTRIES {
                            rec.digits[3] = u16::try_from(vi).unwrap_or(u16::MAX);
                            return Ok(Done::Stop(rec));
                        }
                        units.entries = units.entries.saturating_sub(ITEM_ENTRIES);
                        let it = row.item_at(&head, vi)?;
                        vi = vi.wrapping_add(1);
                        // A long item whose reference a seal moved to its group's list reads as no item.
                        if let Item::Long { img, .. } = it {
                            child = Rec::image(&img);
                            if child.is_some() {
                                break;
                            }
                        }
                    }
                    if let Some(c) = child {
                        rec.digits[3] = u16::try_from(vi).unwrap_or(u16::MAX);
                        return push_child(h, tbl, rec, &c);
                    }
                }
                rec.digits[2] = rec.digits[2].wrapping_add(1);
                rec.digits[3] = 0;
                if let RowLoc::Long(img) = loc {
                    if let Some(c) = Rec::image(&img) {
                        return push_child(h, tbl, rec, &c);
                    }
                }
            }
            1 => {
                let k = u64::from(rec.digits[1]);
                if k >= blocks {
                    rec.digits[0] = 2;
                    continue;
                }
                let b = buffer::u64_of(h, d, buffer::D_RECORDS.wrapping_add(k.wrapping_mul(16)))?;
                free_block(h, tbl, 0, b)?;
                units.blocks = units.blocks.saturating_sub(1);
                rec.digits[1] = rec.digits[1].wrapping_add(1);
                if units.blocks < u64::from(HEIGHT_MAX) {
                    return Ok(Done::Stop(rec));
                }
            }
            2 => {
                // An extremes block the descriptor still names (a seal's flip moves it to the group page).
                rec.digits[0] = 3;
                let ext = buffer::u64_of(h, d, buffer::D_EXT)?;
                if ext != 0 {
                    return push_child(h, tbl, rec, &Rec::new(K_BLOCK, ext, 0));
                }
            }
            _ => {
                // The next descriptor waiting takes this record's place.
                let ours = root::u64_at(&*h, tbl, WALKS)? == d;
                let next = if ours { buffer::u64_of(h, d, buffer::D_NEXT)? } else { 0 };
                free_block(h, tbl, 0, d)?;
                units.blocks = units.blocks.saturating_sub(1);
                if ours {
                    root::set_u64(h, tbl, WALKS, next)?;
                    if next != 0 {
                        return Ok(Done::Stop(Rec::new(K_DESCRIPTOR, next, 0)));
                    }
                    root::set_u64(h, tbl, WALKS_LAST, 0)?;
                }
                return Ok(Done::Pop);
            }
        }
    }
}

/// Where a chain's next block lies, by the record's height: 0 at @0 (patch value chains), 2 at @4,088 (patch
/// directories), 3 at @16 (compaction queue blocks).
fn chain_next(height: u8) -> u64 {
    match height {
        2 => 4_088,
        _ => 0,
    }
}

/// Frees a chain of blocks whose next block lies at `next_at`, as many per unit as the allowance lets.
fn chain<H: PageHost>(h: &mut H, tbl: u64, mut rec: Rec, units: &mut Units, next_at: u64) -> Result<Done, Fault> {
    loop {
        let b = rec.root;
        if b == 0 {
            return Ok(Done::Pop);
        }
        if units.blocks < u64::from(HEIGHT_MAX) {
            return Ok(Done::Stop(rec));
        }
        let next = rd_u64(h.get(Region::Block(b))?, next_at)?;
        free_block(h, tbl, 0, b)?;
        units.blocks = units.blocks.saturating_sub(1);
        rec.root = next;
    }
}

/// A patch directory: digit 0 counts the unit records handed on (each unit's patch block chain as a record of
/// its own); then the directory's blocks go as a chain.
fn patch_dir<H: PageHost>(h: &mut H, tbl: u64, mut rec: Rec, units: &mut Units) -> Result<Done, Fault> {
    let dir = rec.root;
    let total = super::patch::units(&*h, dir)?;
    let idx = u64::from(rec.digits[0]);
    if idx < total {
        let per = crate::consts::PATCH_UNITS_PER_DIR;
        let mut b = dir;
        for _ in 0..idx / per {
            b = rd_u64(h.get(Region::Block(b))?, super::patch::D_LINK.wrapping_add(8))?;
            if b == 0 {
                return Err(rt(Corruption::Link, dir));
            }
        }
        let at = super::patch::D_RECORDS.wrapping_add((idx % per).wrapping_mul(super::patch::D_REC));
        let first = rd_u64(h.get(Region::Block(b))?, at.wrapping_add(8))?;
        rec.digits[0] = u16::try_from(idx.wrapping_add(1)).map_err(|_| rt(Corruption::Count, idx))?;
        units.entries = units.entries.saturating_sub(1);
        if first != 0 {
            return push_child(h, tbl, rec, &Rec::new(K_PATCH_DIR, first, 1));
        }
        return Ok(Done::Stop(rec));
    }
    // The directory's blocks, chained at @4,088.
    Ok(Done::Stop(Rec::new(K_CHAIN, dir, 2)))
}

/// One unit's patch blocks: each live entry's value chain or long value to the drain (digit 0 the entry),
/// then the block, following the chain.
fn patch_blocks<H: PageHost>(h: &mut H, tbl: u64, mut rec: Rec, units: &mut Units) -> Result<Done, Fault> {
    use super::patch as pt;
    loop {
        let b = rec.root;
        if b == 0 {
            return Ok(Done::Pop);
        }
        let (n, next, first_block) = {
            let s = h.get(Region::Block(b))?;
            // A chain's first block holds the bitmap: its entries start after it. Digit 1 tells the walk
            // whether it is past the first block.
            (u64::from(rd_u16(s, pt::B_ENTRIES)?), rd_u64(s, pt::B_NEXT)?, rec.digits[1] == 0)
        };
        let base = if first_block { pt::FIRST_ENTRIES } else { pt::LATER_ENTRIES };
        let mut i = u64::from(rec.digits[0]);
        while i < n {
            if units.entries == 0 {
                rec.digits[0] = u16::try_from(i).unwrap_or(u16::MAX);
                return Ok(Done::Stop(rec));
            }
            units.entries = units.entries.saturating_sub(1);
            let at = base.wrapping_add(i.wrapping_mul(pt::ENTRY));
            i = i.wrapping_add(1);
            let (flags, data) = {
                let s = h.get(Region::Block(b))?;
                (crate::mem::rd_u8(s, at.wrapping_add(2))?, u64::from(rd_u16(s, at.wrapping_add(6))?))
            };
            if flags & pt::F_DEAD != 0 {
                continue;
            }
            let e = pt::Entry { block: b, at, pos: 0, flags, len: 0, data };
            let child = match pt::old_of(&*h, &e)? {
                pt::Old::None => None,
                pt::Old::Chain { head } => Some(Rec::new(K_CHAIN, head, 0)),
                pt::Old::Long { img } => Rec::image(&img),
            };
            if let Some(c) = child {
                rec.digits[0] = u16::try_from(i).unwrap_or(u16::MAX);
                return push_child(h, tbl, rec, &c);
            }
        }
        if units.blocks < u64::from(HEIGHT_MAX) {
            rec.digits[0] = u16::try_from(n).unwrap_or(u16::MAX);
            return Ok(Done::Stop(rec));
        }
        free_block(h, tbl, 0, b)?;
        units.blocks = units.blocks.saturating_sub(1);
        rec.root = next;
        rec.digits[0] = 0;
        rec.digits[1] = 1;
    }
}

const _: () = assert!(RECORDS + PER_BLOCK * REC == BLOCK);
