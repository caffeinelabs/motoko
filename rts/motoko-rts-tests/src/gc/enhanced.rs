use super::heap::MotokoHeap;
use super::utils::{
    GC, ObjectIdx, WORD_SIZE, get_scalar_value, make_pointer, read_word, unskew_pointer,
};
use crate::gc::{CheckMode, compute_reachable_objects};
use fxhash::{FxHashMap, FxHashSet};
use motoko_rts::types::*;
use std::fmt::Write;

impl GC {
    pub fn run(&self, heap: &mut MotokoHeap, _round: usize) -> bool {
        let static_root = heap.static_root_array_variable_address() as *mut Value;
        let continuation_table_location = heap.continuation_table_variable_address() as *mut Value;
        let region0_pointer_location = heap.region0_pointer_variable_address() as *mut Value;
        let unused_root = &mut Value::from_scalar(0) as *mut Value;

        unsafe {
            use motoko_rts::gc::incremental::{IncrementalGC, get_incremental_gc_state};
            const INCREMENTS_UNTIL_COMPLETION: usize = 16;
            for _ in 0..INCREMENTS_UNTIL_COMPLETION {
                let roots = [
                    static_root,
                    continuation_table_location,
                    region0_pointer_location,
                    unused_root,
                    unused_root,
                    unused_root,
                ];
                IncrementalGC::instance(heap, get_incremental_gc_state())
                    .empty_call_stack_increment(roots);
            }
            false
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
    static_root_array_variable_offset: usize,
    continuation_table_variable_offset: usize,
    region0_ptr_offset: usize,
) {
    let objects_map: FxHashMap<ObjectIdx, &[ObjectIdx]> = objects
        .iter()
        .map(|(obj, refs)| (*obj, refs.as_slice()))
        .collect();

    // Current offset in the heap
    let mut offset = heap_base_offset;

    // Maps objects to their addresses (not offsets!). Used when debugging duplicate objects.
    let mut seen: FxHashMap<ObjectIdx, usize> = Default::default();

    let static_root_array_address =
        unskew_pointer(read_word(heap, static_root_array_variable_offset));
    let static_root_array_offset = static_root_array_address as usize - heap.as_ptr() as usize;

    let continuation_table_address =
        unskew_pointer(read_word(heap, continuation_table_variable_offset));
    let continuation_table_offset = continuation_table_address as usize - heap.as_ptr() as usize;

    let region0_addr = unskew_pointer(read_word(heap, region0_ptr_offset));

    while offset < heap_ptr_offset {
        let object_offset = offset;

        // Address of the current object. Used for debugging.
        let address = offset + heap.as_ptr() as usize;

        if object_offset == static_root_array_offset {
            check_static_root_array(object_offset, roots, heap);
            offset += (size_of::<Array>() + Words(roots.len()))
                .to_bytes()
                .as_usize();
            continue;
        }

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
        } else if tag == TAG_FREE_SPACE {
            let words = read_word(heap, offset);
            offset += WORD_SIZE;
            offset += words * WORD_SIZE;
        } else {
            let forward;
            forward = read_word(heap, offset);
            offset += WORD_SIZE;

            let is_forwarded = forward != make_pointer(address);

            if tag == TAG_MUTBOX {
                // MutBoxes of static root array, will be scanned indirectly when checking the static root array.
                offset += WORD_SIZE;
            } else if tag == TAG_BLOB_B {
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
            } else if mode == CheckMode::Stabilzation && tag == TAG_MUTBOX {
                offset += WORD_SIZE;
            } else {
                assert!(is_array_or_slice_tag(tag));

                if is_forwarded {
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

                        let pointee_idx = read_object_id(pointee_address, heap);
                        let expected_pointee_idx =
                            object_expected_pointees[(field_idx - 1) as usize];
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

        skip_empty_partition_space(heap, &mut offset, heap_ptr_offset);
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
            "{mode:?}: Reachable objects missing in the heap: {missing_objects:?}",
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

fn check_static_root_array(mut offset: usize, roots: &[ObjectIdx], heap: &[u8]) {
    let array_address = heap.as_ptr() as usize + offset;
    assert_eq!(read_word(heap, offset), TAG_ARRAY_M);
    offset += WORD_SIZE;

    assert_eq!(read_word(heap, offset), make_pointer(array_address));
    offset += WORD_SIZE;

    assert_eq!(read_word(heap, offset), roots.len());
    offset += WORD_SIZE;

    for obj in roots.iter() {
        let mutbox_address = unskew_pointer(read_word(heap, offset));
        offset += WORD_SIZE;

        let object_address = unskew_pointer(read_mutbox_field(mutbox_address, heap));
        let idx = read_object_id(object_address, heap);
        assert_eq!(idx, *obj);
    }
}

fn read_mutbox_field(mutbox_address: usize, heap: &[u8]) -> usize {
    let mut mutbox_offset = mutbox_address - heap.as_ptr() as usize;

    let mutbox_tag = read_word(heap, mutbox_offset);
    assert_eq!(mutbox_tag, TAG_MUTBOX);
    mutbox_offset += WORD_SIZE;

    assert_eq!(read_word(heap, mutbox_offset), make_pointer(mutbox_address));
    mutbox_offset += WORD_SIZE;

    read_word(heap, mutbox_offset)
}

fn check_continuation_table(mut offset: usize, continuation_table: &[ObjectIdx], heap: &[u8]) {
    let table_addr = heap.as_ptr() as usize + offset;
    assert_eq!(read_word(heap, offset), TAG_ARRAY_M);
    offset += WORD_SIZE;

    assert_eq!(read_word(heap, offset), make_pointer(table_addr));
    offset += WORD_SIZE;

    assert_eq!(read_word(heap, offset), continuation_table.len());
    offset += WORD_SIZE;

    for obj in continuation_table.iter() {
        let ptr = unskew_pointer(read_word(heap, offset));
        offset += WORD_SIZE;

        let idx = read_object_id(ptr, heap);
        assert_eq!(idx, *obj);
    }
}

fn read_object_id(object_address: usize, heap: &[u8]) -> ObjectIdx {
    let tag = read_word(heap, object_address - heap.as_ptr() as usize);
    assert!(is_array_or_slice_tag(tag));

    // Skip object header for idx
    let idx_address = object_address + size_of::<Array>().to_bytes().as_usize();
    get_scalar_value(read_word(heap, idx_address - heap.as_ptr() as usize))
}

fn skip_empty_partition_space(heap: &[u8], offset: &mut usize, heap_ptr_offset: usize) {
    use motoko_rts::gc::incremental::{get_partitioned_heap, partitioned_heap::PARTITION_SIZE};
    let heap_start = heap.as_ptr() as usize;
    while *offset < heap_ptr_offset {
        let address = *offset + heap_start;
        let partition_index = address / PARTITION_SIZE;
        let partition = unsafe { get_partitioned_heap().get_partition(partition_index) };
        if address < partition.dynamic_space_end() {
            return;
        }
        *offset = (partition_index + 1) * PARTITION_SIZE - heap_start;
    }
}
