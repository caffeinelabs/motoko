use crate::memory::{TestMemory, initialize_test_memory, reset_test_memory};
use std::ffi::c_void;

use motoko_rts::bigint::{self, *};
use motoko_rts::buf::Buf;
use motoko_rts::types::{Bytes, Value};

// mp functions below are implemented separately for tests as we can't modify mp_int source code to
// pass a generic heap argument (then monomorphise it for IC).

// This global is used to pass a reference to heap to the mp functions
static mut HEAP: *mut TestMemory = std::ptr::null_mut();

pub unsafe fn set_bigint_heap(heap: *mut TestMemory) {
    HEAP = heap;
}

#[unsafe(no_mangle)]
unsafe extern "C" fn mp_calloc(n_elems: usize, elem_size: Bytes<usize>) -> *mut c_void {
    bigint::mp_calloc(&mut *HEAP, n_elems, elem_size)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn mp_realloc(
    ptr: *mut c_void,
    old_size: Bytes<usize>,
    new_size: Bytes<usize>,
) -> *mut c_void {
    bigint::mp_realloc(&mut *HEAP, ptr, old_size, new_size)
}

pub unsafe fn test() {
    println!("Testing BigInt ...");

    let mut heap = initialize_test_memory();
    set_bigint_heap(&mut heap);

    assert!(bigint_eq(
        bigint_pow(bigint_of_word64(70), bigint_of_word64(32)),
        bigint_mul(
            bigint_pow(bigint_of_word64(70), bigint_of_word64(31)),
            bigint_of_word64(70)
        )
    ));

    //
    // (s)leb128 encoding
    //

    let one = bigint_of_word64(1);
    let two = bigint_of_word64(2);
    for i in 0..100 {
        let two_pow_i = bigint_pow(two, bigint_of_word64(i));
        let minus_one = bigint_sub(two_pow_i, one);
        let plus_one = bigint_add(two_pow_i, one);

        test_bigint_leb128(minus_one);
        test_bigint_leb128(two_pow_i);
        test_bigint_leb128(plus_one);

        test_bigint_sleb128(minus_one);
        test_bigint_sleb128(two_pow_i);
        test_bigint_sleb128(plus_one);
        test_bigint_sleb128(bigint_neg(minus_one));
        test_bigint_sleb128(bigint_neg(two_pow_i));
        test_bigint_sleb128(bigint_neg(plus_one));
    }

    set_bigint_heap(std::ptr::null_mut());
    reset_test_memory();
    drop(heap);
}

// Check leb128 encode/decode roundtrip
unsafe fn test_bigint_leb128(n: Value) {
    let mut buf = [0u8; 100];
    let s = bigint_leb128_size(n);
    let mut buf_ = Buf {
        ptr: buf.as_mut_ptr(),
        end: buf.as_mut_ptr().add(100),
    };
    bigint_leb128_encode(n, buf.as_mut_ptr());
    let n2 = bigint_leb128_decode(&mut buf_ as *mut _);
    assert!(bigint_eq(n, n2));
    assert_eq!(buf_.ptr.offset_from(buf.as_ptr()), s as isize);
}

// Check sleb128 encode/decode roundtrip
unsafe fn test_bigint_sleb128(n: Value) {
    let mut buf = [0u8; 100];
    let s = bigint_sleb128_size(n);
    bigint_sleb128_encode(n, buf.as_mut_ptr());
    let mut buf_ = Buf {
        ptr: buf.as_mut_ptr(),
        end: buf.as_mut_ptr().add(100),
    };
    let n2 = bigint_sleb128_decode(&mut buf_ as *mut _);
    assert!(bigint_eq(n, n2));
    assert_eq!(buf_.ptr.offset_from(buf.as_ptr()), s as isize);
}
