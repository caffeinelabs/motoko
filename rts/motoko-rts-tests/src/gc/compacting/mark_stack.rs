use crate::memory::TestMemory;

use motoko_rts::gc::mark_compact::mark_stack::{
    INIT_STACK_SIZE, STACK_BASE, STACK_PTR, STACK_TOP, alloc_mark_stack, free_mark_stack,
    grow_stack, pop_mark_stack, push_mark_stack,
};
use motoko_rts::memory::Memory;
use motoko_rts::types::*;

use proptest::test_runner::{Config, TestCaseError, TestCaseResult, TestRunner};

pub unsafe fn test() {
    println!("  Testing mark stack ...");

    test_push_pop();
    test_grow_stack();
}

fn test_push_pop() {
    println!("    Testing push/pop");

    let mut proptest_runner = TestRunner::new(Config {
        cases: 100,
        failure_persistence: None,
        ..Default::default()
    });

    proptest_runner
        .run(&(0u32..1000u32), |n_objs| {
            let mut mem = TestMemory::new(Words(1024 * 1024));
            test_(&mut mem, n_objs)
        })
        .unwrap();
}

static TAGS: [Tag; 23] = [
    TAG_OBJECT,
    TAG_ARRAY_I,
    TAG_ARRAY_M,
    TAG_ARRAY_T,
    TAG_ARRAY_S,
    TAG_BITS64_U,
    TAG_BITS64_S,
    TAG_BITS64_F,
    TAG_MUTBOX,
    TAG_CLOSURE,
    TAG_SOME,
    TAG_VARIANT,
    TAG_BLOB_B,
    TAG_BLOB_T,
    TAG_BLOB_P,
    TAG_BLOB_A,
    TAG_FWD_PTR,
    TAG_BITS32_U,
    TAG_BITS32_S,
    TAG_BITS32_F,
    TAG_BIGINT,
    TAG_CONCAT,
    TAG_NULL,
];

fn test_<M: Memory>(mem: &mut M, n_objs: u32) -> TestCaseResult {
    let objs: Vec<u32> = (0..n_objs).collect();

    unsafe {
        alloc_mark_stack(mem);

        for obj in &objs {
            push_mark_stack(mem, *obj as usize, TAGS[(*obj as usize) % TAGS.len()]);
        }

        for obj in objs.iter().copied().rev() {
            let popped = pop_mark_stack();
            if popped != Some((obj as usize, TAGS[(obj as usize) % TAGS.len()])) {
                free_mark_stack();
                return Err(TestCaseError::Fail(
                    format!(
                        "Unexpected object popped, expected={:?}, popped={:?}",
                        obj, popped
                    )
                    .into(),
                ));
            }
        }

        free_mark_stack();
    }

    Ok(())
}

unsafe fn test_grow_stack() {
    println!("    Testing grow_stack");

    // Allow doubling twice
    let mut mem = TestMemory::new(
        size_of::<Blob>() + INIT_STACK_SIZE + INIT_STACK_SIZE + INIT_STACK_SIZE * 2,
    );

    alloc_mark_stack(&mut mem);

    let mut current_size = INIT_STACK_SIZE.as_usize();
    assert_eq!(STACK_BASE.add(current_size), STACK_TOP);
    assert_eq!(STACK_BASE, STACK_PTR);

    grow_stack(&mut mem);
    current_size *= 2;
    assert_eq!(STACK_BASE.add(current_size), STACK_TOP);
    assert_eq!(STACK_BASE, STACK_PTR);

    grow_stack(&mut mem);
    current_size *= 2;
    assert_eq!(STACK_BASE.add(current_size), STACK_TOP);
    assert_eq!(STACK_BASE, STACK_PTR);
}
