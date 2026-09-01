use crate::memory::TestMemory;

use motoko_rts::constants::WORD_SIZE;
use motoko_rts::gc::mark_compact::bitmap::{
    BITMAP_ITER_END, alloc_bitmap, free_bitmap, get_bit, iter_bits, set_bit,
};
use motoko_rts::memory::Memory;
use motoko_rts::types::{Bytes, Words};

use std::collections::HashSet;

use proptest::strategy::Strategy;
use proptest::test_runner::{Config, TestCaseError, TestCaseResult, TestRunner};

pub unsafe fn test() {
    println!("  Testing bitmap ...");
    println!("    Testing set_bit/get_bit");

    {
        let mut mem = TestMemory::new(Words(1024));
        test_set_get(&mut mem, vec![0, 33]).unwrap();
    }

    let mut proptest_runner = TestRunner::new(Config {
        cases: 100,
        failure_persistence: None,
        ..Default::default()
    });

    proptest_runner
        .run(&bit_index_vector_strategy(), |bits| {
            // Max bit idx = 65,534, requires 2048 words. Add 2 words for Blob header (header +
            // length).
            let mut mem = TestMemory::new(Words(2051));
            test_set_get_proptest(&mut mem, bits)
        })
        .unwrap();

    println!("    Testing bit iteration");
    proptest_runner
        .run(&bit_index_vector_strategy(), |bits| {
            let mut hash_set = HashSet::new();
            for value in bits {
                hash_set.insert(value);
            }
            let mut mem = TestMemory::new(Words(2051));
            test_bit_iter(&mut mem, hash_set)
        })
        .unwrap();
}

/// Generates vectors of bit indices
fn bit_index_vector_strategy() -> impl Strategy<Value = Vec<u16>> {
    proptest::collection::vec(0u16..u16::MAX, 0..1_000)
}

fn test_set_get_proptest<M: Memory>(mem: &mut M, bits: Vec<u16>) -> TestCaseResult {
    test_set_get(mem, bits).map_err(|err| TestCaseError::Fail(err.into()))
}

fn test_set_get<M: Memory>(mem: &mut M, mut bits: Vec<u16>) -> Result<(), String> {
    if bits.is_empty() {
        return Ok(());
    }

    unsafe {
        alloc_bitmap(
            mem,
            Bytes((*bits.iter().max().unwrap() as usize + 1) * WORD_SIZE),
            0,
        );

        for bit in &bits {
            set_bit(*bit as usize);
            if !get_bit(*bit as usize) {
                return Err("set-get error".to_string());
            }
        }

        bits.sort();

        let mut last_bit: Option<u16> = None;
        for bit in bits {
            // Bits from the last set bit up to current bit should be 0
            if let Some(last_bit) = last_bit {
                for i in last_bit + 1..bit {
                    if get_bit(i as usize) {
                        return Err(format!("get_bit({}) of unset bit is true", i));
                    }
                }
            }

            // Current bit should be set
            if !get_bit(bit as usize) {
                return Err("get_bit of set bit is false".to_string());
            }

            last_bit = Some(bit);
        }

        free_bitmap()
    }

    Ok(())
}

fn test_bit_iter<M: Memory>(mem: &mut M, bits: HashSet<u16>) -> TestCaseResult {
    // If the max bit is N, the heap size is at least N+1 words
    let heap_size = Words(
        bits.iter()
            .max()
            .map(|max_bit| *max_bit as usize + 1)
            .unwrap_or(0),
    )
    .to_bytes();

    unsafe {
        alloc_bitmap(mem, heap_size, 0);

        for bit in bits.iter() {
            set_bit(*bit as usize);
        }

        let mut bits_sorted = bits.into_iter().collect::<Vec<_>>();
        bits_sorted.sort();

        let mut bit_vec_iter = bits_sorted.into_iter();
        let mut bit_map_iter = iter_bits();

        while let Some(vec_bit) = bit_vec_iter.next() {
            match bit_map_iter.next() {
                BITMAP_ITER_END => {
                    return Err(TestCaseError::Fail(
                        "bitmap iterator didn't yield but there are more bits".into(),
                    ));
                }
                map_bit => {
                    if map_bit != vec_bit as usize {
                        return Err(TestCaseError::Fail(
                            format!(
                                "bitmap iterator yields {}, but actual bit is {}",
                                map_bit, vec_bit
                            )
                            .into(),
                        ));
                    }
                }
            }
        }

        let map_bit = bit_map_iter.next();
        if map_bit != BITMAP_ITER_END {
            return Err(TestCaseError::Fail(
                format!(
                    "bitmap iterator yields {}, but there are no more bits left",
                    map_bit
                )
                .into(),
            ));
        }

        free_bitmap()
    }

    Ok(())
}
