# Stable Region API

See StableMemory.md for context of the current experimental API.

This document aims to specify the API and memory representations for a generalization
of this API that permits multiple isolated _regions_ of stable memory, where each can be
grown independently.

The **region manager** is the state and logic to support this generalization.


## Role for "stable regions" in Motoko

The current stable memory module in `base` has been "experimental" for a long time, and requires a more composable API to graduate from this status.

Stable regions address the problem that the deprecated `ExperimentalStableMemory` module only provided a single, monolithic memory that makes it unsuitable for directly building composable software parts.

Stable regions permit a new API that supports composable use cases.

Stable regions also bring Motoko closer to parity with Rust canister development support today, by giving a run-time-system-based analog of a special Rust library for stable data structures that allocates “pages” for them from stable memory in separate, isolated, memory regions.


## Design space

The design space for the page allocator is defined by at least two
tensions:

 1. fully-stable representation of allocator meta data **versus** fast load/store operations.

 2. Total scaling capacity **versus** minimum footprint for meta data.


**Tension 1** is introduced because we want to avoid relying on the Motoko heap as the "ground truth" about the allocator's state.  If this heap is lost, as it is during an upgrade, then a developer may still want to recover all of the regions' data and meta data.

Tension 1 is resolved by storing the ground truth in stable memory, keeping it in sync with heap structures that permit faster access operations.

Compared with the Rust version, we store enough extra meta data to permit:

 - Regions whose page blocks are in arbitrary order, not
   necessarily in order of smallest to highest address.

 - 2^64-1 Regions max (instead of 255 Regions max).
   Due to the limit on blocks, only 2^16-1 can have non-zero page size.

We want to permit arbitrary page block orders to make a smooth
transition to region reclamation and re-allocation in the near
future, with potential integration into the Motoko GC.  The
extra complexity is modest, and seems "worth" the cost.

We change the maximum region limit because 255 may be too small in
some extreme cases and incompatible with GC.
Instead, we can freely allocate new regions, recycling blocks, but not
Region ids. The id of a Region is invariant and will not change, even with GC.

We address the question of whether the new limit of 32k regions is
"enough" in the Q&A section (it is, for all practical purposes)


**Tension 2** is introduced because we want a design that will continue
to work even when canisters can store more stable data than today (32GB).

Tension 2 is resolved by making prudent representation choices.

The representations we choose for regions and region identifiers
permit a scaling to 256GB of stable data while still permitting meta
data to be repeated in both stable and non-stable arenas.  These are
the same limits imposed by the Rust implementation, for the same
reasons.  See Q&A for more discussion.


## Definitions and constants

 - a **page** is 65536 bytes.
 - a **page block** is a contiguous sequence of 128 pages (~8MB).
 - a **page block index** is a 16 bit, index-based identifier for a page block.
 - a **region** is a sequence of (generally non-contiguous) **page blocks**.
 - the maximum number of page blocks is 32768.
 - the maximum amount of stable memory for all regions is 256GB.


## Questions and answers

### Q: What determines the 8MB non-empty region minimum?

Mostly, we want to reduce the amount of metadata we need to track, so instead of per-page metadata (lots) we only need per-block metadata (less).
This size means we grow a region by more than one physical page at
a time (in terms of the way that the canister interacts with the
system API, at least).  Rather than actually grow by a single page,
the implementation grows by a "page block" (8MB) at a time.

This choice means that there are 128 pages per page block, and that
the maximum number of regions and region blocks are each relatively
small (32k each).  Consequently, they can each be identified with a
2-byte identifier, and we can pre-allocate tables to store certain
relations about them, which is critical.

### Q: Are 32767 regions enough?

A: Permitting more than 32k regions may seem theoretically
interesting, but is not practical given other parameters today that
control the minimal footprint of a region (8MB) and dictate the
maximum size of stable memory for a canister today (32GB).  With 32k
regions at 8MB each, well over the maximum stable memory size is used
(256GB compared to 32GB, the limit today)

### Q: When is stable memory becoming less costly?

Spring 2023.

### Q: How does the cheaper stable memory access cost compare with ordinary heap memory access cost?

2-3x slower than ordinary heap memory.


## Design details

### API

Internal region allocator operations:

 - `initialize` -- called by the RTS, not by the Motoko developer.

User-facing region allocator operations:

 - `region_new` -- create a dynamic region.
 - `region_grow` -- grow region by a specified number of pages.
 - `region_load` -- read some data from the region.
 - `region_store` -- store some data into the region.

### FUTURE WORK

Add a special operation, for testing our design for future GC integration (bonus):

- `region_release` -- release region and reuse region's page blocks.

The `_release` operation is *not* part of the user-facing API nor part of the MVP,
but supporting it is important because it means we can transition quickly to an integration
with the ambient Motoko GC if we can support it.

Another special operation, for disaster recovery:

 - `rebuild` -- not needed unless we need to recreate all Region objects from their stable-memory counterparts.


## Internal footprint

The state of the allocator is stored in a combination of:

 - stable memory fields and tables and
 - stable heap memory, in the form of objects of opaque type `Region`.

The stable memory state is sufficient to reconstitute the stable heap objects
(`rebuild` operation, described in a subsection below).

That means that even if the stable parts of the heap are lost, the
stable memory state can fully describe the region objects that will be rebuilt when it succeeds.

### stable memory fields

 - total allocated blocks, `u16`, max value is `32768`.
 - total allocated regions, `u64`, max value is 2^64-1 (one region is reserved for "no region" in block-region table).
 - The `block` table (fixed size, about 6 pages).

### representation of values of type `Region`

 - A singleton, heap-allocated object with mutable fields.
 - While being heap-allocated, the object is also `stable` (can be stored in a `stable var`, etc).
 - `RegionObject { id_lower: u32, id_upper: u32; mut page_count: u32; mut vec_pages: Value }`
 - Fields id_lower (lower 32-bits)  and id_upper (upper 32-bits) gives the Region's numerical 64-bit (id = (id_upper \<\< 32 | id_lower)).
 - Field `page_count` gives the number of pages allocated to the Region.
 - Field `vec_pages` points at a heap-allocated `Blob` value, and it works with `page_count`
   to represent a growable vector that we call the region's **"access
   vector"** (because "blocks vector" sounds a bit strange, and its
   used to support O(1) access operations):
   - the access vector has `vec_capacity` slots.
   - each slot is a `u16`.
   - the first `page_count + 127 / 128` slots contain a valid page block ID for the region.
   - during an upgrade, the access vectors get serialized and deserialized as data `Blobs`.


### region-blocks relation

The `region-blocks` relation is not materialized into a table in stable memory (doing so with a pre-allocated table would be prohibitively large).

Instead, this relation is represented in two ways at the same time:
 1. by the set of heap-allocated region objects, and their access vectors.  The access vectors provide O(1) store and load support.
 2. by the `block-region` table, which together are sufficient to recreate all of the heap-allocated region objects.

In ordinary operation, the second feature is not required.  In the event of an upgrade failure, however, it could be vital (See `rebuild`).

### block-region table

 - purpose:
   - relate a block ID ("page block ID") to its region (if any), its position (or rank) in that region (see `rebuild`) and its current size in (used) pages (`<=128`).
     All but the last block owned by a region should have all pages 128 allocated.

   - NB: The organization of this table is not useful for efficient
     access calculations of load or store (they require a linear
     search that would be prohibitively slow).  OTOH, this
     table is suitable to do a "batch" rebuild of the dynamic heap-allocated vectors
     in that table, if need be (see `rebuild`).

 - 32768 entries (statically sized).
 - 8 (id) +2 (rank) + 1 (used) = 11 bytes per entry.
 - entry type = `BlockRegion { region : u64, position : u16, size: u8 }`
 - the location of each entry gives its corresponding block ID.


### Overview of `rebuild`

When upgrades work as expected, stable `Regions` are serialized and deserialized just like other stable data.

For disaster recovery, we can **also** rebuild all of the region objects from data in stable memory.

We use the `block-region` tables in stable memory to rebuild the regions' objects:

 - The `block-region` table gives a relative position and region ID for each block ID together with utilized page count.

Once each regions' vectors have been sized (by a linear scan of block-region, summing block sizes) and allocated, the block-region table says how to fill them, one entry at a time.
Unlike the Rust design, vector entries can be populated out-of-order.

Currently, we need only recover region 0 (when upgrading).


### Special (reserved) regions

  - Region 0 -- Anonymous region, for supporting the legacy API that we have today, which lacks `Region` values.
  - Region 1 -- "Reclaimed blocks" region that consists of blocks reclaimed from GC'd regions.
  - Regions 2-15 -- Future use by Motoko RTS (TBD).

### Overview of GC support (future work)

- Regions are represented (see special subsection) with heap objects that are `stable`, but mutable.
- They have special GC headers to recognize their special structure.
- The block-region table (or a more transient bitmap)  keeps track of which blocks are in use as Region heap values are GC'd.

Blocks can be marked during normal GC, with unmarked blocks returned to a transient free-list. In this design, blocks are recycled during
the lifetime of a single canister version.

Alternatively, Blocks can be marked only during deserialization after an upgrade, for bespoke, Region-only, GC during upgrades, with unmarked blocks
returned to a free list.
In this design, blocks are only recycled during upgrade from one version to the next, meaning long-lived canisters that create garbage regions will leak
space.

### Migration from earlier designs into region system

#### Version overview

Including this design, there are three possible verions (`0`, `1`, or `2`):

 0. Stable vars only.
 1. Stable vars *plus* direct access to IC0 API, including `grow`.
    This access is exposed via the Experimental Stable Memory API.
 2. This Region system, where direct access still works through region zero.


#### Key points

- Version 0:
  - will never be forced to "migrate" to other versions (assuming no stable memory API use).
  - will never incur the space overhead of the region system.

- Migration from 0 to version 1 occurs when:
  - Experimental Stable Memory function `grow` is invoked for the first time.
  This will not incur the space overhead of the region system.

- Migration from version 0 or version 1 to version 2 occurs when:
  - An initial region is allocated via `Region.new`.
  This will incur the space overhead of the region system.
  The space overhead is 16 pages (1MB) when migration from version 0, and 128 pages (8MiB) when migrating from version 1.

#### Compiler flag

Compiler flag

```
  --stable-regions
```

Affects upgrades only and forces migration directly into version 2 from version 0 or 1.
It is provided for testing purposes and *not* required for use of regions.

#### Format version details

The first 32 bits of stable memory record a "marker," which indicates how to determine the "version number"
for Motoko stable memory.  This version number is stored either:
  - *implicitly*, when the marker is non-zero, and version is `0`.
  - *explicitly*, when the marker is zero, and version is stored elsewhere (but currently always `1`).

Including this design, there are three possible verions (`0`, `1`, or `2`).  See preceeding section.

In the first cases (`0`) and (`1`), we wish to *selectively* migrate into the region system (`2`), with its own internal versioning.


#### Opt-in mechanism

The opt-in mechanism for using version 2 consists of using:

 - dynamically calling `Region.new()` form a canister currently in version 0 or 1;
 - staticly specifying compiler flag `--stable-regions`. This is mostly useful for testing.

Critically,

1. The use of physical stable memory is pay-as-you-go: canisters that do not use regions do not pay for that priviledge.
2. There is no provision for "downgrading" back to earlier, pre-region systems.

##### Version 0 migration.

To migrate from version 0 to version 2, there is nothing additional to do for existing data.

The region system detects this case by measuring the zero-sized stable memory during its initialization and
starts allocating blocks from address 16*2^16 (1MiB overhead), leaving 10 pages unused for future use.

##### Version 1 migration.

Migrating version 1 stable memory renames it as "region 0" in Version 2.

Critically, to migrate from version 1, we must perserve existing data, but reorganize it slightly.

In effect, all existing data will retain its logical order as (pre-allocated) region 0.

To accomodate the meta data of the region system, we move the first block of region 0, physically.

Then, we reformat the first block of stable memory as the region meta data block.

The rest of the blocks become part of region 0, with its first block stored at the end of physical memory.

The region system starts allocating blocks from address 128*2^16 (8MiB overhead), leaving 122 pages unused for future use.

Since we do not move the remaining blocks of region 0, the first block of memory (excluding meta-data) is unused space.

This design ensures that an existing canister using very large amounts of experimental stable memory can be migrated with only constant-cost movement
of the first block (128 pages) of memory.

## Orthogonal Persistence

Stable regions can be used together with orthogonal persistence, see [Enhanced Orthogonal Persistence](OrthogonalPersistence.md).