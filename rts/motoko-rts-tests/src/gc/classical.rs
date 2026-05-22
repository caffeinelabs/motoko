use super::heap::MotokoHeap;
use super::utils::{
    GC, ObjectIdx, WORD_SIZE, get_scalar_value, make_pointer, read_word, unskew_pointer,
};
use crate::gc::{CheckMode, compute_reachable_objects};
use fxhash::{FxHashMap, FxHashSet};
use motoko_rts::types::*;
use motoko_rts_macros::{incremental_gc, is_incremental_gc, non_incremental_gc};
use std::fmt::Write;

impl GC {
    #[non_incremental_gc]
    pub fn run(&self, heap: &mut MotokoHeap, _round: usize) -> bool {
        let heap_base = heap.heap_base_address();
        let static_roots = Value::from_ptr(heap.static_root_array_variable_address());
        let mut region_0 = Value::from_scalar(0);
        let continuation_table_ptr_address =
            heap.continuation_table_variable_address() as *mut Value;

        let heap_1 = heap.clone();
        let heap_2 = heap.clone();

        match self {
            GC::Copying => {
                unsafe {
                    motoko_rts::gc::copying::copying_gc_internal(
                        heap,
                        heap_base,
                        // get_hp
                        || heap_1.heap_ptr_address(),
                        // set_hp
                        move |hp| heap_2.set_heap_ptr_address(hp),
                        static_roots,
                        continuation_table_ptr_address,
                        &mut region_0,
                        // note_live_size
                        |_live_size| {},
                        // note_reclaimed
                        |_reclaimed| {},
                    );
                }
                true
            }

            GC::MarkCompact => {
                unsafe {
                    motoko_rts::gc::mark_compact::compacting_gc_internal(
                        heap,
                        heap_base,
                        // get_hp
                        || heap_1.heap_ptr_address(),
                        // set_hp
                        move |hp| heap_2.set_heap_ptr_address(hp),
                        static_roots,
                        continuation_table_ptr_address,
                        &mut region_0,
                        // note_live_size
                        |_live_size| {},
                        // note_reclaimed
                        |_reclaimed| {},
                    );
                }
                true
            }

            GC::Generational => {
                use motoko_rts::gc::{
                    generational::{
                        GenerationalGC, Strategy,
                        write_barrier::{LAST_HP, REMEMBERED_SET},
                    },
                    remembered_set::RememberedSet,
                };

                let strategy = match _round {
                    0 => Strategy::Young,
                    _ => Strategy::Full,
                };
                unsafe {
                    REMEMBERED_SET = Some(RememberedSet::new(heap));
                    LAST_HP = heap_1.last_ptr_address();

                    let limits = motoko_rts::gc::generational::Limits {
                        base: heap_base,
                        last_free: heap_1.last_ptr_address(),
                        free: heap_1.heap_ptr_address(),
                    };
                    let roots = motoko_rts::gc::generational::Roots {
                        static_roots,
                        continuation_table_ptr_loc: continuation_table_ptr_address,
                        region0_ptr_loc: &mut region_0,
                    };
                    let gc_heap = motoko_rts::gc::generational::Heap {
                        mem: heap,
                        limits,
                        roots,
                    };
                    let mut gc = GenerationalGC::new(gc_heap, strategy);
                    gc.run();
                    let free = gc.heap.limits.free;
                    heap.set_last_ptr_address(free);
                    heap.set_heap_ptr_address(free);
                }
                _round >= 2
            }
        }
    }

    #[incremental_gc]
    pub fn run(&self, heap: &mut MotokoHeap, _round: usize) -> bool {
        let static_roots = Value::from_ptr(heap.static_root_array_variable_address());
        let continuation_table_ptr_address =
            heap.continuation_table_variable_address() as *mut Value;
        let region0_ptr_address = heap.region0_pointer_variable_address() as *mut Value;

        match self {
            GC::Incremental => unsafe {
                use motoko_rts::gc::incremental::{IncrementalGC, get_incremental_gc_state};
                const INCREMENTS_UNTIL_COMPLETION: usize = 16;
                for _ in 0..INCREMENTS_UNTIL_COMPLETION {
                    let roots = motoko_rts::gc::incremental::roots::Roots {
                        static_roots,
                        continuation_table_location: continuation_table_ptr_address,
                        region0_ptr_location: region0_ptr_address,
                    };
                    IncrementalGC::instance(heap, get_incremental_gc_state())
                        .empty_call_stack_increment(roots);
                }
                false
            },
        }
    }
}

/// Check the dynamic heap:
///
/// - All (and in post-gc mode, only) reachable objects should be in the heap. Reachable objects
///   are those in the transitive closure of roots.
///
/// - Objects should point to right objects. E.g. if object with index X points to objects with
///   indices Y and Z in the `objects` map, it should point to objects with indices Y and Z on the
///   heap.
///
pub fn check_dynamic_heap(
    mode: CheckMode,
    objects: &[(ObjectIdx, Vec<ObjectIdx>)],
    roots: &[ObjectIdx],
    continuation_table: &[ObjectIdx],
    heap: &[u8],
    heap_base_offset: usize,
    heap_ptr_offset: usize,
    continuation_table_ptr_offset: usize,
    region0_ptr_offset: usize,
) {
    let incremental = cfg!(feature = "incremental_gc");
    let objects_map: FxHashMap<ObjectIdx, &[ObjectIdx]> = objects
        .iter()
        .map(|(obj, refs)| (*obj, refs.as_slice()))
        .collect();

    // Current offset in the heap
    let mut offset = heap_base_offset;

    // Maps objects to their addresses (not offsets!). Used when debugging duplicate objects.
    let mut seen: FxHashMap<ObjectIdx, usize> = Default::default();

    let continuation_table_addr = unskew_pointer(read_word(heap, continuation_table_ptr_offset));
    let continuation_table_offset = continuation_table_addr - heap.as_ptr() as usize;

    let region0_addr = unskew_pointer(read_word(heap, region0_ptr_offset));

    while offset < heap_ptr_offset {
        let object_offset = offset;

        // Address of the current object. Used for debugging.
        let address = offset + heap.as_ptr() as usize;

        if object_offset == continuation_table_offset {
            check_continuation_table(object_offset, continuation_table, heap);
            offset += (size_of::<Array>() + Words(continuation_table.len()))
                .to_bytes()
                .as_usize();
            continue;
        }

        let tag = read_word(heap, offset);
        offset += WORD_SIZE;

        if tag == TAG_ONE_WORD_FILLER {
            assert!(incremental);
        } else if tag == TAG_FREE_SPACE {
            assert!(incremental);
            let words = read_word(heap, offset);
            offset += WORD_SIZE;
            offset += words * WORD_SIZE;
        } else {
            let forward;
            if incremental {
                forward = read_word(heap, offset);
                offset += WORD_SIZE;
            } else {
                forward = make_pointer(address);
            }

            let is_forwarded = forward != make_pointer(address);

            if incremental && tag == TAG_BLOB_B {
                assert!(!is_forwarded);
                // in-heap mark stack blobs
                let length = read_word(heap, offset);
                offset += WORD_SIZE + length;
            } else if tag == TAG_REGION {
                if !is_forwarded {
                    assert_eq!(address, region0_addr);
                }
                offset += (size_of::<Region>() - size_of::<Obj>())
                    .to_bytes()
                    .as_usize();
            } else {
                if incremental {
                    assert!(is_array_or_slice_tag(tag));
                } else {
                    assert!(is_base_array_tag(tag));
                }

                if is_forwarded {
                    assert!(incremental);

                    let forward_offset = forward - heap.as_ptr() as usize;
                    let length = read_word(
                        heap,
                        forward_offset + size_of::<Obj>().to_bytes().as_usize(),
                    );

                    // Skip stale object version that has been relocated during incremental GC.
                    offset += length * WORD_SIZE;
                } else {
                    let n_fields = read_word(heap, offset);
                    offset += WORD_SIZE;

                    // There should be at least one field for the index
                    assert!(n_fields >= 1);

                    let object_idx = get_scalar_value(read_word(heap, offset));
                    offset += WORD_SIZE;

                    let old = seen.insert(object_idx, address);
                    if let Some(old) = old {
                        panic!(
                            "Object with index {} seen multiple times: {:#x}, {:#x}",
                            object_idx, old, address
                        );
                    }

                    let object_expected_pointees =
                        objects_map.get(&object_idx).unwrap_or_else(|| {
                            panic!("Object with index {} is not in the objects map", object_idx)
                        });

                    for field_idx in 1..n_fields {
                        let field = read_word(heap, offset);
                        offset += WORD_SIZE;
                        // Get index of the object pointed by the field
                        let pointee_address = field.wrapping_add(1); // unskew
                        let pointee_offset = pointee_address - heap.as_ptr() as usize;
                        let pointee_idx_offset =
                            pointee_offset + size_of::<Array>().to_bytes().as_usize(); // skip array header (incl. length)
                        let pointee_idx = get_scalar_value(read_word(heap, pointee_idx_offset));
                        let expected_pointee_idx = object_expected_pointees[field_idx - 1];
                        assert_eq!(
                            pointee_idx,
                            expected_pointee_idx,
                            "Object with index {} points to {} in field {}, but expected to point to {}",
                            object_idx,
                            pointee_idx,
                            field_idx - 1,
                            expected_pointee_idx,
                        );
                    }
                }
            }
        }
    }

    // At this point we've checked that all seen objects point to the expected objects (as
    // specified by `objects`). Check that we've seen the reachable objects and only the reachable
    // objects.
    let reachable_objects = compute_reachable_objects(roots, continuation_table, &objects_map);

    // Objects we've seen in the heap
    let seen_objects: FxHashSet<ObjectIdx> = seen.keys().copied().collect();

    // Reachable objects that we haven't seen in the heap
    let missing_objects: Vec<ObjectIdx> = reachable_objects
        .difference(&seen_objects)
        .copied()
        .collect();

    let mut error_message = String::new();

    if !missing_objects.is_empty() {
        write!(
            &mut error_message,
            "Reachable objects missing in the heap: {missing_objects:?}",
        )
        .unwrap();
    }

    if mode == CheckMode::AllReclaimed {
        // Unreachable objects that we've seen in the heap
        let extra_objects: Vec<ObjectIdx> = seen_objects
            .difference(&reachable_objects)
            .copied()
            .collect();

        if !extra_objects.is_empty() {
            if !error_message.is_empty() {
                error_message.push('\n');
            }

            write!(
                &mut error_message,
                "Unreachable objects seen in the post-GC heap: {:?}",
                extra_objects,
            )
            .unwrap();
        }
    }

    if !error_message.is_empty() {
        panic!("{}", error_message);
    }
}

fn check_continuation_table(mut offset: usize, continuation_table: &[ObjectIdx], heap: &[u8]) {
    let table_addr = heap.as_ptr() as usize + offset;
    assert_eq!(read_word(heap, offset), TAG_ARRAY_M);
    offset += WORD_SIZE;

    if is_incremental_gc!() {
        assert_eq!(read_word(heap, offset), make_pointer(table_addr));
        offset += WORD_SIZE;
    }

    assert_eq!(read_word(heap, offset), continuation_table.len());
    offset += WORD_SIZE;

    for obj in continuation_table.iter() {
        let ptr = unskew_pointer(read_word(heap, offset));
        offset += WORD_SIZE;

        // Skip object header for idx
        let idx_address = ptr + size_of::<Array>().to_bytes().as_usize();
        let idx = get_scalar_value(read_word(heap, idx_address - heap.as_ptr() as usize));

        assert_eq!(idx, *obj);
    }
}
