use super::{MotokoHeapInner, heap_size_for_gc};

use crate::gc::utils::{GC, ObjectIdx, WORD_SIZE, make_pointer, make_scalar, write_word};

use motoko_rts::types::*;

use fxhash::{FxHashMap, FxHashSet};

pub(super) fn new_heap(
    map: &[(ObjectIdx, Vec<ObjectIdx>)],
    roots: &[ObjectIdx],
    continuation_table: &[ObjectIdx],
    free_space: usize,
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

    // Three pointers: Static root array, continuation table, and region 0.
    let root_pointers_size_bytes = 3 * WORD_SIZE;

    // Each object will have array header plus one word for id per object + one word for each reference.
    // The static root is an array (header + length) with one element, one MutBox for each static variable.
    let static_root_set_size_bytes = (size_of::<Array>().as_usize()
        + roots.len()
        + roots.len() * size_of::<MutBox>().as_usize())
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

    let dynamic_heap_size_bytes = dynamic_heap_size_without_roots
        + static_root_set_size_bytes
        + continuation_table_size
        + region0_size;

    let total_heap_size_bytes = root_pointers_size_bytes + dynamic_heap_size_bytes;

    let heap_size = heap_size_for_gc(GC::Incremental, total_heap_size_bytes, map.len());

    const HEAP_ALIGNMENT: usize = usize::BITS as usize;
    // The Worst-case unalignment is one word less than the intended heap alignment
    // (assuming that we have general word alignment). So we over-allocate `HEAP_ALIGNMENT - WORD_SIZE` bytes.
    let mut heap = vec![0u8; heap_size + HEAP_ALIGNMENT - WORD_SIZE + free_space];

    // Align the dynamic heap start.
    let realign = (HEAP_ALIGNMENT
        - (heap.as_ptr() as usize + root_pointers_size_bytes) % HEAP_ALIGNMENT)
        % HEAP_ALIGNMENT;
    assert_eq!(realign % WORD_SIZE, 0);

    // Maps `ObjectIdx`s into their offsets in the heap.
    let (static_root_array_address, continuation_table_address, region0_address) =
        create_dynamic_heap(
            map,
            roots,
            continuation_table,
            &mut heap[root_pointers_size_bytes + realign..heap_size + realign],
        );

    // Root pointers in static memory space.
    let static_root_array_variable_offset = root_pointers_size_bytes - 3 * WORD_SIZE;
    let continuation_table_variable_offset = root_pointers_size_bytes - 2 * WORD_SIZE;
    let region0_pointer_variable_offset = root_pointers_size_bytes - WORD_SIZE;
    create_static_memory(
        static_root_array_variable_offset,
        continuation_table_variable_offset,
        region0_pointer_variable_offset,
        static_root_array_address,
        continuation_table_address,
        region0_address,
        &mut heap[realign..root_pointers_size_bytes + realign],
    );

    MotokoHeapInner {
        heap: heap.into_boxed_slice(),
        heap_base_offset: root_pointers_size_bytes + realign,
        _heap_ptr_last: root_pointers_size_bytes + realign,
        heap_ptr_offset: total_heap_size_bytes + realign,
        static_root_array_variable_offset: static_root_array_variable_offset + realign,
        continuation_table_variable_offset: continuation_table_variable_offset + realign,
        region0_pointer_variable_offset: region0_pointer_variable_offset + realign,
    }
}

/// Given a heap description (as a map from objects to objects), and the dynamic part of the heap
/// (as an array), initialize the dynamic heap with objects.
///
/// Returns a pair containing the address of the static root array and the address of the continuation table.
fn create_dynamic_heap(
    refs: &[(ObjectIdx, Vec<ObjectIdx>)],
    static_roots: &[ObjectIdx],
    continuation_table: &[ObjectIdx],
    dynamic_heap: &mut [u8],
) -> (usize, usize, usize) {
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

            write_word(dynamic_heap, heap_offset, make_pointer(address)); // forwarding pointer
            heap_offset += WORD_SIZE;

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

    // Add the static root table
    let n_objects = refs.len();
    // fields+1 for the scalar field (idx)
    let n_fields: usize = refs.iter().map(|(_, fields)| fields.len() + 1).sum();
    let root_section_offset =
        (size_of::<Array>() * n_objects).to_bytes().as_usize() + n_fields * WORD_SIZE;

    let mut heap_offset = root_section_offset;
    let mut root_mutboxes = vec![];
    {
        for root_id in static_roots {
            let mutbox_address = heap_start + heap_offset;
            root_mutboxes.push(mutbox_address);
            write_word(dynamic_heap, heap_offset, TAG_MUTBOX);
            heap_offset += WORD_SIZE;

            write_word(dynamic_heap, heap_offset, make_pointer(mutbox_address));
            heap_offset += WORD_SIZE;

            let root_ptr = *object_addrs.get(root_id).unwrap();
            write_word(dynamic_heap, heap_offset, make_pointer(root_ptr));
            heap_offset += WORD_SIZE;
        }
    }
    let static_root_array_address = heap_start + heap_offset;
    {
        write_word(dynamic_heap, heap_offset, TAG_ARRAY_M);
        heap_offset += WORD_SIZE;

        write_word(
            dynamic_heap,
            heap_offset,
            make_pointer(static_root_array_address),
        );
        heap_offset += WORD_SIZE;

        assert_eq!(static_roots.len(), root_mutboxes.len());
        write_word(dynamic_heap, heap_offset, root_mutboxes.len());
        heap_offset += WORD_SIZE;

        for mutbox_address in root_mutboxes {
            write_word(dynamic_heap, heap_offset, make_pointer(mutbox_address));
            heap_offset += WORD_SIZE;
        }
    }

    let continuation_table_address = heap_start + heap_offset;
    {
        write_word(dynamic_heap, heap_offset, TAG_ARRAY_M);
        heap_offset += WORD_SIZE;

        write_word(
            dynamic_heap,
            heap_offset,
            make_pointer(continuation_table_address),
        );
        heap_offset += WORD_SIZE;

        write_word(dynamic_heap, heap_offset, continuation_table.len());
        heap_offset += WORD_SIZE;

        for idx in continuation_table {
            let idx_ptr = *object_addrs.get(idx).unwrap();
            write_word(dynamic_heap, heap_offset, make_pointer(idx_ptr));
            heap_offset += WORD_SIZE;
        }
    }

    // Add region0
    let region0_address = heap_start + heap_offset;
    {
        write_word(dynamic_heap, heap_offset, TAG_REGION);
        heap_offset += WORD_SIZE;

        write_word(dynamic_heap, heap_offset, make_pointer(region0_address));
        heap_offset += WORD_SIZE;

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

    (
        static_root_array_address,
        continuation_table_address,
        region0_address,
    )
}

/// Static memory part containing the root pointers.
fn create_static_memory(
    static_root_array_variable_offset: usize,
    continuation_table_variable_offset: usize,
    region0_pointer_variable_offset: usize,
    static_root_array_address: usize,
    continuation_table_address: usize,
    region0_address: usize,
    heap: &mut [u8],
) {
    // Write static array pointer as the third last word in static memory
    write_word(
        heap,
        static_root_array_variable_offset,
        make_pointer(static_root_array_address),
    );

    // Write continuation table pointer as the second last word in static memory
    write_word(
        heap,
        continuation_table_variable_offset,
        make_pointer(continuation_table_address),
    );

    // Write region 0 pointer as the very last word in static memory
    write_word(
        heap,
        region0_pointer_variable_offset,
        make_pointer(region0_address),
    );
}
