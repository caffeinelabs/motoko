use motoko_rts_macros::{
    classical_persistence, enhanced_orthogonal_persistence, uses_enhanced_orthogonal_persistence,
};

use std::{array::from_fn, mem::size_of, ptr::null_mut};

use motoko_rts::{
    gc::incremental::roots::{Roots, visit_roots},
    types::{Array, TAG_REGION, Value},
};

use crate::gc::{
    heap::MotokoHeap,
    utils::{GC, ObjectIdx, WORD_SIZE},
};

pub unsafe fn test() {
    println!("  Testing roots...");

    let object_map: [(ObjectIdx, Vec<ObjectIdx>); 10] = from_fn(|id| (id, vec![]));
    let root_ids = [2, 4, 6, 8];
    let continuation_ids = [3, 5, 7];

    let heap = MotokoHeap::new(
        &object_map,
        &root_ids,
        &continuation_ids,
        GC::Incremental,
        0,
    );
    check_visit_static_roots(&heap, &root_ids);
    check_visit_continuation_table(&heap, &continuation_ids);
    check_visit_region0(&heap);
}

unsafe fn check_visit_static_roots(heap: &MotokoHeap, root_ids: &[ObjectIdx]) {
    let roots = get_roots(heap);
    let mut visited_static_roots = vec![];
    visit_roots(
        roots,
        heap.heap_base_address(),
        &mut visited_static_roots,
        |context, field| {
            let object = *field;
            if object.tag() != TAG_REGION {
                let array = object.as_array();
                if uses_enhanced_orthogonal_persistence!() {
                    if array.len() == root_ids.len() {
                        for index in 0..array.len() {
                            let mutbox_value = array.get(index);
                            let mutbox = mutbox_value.as_mutbox();
                            let root_address = (*mutbox).field.get_ptr();
                            let root_id = object_id(heap, root_address);
                            context.push(root_id);
                        }
                    }
                } else {
                    if array.len() == 1 {
                        let id = object_id(&heap, array as usize);
                        context.push(id);
                    }
                }
            }
        },
    );
    assert_eq!(visited_static_roots, root_ids);
}

unsafe fn check_visit_continuation_table(heap: &MotokoHeap, continuation_ids: &[ObjectIdx]) {
    let roots = get_roots(heap);
    let mut visited_continuations = vec![];
    visit_roots(
        roots,
        heap.heap_base_address(),
        &mut visited_continuations,
        |context, field| {
            let object = *field;
            if object.tag() != TAG_REGION {
                let array = object.as_array();
                if array.len() == continuation_ids.len() {
                    assert_eq!(context.len(), 0);
                    for index in 0..array.len() {
                        let element = array.get(index);
                        let id = object_id(&heap, element.get_ptr());
                        context.push(id);
                    }
                }
            }
        },
    );
    assert_eq!(visited_continuations, continuation_ids);
}

unsafe fn check_visit_region0(heap: &MotokoHeap) {
    let roots = get_roots(heap);
    let mut visited_region0 = false;
    visit_roots(
        roots,
        heap.heap_base_address(),
        &mut visited_region0,
        |visited, field| {
            let object = *field;
            if object.tag() == TAG_REGION {
                assert!(!*visited);
                *visited = true;
            }
        },
    );
    assert!(visited_region0);
}

#[classical_persistence]
unsafe fn get_roots(heap: &MotokoHeap) -> Roots {
    let static_roots = Value::from_ptr(heap.static_root_array_variable_address());
    let continuation_table_location = heap.continuation_table_variable_address() as *mut Value;
    let region0_ptr_location = heap.region0_pointer_variable_address() as *mut Value;
    assert_ne!(continuation_table_location, null_mut());
    Roots {
        static_roots,
        continuation_table_location,
        region0_ptr_location,
    }
}

#[enhanced_orthogonal_persistence]
unsafe fn get_roots(heap: &MotokoHeap) -> Roots {
    let static_root = heap.static_root_array_variable_address() as *mut Value;
    let continuation_table_location = heap.continuation_table_variable_address() as *mut Value;
    let region0_ptr_location = heap.region0_pointer_variable_address() as *mut Value;
    let unused_root = &mut Value::from_scalar(0) as *mut Value;
    assert_ne!(continuation_table_location, null_mut());
    [
        static_root,
        continuation_table_location,
        region0_ptr_location,
        unused_root,
        unused_root,
        unused_root,
    ]
}

fn object_id(heap: &MotokoHeap, address: usize) -> usize {
    let offset = address - heap.heap_base_address();
    const OBJECT_SIZE: usize = size_of::<Array>() + WORD_SIZE;
    assert_eq!(offset % OBJECT_SIZE, 0);
    offset / OBJECT_SIZE
}
