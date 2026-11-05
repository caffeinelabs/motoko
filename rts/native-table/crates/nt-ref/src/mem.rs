//! Memory by address, as the runtime of Rust EOP (a Rust library for upgrades that keep the heap in place)
//! reaches it, and the allocator the maps call.
//!
//! On wasm an address is the pointer itself (`addr as usize as *mut T`), as in a canister. Natively, and
//! under Miri with strict provenance, an address becomes a pointer derived from the thread's region
//! pointer, as Rust EOP's native host mode does, so every access keeps the region's provenance.
//!
//! The region is one allocation split into arenas. Each arena hands out 4,096-byte node slots (16-aligned,
//! like dlmalloc blocks) and box slots of two sizes, each kind from its own LIFO free list kept inside the
//! freed slots, then by bumping. `select` picks the arena the next allocations come from, so several maps that run the
//! same operations see the same allocator states.

use std::cell::Cell;

/// Bytes of a node slot: a node's 4,080 bytes and 16 the node format never touches.
pub const NODE_SLOT: u64 = 4_096;
/// Bytes of the two box slot sizes: an 8-byte prefix {payload bytes u32, layout u32}, then the payload. Small
/// slots hold strings' buffers, large ones boxed parts.
pub const BOX_SLOTS: [u64; 2] = [48, 1_040];
/// The largest boxed payload.
pub const BOX_MAX: u64 = 1_040 - 8;
/// Arenas in a region.
pub const ARENAS: usize = 3;

/// Traps: aborts on wasm, as a canister's trap ends the message; panics natively, so tests report it.
#[cold]
#[inline(never)]
pub fn trap(msg: &str) -> ! {
    #[cfg(target_family = "wasm")]
    {
        let _ = msg;
        std::process::abort()
    }
    #[cfg(not(target_family = "wasm"))]
    panic!("{msg}")
}

struct Arena {
    bump: Cell<u64>,
    end: Cell<u64>,
    node_free: Cell<u64>,
    box_free: [Cell<u64>; 2],
    nodes: Cell<u64>,
    boxes: Cell<u64>,
}

impl Arena {
    const fn new() -> Self {
        Arena {
            bump: Cell::new(0),
            end: Cell::new(0),
            node_free: Cell::new(0),
            box_free: [Cell::new(0), Cell::new(0)],
            nodes: Cell::new(0),
            boxes: Cell::new(0),
        }
    }
}

struct RegionState {
    base: Cell<*mut u8>,
    start: Cell<u64>,
    len: Cell<u64>,
    cur: Cell<usize>,
    arenas: [Arena; ARENAS],
}

thread_local! {
    static REGION: RegionState = const {
        RegionState {
            base: Cell::new(core::ptr::null_mut()),
            start: Cell::new(0),
            len: Cell::new(0),
            cur: Cell::new(0),
            arenas: [Arena::new(), Arena::new(), Arena::new()],
        }
    };
}

/// The thread's region: `ARENAS` arenas of `arena_bytes` each, zeroed. Dropping it frees the memory.
pub struct Region {
    base: *mut u8,
    layout: std::alloc::Layout,
}

impl Region {
    /// Installs a region of `ARENAS` · `arena_bytes` bytes for this thread; arena 0 is selected.
    pub fn install(arena_bytes: u64) -> Region {
        let arena_bytes = arena_bytes.div_ceil(NODE_SLOT) * NODE_SLOT;
        let total = arena_bytes * ARENAS as u64;
        let layout = std::alloc::Layout::from_size_align(total as usize, NODE_SLOT as usize)
            .unwrap_or_else(|_| trap("region too large"));
        // SAFETY: a non-zero size.
        let base = unsafe { std::alloc::alloc_zeroed(layout) };
        if base.is_null() {
            trap("region allocation failed");
        }
        let start = base as usize as u64;
        REGION.with(|r| {
            if !r.base.get().is_null() {
                trap("a region is installed already");
            }
            r.base.set(base);
            r.start.set(start);
            r.len.set(total);
            r.cur.set(0);
            for (i, a) in r.arenas.iter().enumerate() {
                let s = start + i as u64 * arena_bytes;
                a.bump.set(s);
                a.end.set(s + arena_bytes);
                a.node_free.set(0);
                a.box_free[0].set(0);
                a.box_free[1].set(0);
                a.nodes.set(0);
                a.boxes.set(0);
            }
        });
        Region { base, layout }
    }

    /// Empties arena i: its slots are free again, by bumping from its start. Every map over it is gone.
    pub fn reset(&self, i: usize) {
        REGION.with(|r| {
            let arena_bytes = r.len.get() / ARENAS as u64;
            let a = arena(r, i);
            let s = r.start.get() + i as u64 * arena_bytes;
            a.bump.set(s);
            a.node_free.set(0);
            a.box_free[0].set(0);
            a.box_free[1].set(0);
            a.nodes.set(0);
            a.boxes.set(0);
        });
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        REGION.with(|r| {
            r.base.set(core::ptr::null_mut());
            r.start.set(0);
            r.len.set(0);
        });
        // SAFETY: allocated in `install` with this layout.
        unsafe { std::alloc::dealloc(self.base, self.layout) }
    }
}

#[inline(always)]
fn arena(r: &RegionState, i: usize) -> &Arena {
    match r.arenas.get(i) {
        Some(a) => a,
        None => trap("no such arena"),
    }
}

/// Selects the arena the next allocations and hand-backs use.
#[inline(always)]
pub fn select(i: usize) {
    if i >= ARENAS {
        trap("no such arena");
    }
    REGION.with(|r| r.cur.set(i));
}

/// (live nodes, live boxes) of arena i.
pub fn live(i: usize) -> (u64, u64) {
    REGION.with(|r| {
        let a = arena(r, i);
        (a.nodes.get(), a.boxes.get())
    })
}

/// The pointer to the byte at `addr`.
#[cfg(target_family = "wasm")]
#[inline(always)]
pub fn at<T>(addr: u64) -> *mut T {
    addr as usize as *mut T
}

/// The pointer to the byte at `addr`, derived from the region's pointer.
#[cfg(not(target_family = "wasm"))]
#[inline(always)]
pub fn at<T>(addr: u64) -> *mut T {
    REGION.with(|r| {
        let start = r.start.get();
        if addr < start || addr - start >= r.len.get() {
            trap("address outside the region");
        }
        r.base.get().wrapping_add((addr - start) as usize).cast()
    })
}

/// Reads the little-endian u16 at `addr`.
///
/// # Safety
///
/// `[addr, addr + 2)` lies in memory.
#[inline(always)]
pub unsafe fn read_u16(addr: u64) -> u16 {
    // SAFETY: the caller's contract; `[u8; 2]` has alignment 1.
    u16::from_le_bytes(unsafe { at::<[u8; 2]>(addr).read() })
}

/// Writes `value` as the little-endian u16 at `addr`.
///
/// # Safety
///
/// `[addr, addr + 2)` lies in memory and nothing borrowed covers it.
#[inline(always)]
pub unsafe fn write_u16(addr: u64, value: u16) {
    // SAFETY: the caller's contract; `[u8; 2]` has alignment 1.
    unsafe { at::<[u8; 2]>(addr).write(value.to_le_bytes()) }
}

/// Reads the little-endian u32 at `addr`.
///
/// # Safety
///
/// `[addr, addr + 4)` lies in memory.
#[inline(always)]
pub unsafe fn read_u32(addr: u64) -> u32 {
    // SAFETY: the caller's contract; `[u8; 4]` has alignment 1.
    u32::from_le_bytes(unsafe { at::<[u8; 4]>(addr).read() })
}

/// Writes `value` as the little-endian u32 at `addr`.
///
/// # Safety
///
/// `[addr, addr + 4)` lies in memory and nothing borrowed covers it.
#[inline(always)]
pub unsafe fn write_u32(addr: u64, value: u32) {
    // SAFETY: the caller's contract; `[u8; 4]` has alignment 1.
    unsafe { at::<[u8; 4]>(addr).write(value.to_le_bytes()) }
}

/// Reads the little-endian u64 at `addr`.
///
/// # Safety
///
/// `[addr, addr + 8)` lies in memory.
#[inline(always)]
pub unsafe fn read_u64(addr: u64) -> u64 {
    // SAFETY: the caller's contract; `[u8; 8]` has alignment 1.
    u64::from_le_bytes(unsafe { at::<[u8; 8]>(addr).read() })
}

/// Writes `value` as the little-endian u64 at `addr`.
///
/// # Safety
///
/// `[addr, addr + 8)` lies in memory and nothing borrowed covers it.
#[inline(always)]
pub unsafe fn write_u64(addr: u64, value: u64) {
    // SAFETY: the caller's contract; `[u8; 8]` has alignment 1.
    unsafe { at::<[u8; 8]>(addr).write(value.to_le_bytes()) }
}

/// A node slot with the header {count 0, `flags`, `layout`}; bytes 8..4,080 are whatever the slot held.
#[inline(never)]
pub fn node_alloc(layout: u32, flags: u16) -> u64 {
    let n = REGION.with(|r| {
        let a = arena(r, r.cur.get());
        let head = a.node_free.get();
        let n = if head != 0 {
            // SAFETY: a freed slot holds the next free slot's address in its first word.
            a.node_free.set(unsafe { read_u64(head) });
            head
        } else {
            let n = a.bump.get();
            if a.end.get() - n < NODE_SLOT {
                trap("arena out of node slots");
            }
            a.bump.set(n + NODE_SLOT);
            n
        };
        a.nodes.set(a.nodes.get() + 1);
        n
    });
    // SAFETY: a slot of this arena, which nothing references.
    unsafe {
        write_u16(n, 0);
        write_u16(n + 2, flags);
        write_u32(n + 4, layout);
    }
    n
}

/// Returns node slot `n` to the selected arena.
///
/// # Safety
///
/// `n` came from `node_alloc` on the selected arena and nothing references it any more.
#[inline(never)]
pub unsafe fn hand_back(n: u64) {
    REGION.with(|r| {
        let a = arena(r, r.cur.get());
        // SAFETY: the caller's contract: the slot is ours to overwrite.
        unsafe { write_u64(n, a.node_free.get()) };
        a.node_free.set(n);
        a.nodes.set(a.nodes.get() - 1);
    });
}

/// The slot size class of a box of `payload` bytes.
#[inline(always)]
fn box_class(payload: u64) -> usize {
    usize::from(payload > BOX_SLOTS[0] - 8)
}

/// A box of `payload` ≤ `BOX_MAX` bytes; returns its payload address, ≡ 8 (mod 16).
#[inline(never)]
pub fn box_alloc(payload: u64) -> u64 {
    if payload > BOX_MAX {
        trap("allocation too large");
    }
    let class = box_class(payload);
    let size = BOX_SLOTS[class];
    REGION.with(|r| {
        let a = arena(r, r.cur.get());
        let free = &a.box_free[class];
        let head = free.get();
        let b = if head != 0 {
            // SAFETY: a freed box slot holds the next free slot's address in its first word.
            free.set(unsafe { read_u64(head) });
            head
        } else {
            let b = a.bump.get();
            if a.end.get() - b < size {
                trap("arena out of box slots");
            }
            a.bump.set(b + size);
            b
        };
        a.boxes.set(a.boxes.get() + 1);
        // SAFETY: a slot of this arena: its prefix {payload bytes, layout}.
        unsafe {
            write_u32(b, payload as u32);
            write_u32(b + 4, 3);
        }
        b + 8
    })
}

/// Returns the box at payload address `p` to the selected arena; its prefix names its size.
///
/// # Safety
///
/// `p` came from `box_alloc` on the selected arena and nothing references it any more.
#[inline(never)]
pub unsafe fn box_hand_back(p: u64) {
    REGION.with(|r| {
        let a = arena(r, r.cur.get());
        let b = p - 8;
        // SAFETY: the caller's contract: the slot and its prefix are ours.
        let class = box_class(u64::from(unsafe { read_u32(b) }));
        let free = &a.box_free[class];
        // SAFETY: as above.
        unsafe { write_u64(b, free.get()) };
        free.set(b);
        a.boxes.set(a.boxes.get() - 1);
    });
}
