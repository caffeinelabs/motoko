//! The store host of the native-table crate: page allocation, the free-page stack and the
//! reference gate.

use crate::memory::TestMemory;

use motoko_rts::gc::incremental::partitioned_heap::PARTITION_SIZE;
use motoko_rts::gc::incremental::{IncrementalGC, set_incremental_gc_state};
use motoko_rts::native_table::store::StoreHost;
use motoko_rts::types::Bytes;
use native_table::consts::{BLOCK, PAGE};
use native_table::{Fault, PageHost, Region};
use oorandom::Rand32;

use std::collections::BTreeSet;

/// Alloc/free pairs of the random run.
const PAIRS: u32 = 100_000;
/// Live pages at most in the random run: more than one store partition holds.
const MAX_LIVE: usize = 1_500;
/// Consecutive frees whose changed blocks are counted, 2,000 in all.
const FREE_RUNS: [usize; 8] = [1, 2, 509, 511, 512, 1, 1, 463];
/// References a stack block holds.
const STACK_SLOTS: usize = 511;

struct Store {
    mem: TestMemory,
    /// The persistent word that names the state page.
    store: usize,
    live: Vec<u64>,
    handed_out: BTreeSet<u64>,
}

impl Store {
    fn host(&mut self) -> StoreHost<'_, TestMemory> {
        StoreHost::new(&mut self.mem, &mut self.store)
    }

    /// Takes a page and writes each block's reference into the block's first word.
    fn alloc(&mut self) -> u64 {
        let mut host = self.host();
        let page = host.page_alloc().unwrap();
        let bytes = host.get_mut(Region::Page(page)).unwrap();
        for block in 0..PAGE / BLOCK {
            let at = (block * BLOCK) as usize;
            bytes[at..at + 8].copy_from_slice(&(page + block * BLOCK).to_le_bytes());
        }
        assert_ne!(page, self.store as u64);
        assert!(
            self.handed_out.insert(page),
            "page {page:#x} handed out twice"
        );
        self.live.push(page);
        page
    }

    /// Frees the live page at `index`, after checking that no block of it was overwritten.
    fn free(&mut self, index: usize) {
        let page = self.live.swap_remove(index);
        let mut host = self.host();
        let bytes = host.get(Region::Page(page)).unwrap();
        for block in 0..PAGE / BLOCK {
            let at = (block * BLOCK) as usize;
            let word = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
            assert_eq!(
                word,
                page + block * BLOCK,
                "live page {page:#x} overwritten"
            );
        }
        host.page_free(page).unwrap();
        assert!(self.handed_out.remove(&page));
    }

    /// The bytes of the two store partitions.
    unsafe fn partitions(&self) -> &[u8] {
        std::slice::from_raw_parts(self.store as *const u8, 2 * PARTITION_SIZE)
    }
}

pub unsafe fn test() {
    println!("Testing native-table store ...");

    // The partition the test memory starts in, and room for two store partitions after it.
    let mem = TestMemory::new(Bytes(3 * PARTITION_SIZE).to_words());
    set_incremental_gc_state(Some(IncrementalGC::<TestMemory>::initial_gc_state(
        mem.heap_base(),
    )));
    let mut store = Store {
        mem,
        store: 0,
        live: vec![],
        handed_out: BTreeSet::new(),
    };

    // No reference passes before the store exists.
    let page = store.mem.heap_base() as u64 / PAGE * PAGE + PAGE;
    assert_eq!(
        store.host().get(Region::Page(page)).err(),
        Some(Fault::BadRef)
    );
    test_random_pairs(&mut store);
    test_free_blocks(&mut store);
    test_gate(&mut store);

    set_incremental_gc_state(None);
}

/// No page is handed out twice, and no live page is written by the allocator.
fn test_random_pairs(store: &mut Store) {
    println!("  Testing {PAIRS} random alloc/free pairs ...");
    let mut random = Rand32::new(4711);
    let (mut allocs, mut frees, mut growing, mut peak) = (0, 0, true, 0);
    while allocs < PAIRS || !store.live.is_empty() {
        if store.live.len() >= MAX_LIVE {
            growing = false;
        }
        if store.live.is_empty() {
            growing = true;
        }
        let odds = if growing { 3 } else { 1 };
        if allocs < PAIRS && (store.live.is_empty() || random.rand_range(0..4) < odds) {
            store.alloc();
            allocs += 1;
            peak = peak.max(store.live.len());
        } else {
            let index = random.rand_range(0..store.live.len() as u32) as usize;
            store.free(index);
            frees += 1;
        }
    }
    assert_eq!(frees, PAIRS);
    assert!(peak >= MAX_LIVE);
}

/// n consecutive frees change at most 1 + ⌈n/511⌉ blocks besides the state block. Every live
/// page is filled with a pattern first, so any block the allocator writes differs afterwards.
fn test_free_blocks(store: &mut Store) {
    println!("  Testing changed blocks of consecutive frees ...");
    let total: usize = FREE_RUNS.iter().sum();
    for _ in 0..total {
        store.alloc();
    }
    for index in 0..store.live.len() {
        let page = store.live[index];
        let mut host = store.host();
        let bytes = host.get_mut(Region::Page(page)).unwrap();
        for block in 0..PAGE / BLOCK {
            let at = (block * BLOCK) as usize;
            bytes[at + 8..at + BLOCK as usize].fill(0xA5);
        }
    }
    // Both store partitions are in use.
    let second = (store.store + PARTITION_SIZE) as u64;
    assert!(store.host().get(Region::Page(second)).is_ok());

    let mut random = Rand32::new(42);
    let mut before = vec![0u8; 2 * PARTITION_SIZE];
    for n in FREE_RUNS {
        before.copy_from_slice(unsafe { store.partitions() });
        for _ in 0..n {
            let index = random.rand_range(0..store.live.len() as u32) as usize;
            store.free(index);
        }
        let after = unsafe { store.partitions() };
        let block = BLOCK as usize;
        // Block 0 is the state block.
        let changed = (1..before.len() / block)
            .filter(|b| before[b * block..(b + 1) * block] != after[b * block..(b + 1) * block])
            .count();
        let bound = 1 + n.div_ceil(STACK_SLOTS);
        println!("    {n} frees changed {changed} stack blocks (at most {bound})");
        assert!(changed <= bound);
        assert!(changed >= 1);
    }
    assert!(store.live.is_empty());
}

/// References outside store pages, into the state page, misaligned or overlapping get BadRef.
fn test_gate(store: &mut Store) {
    println!("  Testing the reference gate ...");
    let page = store.alloc();
    let state = store.store as u64;
    let partition = PARTITION_SIZE as u64;
    let bad = [
        Region::Page(0),
        Region::Block(0),
        // The last page before the first store partition.
        Region::Page(state - PAGE),
        Region::Block(state - BLOCK),
        // The first page after the second store partition.
        Region::Page(state + 2 * partition),
        Region::Page(1 << 62),
        Region::Block(u64::MAX - BLOCK + 1),
        // The state page.
        Region::Page(state),
        Region::Block(state),
        Region::Block(state + BLOCK),
        // Misaligned.
        Region::Page(page + BLOCK),
        Region::Block(page + 8),
    ];
    let mut host = store.host();
    assert!(host.get(Region::Page(page)).is_ok());
    for r in bad {
        assert_eq!(host.get(r).err(), Some(Fault::BadRef));
        assert_eq!(host.get_mut(r).err(), Some(Fault::BadRef));
        assert_eq!(
            host.pair_mut(r, Region::Page(page)).err(),
            Some(Fault::BadRef)
        );
        assert_eq!(
            host.pair_mut(Region::Page(page), r).err(),
            Some(Fault::BadRef)
        );
        assert_eq!(
            host.read_write(r, Region::Page(page)).err(),
            Some(Fault::BadRef)
        );
        assert_eq!(
            host.read_write(Region::Page(page), r).err(),
            Some(Fault::BadRef)
        );
        if let Region::Page(p) = r {
            assert_eq!(host.page_free(p), Err(Fault::BadRef));
        }
    }
    // Overlapping pairs.
    let (p, b) = (Region::Page(page), Region::Block(page + BLOCK));
    assert_eq!(host.pair_mut(p, b).err(), Some(Fault::BadRef));
    assert_eq!(host.pair_mut(b, b).err(), Some(Fault::BadRef));
    assert_eq!(host.read_write(b, p).err(), Some(Fault::BadRef));
    assert!(host.pair_mut(Region::Block(page), b).is_ok());
    store.free(0);
}
