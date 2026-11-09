//! Radix arrays: trees of area-0 blocks of 512 u64 references.
//!
//! Height d covers 512^d entries; entry i is found by i's digits in base 512, most significant first. At
//! height 0 the root is entry 0 itself, as for an image reference of one block. Growth adds a root
//! above the old one, which becomes its entry 0, in O(1). Every radix block is zero-filled when it is
//! allocated, so an entry no write has set reads as absent: entry value 0 means absent. Values are
//! opaque to the array: a directory entry may carry bit 63 as its own flag.
//!
//! Uses: the group directory, image block lists, area page lists, the block lists of rewrite work spaces and
//! the totals space, and key-log block lists. A walk examines a block's 512 entries in order and skips an
//! absent entry at any level at once.

use crate::area::{alloc_zeroed, free_block};
use crate::consts::RADIX_FAN;
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::mem::{put_u64, rd_u64, PageHost, Region};

/// The greatest height a radix array takes: 512^7 = 2^63 entries, more than any count the formats name
/// (`GROUP_MAX` < 2^49 groups, `IMAGE_MAX` / `PAGE` = 2^44 image blocks). The format names no maximum;
/// this bound keeps every capacity and position inside a u64.
pub const HEIGHT_MAX: u8 = 7;

/// Base-2 logarithm of `RADIX_FAN`.
const BITS: u32 = 9;
/// The low digit mask.
const DIGIT: u64 = RADIX_FAN - 1;
/// Positions of the path stack, one per level below the height.
const LEVELS: usize = HEIGHT_MAX as usize;

const _: () = assert!(RADIX_FAN == 1 << BITS);

/// The base-512 digit of `i` at `level` (0 is the leaf level); `level` < `HEIGHT_MAX`.
#[inline]
fn digit(i: u64, level: u8) -> u64 {
    // level ≤ 6, so the shift is at most 54.
    i.wrapping_shr(BITS.wrapping_mul(u32::from(level))) & DIGIT
}

/// The byte offset of entry d in a radix block; d < 512, so at most 4,088.
#[inline]
fn slot(d: u64) -> u64 {
    // d ≤ 511.
    d.wrapping_mul(8)
}

/// 512^level, for level ≤ `HEIGHT_MAX`: at most 2^63.
#[inline]
fn span(level: u8) -> u64 {
    // level ≤ 7, so the shift is at most 63.
    1u64.wrapping_shl(BITS.wrapping_mul(u32::from(level)))
}

/// Whether every byte of `b` is 0, read 8 bytes at a time.
fn zeroed(b: &[u8]) -> bool {
    let words = b.chunks_exact(8);
    let rest = words.remainder();
    words.into_iter().all(|w| <[u8; 8]>::try_from(w).is_ok_and(|x| u64::from_le_bytes(x) == 0))
        && rest.iter().all(|&x| x == 0)
}

/// The root and height of one radix array, as its owner stores them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Radix {
    /// At height 0, entry 0 itself; else the root block. 0: empty.
    pub root: u64,
    pub height: u8,
}

impl Radix {
    /// The empty array.
    pub const EMPTY: Radix = Radix { root: 0, height: 0 };

    /// `Corrupt(Runtime, Depth)` when the stored height passes `HEIGHT_MAX`.
    #[inline]
    pub fn check(self) -> Result<(), Fault> {
        if self.height > HEIGHT_MAX {
            return Err(corrupt(ImageId::Runtime, Corruption::Depth, self.root));
        }
        Ok(())
    }

    /// 512^height: the entries the array covers. Call after `check`.
    #[inline]
    pub fn capacity(self) -> u64 {
        span(self.height.min(HEIGHT_MAX))
    }

    /// Entry i; 0 when absent, past the capacity included. Costs one block read per level.
    pub fn get<H: PageHost + ?Sized>(self, h: &H, i: u64) -> Result<u64, Fault> {
        self.check()?;
        if i >= self.capacity() {
            return Ok(0);
        }
        let mut node = self.root;
        let mut level = self.height;
        while level > 0 {
            if node == 0 {
                return Ok(0);
            }
            // level > 0.
            level = level.wrapping_sub(1);
            node = rd_u64(h.get(Region::Block(node))?, slot(digit(i, level)))?;
        }
        Ok(node)
    }

    /// Sets entry i to v, growing the array and creating the missing blocks on the path, each taken from area
    /// 0 of the table whose root block is `tbl` and zero-filled: at most one new block per level, and one new
    /// root per added level. v = 0 clears the entry (`clear`).
    pub fn set<H: PageHost>(&mut self, h: &mut H, tbl: u64, i: u64, v: u64) -> Result<(), Fault> {
        if v == 0 {
            return self.clear(h, tbl, i);
        }
        self.check()?;
        while i >= self.capacity() {
            if self.height >= HEIGHT_MAX {
                return Err(corrupt(ImageId::Runtime, Corruption::Range, i));
            }
            if self.root != 0 {
                let b = alloc_zeroed(h, tbl, 0)?;
                put_u64(h.get_mut(Region::Block(b))?, 0, self.root)?;
                self.root = b;
            }
            // height < HEIGHT_MAX.
            self.height = self.height.wrapping_add(1);
        }
        if self.height == 0 {
            self.root = v;
            return Ok(());
        }
        if self.root == 0 {
            self.root = alloc_zeroed(h, tbl, 0)?;
        }
        let mut node = self.root;
        let mut level = self.height;
        loop {
            // level ≥ 1 on entry: the loop returns at level 0.
            level = level.wrapping_sub(1);
            let at = slot(digit(i, level));
            if level == 0 {
                return put_u64(h.get_mut(Region::Block(node))?, at, v);
            }
            let mut child = rd_u64(h.get(Region::Block(node))?, at)?;
            if child == 0 {
                child = alloc_zeroed(h, tbl, 0)?;
                put_u64(h.get_mut(Region::Block(node))?, at, child)?;
            }
            node = child;
        }
    }

    /// Clears entry i. Then, from the leaf up, a block whose 512 entries all read 0 is freed with
    /// `free_block(0, ·)` and its entry in the parent cleared, up to the root: at most height × 512 word
    /// reads (`zeroed`). An emptied root block is freed too and the root becomes 0; the height is kept.
    pub fn clear<H: PageHost>(&mut self, h: &mut H, tbl: u64, i: u64) -> Result<(), Fault> {
        self.check()?;
        if self.root == 0 || i >= self.capacity() {
            return Ok(());
        }
        if self.height == 0 {
            self.root = 0;
            return Ok(());
        }
        let mut path = [0u64; LEVELS];
        let mut node = self.root;
        let mut level = self.height;
        loop {
            // level ≥ 1 on entry: the loop breaks at level 0.
            level = level.wrapping_sub(1);
            *path.get_mut(usize::from(level)).ok_or(corrupt(ImageId::Runtime, Corruption::Depth, node))? = node;
            if level == 0 {
                break;
            }
            node = rd_u64(h.get(Region::Block(node))?, slot(digit(i, level)))?;
            if node == 0 {
                return Ok(());
            }
        }
        let mut level = 0u8;
        let mut block = node;
        put_u64(h.get_mut(Region::Block(block))?, slot(digit(i, 0)), 0)?;
        loop {
            if !zeroed(h.get(Region::Block(block))?) {
                return Ok(());
            }
            free_block(h, tbl, 0, block)?;
            // level < height ≤ HEIGHT_MAX.
            level = level.wrapping_add(1);
            if level >= self.height {
                self.root = 0;
                return Ok(());
            }
            block = path.get(usize::from(level)).copied().ok_or(corrupt(ImageId::Runtime, Corruption::Depth, block))?;
            put_u64(h.get_mut(Region::Block(block))?, slot(digit(i, level)), 0)?;
        }
    }

    /// Frees the whole array: each present leaf entry is handed to `leaf` (which frees what it names) and each
    /// radix block is freed with `free_block(0, ·)` once every entry below it is done, so a block's entries go
    /// before the block.
    pub fn release<H, F>(self, h: &mut H, tbl: u64, mut leaf: F) -> Result<(), Fault>
    where
        H: PageHost,
        F: FnMut(&mut H, u64) -> Result<(), Fault>,
    {
        let mut c = Cursor::new(self)?;
        while !c.done {
            if let Event::Leaf { value } = c.advance(h)? {
                if value != 0 {
                    leaf(h, value)?;
                }
            }
            for &b in c.finished.get(..usize::from(c.n_finished)).unwrap_or(&[]) {
                free_block(h, tbl, 0, b)?;
            }
        }
        Ok(())
    }
}

/// What one examination met.
enum Event {
    /// A leaf entry, present or absent (value 0).
    Leaf { value: u64 },
    /// An inner entry: present (the walk descends) or absent (the walk skips everything below it).
    Inner,
}

/// A depth-first position in a radix array: `pos` is the next index, `level` the level of the next
/// examination and `stack[l]` the block of level l that holds `pos`, for l from `level` up to the root.
struct Cursor {
    root: u64,
    height: u8,
    pos: u64,
    level: u8,
    stack: [u64; LEVELS],
    done: bool,
    /// The blocks the last examination finished, from its level up.
    finished: [u64; LEVELS],
    n_finished: u8,
}

impl Cursor {
    fn new(r: Radix) -> Result<Cursor, Fault> {
        r.check()?;
        let level = r.height.saturating_sub(1);
        let mut stack = [0u64; LEVELS];
        if r.height > 0 {
            *stack.get_mut(usize::from(level)).ok_or(corrupt(ImageId::Runtime, Corruption::Depth, r.root))? = r.root;
        }
        Ok(Cursor {
            root: r.root,
            height: r.height,
            pos: 0,
            level,
            stack,
            done: r.root == 0,
            finished: [0; LEVELS],
            n_finished: 0,
        })
    }

    /// Examines one entry and moves past it; the walk is not done.
    fn advance<H: PageHost>(&mut self, h: &H) -> Result<Event, Fault> {
        self.n_finished = 0;
        if self.height == 0 {
            self.done = true;
            self.pos = 1;
            return Ok(Event::Leaf { value: self.root });
        }
        let level = self.level;
        let bad = corrupt(ImageId::Runtime, Corruption::Depth, self.root);
        let block = self.stack.get(usize::from(level)).copied().ok_or(bad)?;
        let value = rd_u64(h.get(Region::Block(block))?, slot(digit(self.pos, level)))?;
        if level > 0 && value != 0 {
            // level > 0.
            let below = level.wrapping_sub(1);
            *self.stack.get_mut(usize::from(below)).ok_or(bad)? = value;
            self.level = below;
            return Ok(Event::Inner);
        }
        // Past the examined entry: the next multiple of 512^level. pos < capacity ≤ 2^63, so the sum is at
        // most the capacity.
        let step = span(level);
        let next = (self.pos & !step.wrapping_sub(1)).checked_add(step).ok_or(bad)?;
        self.pos = next;
        // The block of level l is finished when the move wrapped its digit to 0; then its parent's digit moved.
        let mut l = level;
        loop {
            if digit(next, l) != 0 {
                break;
            }
            let b = self.stack.get(usize::from(l)).copied().ok_or(bad)?;
            *self.finished.get_mut(usize::from(self.n_finished)).ok_or(bad)? = b;
            // n_finished ≤ l − level < HEIGHT_MAX.
            self.n_finished = self.n_finished.wrapping_add(1);
            // l < height ≤ HEIGHT_MAX.
            l = l.wrapping_add(1);
            if l >= self.height {
                self.done = true;
                break;
            }
        }
        self.level = l;
        Ok(if level == 0 { Event::Leaf { value } } else { Event::Inner })
    }
}
