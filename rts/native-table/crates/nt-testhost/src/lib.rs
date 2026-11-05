//! The test host: a `Vec`-backed main memory that logs every host call and counts the
//! distinct 4 KiB blocks each simulated message changes.
//!
//! The production allocators zero nothing, so neither does this host: every page `page_alloc` returns,
//! reused pages included, is filled with the fill pattern P. P takes two values, 0xA5 and 0x5A, which differ
//! in every bit; a test that runs once per P and gets different results read a byte nobody wrote.

use std::cell::Cell;
use std::collections::BTreeSet;

use native_table::consts::{BLOCK, PAGE};
use native_table::{Budget, Fault, Owner, PageHost, Region, Scratch};

/// The two fill patterns.
pub const FILLS: [u8; 2] = [0xA5, 0x5A];

/// Where the allocator's own blocks lie as the changed-block count sees them: the host state block, then the
/// free-page stack's blocks. No page the host lends lies there.
pub const ALLOCATOR: u64 = 1 << 61;

/// One host call, as logged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call {
    Get(Region),
    GetMut(Region),
    PairMut(Region, Region),
    ReadWrite(Region, Region),
    PageAlloc(u64),
    PageFree(u64),
}

/// Main memory in a `Vec`. Page k (k ≥ 1) has reference k · 65,536 and block i of it k · 65,536 + i · 4,096,
/// shaped like the addresses of Rust EOP (a Rust library for upgrades that keep the heap in place); 0 is never
/// a reference.
#[derive(Clone)]
pub struct VecHost {
    mem: Vec<u8>,
    free: Vec<u64>,
    live: BTreeSet<u64>,
    fill: u8,
    log: Vec<Call>,
    logging: bool,
    changed: BTreeSet<u64>,
    /// Whether the allocator's own blocks count among the changed blocks.
    count_allocator: bool,
    /// Pages whose reads are counted, and the count.
    pub watched: BTreeSet<u64>,
    pub watched_reads: Cell<u64>,
}

impl VecHost {
    /// A host with fill pattern `fill`.
    pub fn new(fill: u8) -> Self {
        VecHost {
            mem: Vec::new(),
            free: Vec::new(),
            live: BTreeSet::new(),
            fill,
            log: Vec::new(),
            logging: true,
            changed: BTreeSet::new(),
            count_allocator: false,
            watched: BTreeSet::new(),
            watched_reads: Cell::new(0),
        }
    }

    pub fn fill_pattern(&self) -> u8 {
        self.fill
    }

    /// The host calls since the last `take_log`.
    pub fn take_log(&mut self) -> Vec<Call> {
        std::mem::take(&mut self.log)
    }

    pub fn set_logging(&mut self, on: bool) {
        self.logging = on;
    }

    /// Distinct 4 KiB blocks written since the last `end_message`: the page-fee count of one message.
    pub fn changed_blocks(&self) -> u64 {
        self.changed.len() as u64
    }

    /// The distinct 4 KiB blocks written since the last `end_message`, in address order.
    pub fn changed_set(&self) -> Vec<u64> {
        self.changed.iter().copied().collect()
    }

    /// Ends a simulated message: returns its changed-block count and starts the next.
    pub fn end_message(&mut self) -> u64 {
        let n = self.changed_blocks();
        self.changed.clear();
        n
    }

    /// The whole main memory, every page ever allocated, freed ones included.
    pub fn memory(&self) -> &[u8] {
        &self.mem
    }

    /// Counts the allocator's own blocks among the changed blocks from now on (`on`), or stops: a
    /// `page_alloc` changes the host state block (the free-page stack's count or the bump pointer), a
    /// `page_free` the host state block and the free-page stack's block its entry lands in (511 entries a
    /// block). They lie outside every page the host lends, at `ALLOCATOR` and up.
    pub fn count_allocator(&mut self, on: bool) {
        self.count_allocator = on;
    }

    /// Pages allocated and not freed.
    pub fn live_pages(&self) -> usize {
        self.live.len()
    }

    fn record(&mut self, c: Call) {
        if self.logging {
            self.log.push(c);
        }
    }

    fn pages(&self) -> u64 {
        self.mem.len() as u64 / PAGE
    }

    /// The byte range of a region, if it names a live page or a block of one.
    fn span(&self, r: Region) -> Result<std::ops::Range<usize>, Fault> {
        let (base, len, page) = match r {
            Region::Page(p) => {
                if p == 0 || p % PAGE != 0 {
                    return Err(Fault::BadRef);
                }
                (p - PAGE, PAGE, p)
            }
            Region::Block(b) => {
                if b < PAGE || b % BLOCK != 0 {
                    return Err(Fault::BadRef);
                }
                (b - PAGE, BLOCK, b - b % PAGE)
            }
        };
        if !self.live.contains(&page) || base + len > self.pages() * PAGE {
            return Err(Fault::BadRef);
        }
        Ok(base as usize..(base + len) as usize)
    }

    fn mark_changed(&mut self, r: Region) {
        match r {
            Region::Block(b) => {
                self.changed.insert(b);
            }
            Region::Page(p) => {
                for i in 0..PAGE / BLOCK {
                    self.changed.insert(p + i * BLOCK);
                }
            }
        }
    }
}

impl PageHost for VecHost {
    fn block_of(&self, page: u64, i: u32) -> u64 {
        page + u64::from(i) * BLOCK
    }

    fn get(&self, r: Region) -> Result<&[u8], Fault> {
        let s = self.span(r)?;
        if !self.watched.is_empty() {
            let page = match r {
                Region::Page(p) => p,
                Region::Block(b) => b - b % PAGE,
            };
            if self.watched.contains(&page) {
                self.watched_reads.set(self.watched_reads.get() + 1);
            }
        }
        Ok(&self.mem[s])
    }

    fn get_mut(&mut self, r: Region, _w: Owner) -> Result<&mut [u8], Fault> {
        let s = self.span(r)?;
        self.record(Call::GetMut(r));
        self.mark_changed(r);
        Ok(&mut self.mem[s])
    }

    fn pair_mut(&mut self, a: Region, b: Region, _w: Owner) -> Result<(&mut [u8], &mut [u8]), Fault> {
        let (sa, sb) = (self.span(a)?, self.span(b)?);
        if sa.start < sb.end && sb.start < sa.end {
            return Err(Fault::BadRef);
        }
        self.record(Call::PairMut(a, b));
        self.mark_changed(a);
        self.mark_changed(b);
        if sa.start < sb.start {
            let (lo, hi) = self.mem.split_at_mut(sb.start);
            Ok((&mut lo[sa], &mut hi[..sb.end - sb.start]))
        } else {
            let (lo, hi) = self.mem.split_at_mut(sa.start);
            let bb = &mut lo[sb];
            Ok((&mut hi[..sa.end - sa.start], bb))
        }
    }

    fn read_write(&mut self, r: Region, wr: Region, _w: Owner) -> Result<(&[u8], &mut [u8]), Fault> {
        let (sa, sb) = (self.span(r)?, self.span(wr)?);
        if sa.start < sb.end && sb.start < sa.end {
            return Err(Fault::BadRef);
        }
        self.record(Call::ReadWrite(r, wr));
        self.mark_changed(wr);
        if sa.start < sb.start {
            let (lo, hi) = self.mem.split_at_mut(sb.start);
            Ok((&lo[sa], &mut hi[..sb.end - sb.start]))
        } else {
            let (lo, hi) = self.mem.split_at_mut(sa.start);
            let w = &mut lo[sb];
            Ok((&hi[..sa.end - sa.start], w))
        }
    }

    fn page_alloc(&mut self, _w: Owner) -> Result<u64, Fault> {
        if self.count_allocator {
            self.changed.insert(ALLOCATOR);
        }
        let p = match self.free.pop() {
            Some(p) => p,
            None => {
                let k = self.pages() + 1;
                self.mem.resize((k * PAGE) as usize, self.fill);
                k * PAGE
            }
        };
        self.live.insert(p);
        let s = self.span(Region::Page(p))?;
        self.mem[s].fill(self.fill);
        self.record(Call::PageAlloc(p));
        Ok(p)
    }

    fn page_free(&mut self, p: u64, _w: Owner) -> Result<(), Fault> {
        self.span(Region::Page(p))?;
        if !self.live.remove(&p) {
            return Err(Fault::BadRef);
        }
        if self.count_allocator {
            self.changed.insert(ALLOCATOR);
            self.changed.insert(ALLOCATOR + (1 + self.free.len() as u64 / 511) * BLOCK);
        }
        self.free.push(p);
        self.record(Call::PageFree(p));
        Ok(())
    }

    fn alloc_changes(&self) -> u64 {
        1
    }

    /// The test host always runs in test mode: `alloc_block` overwrites the stale free-stack link of every
    /// block it takes with P.
    fn test_fill(&self) -> Option<u8> {
        Some(self.fill)
    }
}

/// A budget whose meter the test advances.
pub struct TestBudget {
    pub used: Cell<u64>,
    pub limit: u64,
}

impl TestBudget {
    pub fn new(limit: u64) -> Self {
        TestBudget { used: Cell::new(0), limit }
    }
    pub fn spend(&self, n: u64) {
        self.used.set(self.used.get() + n);
    }
}

impl Budget for TestBudget {
    fn used(&self) -> u64 {
        self.used.get()
    }
    fn limit(&self) -> u64 {
        self.limit
    }
}

/// A scratch source that grows on demand; new bytes carry the fill pattern, so a unit that reads scratch it
/// did not write is caught by the two-pattern runs.
pub struct VecScratch {
    buf: Vec<u8>,
    fill: u8,
    pub grows: u64,
}

impl VecScratch {
    pub fn new(fill: u8) -> Self {
        VecScratch { buf: Vec::new(), fill, grows: 0 }
    }
}

impl Scratch for VecScratch {
    fn get(&mut self, need: usize) -> Result<&mut [u8], Fault> {
        if self.buf.len() < need {
            self.buf.resize(need, self.fill);
            self.grows += 1;
        }
        Ok(&mut self.buf[..need])
    }
}

/// A deterministic xorshift generator for tests.
pub struct XorShift(pub u64);

impl XorShift {
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
}

/// A key sink into a `Vec`.
#[derive(Default)]
pub struct VecSink(pub Vec<u8>);

impl native_table::key::Sink for VecSink {
    fn put(&mut self, bytes: &[u8]) -> Result<(), Fault> {
        self.0.extend_from_slice(bytes);
        Ok(())
    }
}

/// Hex of bytes, uppercase, space separated.
pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(" ")
}

/// Bytes from hex with optional spaces.
pub fn unhex(s: &str) -> Vec<u8> {
    let t: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    (0..t.len()).step_by(2).map(|i| u8::from_str_radix(&t[i..i + 2], 16).unwrap()).collect()
}

pub mod big;
pub mod index;
pub mod rows;
pub mod runs;
pub mod typed;
pub mod uploads;
