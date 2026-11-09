//! Store memory: the pages the native-table crate keeps its tables in.
//!
//! Store memory is made of whole 64 MiB partitions that the partitioned heap hands out for
//! good, so the collector never scans, moves or frees them. A store partition holds 1,024
//! store pages of 64 KiB, and a page is made of 16 blocks of 4 KiB. A reference is a byte
//! address: block i of page p is p + 4,096·i. A reference is never 0, since partition 0 holds
//! the static space, and bits 62 and 63 are never set, since memory is at most 64 GiB.
//!
//! The first page of the first store partition is the state page; its first block holds the
//! `State`. The persistent metadata keeps the state page's address, so the store stays in
//! place across upgrades. Nothing is kept in Rust statics.
//!
//! The state also names the root-block maps: pages of one bit per block of store memory, set for
//! the root block of every table the store has created. A map page covers 32 partitions; the state
//! holds one map page reference per 32 partitions, 0 until a root block in them is recorded.
//!
//! Freed pages form a stack. A stack block is the first block of a free page: word 0 links to
//! the block below (0 at the bottom) and words 1 to 511 hold page references. Every block but
//! the top one is full. When the top block is full, the next freed page's first block becomes
//! the new top; when a pop finds the top block empty, that block's page is the page handed out.
//! So n frees change at most 1 + ⌈n/511⌉ stack blocks besides the state block.

use ::native_table::consts::{BLOCK, PAGE};
use ::native_table::{Fault, PageHost, Region};

use crate::gc::incremental::get_partitioned_heap;
use crate::gc::incremental::partitioned_heap::PARTITION_SIZE;
use crate::memory::{MAXIMUM_MEMORY_SIZE, Memory};

/// Page references a stack block holds after its link word.
const STACK_SLOTS: u64 = BLOCK / WORD - 1;

const WORD: u64 = 8;

/// Partitions of the largest memory.
const MAX_PARTITIONS: usize = MAXIMUM_MEMORY_SIZE.0 as usize / PARTITION_SIZE;

/// Blocks one root-block map page covers: one bit each.
const MAP_BLOCKS: u64 = PAGE * 8;

/// Partitions one root-block map page covers.
const MAP_PARTITIONS: usize = (MAP_BLOCKS * BLOCK) as usize / PARTITION_SIZE;

/// The allocator's state, at the start of the state page.
#[repr(C)]
struct State {
    /// The next page never handed out.
    next: u64,
    /// The end of the store partition that `next` lies in.
    end: u64,
    /// The top of the free-page stack: the address of the top block's last filled slot, or
    /// of the block itself when it holds no reference; 0 when the stack is empty.
    top: u64,
    /// Bit i is set when partition i is a store partition.
    partitions: [u64; MAX_PARTITIONS / 64],
    /// The root-block map page of each run of `MAP_PARTITIONS` partitions, 0 for none.
    roots: [u64; MAX_PARTITIONS / MAP_PARTITIONS],
}

impl State {
    fn is_store_partition(&self, index: usize) -> bool {
        index < MAX_PARTITIONS && self.partitions[index / 64] & (1 << (index % 64)) != 0
    }

    fn add_store_partition(&mut self, start: usize) {
        let index = start / PARTITION_SIZE;
        self.partitions[index / 64] |= 1 << (index % 64);
        self.next = start as u64;
        self.end = (start + PARTITION_SIZE) as u64;
    }
}

/// The crate's `PageHost` over store pages.
pub struct StoreHost<'a, M: Memory> {
    mem: &'a mut M,
    /// The state page's address, 0 before the first page is handed out: the persistent
    /// metadata's word in the runtime.
    store: &'a mut usize,
}

impl<'a, M: Memory> StoreHost<'a, M> {
    pub fn new(mem: &'a mut M, store: &'a mut usize) -> Self {
        StoreHost { mem, store }
    }

    fn state(&self) -> *mut State {
        *self.store as *mut State
    }

    /// The address and length of a region of a store page other than the state page, or
    /// `BadRef`.
    fn check(&self, region: Region) -> Result<(usize, usize), Fault> {
        let (address, size) = match region {
            Region::Block(block) => (block, BLOCK),
            Region::Page(page) => (page, PAGE),
        };
        let page = address - address % PAGE;
        if *self.store == 0 || address % size != 0 || page == *self.store as u64 {
            return Err(Fault::BadRef);
        }
        let partition = (address / PARTITION_SIZE as u64) as usize;
        if !unsafe { (*self.state()).is_store_partition(partition) } {
            return Err(Fault::BadRef);
        }
        Ok((address as usize, size as usize))
    }

    /// Two checked regions that do not overlap.
    fn check_pair(&self, a: Region, b: Region) -> Result<((usize, usize), (usize, usize)), Fault> {
        let (a, b) = (self.check(a)?, self.check(b)?);
        if a.0 < b.0 + b.1 && b.0 < a.0 + a.1 {
            return Err(Fault::BadRef);
        }
        Ok((a, b))
    }

    unsafe fn take_partition(&mut self) -> usize {
        get_partitioned_heap().take_store_partition(self.mem)
    }

    /// Creates the store when none exists: the first store partition, whose first page becomes
    /// the state page.
    unsafe fn init(&mut self) {
        if *self.store != 0 {
            return;
        }
        let start = self.take_partition();
        let state = start as *mut State;
        *state = State {
            next: 0,
            end: 0,
            top: 0,
            partitions: [0; MAX_PARTITIONS / 64],
            roots: [0; MAX_PARTITIONS / MAP_PARTITIONS],
        };
        (*state).add_store_partition(start);
        (*state).next += PAGE;
        *self.store = start;
    }

    /// The slot in `roots` of the map page that covers the block at `address`, and the block's
    /// bit in that page; `None` before the store exists or past the largest memory.
    fn map_bit(&self, address: u64) -> Option<(usize, u64)> {
        let slot = (address / BLOCK / MAP_BLOCKS) as usize;
        if *self.store == 0 || slot >= MAX_PARTITIONS / MAP_PARTITIONS {
            return None;
        }
        Some((slot, address / BLOCK % MAP_BLOCKS))
    }

    /// Records `id`, the root block of a table the store has just created.
    pub fn add_root(&mut self, id: u64) -> Result<(), Fault> {
        let (slot, bit) = self.map_bit(id).ok_or(Fault::BadRef)?;
        unsafe {
            let mut map = (*self.state()).roots[slot];
            if map == 0 {
                map = self.page_alloc()?;
                core::ptr::write_bytes(map as *mut u8, 0, PAGE as usize);
                (*self.state()).roots[slot] = map;
            }
            *((map + bit / 8) as *mut u8) |= 1u8 << (bit % 8);
        }
        Ok(())
    }

    /// Whether `id` is the root block of a table the store has created.
    pub fn is_root(&self, id: u64) -> bool {
        if id % BLOCK != 0 {
            return false;
        }
        let Some((slot, bit)) = self.map_bit(id) else {
            return false;
        };
        let map = unsafe { (*self.state()).roots[slot] };
        map != 0 && (unsafe { *((map + bit / 8) as *const u8) } >> (bit % 8)) & 1 == 1
    }
}

impl<'a, M: Memory> PageHost for StoreHost<'a, M> {
    fn block_of(&self, page: u64, i: u32) -> u64 {
        page + i as u64 * BLOCK
    }

    fn get(&self, r: Region) -> Result<&[u8], Fault> {
        let (address, size) = self.check(r)?;
        Ok(unsafe { core::slice::from_raw_parts(address as *const u8, size) })
    }

    fn get_mut(&mut self, r: Region) -> Result<&mut [u8], Fault> {
        let (address, size) = self.check(r)?;
        Ok(unsafe { core::slice::from_raw_parts_mut(address as *mut u8, size) })
    }

    fn pair_mut(&mut self, a: Region, b: Region) -> Result<(&mut [u8], &mut [u8]), Fault> {
        let (a, b) = self.check_pair(a, b)?;
        Ok(unsafe {
            (
                core::slice::from_raw_parts_mut(a.0 as *mut u8, a.1),
                core::slice::from_raw_parts_mut(b.0 as *mut u8, b.1),
            )
        })
    }

    fn read_write(&mut self, r: Region, wr: Region) -> Result<(&[u8], &mut [u8]), Fault> {
        let (r, wr) = self.check_pair(r, wr)?;
        Ok(unsafe {
            (
                core::slice::from_raw_parts(r.0 as *const u8, r.1),
                core::slice::from_raw_parts_mut(wr.0 as *mut u8, wr.1),
            )
        })
    }

    fn page_alloc(&mut self) -> Result<u64, Fault> {
        unsafe {
            self.init();
            let state = self.state();
            let top = (*state).top;
            if top != 0 {
                let slot = top % BLOCK;
                if slot != 0 {
                    (*state).top = top - WORD;
                    return Ok(*(top as *const u64));
                }
                // The top block is empty: hand out its page. The block below is full.
                let below = *(top as *const u64);
                (*state).top = if below == 0 {
                    0
                } else {
                    below + STACK_SLOTS * WORD
                };
                return Ok(top);
            }
            if (*state).next == (*state).end {
                let start = self.take_partition();
                (*state).add_store_partition(start);
            }
            let page = (*state).next;
            (*state).next += PAGE;
            Ok(page)
        }
    }

    fn page_free(&mut self, p: u64) -> Result<(), Fault> {
        self.check(Region::Page(p))?;
        unsafe {
            let state = self.state();
            let top = (*state).top;
            if top != 0 && top % BLOCK < STACK_SLOTS * WORD {
                (*state).top = top + WORD;
                *((top + WORD) as *mut u64) = p;
            } else {
                // The stack is empty or its top block is full: p's first block becomes the top.
                *(p as *mut u64) = top - top % BLOCK;
                (*state).top = p;
            }
        }
        Ok(())
    }
}
