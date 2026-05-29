use super::{MotokoHeapInner, heap_size_for_gc};

use crate::gc::utils::{GC, ObjectIdx, WORD_SIZE, make_pointer, make_scalar, write_word};

use motoko_rts::types::*;

use fxhash::{FxHashMap, FxHashSet};

pub(super) fn new_heap(
    map: &[(ObjectIdx, Vec<ObjectIdx>)],
    roots: &[ObjectIdx],
    continuation_table: &[ObjectIdx],
    gc: GC,
) -> MotokoHeapInner {
    // Check test correctness: an object should appear at most once in `map`
    {
        let heap_objects: FxHashSet<ObjectIdx> = map.iter().map(|(obj, _)| *obj).collect();
        assert_eq!(
            heap_objects.len(),
            map.len(),
            "Invalid test heap: some objects appear multiple times"
        );
    }

    // Each object will have array header plus one word for id per object + one word for each reference. Static heap will
    // have an array (header + length) with one element, one MutBox for each root. +1 for
    // continuation table pointer.
    let static_heap_size_bytes = (size_of::<Array>().as_usize()
        + roots.len()
        + (roots.len() * size_of::<MutBox>().as_usize())
        + 2)
        * WORD_SIZE;

    let dynamic_heap_size_without_roots = {
        let object_headers_words = map.len() * (size_of::<Array>().as_usize() + 1);
        let references_words = map.iter().map(|(_, refs)| refs.len()).sum::<usize>();
        (object_headers_words + references_words) * WORD_SIZE
    };

    let continuation_table_size = (size_of::<Array>() + Words(continuation_table.len()))
        .to_bytes()
        .as_usize();

    let region0_size = size_of::<Region>().to_bytes().as_usize();

    let dynamic_heap_size_bytes =
        dynamic_heap_size_without_roots + continuation_table_size + region0_size;

    let total_heap_size_bytes = static_heap_size_bytes + dynamic_heap_size_bytes;

    let heap_size = heap_size_for_gc(gc, total_heap_size_bytes, map.len());

    // The Worst-case unalignment w.r.t. 32-byte alignment is 28 (assuming
    // that we have general word alignment). So we over-allocate 28 bytes.
    let mut heap = vec![0u8; heap_size + 28];

    // Align the dynamic heap starts at a 32-byte multiple.
    let realign = (32 - (heap.as_ptr() as usize + static_heap_size_bytes) % 32) % 32;
    assert_eq!(realign % 4, 0);

    // Maps `ObjectIdx`s into their offsets in the heap.
    let object_addrs: FxHashMap<ObjectIdx, usize> = create_dynamic_heap(
        map,
        continuation_table,
        &mut heap[static_heap_size_bytes + realign..heap_size + realign],
    );

    // Closure table pointer is the second last word in the static heap.
    let continuation_table_ptr_offset = static_heap_size_bytes - WORD_SIZE * 2;

    // Region0 pointer is the very last word in the static heap.
    let region0_ptr_location_offset = static_heap_size_bytes - WORD_SIZE;

    create_static_heap(
        roots,
        &object_addrs,
        continuation_table_ptr_offset,
        static_heap_size_bytes + dynamic_heap_size_without_roots,
        region0_ptr_location_offset,
        static_heap_size_bytes + dynamic_heap_size_without_roots + continuation_table_size,
        &mut heap[realign..static_heap_size_bytes + realign],
    );

    MotokoHeapInner {
        heap: heap.into_boxed_slice(),
        heap_base_offset: static_heap_size_bytes + realign,
        _heap_ptr_last: static_heap_size_bytes + realign,
        heap_ptr_offset: total_heap_size_bytes + realign,
        static_root_array_variable_offset: realign,
        continuation_table_variable_offset: continuation_table_ptr_offset + realign,
        region0_pointer_variable_offset: region0_ptr_location_offset + realign,
    }
}

/// Given a heap description (as a map from objects to objects), and the dynamic part of the heap
/// (as an array), initialize the dynamic heap with objects.
///
/// Returns a mapping from object indices (`ObjectIdx`) to their addresses (see module
/// documentation for "offset" and "address" definitions).
fn create_dynamic_heap(
    refs: &[(ObjectIdx, Vec<ObjectIdx>)],
    continuation_table: &[ObjectIdx],
    dynamic_heap: &mut [u8],
) -> FxHashMap<ObjectIdx, usize> {
    let incremental = cfg!(feature = "incremental_gc");
    let heap_start = dynamic_heap.as_ptr() as usize;

    // Maps objects to their addresses
    let mut object_addrs: FxHashMap<ObjectIdx, usize> = Default::default();

    // First pass allocates objects without fields
    {
        let mut heap_offset = 0;
        for (obj, refs) in refs {
            object_addrs.insert(*obj, heap_start + heap_offset);

            // Store object header
            let address = heap_start + heap_offset;
            write_word(dynamic_heap, heap_offset, TAG_ARRAY_M);
            heap_offset += WORD_SIZE;

            if incremental {
                write_word(dynamic_heap, heap_offset, make_pointer(address)); // forwarding pointer
                heap_offset += WORD_SIZE;
            }

            // Store length: idx + refs
            write_word(dynamic_heap, heap_offset, refs.len() + 1);
            heap_offset += WORD_SIZE;

            // Store object value (idx)
            write_word(dynamic_heap, heap_offset, make_scalar(*obj));
            heap_offset += WORD_SIZE;

            // Leave space for the fields
            heap_offset += refs.len() * WORD_SIZE;
        }
    }

    // println!("object addresses={:#?}", object_addrs);

    // Second pass adds fields
    for (obj, refs) in refs {
        let obj_offset = object_addrs.get(obj).unwrap() - heap_start;
        for (ref_idx, ref_) in refs.iter().enumerate() {
            let ref_addr = make_pointer(*object_addrs.get(ref_).unwrap());
            let field_offset = obj_offset
                + (size_of::<Array>() + Words(1 + ref_idx))
                    .to_bytes()
                    .as_usize();
            write_word(dynamic_heap, field_offset, ref_addr);
        }
    }

    // Add the continuation table
    let n_objects = refs.len();
    // fields+1 for the scalar field (idx)
    let n_fields: usize = refs.iter().map(|(_, fields)| fields.len() + 1).sum();
    let continuation_table_offset =
        (size_of::<Array>() * n_objects).to_bytes().as_usize() + n_fields * WORD_SIZE;
    let continuation_table_size =
        size_of::<Array>().to_bytes().as_usize() + continuation_table.len() * WORD_SIZE;

    {
        let mut heap_offset = continuation_table_offset;

        let continuation_table_address = heap_start + heap_offset;
        write_word(dynamic_heap, heap_offset, TAG_ARRAY_M);
        heap_offset += WORD_SIZE;

        if incremental {
            write_word(
                dynamic_heap,
                heap_offset,
                make_pointer(continuation_table_address),
            );
            heap_offset += WORD_SIZE;
        }

        write_word(dynamic_heap, heap_offset, continuation_table.len());
        heap_offset += WORD_SIZE;

        for idx in continuation_table {
            let idx_ptr = *object_addrs.get(idx).unwrap();
            write_word(dynamic_heap, heap_offset, make_pointer(idx_ptr));
            heap_offset += WORD_SIZE;
        }
    }

    // Add region0
    let region0_offset = continuation_table_offset + continuation_table_size;
    {
        let mut heap_offset = region0_offset;
        let region0_address = heap_start + heap_offset;
        write_word(dynamic_heap, heap_offset, TAG_REGION);
        heap_offset += WORD_SIZE;

        if incremental {
            write_word(dynamic_heap, heap_offset, make_pointer(region0_address));
            heap_offset += WORD_SIZE;
        }

        // lower part of region id
        write_word(dynamic_heap, heap_offset, 0);
        heap_offset += WORD_SIZE;
        // upper part of region id
        write_word(dynamic_heap, heap_offset, 0);
        heap_offset += WORD_SIZE;
        // zero pages
        write_word(dynamic_heap, heap_offset, 0);
        heap_offset += WORD_SIZE;
        // Simplification: Skip the vector pages blob
        write_word(dynamic_heap, heap_offset, make_scalar(0));
    }

    object_addrs
}

/// Given a root set (`roots`, may contain duplicates), a mapping from object indices to addresses
/// (`object_addrs`), and the static part of the heap, initialize the static heap with the static
/// root array.
fn create_static_heap(
    roots: &[ObjectIdx],
    object_addrs: &FxHashMap<ObjectIdx, usize>,
    continuation_table_ptr_offset: usize,
    continuation_table_offset: usize,
    region0_ptr_offset: usize,
    region0_offset: usize,
    heap: &mut [u8],
) {
    let incremental = cfg!(feature = "incremental_gc");
    let root_addresses: Vec<usize> = roots
        .iter()
        .map(|obj| *object_addrs.get(obj).unwrap())
        .collect();

    // Create static root array. Each element of the array is a MutBox pointing to the actual
    // root.
    let array_addr = heap.as_ptr() as usize;
    let mut offset = 0;
    write_word(heap, offset, TAG_ARRAY_M);
    offset += WORD_SIZE;

    if incremental {
        write_word(heap, offset, make_pointer(array_addr));
        offset += WORD_SIZE;
    }

    write_word(heap, offset, roots.len());
    offset += WORD_SIZE;

    // Current offset in the heap for the next static roots array element
    let mut root_addr_offset = size_of::<Array>().to_bytes().as_usize();
    assert_eq!(offset, root_addr_offset);

    // Current offset in the heap for the MutBox of the next root
    let mut mutbox_offset = (size_of::<Array>().as_usize() + roots.len()) * WORD_SIZE;

    for root_address in root_addresses {
        // Add a MutBox for the object
        let mutbox_addr = heap.as_ptr() as usize + mutbox_offset;
        let mutbox_ptr = make_pointer(mutbox_addr);

        offset = mutbox_offset;
        write_word(heap, offset, TAG_MUTBOX);
        offset += WORD_SIZE;

        if incremental {
            write_word(heap, offset, mutbox_ptr);
            offset += WORD_SIZE;
        }

        write_word(heap, offset, make_pointer(root_address));
        offset += WORD_SIZE;

        write_word(heap, root_addr_offset, mutbox_ptr);

        root_addr_offset += WORD_SIZE;
        mutbox_offset += size_of::<MutBox>().to_bytes().as_usize();
        assert_eq!(offset, mutbox_offset);
    }

    // Write continuation table pointer as the second last word in static heap
    let continuation_table_ptr = continuation_table_offset + heap.as_ptr() as usize;
    write_word(
        heap,
        continuation_table_ptr_offset,
        make_pointer(continuation_table_ptr),
    );

    // Write continuation table pointer as the second last word in static heap
    let region0_ptr = region0_offset + heap.as_ptr() as usize;
    write_word(heap, region0_ptr_offset, make_pointer(region0_ptr));
}
