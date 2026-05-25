//! Implements big int support:
//!
//! - libtommath memory management
//! - libtommath wrappers
//! - (s)leb128 encoding/decoding for bigints

/*
A libtommath arbitrary precision integer is a struct (`mp_int`) that contains a pointer to a data
array.

 - The libtommath library never allocates the struct, so we are in full control.

   We allocate that struct on the stack if we need it temporarily, or embed it in a TAG_BIGINT
   object when we persist the number.

 - The data array is allocated with mp_calloc() and mp_realloc(), and these functions are _only_ used
   to allocate the data array.

   We provide these calls, allocate TAG_BIGINT objects (leaving space for the mp_int), and pass a
   pointer _into_ this object back to libtommath, which stores it in the `mp_digit* dp`
   field of the struct.

When persisting a `mp_int` (presumably stack-allocated), we know that the `mp_digit` pointer
points to a `TAG_BIGINT` with sufficient space for the `mp_int` data. We copy the `mp_int`
there, and use the overall `TAG_BIGINT` as the bignum object.

This scheme makes the following assumptions:

 - libtommath never modifies the data on the heap.
   (or put differently, we only pass those to libtommath when they are immutable)
 - libtommath uses mp_calloc() and mp_realloc() _only_ to allocate the `mp_digit *` array.
*/

use crate::barriers::allocation_barrier;
use crate::buf::{Buf, read_byte};
use crate::mem_utils::memcpy_bytes;
use crate::memory::Memory;
use crate::tommath_bindings::*;
use crate::types::{BigInt, Bytes, TAG_BIGINT, Value, size_of};

use crate::libc_declarations::c_void;

#[classical_persistence]
use crate::types::Stream;

use motoko_rts_macros::{classical_persistence, ic_mem_fn};

#[cfg(feature = "ic")]
use motoko_rts_macros::enhanced_orthogonal_persistence;

// Provided by generated code
#[cfg(feature = "ic")]
unsafe extern "C" {
    #[enhanced_orthogonal_persistence]
    fn int_from_i64(value: isize) -> Value;
    #[classical_persistence]
    fn int_from_i32(value: isize) -> Value;
}

#[cfg(feature = "ic")]
#[enhanced_orthogonal_persistence]
unsafe fn int_from_isize(value: isize) -> Value {
    int_from_i64(value)
}

#[cfg(feature = "ic")]
#[classical_persistence]
unsafe fn int_from_isize(value: isize) -> Value {
    int_from_i32(value)
}

unsafe fn mp_alloc<M: Memory>(mem: &mut M, size: Bytes<usize>) -> *mut u8 {
    let ptr = mem.alloc_words(size_of::<BigInt>() + size.to_words());
    // NB. Cannot use as_bigint() here as header is not written yet
    let blob = ptr.get_ptr() as *mut BigInt;
    (*blob).header.tag = TAG_BIGINT;
    (*blob).header.init_forward(ptr);

    // libtommath stores the size of the object in alloc as count of mp_digits (u64)
    let size = size.as_usize();
    debug_assert_eq!((size % core::mem::size_of::<mp_digit>()), 0);
    let count = size / core::mem::size_of::<mp_digit>();
    assert!(count <= i32::MAX as usize);
    (*blob).mp_int.alloc = count as i32;
    allocation_barrier(ptr);
    blob.payload_addr() as *mut u8
}

#[ic_mem_fn]
pub unsafe fn mp_calloc<M: Memory>(
    mem: &mut M,
    n_elems: usize,
    elem_size: Bytes<usize>,
) -> *mut c_void {
    debug_assert_eq!(elem_size.0, core::mem::size_of::<mp_digit>());
    // Overflow check for the following multiplication
    if n_elems > 1 << 30 {
        bigint_trap();
    }
    let size = Bytes(n_elems * elem_size.0);
    let payload = mp_alloc(mem, size) as *mut usize;

    // NB. alloc_bytes rounds up to words so we do the same here to set the whole buffer
    for i in 0..size.to_words().as_usize() {
        *payload.add(i) = 0;
    }

    payload as *mut _
}

#[ic_mem_fn]
pub unsafe fn mp_realloc<M: Memory>(
    mem: &mut M,
    ptr: *mut c_void,
    old_size: Bytes<usize>,
    new_size: Bytes<usize>,
) -> *mut c_void {
    let bigint = BigInt::from_payload(ptr as *mut mp_digit);

    debug_assert_eq!((*bigint).header.tag, TAG_BIGINT);
    debug_assert_eq!(bigint.len(), old_size);

    if new_size > bigint.len() {
        let new_ptr = mp_alloc(mem, new_size);
        memcpy_bytes(new_ptr as usize, ptr as usize, old_size);
        new_ptr as *mut _
    } else if new_size == bigint.len() {
        ptr
    } else {
        // libtommath only shrinks via mp_shrink and we do not use that function, so this should not
        // happen.
        panic!("mp_realloc: trying to shrink");
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_free(_ptr: *mut c_void, _size: usize) {}

/*
Note on libtommath error handling
---------------------------------

Most libtommath operations return an enum to signal error codes. These are (see tommath.h):

   MP_OKAY  = 0,   /* no error */
   MP_ERR   = -1,  /* unknown error */
   MP_MEM   = -2,  /* out of mem */
   MP_VAL   = -3,  /* invalid input */
   MP_ITER  = -4,  /* maximum iterations reached */
   MP_BUF   = -5   /* buffer overflow, supplied buffer too small */

We will never hit MP_MEM, because our allocation functions trap if they cannot allocate. But the
others can happen (e.g. division by 0). In that case, we call a trap function provided by the
compiler.
*/

// Trap function generated by compiler. Originally added in e2ca6a1. I think this could be
// simplified now by calling rts_trap and removing generated code from the compiler.
unsafe extern "C" {
    pub(crate) fn bigint_trap() -> !;
}

#[inline]
pub(crate) unsafe fn check(err: mp_err) {
    if err != 0 {
        bigint_trap();
    }
}

#[inline]
pub(crate) unsafe fn mp_get_u32(p: *const mp_int) -> u32 {
    mp_get_i32(p) as u32
}

#[cfg(feature = "ic")]
unsafe fn mp_get_u64(p: *const mp_int) -> u64 {
    mp_get_i64(p) as u64
}

pub(crate) unsafe fn mp_isneg(p: *const mp_int) -> bool {
    debug_assert_eq!((*p).sign, (*p).sign & 1);
    (*p).sign != 0
}

pub(crate) unsafe fn mp_iszero(p: *const mp_int) -> bool {
    (*p).used == 0
}

// Allocates an mp_int on the stack
pub(crate) unsafe fn tmp_bigint() -> mp_int {
    let mut i: mp_int = core::mem::zeroed();
    check(mp_init(&mut i));
    i
}

// Persists an mp_int from the stack on the heap
pub(crate) unsafe fn persist_bigint(i: mp_int) -> Value {
    if i.dp == core::ptr::null_mut() {
        panic!("persist_bigint: dp == NULL?");
    }
    let r = BigInt::from_payload(i.dp);
    if (*r).mp_int.alloc != i.alloc {
        panic!("persist_bigint: alloc changed?");
    }
    (*r).mp_int = i;
    Value::from_ptr(r as usize)
}

#[unsafe(no_mangle)]
#[classical_persistence]
pub unsafe extern "C" fn bigint_of_word32(w: u32) -> Value {
    let mut i = tmp_bigint();
    mp_set_u32(&mut i, w);
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
#[classical_persistence]
unsafe extern "C" fn bigint_of_int32(j: i32) -> Value {
    let mut i = tmp_bigint();
    mp_set_i32(&mut i, j);
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_to_word32_wrap(p: Value) -> u32 {
    mp_get_u32(p.as_bigint().mp_int_ptr())
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_to_word32_trap(p: Value) -> u32 {
    let mp_int = p.as_bigint().mp_int_ptr();

    if mp_isneg(mp_int) || mp_count_bits(mp_int) > 32 {
        bigint_trap();
    }

    mp_get_u32(mp_int)
}

// p : BigInt, msg : Blob
#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_to_word32_trap_with(p: Value, msg: Value) -> u32 {
    let mp_int = p.as_bigint().mp_int_ptr();

    if mp_isneg(mp_int) || mp_count_bits(mp_int) > 32 {
        let length = msg.as_blob().len().as_usize();
        assert!(length <= u32::MAX as usize);
        crate::rts_trap(msg.as_blob().payload_const(), length as u32);
    }

    mp_get_u32(mp_int)
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_to_word64_wrap(p: Value) -> u64 {
    mp_get_u64(p.as_bigint().mp_int_ptr())
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_to_word64_trap(p: Value) -> u64 {
    let mp_int = p.as_bigint().mp_int_ptr();

    if mp_isneg(mp_int) || mp_count_bits(mp_int) > 64 {
        bigint_trap();
    }

    mp_get_u64(mp_int)
}

// p : BigInt, msg : Blob
#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_to_word64_trap_with(p: Value, msg: Value) -> u64 {
    let mp_int = p.as_bigint().mp_int_ptr();

    if mp_isneg(mp_int) || mp_count_bits(mp_int) > 64 {
        let length = msg.as_blob().len().as_usize();
        assert!(length <= u32::MAX as usize);
        crate::rts_trap(msg.as_blob().payload_const(), length as u32);
    }

    mp_get_u64(mp_int)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bigint_of_word64(w: u64) -> Value {
    let mut i = tmp_bigint();
    mp_set_u64(&mut i, w);
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_of_int64(j: i64) -> Value {
    let mut i = tmp_bigint();
    mp_set_i64(&mut i, j);
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_of_float64(j: f64) -> Value {
    // Fast path: Determine when the integer numbers can be represented as a compact (unboxed) `Int`.
    let is_compact = match usize::BITS {
        u64::BITS => {
            // The integer can be represented in compact 64-bit scalar if it is in the range
            // `-4611686018427387904 == -2 ** 62 <= j as i64 < 2 ** 62 == 4611686018427387904`, by
            // considering that the two most significant bits are reserved for the `BigInt` scalar tag.
            // The closest binary64 float representations in IEEE 754 for the boundaries are:
            // Lower boundary approximation >= -2 ** 62:
            //   `f64::from_bits(0xC3CF_FFFF_FFFF_FFFF`) == -4611686018427387400 > -4611686018427387904
            // Upper boundary approximation < 2 ** 62:
            //   `f64::from_bits(0x43CF_FFFF_FFFF_FFFF) == 4611686018427387400 < 4611686018427387904.
            // IEEE754 double precision encoding:
            // ┌───────────────┬────────────────────────┬─────────────────────────┐
            // │ sign (bit 63) │ exponent (bits 52..62) |  mantissa (bits 0..51)  |
            // └───────────────┴────────────────────────┴─────────────────────────┘
            // The exponent has a bias of 1023:
            // * Example: Exponent 61 is encoded 0x43C == 1023 + 61.
            // The mantissa has an implicit extra most significant bit 1.
            // * Example: Mantissa `F_FFFF_FFFF_FFFF` actually represents `1F_FFFF_FFFF_FFFF`.
            j >= -4611686018427387400.0f64 && j <= 4611686018427387400.0f64
        }
        u32::BITS => {
            // The integer can be represented in compact 32-bit scalar if it is in the range
            // `-1073741824 == 0xc0000000 <= j as i32 <= 0x3fffffff == 1073741823`, by
            // considering that the two most significant bits are reserved for the `BigInt` scalar tag.
            j < 1073741824.0 && j > -1073741825.0
        }
        _ => unreachable!(),
    };
    if is_compact {
        // defer to generated code to create compact or boxed Int value
        return int_from_isize(j as isize);
    }
    let mut i = tmp_bigint();
    check(mp_set_double(&mut i, j));
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_to_float64(p: Value) -> f64 {
    debug_assert!(!p.is_scalar());
    let mp_int = p.as_bigint().mp_int_ptr();
    mp_get_double(mp_int)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bigint_eq(a: Value, b: Value) -> bool {
    mp_cmp(a.as_bigint().mp_int_ptr(), b.as_bigint().mp_int_ptr()) == 0
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_lt(a: Value, b: Value) -> bool {
    mp_cmp(a.as_bigint().mp_int_ptr(), b.as_bigint().mp_int_ptr()) < 0
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_gt(a: Value, b: Value) -> bool {
    mp_cmp(a.as_bigint().mp_int_ptr(), b.as_bigint().mp_int_ptr()) > 0
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_le(a: Value, b: Value) -> bool {
    mp_cmp(a.as_bigint().mp_int_ptr(), b.as_bigint().mp_int_ptr()) <= 0
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_ge(a: Value, b: Value) -> bool {
    mp_cmp(a.as_bigint().mp_int_ptr(), b.as_bigint().mp_int_ptr()) >= 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bigint_add(a: Value, b: Value) -> Value {
    let mut i = tmp_bigint();
    check(mp_add(
        a.as_bigint().mp_int_ptr(),
        b.as_bigint().mp_int_ptr(),
        &mut i,
    ));
    persist_bigint(i)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bigint_sub(a: Value, b: Value) -> Value {
    let mut i = tmp_bigint();
    check(mp_sub(
        a.as_bigint().mp_int_ptr(),
        b.as_bigint().mp_int_ptr(),
        &mut i,
    ));
    persist_bigint(i)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bigint_mul(a: Value, b: Value) -> Value {
    let mut i = tmp_bigint();
    check(mp_mul(
        a.as_bigint().mp_int_ptr(),
        b.as_bigint().mp_int_ptr(),
        &mut i,
    ));
    persist_bigint(i)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bigint_pow(a: Value, b: Value) -> Value {
    let exp = bigint_to_word32_trap(b);
    let mut i = tmp_bigint();
    check(mp_expt_u32(a.as_bigint().mp_int_ptr(), exp, &mut i));
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_div(a: Value, b: Value) -> Value {
    let mut i = tmp_bigint();
    check(mp_div(
        a.as_bigint().mp_int_ptr(),
        b.as_bigint().mp_int_ptr(),
        &mut i,
        core::ptr::null_mut(),
    ));
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_rem(a: Value, b: Value) -> Value {
    let mut i = tmp_bigint();
    check(mp_div(
        a.as_bigint().mp_int_ptr(),
        b.as_bigint().mp_int_ptr(),
        core::ptr::null_mut(),
        &mut i,
    ));
    persist_bigint(i)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bigint_neg(a: Value) -> Value {
    let mut i = tmp_bigint();
    check(mp_neg(a.as_bigint().mp_int_ptr(), &mut i));
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_abs(a: Value) -> Value {
    let mut i = tmp_bigint();
    check(mp_abs(a.as_bigint().mp_int_ptr(), &mut i));
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_isneg(a: Value) -> bool {
    mp_isneg(a.as_bigint().mp_int_ptr())
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_lsh(a: Value, b: isize) -> Value {
    let mut i = tmp_bigint();
    check(mp_mul_2d(a.as_bigint().mp_int_ptr(), b as i32, &mut i));
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_rsh(a: Value, b: isize) -> Value {
    let mut i = tmp_bigint();
    check(mp_div_2d(
        a.as_bigint().mp_int_ptr(),
        b as i32,
        &mut i,
        core::ptr::null_mut(),
    ));
    persist_bigint(i)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_count_bits(a: Value) -> usize {
    mp_count_bits(a.as_bigint().mp_int_ptr()) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bigint_leb128_size(a: Value) -> usize {
    if mp_iszero(a.as_bigint().mp_int_ptr()) {
        1
    } else {
        (bigint_count_bits(a) + 6) / 7 // divide by 7, round up
    }
}

// `add_bit` argument is to make this work for both leb and sleb encoding
unsafe fn bigint_leb128_encode_go(tmp: *mut mp_int, mut buf: *mut u8, add_bit: bool) {
    if mp_isneg(tmp) {
        bigint_trap();
    }

    loop {
        let byte = mp_get_u32(tmp) as u8;
        check(mp_div_2d(tmp, 7, tmp, core::ptr::null_mut()));
        if !mp_iszero(tmp) || (add_bit && byte & (1 << 6) != 0) {
            *buf = byte | (1 << 7);
            buf = buf.add(1);
        } else {
            *buf = byte;
            break;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bigint_leb128_encode(n: Value, buf: *mut u8) {
    let mut tmp: mp_int = core::mem::zeroed(); // or core::mem::uninitialized?
    check(mp_init_copy(&mut tmp, n.as_bigint().mp_int_ptr()));
    bigint_leb128_encode_go(&mut tmp, buf, false)
}

#[unsafe(no_mangle)]
#[classical_persistence]
pub unsafe extern "C" fn bigint_leb128_stream_encode(stream: *mut Stream, n: Value) {
    debug_assert!(!stream.is_forwarded());
    let mut tmp: mp_int = core::mem::zeroed(); // or core::mem::uninitialized?
    check(mp_init_copy(&mut tmp, n.as_bigint().mp_int_ptr()));
    stream.write_leb128(&mut tmp, false)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bigint_2complement_bits(n: Value) -> usize {
    let mp_int = n.as_bigint().mp_int_ptr();
    if mp_isneg(mp_int) {
        let mut tmp: mp_int = core::mem::zeroed(); // or core::mem::uninitialized?
        check(mp_init_copy(&mut tmp, mp_int));
        check(mp_incr(&mut tmp));
        1 + mp_count_bits(&tmp) as usize
    } else {
        1 + mp_count_bits(mp_int) as usize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bigint_sleb128_size(n: Value) -> usize {
    (bigint_2complement_bits(n) + 6) / 7 // divide by 7, round up
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bigint_sleb128_encode(n: Value, buf: *mut u8) {
    let mut tmp: mp_int = core::mem::zeroed(); // or core::mem::uninitialized?
    check(mp_init_copy(&mut tmp, n.as_bigint().mp_int_ptr()));

    if mp_isneg(&tmp) {
        // Turn negative numbers into the two's complement of the right size
        let bytes = bigint_sleb128_size(n);
        let mut big: mp_int = core::mem::zeroed();
        check(mp_init(&mut big));
        check(mp_2expt(&mut big, 7 * bytes as i32));
        check(mp_add(&mut tmp, &big, &mut tmp));
        bigint_leb128_encode_go(&mut tmp, buf, false)
    } else {
        bigint_leb128_encode_go(&mut tmp, buf, true)
    }
}

#[unsafe(no_mangle)]
#[classical_persistence]
pub unsafe extern "C" fn bigint_sleb128_stream_encode(stream: *mut Stream, n: Value) {
    debug_assert!(!stream.is_forwarded());
    let mut tmp: mp_int = core::mem::zeroed(); // or core::mem::uninitialized?
    check(mp_init_copy(&mut tmp, n.as_bigint().mp_int_ptr()));

    if mp_isneg(&tmp) {
        // Turn negative numbers into the two's complement of the right size
        let mut big: mp_int = core::mem::zeroed();
        check(mp_init(&mut big));
        let bytes = bigint_sleb128_size(n);
        check(mp_2expt(&mut big, 7 * bytes as i32));
        check(mp_add(&mut tmp, &big, &mut tmp));
        stream.write_leb128(&mut tmp, false)
    } else {
        stream.write_leb128(&mut tmp, true)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bigint_leb128_decode(buf: *mut Buf) -> Value {
    let mut i = tmp_bigint();
    let mut tmp = tmp_bigint();

    let mut shift = 0;
    loop {
        let byte = read_byte(buf);
        mp_set_u32(&mut tmp, (byte & 0b0111_1111) as u32);
        check(mp_mul_2d(&mut tmp, shift, &mut tmp));
        check(mp_add(&mut i, &tmp, &mut i));
        shift += 7;

        if byte & 0b1000_0000 == 0 {
            break;
        }
    }

    persist_bigint(i)
}

#[cfg(feature = "ic")]
const BITS_PER_CHUNK: usize = 7;

#[cfg(feature = "ic")]
const MAX_CHUNKS_PER_WORD: usize = (usize::BITS as usize + BITS_PER_CHUNK - 1) / BITS_PER_CHUNK;

#[classical_persistence]
#[cfg(feature = "ic")]
const _: () = assert!(MAX_CHUNKS_PER_WORD == 5);

#[enhanced_orthogonal_persistence]
#[cfg(feature = "ic")]
const _: () = assert!(MAX_CHUNKS_PER_WORD == 10);

/// Decode bytes of LEB128 data to a compact bignum `Value`.
/// The number of 7-bit chunks are located in the lower portion of `leb`
/// as indicated by `bits`.
///
#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bigint_leb128_decode_word64(
    mut leb: u64,
    mut bits: u64,
    buf: *mut Buf,
) -> Value {
    let continuations = bits as usize / 8;
    buf.advance(continuations + 1);

    let mut mask: u64 = 0b111_1111; // sliding mask
    let mut acc = 0;
    loop {
        acc |= leb & mask;
        if bits < 8 {
            if continuations == MAX_CHUNKS_PER_WORD - 1 {
                break;
            }
            return int_from_isize(acc as isize);
        }
        bits -= 8;
        mask <<= 7;
        leb >>= 1;
    }

    let tentative = (acc as isize) << 1 >> 1; // top two bits must match
    if tentative as u64 == acc {
        // roundtrip is valid
        return int_from_isize(tentative);
    }

    bigint_of_word64(acc)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bigint_sleb128_decode(buf: *mut Buf) -> Value {
    let mut i = tmp_bigint();
    let mut tmp = tmp_bigint();

    let mut shift = 0;
    let mut last_sign_bit_set;
    loop {
        let byte = read_byte(buf);
        mp_set_u32(&mut tmp, (byte & 0b0111_1111) as u32);
        check(mp_mul_2d(&mut tmp, shift, &mut tmp));
        check(mp_add(&mut i, &tmp, &mut i));
        last_sign_bit_set = byte & 0b0100_0000 != 0;
        shift += 7;

        if byte & 0b1000_0000 == 0 {
            break;
        }
    }

    if last_sign_bit_set {
        // Negative number, un-2-complement it
        let mut big = tmp_bigint();
        check(mp_2expt(&mut big, shift));
        check(mp_sub(&mut i, &big, &mut i));
    }

    persist_bigint(i)
}

/// Decode bytes of SLEB128 data to a compact bignum `Value`.
/// The number of 7-bit chunks are located in the lower portion of `sleb`
/// as indicated by `bits`.
///
#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bigint_sleb128_decode_word64(
    mut sleb: u64,
    mut bits: u64,
    buf: *mut Buf,
) -> Value {
    let continuations = bits as usize / 8;
    buf.advance(continuations + 1);

    let mut mask: u64 = 0b111_1111; // sliding mask
    let mut acc = 0;
    loop {
        acc |= sleb & mask;
        if bits < 8 {
            if continuations == MAX_CHUNKS_PER_WORD - 1 {
                break;
            }
            let sext = usize::BITS as usize - (BITS_PER_CHUNK * (continuations + 1)); // this many top bits will get a copy of the sign
            return int_from_isize((acc as isize) << sext >> sext);
        }
        bits -= 8;
        mask <<= 7;
        sleb >>= 1;
    }

    sleb128_decode_word64_result(acc)
}

#[cfg(feature = "ic")]
#[classical_persistence]
unsafe fn sleb128_decode_word64_result(accumulator: u64) -> Value {
    // Check if it fits into 32-bit or needs boxing to BigInt.
    const UNUSED_BITS: u32 = u64::BITS - (BITS_PER_CHUNK * MAX_CHUNKS_PER_WORD) as u32;
    const _: () = assert!(UNUSED_BITS == 29);
    let signed = (accumulator as i64) << UNUSED_BITS >> UNUSED_BITS;
    let tentative = (signed as isize) << 1 >> 1; // top two bits must match
    if tentative as i64 == signed {
        // roundtrip is valid
        return int_from_isize(tentative);
    }
    bigint_of_int64(signed)
}

#[cfg(feature = "ic")]
#[enhanced_orthogonal_persistence]
unsafe fn sleb128_decode_word64_result(accumulator: u64) -> Value {
    // No unused bits in 64-bit representation. The sign bit is already set at bit 63.
    int_from_isize(accumulator as isize)
}
