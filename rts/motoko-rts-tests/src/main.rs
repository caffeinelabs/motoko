// See: https://doc.rust-lang.org/nightly/edition-guide/rust-2024/static-mut-references.html
#![allow(static_mut_refs)]
// Edition-2024 defer: rewriting every unsafe-op-in-unsafe-fn call site is a separate task.
#![allow(unsafe_op_in_unsafe_fn)]
#![feature(proc_macro_hygiene)]

use motoko_rts_macros::{classical_persistence, enhanced_orthogonal_persistence};

#[macro_use]
mod print;

mod bigint;
mod bitrel;
mod continuation_table;
mod crc32;
mod gc;
mod leb128;
mod memory;
mod principal_id;

#[enhanced_orthogonal_persistence]
mod stabilization;
mod stable_option;
mod text;
mod utf8;

fn main() {
    check_architecture();

    unsafe {
        bigint::test();
        bitrel::test();
        continuation_table::test();
        crc32::test();
        gc::test();
        leb128::test();
        principal_id::test();
        persistence_test();
        stable_option::test();
        text::test();
        utf8::test();
    }
}

// Per-module entry points for parallel test execution via `wasmtime --invoke test_<mod>`.
// No WASI needed — works on wasm64-unknown-unknown.

#[unsafe(no_mangle)]
pub extern "C" fn test_bigint() {
    check_architecture();
    unsafe {
        bigint::test();
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn test_bitrel() {
    check_architecture();
    unsafe {
        bitrel::test();
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn test_continuation_table() {
    check_architecture();
    unsafe {
        continuation_table::test();
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn test_crc32() {
    check_architecture();
    unsafe {
        crc32::test();
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn test_gc() {
    check_architecture();
    gc::test();
}
#[unsafe(no_mangle)]
pub extern "C" fn test_gc_predefined() {
    check_architecture();
    gc::test_predefined();
}
#[unsafe(no_mangle)]
pub extern "C" fn test_gc_components() {
    check_architecture();
    gc::test_gc_components();
}
#[unsafe(no_mangle)]
pub extern "C" fn test_gc_chunk_0() {
    check_architecture();
    gc::test_random_range(0, gc::SEEDS_PER_CHUNK);
}
#[unsafe(no_mangle)]
pub extern "C" fn test_gc_chunk_1() {
    check_architecture();
    gc::test_random_range(gc::SEEDS_PER_CHUNK, 2 * gc::SEEDS_PER_CHUNK);
}
#[unsafe(no_mangle)]
pub extern "C" fn test_gc_chunk_2() {
    check_architecture();
    gc::test_random_range(2 * gc::SEEDS_PER_CHUNK, 3 * gc::SEEDS_PER_CHUNK);
}
#[unsafe(no_mangle)]
pub extern "C" fn test_gc_chunk_3() {
    check_architecture();
    gc::test_random_range(3 * gc::SEEDS_PER_CHUNK, 4 * gc::SEEDS_PER_CHUNK);
}
#[unsafe(no_mangle)]
pub extern "C" fn test_gc_chunk_4() {
    check_architecture();
    gc::test_random_range(4 * gc::SEEDS_PER_CHUNK, 5 * gc::SEEDS_PER_CHUNK);
}
#[unsafe(no_mangle)]
pub extern "C" fn test_gc_chunk_5() {
    check_architecture();
    gc::test_random_range(5 * gc::SEEDS_PER_CHUNK, 6 * gc::SEEDS_PER_CHUNK);
}
#[unsafe(no_mangle)]
pub extern "C" fn test_gc_chunk_6() {
    check_architecture();
    gc::test_random_range(6 * gc::SEEDS_PER_CHUNK, 7 * gc::SEEDS_PER_CHUNK);
}
#[unsafe(no_mangle)]
pub extern "C" fn test_gc_chunk_7() {
    check_architecture();
    gc::test_random_range(7 * gc::SEEDS_PER_CHUNK, 8 * gc::SEEDS_PER_CHUNK);
}
#[unsafe(no_mangle)]
pub extern "C" fn test_gc_chunk_8() {
    check_architecture();
    gc::test_random_range(8 * gc::SEEDS_PER_CHUNK, 9 * gc::SEEDS_PER_CHUNK);
}
#[unsafe(no_mangle)]
pub extern "C" fn test_gc_chunk_9() {
    check_architecture();
    gc::test_random_range(9 * gc::SEEDS_PER_CHUNK, 10 * gc::SEEDS_PER_CHUNK);
}
#[unsafe(no_mangle)]
pub extern "C" fn test_leb128() {
    check_architecture();
    unsafe {
        leb128::test();
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn test_principal_id() {
    check_architecture();
    unsafe {
        principal_id::test();
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn test_persistence() {
    check_architecture();
    persistence_test();
}
#[unsafe(no_mangle)]
pub extern "C" fn test_persistence_small() {
    check_architecture();
    persistence_small_test();
}
#[unsafe(no_mangle)]
pub extern "C" fn test_persistence_20k() {
    check_architecture();
    persistence_20k_test();
}
#[unsafe(no_mangle)]
pub extern "C" fn test_stable_option() {
    check_architecture();
    stable_option::test();
}
#[unsafe(no_mangle)]
pub extern "C" fn test_text() {
    check_architecture();
    unsafe {
        text::test();
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn test_utf8() {
    check_architecture();
    unsafe {
        utf8::test();
    }
}

#[classical_persistence]
fn check_architecture() {
    if std::mem::size_of::<usize>() != 4 {
        println!("Motoko RTS for classical persistence only works on 32-bit architectures");
        std::process::exit(1);
    }
}

#[enhanced_orthogonal_persistence]
fn check_architecture() {
    if std::mem::size_of::<usize>() != 8 {
        println!(
            "Motoko RTS for enhanced orthogonal persistence only works on 64-bit architectures"
        );
        std::process::exit(1);
    }
}

#[enhanced_orthogonal_persistence]
fn persistence_test() {
    unsafe {
        stabilization::test();
    }
}

#[enhanced_orthogonal_persistence]
fn persistence_small_test() {
    stabilization::test_stabilization_small();
}

#[enhanced_orthogonal_persistence]
fn persistence_20k_test() {
    stabilization::test_stabilization_20k();
}

#[classical_persistence]
fn persistence_test() {
    test_read_write_64_bit();
}

#[classical_persistence]
fn persistence_small_test() {}

#[classical_persistence]
fn persistence_20k_test() {}

#[classical_persistence]
fn test_read_write_64_bit() {
    use motoko_rts::types::{read64, write64};
    println!("Testing 64-bit read-write");
    const TEST_VALUE: u64 = 0x1234_5678_9abc_def0;
    let mut lower = 0u32;
    let mut upper = 0u32;
    write64(&mut lower, &mut upper, TEST_VALUE);
    assert_eq!(lower, 0x9abc_def0);
    assert_eq!(upper, 0x1234_5678);
    assert_eq!(read64(lower, upper), TEST_VALUE);
}

// Called by the RTS to panic
#[unsafe(no_mangle)]
extern "C" fn rts_trap(ptr: *const u8, len: u32) -> ! {
    let msg = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    match core::str::from_utf8(msg) {
        Err(err) => panic!(
            "rts_trap_with called with non-UTF8 string (error={:?}, string={:?})",
            err, msg
        ),
        Ok(str) => panic!("rts_trap_with: {:?}", str),
    }
}

// Called by RTS BigInt functions to panic. Normally generated by the compiler
#[unsafe(no_mangle)]
extern "C" fn bigint_trap() -> ! {
    panic!("bigint_trap called");
}

// Called by the RTS for debug prints
#[unsafe(no_mangle)]
extern "C" fn print_ptr(ptr: usize, len: usize) {
    let str: &[u8] = unsafe { core::slice::from_raw_parts(ptr as *const u8, len) };
    println!("[RTS] {}", &String::from_utf8_lossy(str));
}

// Program entry point by wasmtime
#[enhanced_orthogonal_persistence]
#[unsafe(no_mangle)]
pub fn _start() {
    main();
}
