/// Adopted from compacting GC unit test `bitmap.rs`.
use crate::memory::TestMemory;

use motoko_rts::constants::WORD_SIZE;
use motoko_rts::gc::incremental::mark_bitmap::{BITMAP_ITERATION_END, MarkBitmap};
use motoko_rts::gc::incremental::partitioned_heap::PARTITION_SIZE;
use motoko_rts::memory::Memory;
use motoko_rts::types::{Bytes, Value};

use std::collections::HashSet;

use proptest::strategy::Strategy;
use proptest::test_runner::{Config, TestCaseResult, TestRunner};

pub unsafe fn test() {
    println!("  Testing mark bitmap ...");
    let bitmap_size = Bytes(PARTITION_SIZE).to_words();
    let mut mem = TestMemory::new(bitmap_size);
    let bitmap_pointer = mem.alloc_words(bitmap_size);

    test_mark(bitmap_pointer, vec![0, 33]);

    let mut proptest_runner = TestRunner::new(Config {
        cases: 100,
        failure_persistence: None,
        ..Default::default()
    });

    proptest_runner
        .run(&bit_index_vector_strategy(), |bits| {
            test_mark_proptest(bitmap_pointer, bits)
        })
        .unwrap();

    println!("  Testing bit iteration");
    proptest_runner
        .run(&bit_index_vector_strategy(), |bits| {
            let mut hash_set = HashSet::new();
            for value in bits {
                hash_set.insert(value);
            }
            test_iterator_proptest(bitmap_pointer, hash_set)
        })
        .unwrap();

    test_last_bit(bitmap_pointer);
}

const MAX_TEST_BIT_INDEX: usize = u16::MAX as usize;

fn bit_index_vector_strategy() -> impl Strategy<Value = Vec<usize>> {
    proptest::collection::vec(0..MAX_TEST_BIT_INDEX, 0..1_000)
}

fn test_mark_proptest(bitmap_pointer: Value, bits: Vec<usize>) -> TestCaseResult {
    test_mark(bitmap_pointer, bits);
    Ok(())
}

fn address_of_bit(bit: usize) -> usize {
    bit * WORD_SIZE
}

fn test_mark(bitmap_pointer: Value, mut bits: Vec<usize>) {
    unsafe {
        let mut bitmap = MarkBitmap::new();
        bitmap.assign(bitmap_pointer.get_ptr() as *mut u8);
        for bit in &bits {
            assert!(!bitmap.is_marked(address_of_bit(*bit)));
        }
        for bit in &bits {
            bitmap.mark(address_of_bit(*bit));
            assert!(bitmap.is_marked(address_of_bit(*bit)));
        }
        bits.sort();
        let mut last_bit: Option<usize> = None;
        for bit in bits {
            if let Some(last_bit) = last_bit {
                for i in last_bit + 1..bit {
                    assert!(!bitmap.is_marked(address_of_bit(i)));
                }
            }
            assert!(bitmap.is_marked(address_of_bit(bit)));
            last_bit = Some(bit);
        }
        bitmap.release();
    }
}

fn test_iterator_proptest(bitmap_pointer: Value, bits: HashSet<usize>) -> TestCaseResult {
    test_iterator(bitmap_pointer, bits);
    Ok(())
}

fn test_iterator(bitmap_pointer: Value, bits: HashSet<usize>) {
    unsafe {
        let mut bitmap = MarkBitmap::new();
        bitmap.assign(bitmap_pointer.get_ptr() as *mut u8);
        for bit in bits.iter() {
            bitmap.mark(address_of_bit(*bit));
        }
        let mut bits_sorted = bits.into_iter().collect::<Vec<_>>();
        bits_sorted.sort();
        let mut reference_iterator = bits_sorted.into_iter();
        let mut bitmap_iterator = bitmap.iterate();
        while let Some(vec_bit) = reference_iterator.next() {
            let actual_address = bitmap_iterator.current_marked_offset();
            assert_ne!(actual_address, BITMAP_ITERATION_END);
            let expected_address = address_of_bit(vec_bit);
            assert_eq!(actual_address, expected_address);
            bitmap_iterator.next();
        }
        assert_eq!(
            bitmap_iterator.current_marked_offset(),
            BITMAP_ITERATION_END
        );
        bitmap.release();
    }
}

fn test_last_bit(bitmap_pointer: Value) {
    const LAST_OFFSET: usize = PARTITION_SIZE - WORD_SIZE;
    unsafe {
        let mut bitmap = MarkBitmap::new();
        bitmap.assign(bitmap_pointer.get_ptr() as *mut u8);
        bitmap.mark(LAST_OFFSET);
        let mut bitmap_iterator = bitmap.iterate();
        assert_eq!(bitmap_iterator.current_marked_offset(), LAST_OFFSET);
        bitmap_iterator.next();
        assert_eq!(
            bitmap_iterator.current_marked_offset(),
            BITMAP_ITERATION_END
        );
        bitmap.release();
    }
}
