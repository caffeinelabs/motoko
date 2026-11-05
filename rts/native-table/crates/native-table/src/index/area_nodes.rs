//! Index trees in table memory: every node is one 4 KiB block of an area, reached through the page host,
//! and keys longer than the tree's width W are kept whole in the tree's key log.
//!
//! **Records.** Each entry whose key is longer than W has a record in the key log: {payload u64, a length
//! word u64 holding the full key's length, the key's bytes beyond W}; the entry's key area holds the first W
//! bytes, so no byte is stored twice. A row entry's payload is its rid, a count entry's its count, rewritten
//! in place. A record's locator is (its log block's ordinal << 12) | its offset. Removing an entry hands its
//! record back, so the log holds the records of the entries the tree holds, and no more.
//!
//! **Blocks.** The log's blocks are listed by ordinal in a radix array. Ordinal 0 is its head block: {magic
//! `NTKL` u32 @0; zero to 8; the next ordinal u64 @8; a bit per size class whose free list is not empty, u64
//! @16; the first free chunk of each of the 64 classes u64 @24 + 8c (a locator, 0: none)}. Every other block
//! is a run of chunks from offset 0 to its end. A record of up to 4,096 bytes takes a chunk of exactly its bytes
//! (32 at least), cut from the front of a free chunk, whose rest stays free when it is 32 bytes or more (else
//! the record takes the whole chunk). A record's length word holds the key's length in bits 0–47, its chunk's
//! bytes in bits 48–60 and, in bit 63, whether the chunk before it is free. A free chunk is {`FREE` | its
//! bytes u64 @0; the next free chunk of its class u64 @8; the one before it u64 @16; …; its bytes u64 in its
//! last 8}; a record's payload is below 2^62, so a record never reads as free. Size class c of a chunk of s
//! bytes is 8 · (⌊log2 s⌋ − 5) + the three bits of s after its highest: a record takes the first free chunk
//! of its class when that is large enough, else the first of the smallest class above it, else a fresh block.
//! A freed chunk joins the free chunks beside it, a block wholly free again is freed, and the head goes once
//! no other block is left. A longer record takes fresh blocks of consecutive ordinals of its own, from offset
//! 0, its key bytes running on through the blocks after the first (its length word holds the key's length
//! alone), and frees them whole. Every fresh block takes the next ordinal; a freed one's ordinal is not used
//! again.
//!
//! **Keys written over units.** A key whose length is not known yet, too long to write in one unit, is
//! written a piece at a time (`open_key`, `put_key`, `close_key`): a state block of a fresh ordinal holds its
//! first W bytes and the writer's position, the ordinals after it are held for its record, whose blocks are
//! made as its bytes reach them, and the record is made when the key ends. The ordinals held beyond the
//! blocks made are never used. Once the key's row is in or gone, the state block hands back a block a step
//! whatever no entry took, itself last (`give_back`).

use crate::area::{alloc_block, alloc_zeroed, free_block};
use crate::consts::{magic, BLOCK, FLAG_INDEX, FLAG_LEAF, W_MAX};
use crate::fault::{bounds, corrupt, Corruption, Fault, ImageId};
use crate::host::NodeMem;
use crate::key::probe::{payload, IndexPending, ResolveKey, PART};
use crate::mem::{
    copy, copy_bytes, put, put_u16, put_u32, put_u64, rd_u16, rd_u64, shift, sub, sub_mut, Owner, PageHost, Region,
};
use crate::radix::Radix;
use crate::tree::IndexShape;

/// Bytes of a record's head: its payload and its length word.
const REC_HEAD: u64 = 16;
/// Bytes of the smallest chunk: room for a free chunk's three words and its last.
const CHUNK_MIN: u64 = 32;
/// A free chunk's first word, its bytes in the low bits.
const FREE: u64 = 1 << 63;
/// A record's length word: the key's length, its chunk's bytes, and whether the chunk before is free.
const LEN_MASK: u64 = (1 << 48) - 1;
const SIZE_SHIFT: u32 = 48;
const SIZE_MASK: u64 = 0x1FFF;
const PREV_FREE: u64 = 1 << 63;
/// Head block offsets: the next ordinal, the classes' bits, their free lists.
const H_NEXT: u64 = 8;
const H_BITS: u64 = 16;
const H_FREE: u64 = 24;
/// Ordinals stay below 2^49, so every locator fits an entry field's payload.
const ORDINAL_END: u64 = 1 << 49;
/// A key written over units: its state block's offsets (the record's blocks made, the ordinals held, the
/// caller's words, the key's first W bytes, the first ordinal of its second record and that record's blocks
/// made).
const KS_MADE: u64 = 0;
const KS_HELD: u64 = 8;
const KS_WORDS: u64 = 16;
const KS_HEAD: u64 = 64;
const KS_TWIN: u64 = KS_HEAD + W_MAX;
const KS_TWIN_MADE: u64 = KS_TWIN + 8;
/// The caller's words 6 to 16, after the second record's.
const KS_MORE: u64 = KS_TWIN_MADE + 8;
/// The locator of the key's record once moved into a chunk ahead of `close_key` (`move_key`), 0 before.
const KS_CHUNK: u64 = KS_MORE + 11 * 8;

/// A key log's state, kept by its owner (an index descriptor).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyLog {
    pub radix: Radix,
    /// Log blocks held, the head included (0: an empty log, with no block).
    pub blocks: u64,
}

fn rt(at: u64, kind: Corruption) -> Fault {
    corrupt(ImageId::Runtime, kind, at)
}

/// The size class of a chunk of `s` bytes, 32 ≤ s ≤ 4,096: eight classes per power of two, 0 to 56.
fn class(s: u64) -> u64 {
    // ⌊log2 s⌋ ≥ 5.
    let fl = 63u64.wrapping_sub(u64::from(s.max(CHUNK_MIN).leading_zeros()));
    let sl = s.wrapping_shr(u32::try_from(fl.wrapping_sub(3)).unwrap_or(0)) & 7;
    fl.wrapping_sub(5).wrapping_mul(8).wrapping_add(sl)
}

/// The key's length a record's length word holds.
fn key_len(word: u64) -> u64 {
    word & LEN_MASK
}

/// The nodes of one index tree: blocks of area `area` of table `tbl`, of shape `shape`, with key log `log`.
pub struct AreaNodes<'h, H: PageHost> {
    pub host: &'h mut H,
    pub tbl: u64,
    pub area: u32,
    pub shape: IndexShape,
    pub log: KeyLog,
    /// The tree's node count, kept by its owner: raised by every node allocated, lowered by every node
    /// handed back.
    pub nodes: u64,
}

impl<'h, H: PageHost> AreaNodes<'h, H> {
    pub fn new(host: &'h mut H, tbl: u64, area: u32, shape: IndexShape, log: KeyLog) -> Self {
        AreaNodes { host, tbl, area, shape, log, nodes: 0 }
    }

    fn block(&self, n: u64) -> Result<&[u8], Fault> {
        self.host.get(Region::Block(n))
    }

    fn block_mut(&mut self, n: u64) -> Result<&mut [u8], Fault> {
        self.host.get_mut(Region::Block(n), Owner(self.area))
    }

    /// The log block of ordinal k.
    fn log_block(&self, k: u64) -> Result<u64, Fault> {
        log_block(&*self.host, self.log, k)
    }

    /// The block of the chunk at locator `loc`, and the chunk's offset in it.
    fn place(&self, loc: u64) -> Result<(u64, u64), Fault> {
        Ok((self.log_block(loc >> 12)?, loc & 0xFFF))
    }

    /// The words at bytes `off` and `off` + 8 of the chunk at locator `loc`.
    fn words(&self, loc: u64, off: u64) -> Result<(u64, u64), Fault> {
        let (b, at) = self.place(loc)?;
        let s = self.block(b)?;
        let at = at.wrapping_add(off);
        Ok((rd_u64(s, at)?, rd_u64(s, at.wrapping_add(8))?))
    }

    /// Sets the word at byte `off` of the chunk at locator `loc`.
    fn set_word(&mut self, loc: u64, off: u64, v: u64) -> Result<(), Fault> {
        let (b, at) = self.place(loc)?;
        put_u64(self.block_mut(b)?, at.wrapping_add(off), v)
    }

    /// Sets (`free`) or clears the flag of the record at `loc` that the chunk before it is free.
    fn set_prev_free(&mut self, loc: u64, free: bool) -> Result<(), Fault> {
        let (b, at) = self.place(loc)?;
        let s = self.block_mut(b)?;
        let at = at.wrapping_add(8);
        let w = rd_u64(s, at)?;
        put_u64(s, at, if free { w | PREV_FREE } else { w & !PREV_FREE })
    }

    /// The first free chunk of class c.
    fn first_free(&self, head: u64, c: u64) -> Result<u64, Fault> {
        rd_u64(self.block(head)?, H_FREE.wrapping_add(c.wrapping_mul(8)))
    }

    /// Sets the first free chunk of class c, and the class's bit.
    fn set_first_free(&mut self, head: u64, c: u64, loc: u64) -> Result<(), Fault> {
        let s = self.block_mut(head)?;
        put_u64(s, H_FREE.wrapping_add(c.wrapping_mul(8)), loc)?;
        let bit = 1u64.wrapping_shl(u32::try_from(c).unwrap_or(0));
        let bits = rd_u64(s, H_BITS)?;
        put_u64(s, H_BITS, if loc == 0 { bits & !bit } else { bits | bit })
    }

    /// The head block, made with the log's first record.
    fn head(&mut self) -> Result<u64, Fault> {
        if self.log.blocks > 0 {
            return self.log_block(0);
        }
        let b = alloc_zeroed(&mut *self.host, self.tbl, self.area)?;
        {
            let s = self.block_mut(b)?;
            put_u32(s, 0, magic::NTKL)?;
            put_u64(s, H_NEXT, 1)?;
        }
        let mut radix = Radix::EMPTY;
        radix.set(&mut *self.host, self.tbl, 0, b)?;
        self.log = KeyLog { radix, blocks: 1 };
        Ok(b)
    }

    /// `n` fresh log blocks of consecutive ordinals; returns the first ordinal.
    fn fresh(&mut self, head: u64, n: u64) -> Result<u64, Fault> {
        let k = rd_u64(self.block(head)?, H_NEXT)?;
        let end = k.checked_add(n).filter(|&e| e <= ORDINAL_END).ok_or(rt(k, Corruption::Range))?;
        let mut radix = self.log.radix;
        let mut i = k;
        while i < end {
            let b = alloc_block(&mut *self.host, self.tbl, self.area)?;
            radix.set(&mut *self.host, self.tbl, i, b)?;
            i = i.wrapping_add(1);
        }
        self.log.radix = radix;
        self.log.blocks = self.log.blocks.checked_add(n).ok_or(rt(k, Corruption::Count))?;
        put_u64(self.block_mut(head)?, H_NEXT, end)?;
        Ok(k)
    }

    /// Frees the log block of ordinal k.
    fn free_ordinal(&mut self, k: u64) -> Result<(), Fault> {
        let b = self.log_block(k)?;
        free_block(&mut *self.host, self.tbl, self.area, b)?;
        let mut radix = self.log.radix;
        radix.clear(&mut *self.host, self.tbl, k)?;
        self.log.radix = radix;
        self.log.blocks = self.log.blocks.checked_sub(1).ok_or(rt(k, Corruption::Count))?;
        Ok(())
    }

    /// Makes the chunk at `loc`, of `s` bytes, free: its words, and first on its class's list.
    fn push(&mut self, head: u64, loc: u64, s: u64) -> Result<(), Fault> {
        let c = class(s);
        let first = self.first_free(head, c)?;
        {
            let (b, at) = self.place(loc)?;
            let blk = self.block_mut(b)?;
            put_u64(blk, at, FREE | s)?;
            put_u64(blk, at.wrapping_add(8), first)?;
            put_u64(blk, at.wrapping_add(16), 0)?;
            put_u64(blk, at.wrapping_add(s).wrapping_sub(8), s)?;
        }
        if first != 0 {
            self.set_word(first, 16, loc)?;
        }
        self.set_first_free(head, c, loc)
    }

    /// Takes the free chunk at `loc`, of `s` bytes, off its class's list.
    fn unlink(&mut self, head: u64, loc: u64, s: u64) -> Result<(), Fault> {
        let (next, prev) = self.words(loc, 8)?;
        if prev == 0 {
            self.set_first_free(head, class(s), next)?;
        } else {
            self.set_word(prev, 8, next)?;
        }
        if next != 0 {
            self.set_word(next, 16, prev)?;
        }
        Ok(())
    }

    /// The bytes of the free chunk at `loc`, checked against its block.
    fn free_size(&self, loc: u64) -> Result<u64, Fault> {
        let s = self.words(loc, 0)?.0 & SIZE_MASK;
        if s < CHUNK_MIN || (loc & 0xFFF).wrapping_add(s) > BLOCK {
            return Err(rt(loc, Corruption::Bounds));
        }
        Ok(s)
    }

    /// A chunk of at least `n` bytes, 32 ≤ n ≤ 4,096, cut from the front of a free chunk (the first of n's class
    /// when large enough, else the first of the smallest class above, else a fresh block), the rest left free
    /// when it is 32 bytes or more. Returns its locator and its bytes.
    fn take_chunk(&mut self, head: u64, n: u64) -> Result<(u64, u64), Fault> {
        let c = class(n);
        let mut loc = self.first_free(head, c)?;
        if loc != 0 && self.free_size(loc)? < n {
            loc = 0;
        }
        if loc == 0 {
            let bits = rd_u64(self.block(head)?, H_BITS)?
                & u64::MAX.wrapping_shl(u32::try_from(c).unwrap_or(0)).wrapping_shl(1);
            if bits != 0 {
                loc = self.first_free(head, u64::from(bits.trailing_zeros()))?;
            }
        }
        let s = if loc == 0 {
            loc = self.fresh(head, 1)? << 12;
            BLOCK
        } else {
            let s = self.free_size(loc)?;
            self.unlink(head, loc, s)?;
            s
        };
        if s.wrapping_sub(n) >= CHUNK_MIN {
            self.push(head, loc.wrapping_add(n), s.wrapping_sub(n))?;
            return Ok((loc, n));
        }
        // The whole chunk: the record after it no longer follows a free chunk.
        if (loc & 0xFFF).wrapping_add(s) < BLOCK {
            self.set_prev_free(loc.wrapping_add(s), false)?;
        }
        Ok((loc, s))
    }

    /// Makes a record {payload, key length, key[W..]} of `key`, longer than W; returns its locator.
    pub fn new_record(&mut self, payload: u64, key: &[u8]) -> Result<u64, Fault> {
        let w = usize::try_from(self.shape.w).map_err(|_| bounds(0))?;
        let tail = key.get(w..).filter(|t| !t.is_empty()).ok_or(bounds(0))?;
        if payload & FREE != 0 || key.len() as u64 > LEN_MASK {
            return Err(rt(payload, Corruption::Range));
        }
        let size = REC_HEAD.checked_add(tail.len() as u64).ok_or(bounds(0))?;
        let head = self.head()?;
        let (loc, word) = if size > BLOCK {
            (self.fresh(head, size.div_ceil(BLOCK))? << 12, key.len() as u64)
        } else {
            let (loc, s) = self.take_chunk(head, size.max(CHUNK_MIN))?;
            (loc, key.len() as u64 | s << SIZE_SHIFT)
        };
        // The head, then the key's bytes beyond W: to the first block's end, then on through the blocks after.
        let (mut b, mut off) = self.place(loc)?;
        {
            let s = self.block_mut(b)?;
            put_u64(s, off, payload)?;
            put_u64(s, off.wrapping_add(8), word)?;
        }
        off = off.wrapping_add(REC_HEAD);
        let mut k = loc >> 12;
        let mut rest = tail;
        loop {
            let room = usize::try_from(BLOCK.saturating_sub(off)).map_err(|_| bounds(off))?;
            let (now, later) = rest.split_at_checked(room.min(rest.len())).ok_or(bounds(off))?;
            put(self.block_mut(b)?, off, now)?;
            rest = later;
            if rest.is_empty() {
                return Ok(loc);
            }
            k = k.wrapping_add(1);
            (b, off) = (self.log_block(k)?, 0);
        }
    }

    /// Makes a record {payload, key length, …} for a key of `len` bytes, longer than W, whose bytes beyond W the
    /// caller writes next (`write_key`, `copy_record`); returns its locator. A key too long for a stack buffer
    /// reaches the log this way, in pieces.
    pub fn alloc_record(&mut self, payload: u64, len: u64) -> Result<u64, Fault> {
        let w = self.shape.w;
        if len <= w || payload & FREE != 0 || len > LEN_MASK {
            return Err(rt(payload, Corruption::Range));
        }
        // len > W.
        let size = REC_HEAD.checked_add(len.wrapping_sub(w)).ok_or(bounds(0))?;
        let head = self.head()?;
        let (loc, word) = if size > BLOCK {
            (self.fresh(head, size.div_ceil(BLOCK))? << 12, len)
        } else {
            let (loc, s) = self.take_chunk(head, size.max(CHUNK_MIN))?;
            (loc, len | s << SIZE_SHIFT)
        };
        let (b, off) = self.place(loc)?;
        let s = self.block_mut(b)?;
        put_u64(s, off, payload)?;
        put_u64(s, off.wrapping_add(8), word)?;
        Ok(loc)
    }

    /// Writes `bytes` as bytes [at, at + bytes.len()) of the key of `len` bytes whose record is at `loc`, W ≤ at,
    /// ending at or before `len`.
    pub fn write_key(&mut self, loc: u64, len: u64, at: u64, bytes: &[u8]) -> Result<(), Fault> {
        let w = self.shape.w;
        if at < w || at.checked_add(bytes.len() as u64).is_none_or(|e| e > len) {
            return Err(bounds(at));
        }
        let (mut at, mut rest) = (at, bytes);
        while !rest.is_empty() {
            let (k, off) = byte_place(loc, w, at);
            let b = self.log_block(k)?;
            let room = usize::try_from(BLOCK.saturating_sub(off)).map_err(|_| bounds(off))?;
            let (now, later) = rest.split_at_checked(room.min(rest.len())).ok_or(bounds(off))?;
            if now.is_empty() {
                return Err(bounds(at));
            }
            put(self.block_mut(b)?, off, now)?;
            (rest, at) = (later, at.wrapping_add(now.len() as u64));
        }
        Ok(())
    }

    /// A new record of the key of the record at `from`, with payload `payload`: its bytes beyond W copied from
    /// log block to log block. Returns its locator.
    pub fn copy_record(&mut self, from: u64, payload: u64) -> Result<u64, Fault> {
        let (_, len) = record_head(&*self.host, self.log, from)?;
        let to = self.alloc_record(payload, len)?;
        let w = self.shape.w;
        let mut at = w;
        while at < len {
            let (sk, so) = byte_place(from, w, at);
            let (dk, doff) = byte_place(to, w, at);
            // at < len.
            let n = BLOCK.saturating_sub(so).min(BLOCK.saturating_sub(doff)).min(len.wrapping_sub(at));
            if n == 0 {
                return Err(bounds(at));
            }
            let (sb, db) = (self.log_block(sk)?, self.log_block(dk)?);
            if sb == db {
                shift(self.block_mut(sb)?, so, doff, n)?;
            } else {
                let a = self.area;
                copy(&mut *self.host, Region::Block(sb), so, Region::Block(db), doff, n, Owner(a))?;
            }
            at = at.wrapping_add(n);
        }
        Ok(to)
    }

    /// Hands back the record at `loc`: its chunk joined with the free chunks beside it and made free, or its
    /// block freed when wholly free again; a longer record's blocks freed whole.
    pub fn free_record(&mut self, loc: u64) -> Result<(), Fault> {
        // Ordinal 0 is the head block, which holds no record.
        if loc >> 12 == 0 {
            return Err(rt(loc, Corruption::Link));
        }
        let (_, word) = self.words(loc, 0)?;
        let head = self.log_block(0)?;
        let mut s = (word >> SIZE_SHIFT) & SIZE_MASK;
        if s == 0 {
            let size = key_len(word)
                .checked_sub(self.shape.w)
                .and_then(|t| t.checked_add(REC_HEAD))
                .filter(|&n| n > BLOCK)
                .ok_or(rt(loc, Corruption::Bounds))?;
            let mut k = loc >> 12;
            let end = k.checked_add(size.div_ceil(BLOCK)).ok_or(rt(loc, Corruption::Bounds))?;
            while k < end {
                self.free_ordinal(k)?;
                k = k.wrapping_add(1);
            }
        } else {
            let off = loc & 0xFFF;
            if s < CHUNK_MIN || off.wrapping_add(s) > BLOCK {
                return Err(rt(loc, Corruption::Bounds));
            }
            let mut start = loc;
            if off.wrapping_add(s) < BLOCK {
                let next = loc.wrapping_add(s);
                if self.words(next, 0)?.0 & FREE != 0 {
                    let ns = self.free_size(next)?;
                    self.unlink(head, next, ns)?;
                    s = s.wrapping_add(ns);
                }
            }
            if word & PREV_FREE != 0 {
                // The free chunk before ends with its bytes.
                let ps = if off >= CHUNK_MIN { self.words(loc.wrapping_sub(8), 0)?.0 } else { 0 };
                if ps < CHUNK_MIN || ps > off {
                    return Err(rt(loc, Corruption::Bounds));
                }
                start = loc.wrapping_sub(ps);
                self.unlink(head, start, ps)?;
                s = s.wrapping_add(ps);
            }
            if s == BLOCK {
                self.free_ordinal(start >> 12)?;
            } else {
                self.push(head, start, s)?;
                if (start & 0xFFF).wrapping_add(s) < BLOCK {
                    self.set_prev_free(start.wrapping_add(s), true)?;
                }
            }
        }
        // The head alone is left: the log is empty.
        if self.log.blocks == 1 {
            self.free_ordinal(0)?;
            self.log = KeyLog::default();
        }
        Ok(())
    }

    /// Starts a key written into the log over units (`put_key`, `close_key`), its length not known yet, at most
    /// `most` bytes: a zero-filled state block of a fresh ordinal k, and the ordinals after it held for the
    /// key's record, a record of fresh blocks whose blocks are made as its bytes reach them. The state block is
    /// {the record's blocks made u64 @0; the ordinals held u64 @8; the caller's words u64 @16 to @56; the key's
    /// first W bytes from @64}. Returns k.
    pub fn open_key(&mut self, most: u64) -> Result<u64, Fault> {
        let n = most.saturating_sub(self.shape.w).saturating_add(REC_HEAD).div_ceil(BLOCK);
        let head = self.head()?;
        let k = rd_u64(self.block(head)?, H_NEXT)?;
        let end = k
            .checked_add(1)
            .and_then(|e| e.checked_add(n))
            .filter(|&e| e <= ORDINAL_END)
            .ok_or(rt(k, Corruption::Range))?;
        put_u64(self.block_mut(head)?, H_NEXT, end)?;
        let b = self.map_ordinal(k, true)?;
        put_u64(self.block_mut(b)?, KS_HELD, n)?;
        Ok(k)
    }

    /// Makes the log block of ordinal `k`, a fresh one (`zeroed` or not).
    fn map_ordinal(&mut self, k: u64, zeroed: bool) -> Result<u64, Fault> {
        let b = if zeroed {
            alloc_zeroed(&mut *self.host, self.tbl, self.area)?
        } else {
            alloc_block(&mut *self.host, self.tbl, self.area)?
        };
        let mut radix = self.log.radix;
        radix.set(&mut *self.host, self.tbl, k, b)?;
        self.log.radix = radix;
        self.log.blocks = self.log.blocks.checked_add(1).ok_or(rt(k, Corruption::Count))?;
        Ok(b)
    }

    /// The caller's word `i` (0 to 16) of the key written over units whose state block has ordinal `k`.
    pub fn key_word(&self, k: u64, i: u64) -> Result<u64, Fault> {
        rd_u64(self.block(self.log_block(k)?)?, word_at(i))
    }

    /// Sets the caller's word `i` (0 to 16) of the key written over units whose state block has ordinal `k`.
    pub fn set_key_word(&mut self, k: u64, i: u64, v: u64) -> Result<(), Fault> {
        let b = self.log_block(k)?;
        put_u64(self.block_mut(b)?, word_at(i), v)
    }

    /// The key written over units whose state block has ordinal `k`, once it has ended at `len` bytes, longer
    /// than W, as its record will stand until `close_key`: its first W bytes copied into the start of `head`,
    /// and its record's locator (its chunk's once `move_key` moved it), which comparisons read the rest through.
    pub fn key_open_record(&self, k: u64, len: u64, head: &mut [u8]) -> Result<u64, Fault> {
        let w = self.shape.w;
        if len <= w {
            return Err(bounds(len));
        }
        let n = usize::try_from(w).map_err(|_| bounds(w))?;
        let s = self.block(self.log_block(k)?)?;
        copy_bytes(head.get_mut(..n).ok_or(bounds(w))?, sub(s, KS_HEAD, w)?);
        let chunk = rd_u64(s, KS_CHUNK)?;
        if chunk != 0 {
            return Ok(chunk);
        }
        Ok(k.checked_add(1).ok_or(rt(k, Corruption::Range))? << 12)
    }

    /// Writes `bytes` as bytes [at, at + bytes.len()) of the key written over units whose state block has
    /// ordinal `k`: those below W into the state block, the rest into its record, whose blocks are made as the
    /// bytes reach them, within the ordinals held.
    pub fn put_key(&mut self, k: u64, at: u64, bytes: &[u8]) -> Result<(), Fault> {
        let w = self.shape.w;
        let sb = self.log_block(k)?;
        let (mut at, mut rest) = (at, bytes);
        if at < w {
            let n = usize::try_from(w.wrapping_sub(at)).unwrap_or(usize::MAX).min(rest.len());
            let (now, later) = rest.split_at_checked(n).ok_or(bounds(at))?;
            put(self.block_mut(sb)?, KS_HEAD.wrapping_add(at), now)?;
            (rest, at) = (later, at.wrapping_add(n as u64));
        }
        if rest.is_empty() {
            return Ok(());
        }
        let (made, held) = {
            let s = self.block(sb)?;
            (rd_u64(s, KS_MADE)?, rd_u64(s, KS_HELD)?)
        };
        let loc = k.checked_add(1).ok_or(rt(k, Corruption::Range))? << 12;
        let last = at.checked_add((rest.len() as u64).wrapping_sub(1)).ok_or(bounds(at))?;
        let need = byte_place(loc, w, last).0.wrapping_sub(k);
        if need > held {
            return Err(bounds(at));
        }
        let mut m = made;
        while m < need {
            m = m.wrapping_add(1);
            self.map_ordinal(k.wrapping_add(m), false)?;
        }
        if m != made {
            put_u64(self.block_mut(sb)?, KS_MADE, m)?;
        }
        self.write_key(loc, u64::MAX, at, rest)
    }

    /// Copies the first `out.len()` bytes of the key written over units whose state block has ordinal `k` into
    /// `out`: those below W from the state block, the rest from its record's blocks made.
    pub fn read_key(&self, k: u64, out: &mut [u8]) -> Result<(), Fault> {
        let w = self.shape.w;
        let len = out.len() as u64;
        let n = usize::try_from(len.min(w)).map_err(|_| bounds(len))?;
        let (head, mut rest) = out.split_at_mut(n);
        copy_bytes(head, sub(self.block(self.log_block(k)?)?, KS_HEAD, n as u64)?);
        let loc = k.checked_add(1).ok_or(rt(k, Corruption::Range))? << 12;
        let mut at = w;
        while !rest.is_empty() {
            let (kk, off) = byte_place(loc, w, at);
            let room = usize::try_from(BLOCK.saturating_sub(off)).map_err(|_| bounds(off))?.min(rest.len());
            let (now, later) = rest.split_at_mut(room);
            if now.is_empty() {
                return Err(bounds(at));
            }
            copy_bytes(now, sub(self.block(self.log_block(kk)?)?, off, room as u64)?);
            (rest, at) = (later, at.wrapping_add(room as u64));
        }
        Ok(())
    }

    /// The blocks of the second record of the ended key of `len` bytes written over units whose state block has
    /// ordinal `k` not made yet (`twin_step`): none when the key's record fits a chunk of one block.
    pub fn twin_left(&self, k: u64, len: u64) -> Result<u64, Fault> {
        let n = self.rec_blocks(len);
        if n == 0 {
            return Ok(0);
        }
        Ok(n.saturating_sub(rd_u64(self.block(self.log_block(k)?)?, KS_TWIN_MADE)?))
    }

    /// Makes the next block of a second record of the ended key of `len` bytes written over units whose state
    /// block has ordinal `k`, its record longer than a block: a copy of the record's block of the same place,
    /// in a fresh block of the ordinals held for the second record at its first step. A key's count entry
    /// takes that record, so no single call copies the whole key.
    pub fn twin_step(&mut self, k: u64, len: u64) -> Result<(), Fault> {
        let n = self.rec_blocks(len);
        let sb = self.log_block(k)?;
        let (made, mut t, i) = {
            let s = self.block(sb)?;
            (rd_u64(s, KS_MADE)?, rd_u64(s, KS_TWIN)?, rd_u64(s, KS_TWIN_MADE)?)
        };
        if made != n || i >= n {
            return Err(rt(k, Corruption::Count));
        }
        if t == 0 {
            let head = self.head()?;
            t = rd_u64(self.block(head)?, H_NEXT)?;
            let end = t.checked_add(n).filter(|&e| e <= ORDINAL_END).ok_or(rt(t, Corruption::Range))?;
            put_u64(self.block_mut(head)?, H_NEXT, end)?;
            put_u64(self.block_mut(sb)?, KS_TWIN, t)?;
        }
        // i < n ≤ the ordinals held after k, and t + n ≤ ORDINAL_END.
        let from = self.log_block(k.wrapping_add(1).wrapping_add(i))?;
        let to = self.map_ordinal(t.wrapping_add(i), false)?;
        let a = self.area;
        copy(&mut *self.host, Region::Block(from), 0, Region::Block(to), 0, BLOCK, Owner(a))?;
        put_u64(self.block_mut(sb)?, KS_TWIN_MADE, i.wrapping_add(1))
    }

    /// The blocks a record of a key of `len` bytes takes when longer than a block, else 0 (a chunk).
    fn rec_blocks(&self, len: u64) -> u64 {
        let size = REC_HEAD.saturating_add(len.saturating_sub(self.shape.w));
        if len > self.shape.w && size > BLOCK {
            size.div_ceil(BLOCK)
        } else {
            0
        }
    }

    /// Ends the key of `len` bytes written over units whose state block has ordinal `k`: its first W bytes (or
    /// all of them) copied into the start of `head`, its record made {payload 0, its length}, and so its second
    /// record when every block of it was made (`twin_step`). A record of up to 4,096 bytes moves into a chunk,
    /// as `alloc_record` makes it, unless `move_key` moved it there already. The state block stays, for
    /// `give_back`, its record's blocks and chunk no longer its own; the second record's stay its own until an
    /// entry takes it (`take_twin`). Returns the record's locator, 0
    /// for a key of at most W bytes, which has none, and the second record's, 0 when there is none.
    pub fn close_key(&mut self, k: u64, len: u64, head: &mut [u8]) -> Result<(u64, u64), Fault> {
        let w = self.shape.w;
        let sb = self.log_block(k)?;
        let n = usize::try_from(len.min(w)).map_err(|_| bounds(len))?;
        let (made, t, tm, chunk) = {
            let s = self.block(sb)?;
            copy_bytes(head.get_mut(..n).ok_or(bounds(len))?, sub(s, KS_HEAD, n as u64)?);
            (rd_u64(s, KS_MADE)?, rd_u64(s, KS_TWIN)?, rd_u64(s, KS_TWIN_MADE)?, rd_u64(s, KS_CHUNK)?)
        };
        let twin = if t == 0 {
            0
        } else if tm == made && made == self.rec_blocks(len) {
            let s = self.block_mut(self.log_block(t)?)?;
            put_u64(s, 0, 0)?;
            put_u64(s, 8, len)?;
            t << 12
        } else {
            return Err(rt(k, Corruption::Count));
        };
        let first = k.checked_add(1).ok_or(rt(k, Corruption::Range))?;
        let loc = if len <= w {
            0
        } else if chunk != 0 {
            // Moved ahead (`move_key`): the entry takes it.
            put_u64(self.block_mut(sb)?, KS_CHUNK, 0)?;
            chunk
        } else {
            let size = REC_HEAD.checked_add(len.wrapping_sub(w)).ok_or(bounds(len))?;
            if made != size.div_ceil(BLOCK) {
                return Err(rt(k, Corruption::Count));
            }
            if size > BLOCK {
                let s = self.block_mut(self.log_block(first)?)?;
                put_u64(s, 0, 0)?;
                put_u64(s, 8, len)?;
                first << 12
            } else {
                self.chunk_record(first, len)?
            }
        };
        if made != 0 {
            put_u64(self.block_mut(sb)?, KS_MADE, 0)?;
        }
        Ok((loc, twin))
    }

    /// Moves the record of the key of `len` bytes from ordinal `first` into a fresh chunk, whose locator it
    /// returns, and frees its block.
    fn chunk_record(&mut self, first: u64, len: u64) -> Result<u64, Fault> {
        let b = self.log_block(first)?;
        let to = self.alloc_record(0, len)?;
        let (db, off) = self.place(to)?;
        let off = off.wrapping_add(REC_HEAD);
        let (n, a) = (len.saturating_sub(self.shape.w), self.area);
        copy(&mut *self.host, Region::Block(b), REC_HEAD, Region::Block(db), off, n, Owner(a))?;
        self.free_ordinal(first)?;
        Ok(to)
    }

    /// Whether the ended key of `len` bytes written over units whose state block has ordinal `k` has a record
    /// `move_key` moves: one that fits a chunk, not moved yet.
    pub fn key_moves(&self, k: u64, len: u64) -> Result<bool, Fault> {
        let w = self.shape.w;
        if len <= w || REC_HEAD.saturating_add(len.wrapping_sub(w)) > BLOCK {
            return Ok(false);
        }
        Ok(rd_u64(self.block(self.log_block(k)?)?, KS_CHUNK)? == 0)
    }

    /// Moves the record of the ended key of `len` bytes written over units whose state block has ordinal `k`
    /// into a chunk, as `close_key` would, when it has one to move (`key_moves`), so `close_key` copies no
    /// bytes: the state block keeps the chunk's locator, which `key_open_record` gives from then on, and
    /// `give_back` hands the chunk back while no entry has taken it.
    pub fn move_key(&mut self, k: u64, len: u64) -> Result<(), Fault> {
        if !self.key_moves(k, len)? {
            return Ok(());
        }
        let sb = self.log_block(k)?;
        if rd_u64(self.block(sb)?, KS_MADE)? != 1 {
            return Err(rt(k, Corruption::Count));
        }
        let to = self.chunk_record(k.checked_add(1).ok_or(rt(k, Corruption::Range))?, len)?;
        let s = self.block_mut(sb)?;
        put_u64(s, KS_MADE, 0)?;
        put_u64(s, KS_CHUNK, to)
    }

    /// The second record of the key whose state block has ordinal `k`, closed (`close_key`), is an entry's now:
    /// `give_back` leaves it.
    pub fn take_twin(&mut self, k: u64) -> Result<(), Fault> {
        let b = self.log_block(k)?;
        put_u64(self.block_mut(b)?, KS_TWIN_MADE, 0)
    }

    /// The blocks besides itself that the state block of ordinal `k` still has to hand back (`give_back`).
    pub fn key_held(&self, k: u64) -> Result<u64, Fault> {
        let s = self.block(self.log_block(k)?)?;
        let tm = if rd_u64(s, KS_TWIN)? == 0 { 0 } else { rd_u64(s, KS_TWIN_MADE)? };
        let chunk = u64::from(rd_u64(s, KS_CHUNK)? != 0);
        Ok(rd_u64(s, KS_MADE)?.saturating_add(tm).saturating_add(chunk))
    }

    /// Hands back one block of the key written over units whose state block has ordinal `k`, once its row is
    /// in or gone: the last block of its second record still its own (`twin_step`, `take_twin`), else its
    /// record's chunk no entry took (`move_key`), else the last block of its record still its own (`put_key`,
    /// `close_key`), else the state block. Returns whether that was the state block, the last.
    pub fn give_back(&mut self, k: u64) -> Result<bool, Fault> {
        let sb = self.log_block(k)?;
        let (made, t, tm, chunk) = {
            let s = self.block(sb)?;
            (rd_u64(s, KS_MADE)?, rd_u64(s, KS_TWIN)?, rd_u64(s, KS_TWIN_MADE)?, rd_u64(s, KS_CHUNK)?)
        };
        if t != 0 && tm != 0 {
            let i = tm.wrapping_sub(1);
            self.free_ordinal(t.checked_add(i).ok_or(rt(t, Corruption::Range))?)?;
            put_u64(self.block_mut(sb)?, KS_TWIN_MADE, i)?;
            return Ok(false);
        }
        if chunk != 0 {
            self.free_record(chunk)?;
            put_u64(self.block_mut(sb)?, KS_CHUNK, 0)?;
            return Ok(false);
        }
        if made != 0 {
            self.free_ordinal(k.checked_add(made).ok_or(rt(k, Corruption::Range))?)?;
            put_u64(self.block_mut(sb)?, KS_MADE, made.wrapping_sub(1))?;
            return Ok(false);
        }
        self.free_ordinal(k)?;
        self.drop_lone_head()?;
        Ok(true)
    }

    /// Frees the head when it is the log's last block: the log is empty.
    fn drop_lone_head(&mut self) -> Result<(), Fault> {
        if self.log.blocks == 1 {
            self.free_ordinal(0)?;
            self.log = KeyLog::default();
        }
        Ok(())
    }

    /// Rewrites a record's payload in place (a count entry's count).
    pub fn set_record_payload(&mut self, locator: u64, payload: u64) -> Result<(), Fault> {
        if payload & FREE != 0 {
            return Err(rt(payload, Corruption::Range));
        }
        self.set_word(locator, 0, payload)
    }
}

impl<H: PageHost> NodeMem for AreaNodes<'_, H> {
    type Entry<'k> = IndexPending<'k>;
    type Bytes<'a>
        = &'a [u8]
    where
        Self: 'a;

    fn count(&self, n: u64) -> Result<u16, Fault> {
        rd_u16(self.block(n)?, 0)
    }
    fn set_count(&mut self, n: u64, c: u16) -> Result<(), Fault> {
        put_u16(self.block_mut(n)?, 0, c)
    }
    fn flags(&self, n: u64) -> Result<u16, Fault> {
        rd_u16(self.block(n)?, 2)
    }
    fn word(&self, n: u64, off: u64) -> Result<u64, Fault> {
        rd_u64(self.block(n)?, off)
    }
    fn set_word(&mut self, n: u64, off: u64, w: u64) -> Result<(), Fault> {
        put_u64(self.block_mut(n)?, off, w)
    }
    fn bytes(&self, n: u64, off: u64, len: u64) -> Result<&[u8], Fault> {
        sub(self.block(n)?, off, len)
    }
    fn copy(&mut self, s: u64, so: u64, d: u64, doff: u64, len: u64) -> Result<(), Fault> {
        if s == d {
            shift(self.block_mut(s)?, so, doff, len)
        } else {
            let a = self.area;
            copy(&mut *self.host, Region::Block(s), so, Region::Block(d), doff, len, Owner(a))
        }
    }
    fn put_entry(&mut self, n: u64, koff: u64, voff: u64, p: &IndexPending<'_>) -> Result<(), Fault> {
        let w = self.shape.w;
        let ks = w.wrapping_add(8);
        let vs = self.shape.v;
        // Every key part lies before the node's value parts, so the two are written in place through one
        // split of the block, with no buffer.
        if koff.checked_add(ks).is_some_and(|e| e <= voff) {
            let at = usize::try_from(voff).map_err(|_| bounds(voff))?;
            let blk = self.block_mut(n)?;
            let (left, right) = blk.split_at_mut_checked(at).ok_or(bounds(voff))?;
            return p.write(w, sub_mut(left, koff, ks)?, sub_mut(right, 0, vs)?);
        }
        let mut kbuf = [0u8; PART];
        let mut vbuf = [0u8; 256];
        let kb = kbuf.get_mut(..usize::try_from(ks).map_err(|_| bounds(0))?).ok_or(bounds(0))?;
        let vb = vbuf.get_mut(..usize::try_from(vs).map_err(|_| bounds(0))?).ok_or(bounds(0))?;
        p.write(w, kb, vb)?;
        let blk = self.block_mut(n)?;
        copy_bytes(sub_mut(blk, koff, ks)?, kb);
        copy_bytes(sub_mut(blk, voff, vs)?, vb);
        Ok(())
    }
    fn take_entry(&mut self, n: u64, koff: u64, voff: u64, p: &mut IndexPending<'_>) -> Result<(), Fault> {
        let ks = self.shape.w.wrapping_add(8);
        let vs = self.shape.v;
        let blk = self.block(n)?;
        let k = sub(blk, koff, ks)?;
        let v = sub(blk, voff, vs)?;
        let mut kbuf = [0u8; PART];
        let mut vbuf = [0u8; 256];
        let kl = k.len();
        let vl = v.len();
        copy_bytes(&mut kbuf, k);
        copy_bytes(&mut vbuf, v);
        p.take(kbuf.get(..kl).unwrap_or(&[]), vbuf.get(..vl).unwrap_or(&[]))
    }
    fn alloc(&mut self, leaf: bool) -> Result<u64, Fault> {
        // An area node is not filled: a reused block keeps its previous contents.
        let n = alloc_block(&mut *self.host, self.tbl, self.area)?;
        let layout = self.shape.layout();
        let flags = if leaf { FLAG_INDEX | FLAG_LEAF } else { FLAG_INDEX };
        let s = self.block_mut(n)?;
        put_u16(s, 0, 0)?;
        put_u16(s, 2, flags)?;
        put_u32(s, 4, layout)?;
        self.nodes = self.nodes.wrapping_add(1);
        Ok(n)
    }
    fn hand_back(&mut self, n: u64) -> Result<(), Fault> {
        free_block(&mut *self.host, self.tbl, self.area, n)?;
        self.nodes = self.nodes.wrapping_sub(1);
        Ok(())
    }
    fn release(&mut self, root: u64) -> Result<(), Fault> {
        // A whole tree goes to the table's drain, which frees it in bounded units; the drain is not part
        // of this layer, so a tree is released through its owner, never here.
        Err(rt(root, Corruption::Link))
    }
}

impl<H: PageHost> ResolveKey for AreaNodes<'_, H> {
    fn width(&self) -> u64 {
        self.shape.w
    }
    fn record(&self, field: u64) -> Result<(u64, u64), Fault> {
        record_head(&*self.host, self.log, payload(field))
    }
    fn key_piece(&self, field: u64, at: u64, out: &mut [u8]) -> Result<usize, Fault> {
        key_piece(&*self.host, self.shape, self.log, field, at, out)
    }
    fn key_bytes(&self, field: u64, at: u64, len: u64) -> Result<Option<&[u8]>, Fault> {
        log_bytes(&*self.host, self.log.radix, self.shape.w, payload(field), at, len).map(Some)
    }
    fn rid_base(&self) -> u64 {
        0
    }
}

/// The offset of the caller's word `i` (0 to 11) in a key's state block.
fn word_at(i: u64) -> u64 {
    let i = i.min(16);
    if i < 6 {
        KS_WORDS.wrapping_add(i.wrapping_mul(8))
    } else {
        KS_MORE.wrapping_add(i.wrapping_sub(6).wrapping_mul(8))
    }
}

/// Where byte `at` (at ≥ W) of the key whose record has locator `loc` lies in its key log: the log block's
/// ordinal and the offset in it. The bytes beyond W follow the record's head to its block's end (a record in a
/// chunk ends inside it), then fill the blocks after it whole.
#[inline(always)]
fn byte_place(loc: u64, w: u64, at: u64) -> (u64, u64) {
    // at ≥ W at every caller.
    let q = at.wrapping_sub(w);
    let first_off = (loc & 0xFFF).wrapping_add(REC_HEAD);
    let first_room = BLOCK.saturating_sub(first_off);
    if q < first_room {
        (loc >> 12, first_off.wrapping_add(q))
    } else {
        let r = q.wrapping_sub(first_room);
        ((loc >> 12).wrapping_add(1).wrapping_add(r / BLOCK), r % BLOCK)
    }
}

/// Bytes [at, …) of the key of `len` bytes whose record has locator `loc` in the key log listed by `radix`,
/// W ≤ at < len, where the log holds them: to the end of their log block or of the key.
fn log_bytes<H: PageHost + ?Sized>(
    host: &H,
    radix: Radix,
    w: u64,
    loc: u64,
    at: u64,
    len: u64,
) -> Result<&[u8], Fault> {
    if at < w || at >= len {
        return Err(bounds(at));
    }
    let (k, off) = byte_place(loc, w, at);
    let b = radix.get(host, k)?;
    if b == 0 {
        return Err(rt(k, Corruption::Link));
    }
    let n = BLOCK.saturating_sub(off).min(len.wrapping_sub(at));
    sub(host.get(Region::Block(b))?, off, n)
}

/// A key log read in place by a count's comparisons: the log block looked up last kept, a record's head read
/// where it lies, a key's bytes lent where the log holds them.
pub struct LogBytes<'h, H: PageHost + ?Sized> {
    host: &'h H,
    radix: Radix,
    /// The ordinal of the block held (`u64::MAX`: none) and its bytes.
    k: u64,
    b: &'h [u8],
}

impl<'h, H: PageHost + ?Sized> LogBytes<'h, H> {
    pub fn new(host: &'h H, radix: Radix) -> Self {
        LogBytes { host, radix, k: u64::MAX, b: &[] }
    }

    /// Log block k.
    #[inline]
    fn block(&mut self, k: u64) -> Result<&'h [u8], Fault> {
        if k != self.k {
            let r = self.radix.get(self.host, k)?;
            if r == 0 {
                return Err(rt(k, Corruption::Link));
            }
            let host: &'h H = self.host;
            self.b = host.get(Region::Block(r))?;
            self.k = k;
        }
        Ok(self.b)
    }

    /// The head {payload, full key length} of the record at locator `loc`.
    #[inline]
    pub fn head(&mut self, loc: u64) -> Result<(u64, u64), Fault> {
        let b = self.block(loc >> 12)?;
        let off = loc & 0xFFF;
        Ok((rd_u64(b, off)?, key_len(rd_u64(b, off.wrapping_add(8))?)))
    }

    /// Bytes [at, …) of the key of `len` bytes at locator `loc`, W = `w` ≤ at < len, where the log holds them:
    /// to the end of their log block or of the key.
    #[inline]
    pub fn bytes(&mut self, loc: u64, w: u64, at: u64, len: u64) -> Result<&'h [u8], Fault> {
        if at < w || at >= len {
            return Err(bounds(at));
        }
        let (k, off) = byte_place(loc, w, at);
        let n = BLOCK.saturating_sub(off).min(len.wrapping_sub(at));
        sub(self.block(k)?, off, n)
    }
}

/// The log block of ordinal k of key log `log`.
fn log_block<H: PageHost + ?Sized>(h: &H, log: KeyLog, k: u64) -> Result<u64, Fault> {
    let b = log.radix.get(h, k)?;
    if b == 0 {
        return Err(rt(k, Corruption::Link));
    }
    Ok(b)
}

/// A record's payload and its full key's length.
fn record_head<H: PageHost + ?Sized>(h: &H, log: KeyLog, locator: u64) -> Result<(u64, u64), Fault> {
    let b = log_block(h, log, locator >> 12)?;
    let s = h.get(Region::Block(b))?;
    let off = locator & 0xFFF;
    Ok((rd_u64(s, off)?, key_len(rd_u64(s, off.wrapping_add(8))?)))
}

/// Bytes [at, at + out.len()) of an overflowed entry's key, at ≥ W; fewer at its end.
fn key_piece<H: PageHost + ?Sized>(
    h: &H,
    shape: IndexShape,
    log: KeyLog,
    field: u64,
    at: u64,
    out: &mut [u8],
) -> Result<usize, Fault> {
    let loc = payload(field);
    let (_, len) = record_head(h, log, loc)?;
    let w = shape.w;
    if at < w || at >= len {
        return Ok(0);
    }
    let (k, off) = byte_place(loc, w, at);
    let b = log_block(h, log, k)?;
    let avail = BLOCK.saturating_sub(off).min(len.wrapping_sub(at));
    let n = avail.min(out.len() as u64);
    let src = sub(h.get(Region::Block(b))?, off, n)?;
    let dst = out.get_mut(..src.len()).ok_or(bounds(0))?;
    copy_bytes(dst, src);
    Ok(src.len())
}

/// A read-only view of one index tree in table memory, for lookups and cursors that hold the host shared.
/// Its mutating node methods return `Err(BadRef)`: nothing writes through a view.
pub struct AreaView<'h, H: PageHost + ?Sized> {
    pub host: &'h H,
    pub shape: IndexShape,
    pub log: KeyLog,
}

impl<'h, H: PageHost + ?Sized> AreaView<'h, H> {
    pub fn new(host: &'h H, shape: IndexShape, log: KeyLog) -> Self {
        AreaView { host, shape, log }
    }

    fn block(&self, n: u64) -> Result<&'h [u8], Fault> {
        let h: &'h H = self.host;
        h.get(Region::Block(n))
    }
}

impl<H: PageHost + ?Sized> NodeMem for AreaView<'_, H> {
    type Entry<'k> = IndexPending<'k>;
    type Bytes<'a>
        = &'a [u8]
    where
        Self: 'a;

    fn count(&self, n: u64) -> Result<u16, Fault> {
        rd_u16(self.block(n)?, 0)
    }
    fn set_count(&mut self, _n: u64, _c: u16) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
    fn flags(&self, n: u64) -> Result<u16, Fault> {
        rd_u16(self.block(n)?, 2)
    }
    fn word(&self, n: u64, off: u64) -> Result<u64, Fault> {
        rd_u64(self.block(n)?, off)
    }
    fn set_word(&mut self, _n: u64, _off: u64, _w: u64) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
    fn bytes(&self, n: u64, off: u64, len: u64) -> Result<&[u8], Fault> {
        sub(self.block(n)?, off, len)
    }
    fn copy(&mut self, _s: u64, _so: u64, _d: u64, _doff: u64, _len: u64) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
    fn put_entry(&mut self, _n: u64, _koff: u64, _voff: u64, _p: &IndexPending<'_>) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
    fn take_entry(&mut self, _n: u64, _koff: u64, _voff: u64, _p: &mut IndexPending<'_>) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
    fn alloc(&mut self, _leaf: bool) -> Result<u64, Fault> {
        Err(Fault::BadRef)
    }
    fn hand_back(&mut self, _n: u64) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
    fn release(&mut self, _root: u64) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
}

impl<H: PageHost + ?Sized> ResolveKey for AreaView<'_, H> {
    fn width(&self) -> u64 {
        self.shape.w
    }
    fn record(&self, field: u64) -> Result<(u64, u64), Fault> {
        record_head(self.host, self.log, payload(field))
    }
    fn key_piece(&self, field: u64, at: u64, out: &mut [u8]) -> Result<usize, Fault> {
        key_piece(self.host, self.shape, self.log, field, at, out)
    }
    fn key_bytes(&self, field: u64, at: u64, len: u64) -> Result<Option<&[u8]>, Fault> {
        log_bytes(self.host, self.log.radix, self.shape.w, payload(field), at, len).map(Some)
    }
    fn rid_base(&self) -> u64 {
        0
    }
}
