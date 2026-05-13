//! Implements threaded compaction as described in "High-Performance Garbage Collection for
//! Memory-Constrained Environments" section 5.1.2, which is an improved version of the original
//! threaded compaction algorithm described in The Garbage Collection Handbook section 3.3.

pub mod bitmap;
pub mod mark_stack;

use bitmap::{BITMAP_ITER_END, alloc_bitmap, free_bitmap, get_bit, iter_bits, set_bit};
use mark_stack::{alloc_mark_stack, free_mark_stack, pop_mark_stack, push_mark_stack};

use crate::constants::WORD_SIZE;
use crate::mem_utils::memcpy_words;
use crate::memory::Memory;
use crate::types::*;
use crate::visitor::{classical::pointer_to_dynamic_heap, visit_pointer_fields};

use motoko_rts_macros::ic_mem_fn;

// Only designed for 32-bit.
const _: () = assert!(core::mem::size_of::<usize>() == core::mem::size_of::<u32>());

#[unsafe(no_mangle)]
#[cfg(feature = "ic")]
pub unsafe extern "C" fn initialize_compacting_gc() {
    crate::memory::ic::linear_memory::initialize();
}

#[ic_mem_fn(ic_only)]
unsafe fn schedule_compacting_gc<M: Memory>(mem: &mut M) {
    // 512 MiB slack for mark stack + allocation area for the next message
    let slack: u64 = 512 * 1024 * 1024;
    let heap_size_bytes: u64 =
        crate::constants::WASM32_HEAP_SIZE.as_usize() as u64 * WORD_SIZE as u64;
    // Larger than necessary to keep things simple
    let max_bitmap_size_bytes = heap_size_bytes / 32;
    // NB. `max_live` is evaluated in compile time to a constant
    let max_live: Bytes<u64> = Bytes(heap_size_bytes - slack - max_bitmap_size_bytes);

    if super::should_do_gc(max_live) {
        compacting_gc(mem);
    }
}

#[ic_mem_fn(ic_only)]
unsafe fn compacting_gc<M: Memory>(mem: &mut M) {
    use crate::memory::ic::{self, linear_memory};

    compacting_gc_internal(
        mem,
        ic::get_aligned_heap_base(),
        // get_hp
        || linear_memory::get_hp_unskewed(),
        // set_hp
        |hp| linear_memory::set_hp_unskewed(hp),
        ic::get_static_roots(),
        crate::continuation_table::continuation_table_loc(),
        crate::region::region0_get_ptr_loc(),
        // note_live_size
        |live_size| linear_memory::MAX_LIVE = ::core::cmp::max(linear_memory::MAX_LIVE, live_size),
        // note_reclaimed
        |reclaimed| linear_memory::RECLAIMED += Bytes(reclaimed.as_usize() as u64),
    );

    linear_memory::LAST_HP = linear_memory::get_hp_unskewed();
}

pub unsafe fn compacting_gc_internal<
    M: Memory,
    GetHp: Fn() -> usize,
    SetHp: Fn(usize),
    NoteLiveSize: Fn(Bytes<usize>),
    NoteReclaimed: Fn(Bytes<usize>),
>(
    mem: &mut M,
    heap_base: usize,
    get_hp: GetHp,
    set_hp: SetHp,
    static_roots: Value,
    continuation_table_ptr_loc: *mut Value,
    region0_ptr_loc: *mut Value,
    note_live_size: NoteLiveSize,
    note_reclaimed: NoteReclaimed,
) {
    let old_hp = get_hp();

    assert_eq!(heap_base % 32, 0);

    mark_compact(
        mem,
        set_hp,
        heap_base,
        old_hp,
        static_roots,
        continuation_table_ptr_loc,
        region0_ptr_loc,
    );

    let reclaimed = old_hp - get_hp();
    note_reclaimed(Bytes(reclaimed));

    let live = get_hp() - heap_base;
    note_live_size(Bytes(live));
}

unsafe fn mark_compact<M: Memory, SetHp: Fn(usize)>(
    mem: &mut M,
    set_hp: SetHp,
    heap_base: usize,
    heap_end: usize,
    static_roots: Value,
    continuation_table_ptr_loc: *mut Value,
    region0_ptr_loc: *mut Value,
) {
    let mem_size = Bytes(heap_end - heap_base);

    alloc_bitmap(mem, mem_size, heap_base / WORD_SIZE);
    alloc_mark_stack(mem);

    mark_static_roots(mem, static_roots, heap_base);

    if (*continuation_table_ptr_loc).is_ptr() {
        mark_object(mem, *continuation_table_ptr_loc);
        // Similar to `mark_root_mutbox_fields`, `continuation_table_ptr_loc` is in static heap so
        // it will be readable when we unthread the continuation table
        thread(continuation_table_ptr_loc);
    }

    if (*region0_ptr_loc).is_ptr() {
        mark_object(mem, *region0_ptr_loc);
        thread(region0_ptr_loc);
    }

    mark_stack(mem, heap_base);

    update_refs(set_hp, heap_base);

    free_mark_stack();
    free_bitmap();
}

unsafe fn mark_static_roots<M: Memory>(mem: &mut M, static_roots: Value, heap_base: usize) {
    let root_array = static_roots.as_array();

    // Static objects are not in the dynamic heap so don't need marking.
    for i in 0..root_array.len() {
        let obj = root_array.get(i).as_obj();
        // Root array should only have pointers to other static MutBoxes
        debug_assert_eq!(obj.tag(), TAG_MUTBOX); // check tag
        debug_assert!((obj as usize) < heap_base); // check that MutBox is static
        mark_root_mutbox_fields(mem, obj as *mut MutBox, heap_base);
    }
}

unsafe fn mark_object<M: Memory>(mem: &mut M, obj: Value) {
    let obj_tag = obj.tag();
    let obj = obj.get_ptr();

    // Check object alignment to avoid undefined behavior. See also static_checks module.
    debug_assert_eq!(obj % WORD_SIZE, 0);

    let obj_idx = obj / WORD_SIZE;

    if get_bit(obj_idx) {
        // Already marked
        return;
    }

    set_bit(obj_idx);
    push_mark_stack(mem, obj, obj_tag);
}

unsafe fn mark_stack<M: Memory>(mem: &mut M, heap_base: usize) {
    while let Some((obj, tag)) = pop_mark_stack() {
        mark_fields(mem, obj as *mut Obj, tag, heap_base)
    }
}

unsafe fn mark_fields<M: Memory>(mem: &mut M, obj: *mut Obj, obj_tag: Tag, heap_base: usize) {
    visit_pointer_fields(
        mem,
        obj,
        obj_tag,
        heap_base,
        |mem, field_addr| {
            let field_value = *field_addr;
            mark_object(mem, field_value);

            // Thread if backwards or self pointer
            if field_value.get_ptr() <= obj as usize {
                thread(field_addr);
            }
        },
        |mem, slice_start, arr| {
            const SLICE_INCREMENT: usize = 127;
            debug_assert!(SLICE_INCREMENT >= TAG_ARRAY_SLICE_MIN);
            if arr.len() - slice_start > SLICE_INCREMENT {
                let new_start = slice_start + SLICE_INCREMENT;
                // push an entire (suffix) array slice
                push_mark_stack(mem, arr as usize, new_start);
                new_start
            } else {
                arr.len()
            }
        },
    );
}

/// Specialized version of `mark_fields` for root `MutBox`es.
unsafe fn mark_root_mutbox_fields<M: Memory>(mem: &mut M, mutbox: *mut MutBox, heap_base: usize) {
    let field_addr = &mut (*mutbox).field;
    if pointer_to_dynamic_heap(field_addr, heap_base) {
        mark_object(mem, *field_addr);
        // It's OK to thread forward pointers here as the static objects won't be moved, so we will
        // be able to unthread objects pointed by these fields later.
        thread(field_addr);
    }
}

/// Linearly scan the heap, for each live object:
///
/// - Mark step threads all backwards pointers and pointers from roots, so unthread to update those
///   pointers to the objects new location.
///
/// - Move the object
///
/// - Thread forward pointers of the object
///
unsafe fn update_refs<SetHp: Fn(usize)>(set_hp: SetHp, heap_base: usize) {
    let mut free = heap_base;

    let mut bitmap_iter = iter_bits();
    let mut bit = bitmap_iter.next();
    while bit != BITMAP_ITER_END {
        let p = (bit * WORD_SIZE) as *mut Obj;
        let p_new = free;

        // Update backwards references to the object's new location and restore object header
        unthread(p, p_new);

        // Move the object
        let p_size_words = block_size(p as usize);
        if p_new as usize != p as usize {
            memcpy_words(p_new as usize, p as usize, p_size_words);

            debug_assert!(p_size_words.as_usize() > size_of::<Obj>().as_usize());
            // Update forwarding pointer
            let new_obj = p_new as *mut Obj;
            debug_assert!(new_obj.tag() >= TAG_OBJECT && new_obj.tag() <= TAG_NULL);
        }

        free += p_size_words.to_bytes().as_usize();

        // Thread forward pointers of the object
        thread_fwd_pointers(p_new as *mut Obj, heap_base);

        bit = bitmap_iter.next();
    }

    set_hp(free);
}

/// Thread forward pointers in object
unsafe fn thread_fwd_pointers(obj: *mut Obj, heap_base: usize) {
    visit_pointer_fields(
        &mut (),
        obj,
        obj.tag(),
        heap_base,
        |_, field_addr| {
            if (*field_addr).get_ptr() > obj as usize {
                thread(field_addr)
            }
        },
        |_, _, arr| arr.len(),
    );
}

/// Thread a pointer field
unsafe fn thread(field: *mut Value) {
    // Store pointed object's header in the field, field address in the pointed object's header
    let pointed = (*field).as_obj();
    let pointed_header = pointed.tag();
    *field = Value::from_raw(pointed_header);
    (*pointed).tag = field as usize;
}

/// Unthread all references at given header, replacing with `new_loc`. Restores object header.
unsafe fn unthread(obj: *mut Obj, new_loc: usize) {
    let mut header = obj.tag();

    // All objects and fields are word-aligned, and tags have the lowest bit set, so use the lowest
    // bit to distinguish a header (tag) from a field address.
    while header & 0b1 == 0 {
        let tmp = (header as *const Obj).tag();
        (*(header as *mut Value)) = Value::from_ptr(new_loc);
        header = tmp;
    }

    // At the end of the chain is the original header for the object
    debug_assert!(header >= TAG_OBJECT && header <= TAG_NULL);

    (*obj).tag = header;
}
