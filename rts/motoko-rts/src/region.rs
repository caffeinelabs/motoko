use crate::barriers::{allocation_barrier, init_with_barrier, write_with_barrier};
use crate::memory::{Memory, alloc_blob};
use crate::trap_with_prefix;
use crate::types::{Blob, Bytes, Region, TAG_BLOB_B, TAG_REGION, Value, size_of};

// Versions
// Should agree with constants in StableMem in compile.ml

// Legacy versions, used with classical persistence
pub const LEGACY_VERSION_NO_STABLE_MEMORY: usize = 0; // Never manifests in serialized form
pub const LEGACY_VERSION_SOME_STABLE_MEMORY: usize = 1;
pub const LEGACY_VERSION_REGIONS: usize = 2;

// New versions, used with enhanced orthogonal persistence
pub(crate) const VERSION_GRAPH_COPY_NO_REGIONS: usize = 3;
pub(crate) const VERSION_GRAPH_COPY_REGIONS: usize = 4;
pub(crate) const VERSION_STABLE_HEAP_NO_REGIONS: usize = 5;
pub(crate) const VERSION_STABLE_HEAP_REGIONS: usize = 6;
// V1 graph-copy: adds a 16-byte extension block in front of the legacy 40-byte
// last-page record carrying extra GC roots (dedup table, migrations list).
pub(crate) const VERSION_GRAPH_COPY_V1_NO_REGIONS: usize = 7;
pub(crate) const VERSION_GRAPH_COPY_V1_REGIONS: usize = 8;

const _: () = assert!(meta_data::size::PAGE_IN_BYTES == crate::stable_mem::PAGE_SIZE);
const _: () = assert!(meta_data::size::PAGES_IN_BLOCK <= u8::MAX as u32);
const _: () = assert!(meta_data::max::BLOCKS <= u16::MAX);
const _: () = assert!(meta_data::max::REGIONS <= u64::MAX - 1);

use motoko_rts_macros::{
    classical_persistence, enhanced_orthogonal_persistence, ic_mem_fn,
    uses_enhanced_orthogonal_persistence,
};

unsafe fn region_trap_with(msg: &str) -> ! {
    trap_with_prefix("Region error: ", msg)
}

unsafe fn stable_memory_trap_with(msg: &str) -> ! {
    trap_with_prefix("StableMemory ", msg)
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BlockId(pub u16);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RegionId(pub u64);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RegionSizeInPages(pub u64);

// Note: Use this type only in local variables, as it contains raw pointers
// that are not handled by the GCs.
#[derive(Clone)]
pub struct AccessVector(pub *mut Blob);

// Note: Use this type only in local variables, as it contains raw pointers
// that are not handled by the GCs.
#[derive(Clone)]
pub struct RegionObject(pub *mut Region);

const NIL_REGION_ID: u64 = 0;

const LAST_RESERVED_REGION_ID: u64 = 15;

/// All this global state gets reinitialized after an upgrade.
/// Therefore, it is not included in the persistent metadata.

// Mirrored field from stable memory, for handling upgrade logic.
pub(crate) static mut REGION_TOTAL_ALLOCATED_BLOCKS: u32 = 0;

// Base offset for blocks
pub(crate) static mut BLOCK_BASE: u64 = 0;

// Scalar sentinel value recognized in the GC as "no root", i.e. (!`is_ptr()`).
// Same design like `continuation_table::TABLE`.
pub(crate) const NO_REGION: Value = Value::from_scalar(0);

// Region 0 -- classic API for stable memory, as a dedicated region.
pub(crate) static mut REGION_0: Value = NO_REGION;

// This impl encapsulates encoding of optional region IDs within a u64.
// Used by block-region table to encode the (optional) region ID of a block.
impl RegionId {
    pub fn id_is_nil(id: u64) -> bool {
        id == NIL_REGION_ID
    }
    pub fn from_id(id: u64) -> Self {
        RegionId(id)
    }
    pub fn from_u64(id: u64) -> Option<Self> {
        if Self::id_is_nil(id) {
            None
        } else {
            Some(RegionId(id - 1))
        }
    }
    pub fn into_u64(opreg: Option<RegionId>) -> u64 {
        match opreg {
            None => 0,
            Some(id) => {
                debug_assert!(id.0 <= meta_data::max::REGIONS);
                id.0 + 1
            }
        }
    }
}

// This impl encapsulates encoding of optional region (sizes) within a u64.
// Used by region table to encode sizes of allocated regions (the size of which are Some(s)),
// and which regions are available to allocate (the size of which are None).
impl RegionSizeInPages {
    pub fn from_size_in_pages(s: u64) -> Self {
        RegionSizeInPages(s)
    }
    pub fn u64_is_nil(u: u64) -> bool {
        u == 0
    }
    pub fn from_u64(u: u64) -> Option<Self> {
        if Self::u64_is_nil(u) {
            None
        } else {
            Some(RegionSizeInPages(u - 1))
        }
    }
    pub fn into_u64(opreg: Option<RegionSizeInPages>) -> u64 {
        match opreg {
            None => 0,
            Some(s) => {
                debug_assert!(
                    s.0 <= meta_data::max::BLOCKS as u64 * meta_data::size::PAGES_IN_BLOCK as u64
                );
                s.0 + 1
            }
        }
    }
}

impl AccessVector {
    pub unsafe fn from_value(v: &Value) -> Self {
        AccessVector(v.as_blob_mut())
    }

    pub unsafe fn set_ith_block_id(&self, i: u32, block_id: &BlockId) {
        debug_assert!((i as usize) * 2 + 1 < self.0.len().as_usize());
        self.0.set_u16(i as usize, block_id.0)
    }

    pub unsafe fn get_ith_block_id(&self, i: u32) -> BlockId {
        debug_assert!((i as usize) * 2 + 1 < self.0.len().as_usize());
        BlockId(self.0.get_u16(i as usize))
    }
}

impl RegionObject {
    pub unsafe fn from_value(v: &Value) -> Self {
        RegionObject(v.as_region())
    }

    pub unsafe fn id(&self) -> RegionId {
        RegionId(self.0.read_id64())
    }

    pub unsafe fn trap_with(&self, msg: &str) -> ! {
        if (*self).id() == RegionId(0) {
            stable_memory_trap_with(msg)
        } else {
            region_trap_with(msg)
        };
    }

    // Check both offset and [offset,.., offset + len) within bounds *)
    // c.f. StableMem.guard_range in compile.ml *)
    // TODO: simplify and specialize on len
    pub unsafe fn check_relative_range(&self, offset: u64, len: u64) {
        if len <= 1 {
            if offset >= ((*self.0).page_count as u64) * meta_data::size::PAGE_IN_BYTES {
                self.trap_with("offset out of bounds");
            }
        } else {
            if u64::MAX - len < offset {
                self.trap_with("range overflow")
            };
            if offset + len > (((*self.0).page_count as u64) * meta_data::size::PAGE_IN_BYTES) {
                self.trap_with("range out of bounds");
            };
        }
    }

    // Computes absolute offset, BlockId, and remaining length (of the
    // given block) for a relative offset.
    pub unsafe fn relative_into_absolute_info(&self, offset: u64) -> (u64, BlockId, u64) {
        let av = AccessVector::from_value(&(*self.0).vec_pages);

        // Which block (rank relative to this region)?
        let block_rank = offset / meta_data::size::BLOCK_IN_BYTES;

        // Where in that block?
        let intra_block_index = offset % meta_data::size::BLOCK_IN_BYTES;

        // Where is that block located in stable memory (global rank)?
        let block_id = av.get_ith_block_id(block_rank as u32);

        // address of the byte to load from stable memory:
        let offset =
            BLOCK_BASE + block_id.0 as u64 * meta_data::size::BLOCK_IN_BYTES + intra_block_index;
        (
            offset,
            block_id,
            meta_data::size::BLOCK_IN_BYTES - intra_block_index,
        )
    }

    // compute the absolute offset, begin block, remaining length of
    // begin block, and end block for a relative offset and length.
    //
    // NB: BlockIds can be used to do case analysis (same or diff
    // block?) when planning successive reads/writes.
    pub unsafe fn relative_into_absolute_span(
        &self,
        offset: u64,
        len: u64,
    ) -> (u64, BlockId, u64, BlockId) {
        let (off, b1, b1_len) = self.relative_into_absolute_info(offset);
        if len == 0 {
            return (off, b1.clone(), b1_len, b1);
        };
        debug_assert!(u64::MAX - offset >= len - 1);
        let final_offset = offset + len - 1;
        let (_, b2, _) = self.relative_into_absolute_info(final_offset);
        (off, b1, b1_len, b2)
    }
}

// Mutable meta data stored in stable memory header (See motoko/design/StableRegions.md)
mod meta_data {

    pub const fn bytes_of<T>() -> u64 {
        core::mem::size_of::<T>() as u64
    }

    pub mod version {
        pub const MAGIC: &[u8; 8] = b"MOREGION";
        pub const VERSION: u32 = 2;
    }

    /// Maximum number of entities.
    pub mod max {
        pub const BLOCKS: u16 = 32 * 1024;
        pub const REGIONS: u64 = u64::MAX - 1;
    }

    /// Sizes of table entries, and tables.
    pub mod size {
        use super::bytes_of;

        pub const BLOCK_REGION_TABLE_ENTRY: u16 =
            (bytes_of::<u64>() + bytes_of::<u16>() + bytes_of::<u8>()) as u16;

        pub const BLOCK_REGION_TABLE: u64 =
            super::max::BLOCKS as u64 * BLOCK_REGION_TABLE_ENTRY as u64;

        pub const PAGES_IN_BLOCK: u32 = 128;
        pub const PAGE_IN_BYTES: u64 = 1 << 16;
        pub const BLOCK_IN_BYTES: u64 = PAGE_IN_BYTES * (PAGES_IN_BLOCK as u64);

        // Static memory footprint, ignoring any dynamically-allocated pages.
        pub unsafe fn static_mem_in_pages(block_base: u64) -> u64 {
            debug_assert!(block_base % PAGE_IN_BYTES as u64 == 0);
            debug_assert!(block_base != 0);
            block_base / PAGE_IN_BYTES as u64 /* meta data plus slack for future use */
        }

        pub unsafe fn total_required_pages(block_base: u64, total_allocated_blocks: u64) -> u64 {
            static_mem_in_pages(block_base) + (total_allocated_blocks * (PAGES_IN_BLOCK as u64))
        }
    }

    /// Offsets into stable memory for statically-sized fields and tables.
    pub mod offset {

        use super::bytes_of;

        pub const MAGIC: u64 = 0;

        pub const VERSION: u64 = MAGIC + bytes_of::<u64>();

        pub const BLOCK_PAGES: u64 = VERSION + bytes_of::<u32>();

        pub const BLOCK_BASE: u64 = BLOCK_PAGES + bytes_of::<u16>();

        pub const TOTAL_ALLOCATED_BLOCKS: u64 = BLOCK_BASE + bytes_of::<u64>();

        pub const TOTAL_ALLOCATED_REGIONS: u64 = TOTAL_ALLOCATED_BLOCKS + bytes_of::<u32>();

        pub const BLOCK_REGION_TABLE: u64 = TOTAL_ALLOCATED_REGIONS + bytes_of::<u64>();

        pub const FREE: u64 = BLOCK_REGION_TABLE + super::size::BLOCK_REGION_TABLE;

        pub const BASE_LOW: u64 = 16 * super::size::PAGE_IN_BYTES;

        pub const BASE_HIGH: u64 = super::size::BLOCK_IN_BYTES;
    }

    pub mod total_allocated_blocks {
        use super::offset;
        use crate::stable_mem::{read_u32, write_u32};

        use crate::region::REGION_TOTAL_ALLOCATED_BLOCKS;

        pub fn get() -> u32 {
            read_u32(offset::TOTAL_ALLOCATED_BLOCKS)
        }
        pub fn set(n: u32) {
            // Here we keep these copies of the total in sync.
            //
            // NB. The non-stable one is used when the stable one is
            // unavailable (temp relocated by stable variable
            // serialization/deserialization).
            unsafe {
                REGION_TOTAL_ALLOCATED_BLOCKS = n;
            };
            write_u32(offset::TOTAL_ALLOCATED_BLOCKS, n)
        }
    }

    pub mod total_allocated_regions {
        use super::offset;
        use crate::stable_mem::{read_u64, write_u64};

        pub fn get() -> u64 {
            read_u64(offset::TOTAL_ALLOCATED_REGIONS)
        }
        pub fn set(n: u64) {
            write_u64(offset::TOTAL_ALLOCATED_REGIONS, n)
        }
    }

    pub mod block_region_table {
        // invariant: all blocks whose IDs are below the total_allocated_blocks are valid.

        use super::{bytes_of, offset, size};
        use crate::region::{BlockId, RegionId};
        use crate::stable_mem::{read_u8, read_u16, read_u64, write_u8, write_u16, write_u64};

        // Compute an offset in stable memory for a particular block ID (based zero).
        fn index(b: &BlockId) -> u64 {
            offset::BLOCK_REGION_TABLE + (b.0 as u64 * size::BLOCK_REGION_TABLE_ENTRY as u64)
        }

        /// Some(r, j, c) means that the block is in use by region r, at slot j with c allocated pages.
        /// None means that the block is available for (re-)allocation.
        pub fn get(b: BlockId) -> Option<(RegionId, u16, u8)> {
            let region_offset = index(&b);
            let rank_offset = region_offset + bytes_of::<u64>();
            let page_count_offset = rank_offset + bytes_of::<u16>();
            let region = read_u64(region_offset);
            let rid = RegionId::from_u64(region);
            rid.map(|r| (r, read_u16(rank_offset), read_u8(page_count_offset)))
        }

        /// Some(r, j, c) means that the block is in use by region r, at slot j with c allocated pages.
        /// None means that the block is available for (re-)allocation.
        pub fn set(b: BlockId, r: Option<(RegionId, u16, u8)>) {
            let region_offset = index(&b);
            let rank_offset = region_offset + bytes_of::<u64>();
            let page_count_offset = rank_offset + bytes_of::<u16>();
            let (region_id, rank, allocated_pages) = match r {
                None => (None, 0, 0),
                Some((r, j, c)) => (Some(r), j, c),
            };
            write_u64(region_offset, RegionId::into_u64(region_id));
            write_u16(rank_offset, rank);
            write_u8(page_count_offset, allocated_pages);
        }
    }
}

unsafe fn write_magic() {
    use crate::stable_mem::{size, write, write_u16, write_u32, write_u64};
    assert!(size() > 0);
    assert!(BLOCK_BASE >= meta_data::offset::FREE);
    write(meta_data::offset::MAGIC, meta_data::version::MAGIC);
    write_u32(meta_data::offset::VERSION, meta_data::version::VERSION);
    write_u16(
        meta_data::offset::BLOCK_PAGES,
        meta_data::size::PAGES_IN_BLOCK as u16,
    );
    write_u64(meta_data::offset::BLOCK_BASE, BLOCK_BASE);
}

#[ic_mem_fn]
unsafe fn alloc_region<M: Memory>(
    mem: &mut M,
    id: u64,
    page_count: usize,
    vec_pages: Value,
) -> Value {
    let r_ptr = mem.alloc_words(size_of::<Region>());
    // NB. cannot use as_region() here as we didn't write the header yet
    let region = r_ptr.get_ptr() as *mut Region;
    (*region).header.tag = TAG_REGION;
    (*region).header.init_forward(r_ptr);
    debug_assert!(id <= meta_data::max::REGIONS);
    region.write_id64(id);
    debug_assert!(
        page_count
            <= (vec_pages.as_blob().len().as_usize() / meta_data::bytes_of::<u16>() as usize)
                * meta_data::size::PAGES_IN_BLOCK as usize
    );
    (*region).page_count = page_count;
    init_with_barrier(mem, &mut (*region).vec_pages, vec_pages);

    allocation_barrier(r_ptr);
    r_ptr
}

#[ic_mem_fn(ic_only)]
unsafe fn init_region<M: Memory>(
    mem: &mut M,
    r: Value,
    id: u64,
    page_count: usize,
    vec_pages: Value,
) {
    let r = r.as_region();
    debug_assert!(id <= meta_data::max::REGIONS);
    r.write_id64(id);
    debug_assert!(
        page_count
            <= (vec_pages.as_blob().len().as_usize() / meta_data::bytes_of::<u16>() as usize)
                * meta_data::size::PAGES_IN_BLOCK as usize
    );
    (*r).page_count = page_count;
    write_with_barrier(mem, &mut (*r).vec_pages, vec_pages);
}

#[ic_mem_fn]
pub unsafe fn region_id<M: Memory>(_mem: &mut M, r: Value) -> u64 {
    let r = r.as_untagged_region();
    r.read_id64()
}

#[ic_mem_fn]
pub unsafe fn region_page_count<M: Memory>(_mem: &mut M, r: Value) -> usize {
    let r = r.as_untagged_region();
    (*r).page_count
}

#[ic_mem_fn]
pub unsafe fn region_vec_pages<M: Memory>(_mem: &mut M, r: Value) -> Value {
    let r = r.as_untagged_region();
    (*r).vec_pages
}

// Helper for commmon logic that reserves low-valued RegionIds in a certain span for future use.
// When first is some, we are actually reserving.  When first is none, we are checking that the reservation has occured.
unsafe fn region_reserve_id_span<M: Memory>(
    _mem: &mut M,
    first_opt: Option<RegionId>,
    last: RegionId,
) {
    let next_id = meta_data::total_allocated_regions::get();
    match first_opt {
        Some(first) => {
            assert_eq!(first.0 as u64, next_id);
            assert!(first.0 <= last.0);
            meta_data::total_allocated_regions::set((last.0 + 1) as u64);
        }
        None => {
            assert!((last.0 as u64) < next_id);
        }
    }
}

#[ic_mem_fn]
pub unsafe fn region0_get<M: Memory>(_mem: &mut M) -> Value {
    debug_assert_ne!(REGION_0, NO_REGION);
    REGION_0
}

// Expose Region0 object to GC algorithms as root
#[allow(dead_code)]
#[cfg(feature = "ic")]
pub(crate) unsafe fn region0_get_ptr_loc() -> *mut Value {
    use core::ptr::addr_of_mut;

    addr_of_mut!(REGION_0)
}

#[classical_persistence]
unsafe fn migrate_on_new_region<M: Memory>(mem: &mut M) {
    match crate::stable_mem::get_version() {
        LEGACY_VERSION_NO_STABLE_MEMORY => {
            assert_eq!(crate::stable_mem::size(), 0);
            region_migration_from_no_stable_memory(mem);
        }
        LEGACY_VERSION_SOME_STABLE_MEMORY => {
            region_migration_from_some_stable_memory(mem);
        }
        LEGACY_VERSION_REGIONS => {}
        _ => {
            assert!(false);
        }
    }
}

#[enhanced_orthogonal_persistence]
unsafe fn migrate_on_new_region<M: Memory>(mem: &mut M) {
    match crate::stable_mem::get_version() {
        VERSION_STABLE_HEAP_NO_REGIONS | VERSION_GRAPH_COPY_NO_REGIONS => {
            if crate::stable_mem::size() == 0 {
                region_migration_from_no_stable_memory(mem);
            } else {
                region_migration_from_some_stable_memory(mem);
            }
        }
        VERSION_STABLE_HEAP_REGIONS | VERSION_GRAPH_COPY_REGIONS => {}
        _ => {
            assert!(false);
        }
    }
}

#[ic_mem_fn]
pub unsafe fn region_new<M: Memory>(mem: &mut M) -> Value {
    migrate_on_new_region(mem);

    let next_id = meta_data::total_allocated_regions::get();

    if next_id == meta_data::max::REGIONS {
        region_trap_with("out of regions")
    };

    meta_data::total_allocated_regions::set(next_id + 1);

    let vec_pages = alloc_blob(mem, TAG_BLOB_B, Bytes(0));
    allocation_barrier(vec_pages);
    let r_ptr = alloc_region(mem, next_id, 0, vec_pages);

    r_ptr
}

pub unsafe fn region_recover<M: Memory>(mem: &mut M, rid: &RegionId) -> Value {
    use meta_data::bytes_of;
    use meta_data::size::PAGES_IN_BLOCK;

    if rid.0 >= meta_data::total_allocated_regions::get() {
        region_trap_with("cannot recover un-allocated region");
    };

    let tb = meta_data::total_allocated_blocks::get();

    // determine page_count of this region
    let mut page_count: u32 = 0;
    {
        for block_id in 0..tb as u16 {
            match meta_data::block_region_table::get(BlockId(block_id)) {
                None => {}
                Some((rid_, _rank, block_page_count)) => {
                    if &rid_ == rid {
                        page_count += block_page_count as u32;
                    }
                }
            }
        }
    };
    debug_assert!(page_count < (u32::MAX - (PAGES_IN_BLOCK - 1)));

    let block_count = (page_count + PAGES_IN_BLOCK - 1) / PAGES_IN_BLOCK;
    let vec_pages = alloc_blob(
        mem,
        TAG_BLOB_B,
        Bytes(block_count as usize * bytes_of::<u16>() as usize),
    );

    let av = AccessVector(vec_pages.as_blob_mut());
    let mut recovered_blocks = 0;
    let mut block_id: u16 = 0;
    while recovered_blocks < block_count && (block_id as u32) < tb {
        match meta_data::block_region_table::get(BlockId(block_id)) {
            None => {}
            Some((rid_, rank, _page_count)) => {
                if &rid_ == rid {
                    av.set_ith_block_id(rank.into(), &BlockId(block_id));
                    recovered_blocks += 1;
                }
            }
        }
        block_id += 1;
    }
    assert_eq!(recovered_blocks, block_count);
    allocation_barrier(vec_pages);

    let r_ptr = alloc_region(mem, rid.0, page_count as usize, vec_pages);
    r_ptr
}

fn upgrade_version_to_regions() {
    let new_version = if uses_enhanced_orthogonal_persistence!() {
        match crate::stable_mem::get_version() {
            VERSION_STABLE_HEAP_NO_REGIONS => VERSION_STABLE_HEAP_REGIONS,
            VERSION_GRAPH_COPY_NO_REGIONS => VERSION_GRAPH_COPY_REGIONS,
            _ => unreachable!(),
        }
    } else {
        LEGACY_VERSION_REGIONS
    };
    crate::stable_mem::set_version(new_version);
}

pub(crate) unsafe fn region_migration_from_no_stable_memory<M: Memory>(mem: &mut M) {
    use crate::stable_mem::{get_version, grow, size, write};
    use meta_data::size::{PAGE_IN_BYTES, PAGES_IN_BLOCK};

    if uses_enhanced_orthogonal_persistence!() {
        assert!(
            get_version() == VERSION_STABLE_HEAP_NO_REGIONS
                || get_version() == VERSION_GRAPH_COPY_NO_REGIONS
        );
    } else {
        assert!(get_version() == LEGACY_VERSION_NO_STABLE_MEMORY);
    }

    assert_eq!(size(), 0);

    // pages required for meta_data (9/ 960KiB), much less than PAGES_IN_BLOCK (128/ 8MB) for a full block
    let meta_data_pages =
        (meta_data::offset::FREE - 1 + (PAGE_IN_BYTES as u64 - 1)) / PAGE_IN_BYTES as u64;

    assert!(meta_data_pages <= PAGES_IN_BLOCK as u64);

    // initially, only allocate meta_data_pages, not a full block, to reduce overhead for
    // canisters that don't allocate regions
    let prev_pages = grow(meta_data_pages);

    if prev_pages == u64::MAX {
        region_trap_with("migration failure (insufficient pages)");
    };

    debug_assert!(prev_pages == 0);

    // Zero metadata region, for good measure.
    let zero_page: [u8; PAGE_IN_BYTES as usize] = [0; PAGE_IN_BYTES as usize];
    for i in 0..meta_data_pages {
        write((i as u64) * (PAGE_IN_BYTES as u64), &zero_page);
    }

    BLOCK_BASE = meta_data::offset::BASE_LOW;

    // Write magic header
    write_magic();

    upgrade_version_to_regions();

    // Region 0 -- classic API for stable memory, as a dedicated region.
    REGION_0 = region_new(mem);

    assert_eq!(REGION_0.as_region().read_id64(), 0);

    // Regions 1 through LAST_RESERVED_REGION_ID, reserved for future use by future Motoko compiler-RTS features.
    region_reserve_id_span(mem, Some(RegionId(1)), RegionId(LAST_RESERVED_REGION_ID));
}

pub fn block_page_count(rank: u16, block_count: u32, page_count: u32) -> u8 {
    use meta_data::size::PAGES_IN_BLOCK;
    debug_assert!(block_count > 0);
    debug_assert!((rank as u32) < block_count);
    debug_assert_eq!(
        block_count,
        (page_count + (PAGES_IN_BLOCK as u32 - 1)) / meta_data::size::PAGES_IN_BLOCK
    );
    if (rank as u32) == block_count - 1 {
        // final, full or partial block
        let rem = page_count - ((block_count - 1) * PAGES_IN_BLOCK as u32);
        debug_assert!(rem <= PAGES_IN_BLOCK);
        rem as u8
    } else {
        // internal, full block
        meta_data::size::PAGES_IN_BLOCK as u8
    }
}

//
// region manager migration/initialization, with pre-existing stable data.
// Case: Version 1 into version 2.
//
pub(crate) unsafe fn region_migration_from_some_stable_memory<M: Memory>(mem: &mut M) {
    // Grow region0 to nearest block boundary, and add a block to fit a region manager meta data.
    //
    // Existing stable data becomes region 0, with first block relocated for region manager meta data.
    //
    // - allocate a block-sized blob on the heap (8MB).
    // - copy the head block of data into that blob, using a stable memory read of a blob.
    // - copy the head block of data from temp blob into new "final block" (logically still first) for region 0.
    // - initialize the meta data for the region system in vacated initial block.

    use crate::stable_mem::{grow, read, size, write};

    let header_len = meta_data::size::BLOCK_IN_BYTES as u32;

    let stable_mem_pages = size();

    if stable_mem_pages > (meta_data::size::PAGES_IN_BLOCK * meta_data::max::BLOCKS as u32) as u64 {
        region_trap_with("migration failure (too many pages for region0 )")
    };

    let region0_pages = stable_mem_pages as u32;

    let region0_blocks =
        (region0_pages + (meta_data::size::PAGES_IN_BLOCK - 1)) / (meta_data::size::PAGES_IN_BLOCK);
    assert!(region0_blocks > 0);

    let prev_pages = grow(
        (meta_data::size::PAGES_IN_BLOCK + // <-- For new region manager
	 /* Bump out region0 to nearest block boundary: */
	 region0_blocks * meta_data::size::PAGES_IN_BLOCK
            - region0_pages)
            .into(),
    );

    if prev_pages == u64::MAX {
        region_trap_with("migration failure (insufficient pages)")
    };

    debug_assert!(prev_pages == region0_pages.into());

    // Temp for the head block, which we move to be physically last.
    // NB: no allocation_barrier is required: header_val is temporary and can be reclaimed by the next GC increment/run.
    // TODO: instead of allocating an 8MB blob, just stack-allocate a tmp page and zero page, and transfer/zero-init via the stack, using a loop.
    let header_val =
        crate::memory::alloc_blob(mem, TAG_BLOB_B, crate::types::Bytes(header_len as usize));
    let header_blob = header_val.as_blob_mut();
    let header_bytes =
        core::slice::from_raw_parts_mut(header_blob.payload_addr(), header_len as usize);

    // Move it:
    read(0, header_bytes); // Save first block as "head block".
    write(
        (region0_blocks * (meta_data::size::BLOCK_IN_BYTES as u32)).into(),
        header_bytes,
    );

    crate::mem_utils::memzero_bytes(
        (header_blob.payload_addr()) as usize,
        Bytes(header_len as usize),
    );

    write(0, header_bytes); // Zero out first block, for region manager meta data.

    BLOCK_BASE = meta_data::offset::BASE_HIGH;
    // Write magic header
    write_magic();

    /* Initialize meta data as if there is only region 0. */
    meta_data::total_allocated_blocks::set(region0_blocks.into());
    meta_data::total_allocated_regions::set(1);

    /*
     * Initialize region0's table entries, for "recovery".
     */

    /* head block is physically last block, but logically first block in region. */
    let head_block_id: u16 = (region0_blocks - 1) as u16; /* invariant: region0_blocks is non-zero. */
    let head_block_region = RegionId(0);
    let head_block_rank: u16 = 0;
    let head_block_page_count: u8 =
        block_page_count(head_block_rank, region0_blocks, region0_pages);
    meta_data::block_region_table::set(
        BlockId(head_block_id as u16),
        Some((head_block_region, head_block_rank, head_block_page_count)),
    );

    /* Any other blocks that follow head block are numbered [0,...,head_block_id) */
    /* They're logical placements are [1,...,) -- one more than their new identity, as a number. */
    for i in 0..head_block_id {
        let rank = i + 1;
        let page_count: u8 = block_page_count(rank, region0_blocks, region0_pages);
        meta_data::block_region_table::set(BlockId(i), Some((RegionId(0), rank, page_count)))
    }

    upgrade_version_to_regions();

    /* "Recover" the region data into a heap object. */
    REGION_0 = region_recover(mem, &RegionId(0));

    // Ensure that regions 1 through LAST_RESERVED_REGION_ID are already reserved for
    // future use by future Motoko compiler-RTS features.
    region_reserve_id_span(mem, Some(RegionId(1)), RegionId(LAST_RESERVED_REGION_ID));
}

//
// region manager migration/initialization, with pre-existing stable data.
// Case: Version 2 into version 2 ("Trivial migration" case).
//
#[cfg(feature = "ic")]
pub(crate) unsafe fn region_migration_from_regions_plus<M: Memory>(mem: &mut M) {
    use crate::stable_mem::{read, read_u16, read_u32, read_u64, size};

    // Check if the magic in the memory corresponds to this object.
    assert!(size() > 1);
    let mut magic_bytes: [u8; 8] = [0; 8];
    read(meta_data::offset::MAGIC, &mut magic_bytes);
    if &magic_bytes != meta_data::version::MAGIC {
        region_trap_with("migration failure (bad magic bytes)")
    };
    let version = read_u32(meta_data::offset::VERSION);
    if version > meta_data::version::VERSION {
        region_trap_with("migration failure (unexpected higher version)")
    };

    let block_pages = read_u16(meta_data::offset::BLOCK_PAGES);
    if block_pages as u32 != meta_data::size::PAGES_IN_BLOCK {
        region_trap_with("migration failure (unexpected block size)")
    };

    let block_base = read_u64(meta_data::offset::BLOCK_BASE);
    if block_base < meta_data::offset::FREE {
        region_trap_with("migration failure (base too low)")
    };

    BLOCK_BASE = block_base;

    REGION_0 = region_recover(mem, &RegionId(0));

    // Ensure that regions 1 through LAST_RESERVED_REGION_ID are already reserved for
    // future use by future Motoko compiler-RTS features.
    region_reserve_id_span(mem, None, RegionId(LAST_RESERVED_REGION_ID));
}

//
// region manager migration/initialization, with pre-existing stable data.
//
#[classical_persistence]
#[ic_mem_fn(ic_only)]
pub(crate) unsafe fn region_init<M: Memory>(mem: &mut M, use_stable_regions: usize) {
    match crate::stable_mem::get_version() {
        LEGACY_VERSION_NO_STABLE_MEMORY => {
            assert!(crate::stable_mem::size() == 0);
            if use_stable_regions != 0 {
                region_migration_from_no_stable_memory(mem);
                debug_assert!(meta_data::offset::FREE < BLOCK_BASE);
                debug_assert!(BLOCK_BASE == meta_data::offset::BASE_LOW);
            };
        }
        LEGACY_VERSION_SOME_STABLE_MEMORY => {
            assert!(crate::stable_mem::size() > 0);
            if use_stable_regions != 0 {
                region_migration_from_some_stable_memory(mem);
                debug_assert!(meta_data::offset::FREE < BLOCK_BASE);
                debug_assert!(BLOCK_BASE == meta_data::offset::BASE_HIGH);
            };
        }
        _ => {
            region_migration_from_regions_plus(mem); //check format & recover region0
            debug_assert!(meta_data::offset::FREE < BLOCK_BASE);
            debug_assert!(
                BLOCK_BASE == meta_data::offset::BASE_LOW
                    || BLOCK_BASE == meta_data::offset::BASE_HIGH
            );
        }
    }
}

#[enhanced_orthogonal_persistence]
#[ic_mem_fn(ic_only)]
pub(crate) unsafe fn region_init<M: Memory>(mem: &mut M, use_stable_regions: usize) {
    match crate::stable_mem::get_version() {
        VERSION_STABLE_HEAP_NO_REGIONS | VERSION_GRAPH_COPY_NO_REGIONS => {
            if use_stable_regions != 0 {
                if crate::stable_mem::size() == 0 {
                    region_migration_from_no_stable_memory(mem);
                    debug_assert!(meta_data::offset::FREE < BLOCK_BASE);
                    debug_assert!(BLOCK_BASE == meta_data::offset::BASE_LOW);
                } else {
                    region_migration_from_some_stable_memory(mem);
                    debug_assert!(meta_data::offset::FREE < BLOCK_BASE);
                    debug_assert!(BLOCK_BASE == meta_data::offset::BASE_HIGH);
                }
            }
        }
        VERSION_STABLE_HEAP_REGIONS | VERSION_GRAPH_COPY_REGIONS => {
            region_migration_from_regions_plus(mem); //check format & recover region0
            debug_assert!(meta_data::offset::FREE < BLOCK_BASE);
            debug_assert!(
                BLOCK_BASE == meta_data::offset::BASE_LOW
                    || BLOCK_BASE == meta_data::offset::BASE_HIGH
            );
        }
        _ => assert!(false),
    }
}

#[ic_mem_fn]
pub unsafe fn region_size<M: Memory>(_mem: &mut M, r: Value) -> u64 {
    let r = r.as_region();
    (*r).page_count as u64
}

#[ic_mem_fn]
pub unsafe fn region_grow<M: Memory>(mem: &mut M, r: Value, new_pages: u64) -> u64 {
    use meta_data::size::{PAGES_IN_BLOCK, total_required_pages};

    let max_pages_in_region = meta_data::max::BLOCKS as u32 * PAGES_IN_BLOCK;

    let r = r.as_region();
    let old_page_count = (*r).page_count;

    if new_pages > (max_pages_in_region as usize - old_page_count) as u64 {
        return u64::MAX;
    }

    debug_assert!(max_pages_in_region <= u32::MAX);

    let new_pages_ = new_pages as u32;

    let old_block_count = (old_page_count as u32 + (PAGES_IN_BLOCK - 1)) / PAGES_IN_BLOCK;
    let new_block_count =
        (old_page_count as u32 + new_pages_ + (PAGES_IN_BLOCK - 1)) / PAGES_IN_BLOCK;
    let inc_block_count = new_block_count - old_block_count;

    // Determine the required total number of allocated blocks,
    let old_total_blocks = meta_data::total_allocated_blocks::get();
    let new_total_blocks = old_total_blocks as u64 + inc_block_count as u64;

    // Actually grow stable memory with more pages as required,
    // while respecting the global maximum limit on pages.
    {
        let have = crate::stable_mem::size();
        let need = total_required_pages(BLOCK_BASE, new_total_blocks);
        if have < need {
            let diff = need - have;
            if crate::stable_mem::grow(diff) == u64::MAX {
                return u64::MAX; // Propagate error
            }
        }
    }

    // Commit the allocation
    meta_data::total_allocated_blocks::set(new_total_blocks as u32);

    // Update this region's page count, in both places where we record it (heap object, region table).
    {
        let r_id = RegionId::from_id(r.read_id64());

        // Increase both:
        (*r).page_count += new_pages_ as usize;
        if old_block_count > 0 {
            let last_block_rank = (old_block_count - 1) as u16;
            let last_block_id =
                AccessVector((*r).vec_pages.as_blob_mut()).get_ith_block_id(last_block_rank as u32);
            debug_assert_eq!((*r).page_count, old_page_count + new_pages_ as usize);
            let last_page_count =
                block_page_count(last_block_rank, new_block_count, (*r).page_count as u32);
            let assoc = Some((r_id, last_block_rank, last_page_count));
            meta_data::block_region_table::set(last_block_id, assoc);
        }
    }

    let new_vec_pages = alloc_blob(
        mem,
        TAG_BLOB_B,
        Bytes(new_block_count as usize * meta_data::bytes_of::<u16>() as usize),
    );
    let old_vec_byte_count = old_block_count * meta_data::bytes_of::<u16>() as u32;

    // Copy old region-block associations into new heap object.
    crate::mem_utils::memcpy_bytes(
        new_vec_pages.as_blob_mut().payload_addr() as usize,
        (*r).vec_pages.as_blob().payload_const() as usize,
        Bytes(old_vec_byte_count as usize),
    );

    let new_pages = AccessVector::from_value(&new_vec_pages);

    // Record new associations, between the region and each new block:
    // - in block_region_table (stable memory, for persistence).
    // - in region representation (heap memory, for fast access operations).
    for i in old_block_count..new_block_count {
        // rel_i starts at zero.
        let rel_i: u16 = (i - old_block_count) as u16;

        // (to do -- handle case where allocating this way has run out.)
        let block_id: u16 = (old_total_blocks + rel_i as u32) as u16;

        // Update stable memory with new association.
        let block_page_count = block_page_count(i as u16, new_block_count, (*r).page_count as u32);
        let assoc = Some((RegionId::from_id(r.read_id64()), i as u16, block_page_count));
        meta_data::block_region_table::set(BlockId(block_id), assoc);

        new_pages.set_ith_block_id(i, &BlockId(block_id));
    }

    allocation_barrier(new_vec_pages);
    write_with_barrier(mem, &mut (*r).vec_pages, new_vec_pages);
    old_page_count as u64
}

pub(crate) unsafe fn region_load<M: Memory>(_mem: &mut M, r: Value, offset: u64, dst: &mut [u8]) {
    use crate::stable_mem::read;
    use meta_data::size::BLOCK_IN_BYTES;

    let r = RegionObject::from_value(&r);

    r.check_relative_range(offset, dst.len() as u64);

    if dst.len() == 0 {
        return;
    };

    let (b1_off, b1, b1_len, b2) = r.relative_into_absolute_span(offset, dst.len() as u64);
    if b1 == b2 {
        // Case: only uses a single block, and thus only requires one read.
        read(b1_off, dst);
    } else {
        // Case: Staged reads, one per block that holds requested data.

        let mut i = 0; // invariant: i = # of bytes loaded.
        let mut s = b1_off; // source of bytes, as absolute index.
        let mut d = dst.as_mut_ptr(); // dest for bytes.

        // do initial read (a special case, generally not full block length).
        read(s, core::slice::from_raw_parts_mut(d, b1_len as usize));

        // Advance input and output positions (i, d and s respectively).
        i += b1_len;
        d = d.offset(b1_len as isize);

        // Do rest of block-sized reads.
        // (invariant: they always occur at the start of a block).
        loop {
            let (s_, _, b_len) = r.relative_into_absolute_info(offset + i);
            s = s_;
            if i + b_len > dst.len() as u64 {
                // case: last (generally partial) block.
                if dst.len() as u64 > i {
                    read(
                        s,
                        core::slice::from_raw_parts_mut(d, dst.len() - i as usize),
                    );
                }
                break;
            } else {
                // case: internal (full) block.
                read(
                    s,
                    core::slice::from_raw_parts_mut(d, BLOCK_IN_BYTES as usize),
                );
                d = d.offset(BLOCK_IN_BYTES as isize);
                i += BLOCK_IN_BYTES;
            }
        }
    }
}

pub(crate) unsafe fn region_store<M: Memory>(_mem: &mut M, r: Value, offset: u64, src: &[u8]) {
    use crate::stable_mem::write;
    use meta_data::size::BLOCK_IN_BYTES;

    let r = RegionObject::from_value(&r);

    r.check_relative_range(offset, src.len() as u64);

    if src.len() == 0 {
        return;
    };

    let (b1_off, b1, b1_len, b2) = r.relative_into_absolute_span(offset, src.len() as u64);
    if b1 == b2 {
        write(b1_off, src);
    } else {
        // Case: Staged writes, one per block that holds requested data.

        let mut i = 0; // invariant: i = # of bytes stored.
        let mut s = src.as_ptr(); // source for bytes.
        let mut d = b1_off; // dest of bytes, as absolute index.o

        // do initial write (a special case, generally not full block length).
        write(d, core::slice::from_raw_parts(s, b1_len as usize));

        // Advance input and output positions (i, s and d respectively).
        i += b1_len;
        s = s.offset(b1_len as isize);

        // Do rest of block-sized writes.
        // (invariant: they always occur at the start of a block).
        loop {
            let (d_, _, b_len) = r.relative_into_absolute_info(offset + i);
            d = d_;
            if i + b_len > src.len() as u64 {
                // case: last (generally partial) block.
                if src.len() as u64 > i {
                    write(d, core::slice::from_raw_parts(s, src.len() - i as usize));
                }
                break;
            } else {
                // case: internal (full) block.
                write(d, core::slice::from_raw_parts(s, BLOCK_IN_BYTES as usize));
                s = s.offset(BLOCK_IN_BYTES as isize);
                i += BLOCK_IN_BYTES;
            }
        }
    }
}

// -- Region load operations.

#[ic_mem_fn]
pub unsafe fn region_load_word8<M: Memory>(mem: &mut M, r: Value, offset: u64) -> u32 {
    let mut bytes: [u8; 1] = [0];
    region_load(mem, r, offset, &mut bytes);
    core::primitive::u8::from_le_bytes(bytes).into()
}

#[ic_mem_fn]
pub unsafe fn region_load_word16<M: Memory>(mem: &mut M, r: Value, offset: u64) -> u32 {
    let mut bytes: [u8; 2] = [0; 2];
    region_load(mem, r, offset, &mut bytes);
    core::primitive::u16::from_le_bytes(bytes).into()
}

#[ic_mem_fn]
pub unsafe fn region_load_word32<M: Memory>(mem: &mut M, r: Value, offset: u64) -> u32 {
    let mut bytes: [u8; 4] = [0; 4];
    region_load(mem, r, offset, &mut bytes);
    core::primitive::u32::from_le_bytes(bytes).into()
}

#[ic_mem_fn]
pub unsafe fn region_load_word64<M: Memory>(mem: &mut M, r: Value, offset: u64) -> u64 {
    let mut bytes: [u8; 8] = [0; 8];
    region_load(mem, r, offset, &mut bytes);
    core::primitive::u64::from_le_bytes(bytes).into()
}

#[ic_mem_fn]
pub unsafe fn region_load_float64<M: Memory>(mem: &mut M, r: Value, offset: u64) -> f64 {
    let mut bytes: [u8; 8] = [0; 8];
    region_load(mem, r, offset, &mut bytes);
    core::primitive::f64::from_le_bytes(bytes).into()
}

#[ic_mem_fn(ic_only)]
pub(crate) unsafe fn region_load_blob<M: Memory>(
    mem: &mut M,
    r: Value,
    offset: u64,
    len: usize,
) -> Value {
    let blob_val = crate::memory::alloc_blob(mem, TAG_BLOB_B, crate::types::Bytes(len as usize));
    let blob = blob_val.as_blob_mut();

    if len < (isize::MAX as usize) {
        let bytes: &mut [u8] = core::slice::from_raw_parts_mut(blob.payload_addr(), len as usize);
        region_load(mem, r, offset, bytes);
    } else {
        assert!((len / 2) < isize::MAX as usize);
        let bytes_low: &mut [u8] =
            core::slice::from_raw_parts_mut(blob.payload_addr(), (len / 2) as usize);
        region_load(mem, r, offset, bytes_low);
        let bytes_high: &mut [u8] = core::slice::from_raw_parts_mut(
            blob.payload_addr().add((len / 2) as usize),
            (len - len / 2) as usize,
        );
        region_load(mem, r, offset + (len / 2) as u64, bytes_high);
    }
    allocation_barrier(blob_val);
    blob_val
}

// -- Region store operations.

#[ic_mem_fn]
pub unsafe fn region_store_word8<M: Memory>(mem: &mut M, r: Value, offset: u64, val: u32) {
    region_store(mem, r, offset, &core::primitive::u8::to_le_bytes(val as u8))
}

#[ic_mem_fn]
pub unsafe fn region_store_word16<M: Memory>(mem: &mut M, r: Value, offset: u64, val: u32) {
    region_store(
        mem,
        r,
        offset,
        &core::primitive::u16::to_le_bytes(val as u16),
    )
}

#[ic_mem_fn]
pub unsafe fn region_store_word32<M: Memory>(mem: &mut M, r: Value, offset: u64, val: u32) {
    region_store(
        mem,
        r,
        offset,
        &core::primitive::u32::to_le_bytes(val as u32),
    )
}

#[ic_mem_fn]
pub unsafe fn region_store_word64<M: Memory>(mem: &mut M, r: Value, offset: u64, val: u64) {
    region_store(mem, r, offset, &core::primitive::u64::to_le_bytes(val))
}

#[ic_mem_fn]
pub unsafe fn region_store_float64<M: Memory>(mem: &mut M, r: Value, offset: u64, val: f64) {
    region_store(mem, r, offset, &core::primitive::f64::to_le_bytes(val))
}

#[ic_mem_fn(ic_only)]
pub(crate) unsafe fn region_store_blob<M: Memory>(mem: &mut M, r: Value, offset: u64, blob: Value) {
    let blob = blob.as_blob();
    let len = blob.len().0;
    let bytes = blob.payload_const();
    if len < (isize::MAX as usize) {
        let bytes: &[u8] = core::slice::from_raw_parts(bytes, len as usize);
        region_store(mem, r, offset, bytes);
    } else {
        assert!((len / 2) < isize::MAX as usize);
        let bytes_low: &[u8] = core::slice::from_raw_parts(bytes, (len / 2) as usize);
        region_store(mem, r, offset, bytes_low);
        let bytes_high: &[u8] =
            core::slice::from_raw_parts(bytes.add((len / 2) as usize), (len - len / 2) as usize);
        region_store(mem, r, offset + (len / 2) as u64, bytes_high);
    }
}
