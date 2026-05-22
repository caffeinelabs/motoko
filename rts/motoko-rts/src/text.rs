//! The implementation of the Text type in Motoko
//!
//! One main goal of this datastructure (inspired by ropes and similar) is to support constant time
//! concatenation, by having a dedicated heap object for the concatenation of two strings.
//!
//! The first goal was to wire up this Rust code with the RTS that encapsulates the internals of
//! strings.
//!
//! This encapsulation is not complete (and likely never will)
//!  * the compiler needs to emit static text literals,
//!  * the garbage collector needs to know some of the internals.
//!
//! In a subsequent step, the actual concatenation node has been introduced.
//!
//! From here on, there are stretch goals like:
//!  - restructure recursive code to not use unbounded Rust stack
//!  - maybe rebalancing

// Layout of a concat node:
//
// ┌────────────┬─────────┬───────┬───────┐
// │ obj header │ n_bytes │ text1 │ text2 │
// └────────────┴─────────┴───────┴───────┘
//
// The object header includes tag (`TAG_CONCAT`) and forwarding pointer.
// Note that `CONCAT_LEN` and `BLOB_LEN` are identical, so no need to check the tag to know the
// size of the text.

use crate::barriers::allocation_barrier;
use crate::mem_utils::memcpy_bytes;
use crate::memory::{Memory, alloc_blob};
use crate::rts_trap_with;
use crate::types::{Blob, Bytes, Concat, TAG_BLOB_T, TAG_CONCAT, Value, size_of};

use alloc::string::String;
use core::cmp::{Ordering, min};
use core::{slice, str};
use motoko_rts_macros::classical_persistence;

use crate::libc_declarations::memcmp;

#[classical_persistence]
use crate::types::Stream;

use motoko_rts_macros::ic_mem_fn;

const MAX_STR_SIZE: Bytes<usize> = Bytes((1 << (usize::BITS - 2)) - 1);

// Strings smaller than this must be blobs
// Make this MAX_STR_SIZE to disable the use of ropes completely, e.g. for debugging
const MIN_CONCAT_SIZE: Bytes<usize> = Bytes(9);

// Note: Post allocation barrier needs to be applied after initilization.
unsafe fn alloc_text_blob<M: Memory>(mem: &mut M, size: Bytes<usize>) -> Value {
    if size > MAX_STR_SIZE {
        rts_trap_with("alloc_text_blob: Text too large");
    }
    alloc_blob(mem, TAG_BLOB_T, size)
}

#[ic_mem_fn]
pub unsafe fn text_of_ptr_size<M: Memory>(mem: &mut M, buf: *const u8, n: Bytes<usize>) -> Value {
    let blob = alloc_text_blob(mem, n);
    let payload_addr = blob.as_blob_mut().payload_addr();
    memcpy_bytes(payload_addr as usize, buf as usize, n);
    allocation_barrier(blob)
}

pub unsafe fn text_of_str<M: Memory>(mem: &mut M, s: &str) -> Value {
    text_of_ptr_size(mem, s.as_ptr(), Bytes(s.len()))
}

#[ic_mem_fn]
pub unsafe fn text_concat<M: Memory>(mem: &mut M, s1: Value, s2: Value) -> Value {
    let blob1_len = text_size(s1);
    let blob2_len = text_size(s2);

    if blob1_len == Bytes(0) {
        return s2;
    }

    if blob2_len == Bytes(0) {
        return s1;
    }

    let new_len = blob1_len + blob2_len;

    // Short texts are copied into a single blob
    if new_len < MIN_CONCAT_SIZE {
        // Because lengths of texts are smaller than MIN_CONCAT_SIZE we know both are blobs so the
        // casts are safe
        let blob1 = s1.as_blob();
        let blob2 = s2.as_blob();

        let r = alloc_text_blob(mem, new_len);
        let r_payload: *mut u8 = r.as_blob_mut().payload_addr();
        memcpy_bytes(
            r_payload as usize,
            blob1.payload_const() as usize,
            blob1_len,
        );
        memcpy_bytes(
            r_payload.add(blob1_len.as_usize()) as usize,
            blob2.payload_const() as usize,
            blob2_len,
        );
        return allocation_barrier(r);
    }

    // Check max size
    if new_len > MAX_STR_SIZE {
        rts_trap_with("text_concat: Text too large");
    }

    // Create concat node
    let r = mem.alloc_words(size_of::<Concat>());
    let r_concat = r.get_ptr() as *mut Concat;
    (*r_concat).header.tag = TAG_CONCAT;
    (*r_concat).header.init_forward(r);
    (*r_concat).n_bytes = new_len;
    (*r_concat).text1 = s1.forward_if_possible();
    (*r_concat).text2 = s2.forward_if_possible();
    allocation_barrier(r)
}

// Leaving breadcrumbs in the destination buffer for which concat node/blob to continue
// serializing
#[repr(packed)]
struct Crumb {
    /// Pointer to the concat node/blob to serialize
    t: Value,
    /// Where to serialize the concat node/blob
    next: *const Crumb,
}

#[unsafe(no_mangle)]
unsafe extern "C" fn text_to_buf(mut s: Value, mut buf: *mut u8) {
    let mut next_crumb: *const Crumb = core::ptr::null();

    loop {
        let s_ptr = s.as_obj();
        if s_ptr.tag() == TAG_BLOB_T {
            let blob = s_ptr.as_blob();
            memcpy_bytes(buf as usize, blob.payload_addr() as usize, blob.len());

            if next_crumb.is_null() {
                return;
            }

            buf = next_crumb as *mut u8;
            s = (*next_crumb).t;
            next_crumb = (*next_crumb).next;
        } else {
            let concat = s_ptr.as_concat();
            let s1 = concat.text1();
            let s2 = concat.text2();

            let s1_len = text_size(s1);
            let s2_len = text_size(s2);

            if s2_len < Bytes(core::mem::size_of::<Crumb>()) {
                // If second string is smaller than size of a crumb just do it directly
                text_to_buf(s2, buf.add(s1_len.as_usize()));
                s = s1;
            } else {
                // Otherwise leave a breadcrumb to the location of the second string
                let new_crumb: *mut Crumb = buf.add(s1_len.as_usize()) as *mut Crumb;
                (*new_crumb).t = s2;
                (*new_crumb).next = next_crumb;
                next_crumb = new_crumb;
                s = s1;
            }
        }
    }
}

#[unsafe(no_mangle)]
#[classical_persistence]
unsafe extern "C" fn stream_write_text(stream: *mut Stream, mut s: Value) {
    use crate::types::TAG_BLOB_B;
    loop {
        let s_ptr = s.as_obj();
        if s_ptr.tag() == TAG_BLOB_B || s_ptr.tag() == TAG_BLOB_T {
            let blob = s_ptr.as_blob();
            stream.cache_bytes(blob.payload_addr(), blob.len());
            break;
        } else {
            let concat = s_ptr.as_concat();
            stream_write_text(stream, concat.text1());
            s = concat.text2()
        }
    }
}

// Straighten into contiguous memory, if needed (e.g. for system calls)
#[ic_mem_fn]
pub unsafe fn blob_of_text<M: Memory>(mem: &mut M, s: Value) -> Value {
    let obj = s.as_obj();
    if obj.tag() == TAG_BLOB_T {
        s
    } else {
        let concat = obj.as_concat();
        let r = alloc_text_blob(mem, (*concat).n_bytes);
        text_to_buf(s, r.as_blob_mut().payload_addr());
        allocation_barrier(r)
    }
}

/// Size of the text, in bytes
#[unsafe(no_mangle)]
pub unsafe extern "C" fn text_size(s: Value) -> Bytes<usize> {
    // We don't know whether the string is a blob or concat, but both types have the length in same
    // location so using any of the types to get the length is fine
    // NB. We can't use `s.as_blob()` here as that method checks the tag in debug mode
    s.check_forwarding_pointer();
    (s.forward().get_ptr() as *mut Blob).len()
}

/// Compares texts from given offset on for the given number of bytes. All assumed to be in range.
unsafe fn text_compare_range(
    s1: Value,
    offset1: Bytes<usize>,
    s2: Value,
    offset2: Bytes<usize>,
    n: Bytes<usize>,
) -> Ordering {
    // Follow the left/right strings of concat nodes until we reach to blobs or concats that cannot
    // be split further (the range spans left and right strings)
    let (s1, offset1) = text_get_range(s1, offset1, n);
    let (s2, offset2) = text_get_range(s2, offset2, n);

    let s1_obj = s1.as_obj();
    let s2_obj = s2.as_obj();

    // Decompose concats
    if s1_obj.tag() == TAG_CONCAT {
        let s1_concat = s1_obj.as_concat();
        let n_compared = text_size(s1_concat.text1()) - offset1;
        let cmp = text_compare_range(s1_concat.text1(), offset1, s2, offset2, n_compared);
        match cmp {
            Ordering::Less | Ordering::Greater => cmp,
            Ordering::Equal => text_compare_range(
                s1_concat.text2(),
                Bytes(0),
                s2,
                offset2 + n_compared,
                n - n_compared,
            ),
        }
    } else if s2_obj.tag() == TAG_CONCAT {
        let s2_concat = s2_obj.as_concat();
        let n_compared = text_size(s2_concat.text1()) - offset2;
        let cmp = text_compare_range(s1, offset1, s2_concat.text1(), offset2, n_compared);
        match cmp {
            Ordering::Less | Ordering::Greater => cmp,
            Ordering::Equal => text_compare_range(
                s1,
                offset1 + n_compared,
                s2_concat.text2(),
                Bytes(0),
                n - n_compared,
            ),
        }
    } else {
        debug_assert_eq!(s1_obj.tag(), TAG_BLOB_T);
        debug_assert_eq!(s2_obj.tag(), TAG_BLOB_T);

        let s1_blob = s1_obj.as_blob();
        let s2_blob = s2_obj.as_blob();

        let cmp = memcmp(
            s1_blob.payload_addr().add(offset1.as_usize()) as *const _,
            s2_blob.payload_addr().add(offset2.as_usize()) as *const _,
            n.as_usize(),
        );

        if cmp < 0 {
            Ordering::Less
        } else if cmp == 0 {
            Ordering::Equal
        } else {
            Ordering::Greater
        }
    }
}

/// Follow left/right strings of concat nodes until we reach to a BLOB or a CONCAT that can't be
/// split further (i.e. range spans left and right nodes). Returns a BLOB or CONCAT.
unsafe fn text_get_range(
    mut s: Value,
    mut offset: Bytes<usize>,
    n: Bytes<usize>,
) -> (Value, Bytes<usize>) {
    loop {
        let s_obj = s.as_obj();

        if s_obj.tag() == TAG_CONCAT {
            let s_concat = s_obj.as_concat();

            let left = s_concat.text1();
            let left_size = text_size(left);

            // Follow left node?
            if left_size >= offset + n {
                s = left;
                continue;
            }

            // Follow right node?
            if offset >= left_size {
                s = s_concat.text2();
                offset -= left_size;
                continue;
            }
        } else {
            debug_assert_eq!(s_obj.tag(), TAG_BLOB_T);
        }

        break;
    }

    (s, offset)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn text_compare(s1: Value, s2: Value) -> isize {
    let n1 = text_size(s1);
    let n2 = text_size(s2);
    let n = min(n1, n2);

    match text_compare_range(s1, Bytes(0), s2, Bytes(0), n) {
        Ordering::Less => -1,
        Ordering::Greater => 1,
        Ordering::Equal => {
            if n1 > n {
                1
            } else if n2 > n {
                -1
            } else {
                0
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blob_compare(s1: Value, s2: Value) -> isize {
    let n1 = text_size(s1);
    let n2 = text_size(s2);
    let n = min(n1, n2);

    let payload1 = s1.as_blob().payload_const();
    let payload2 = s2.as_blob().payload_const();
    let cmp = memcmp(payload1 as *const _, payload2 as *const _, n.as_usize()) as isize;

    if cmp == 0 {
        if n1 < n2 {
            -1
        } else if n1 > n2 {
            1
        } else {
            0
        }
    } else {
        cmp
    }
}

/// Length in characters
#[unsafe(no_mangle)]
pub unsafe extern "C" fn text_len(text: Value) -> usize {
    if text.tag() == TAG_BLOB_T {
        let blob = text.as_blob();
        let payload_addr = blob.payload_const();
        let len = blob.len();

        str::from_utf8_unchecked(slice::from_raw_parts(
            payload_addr as *const u8,
            len.as_usize(),
        ))
        .chars()
        .count()
    } else {
        let concat = text.as_concat();
        text_len(concat.text1()) + text_len(concat.text2())
    }
}

/// Decodes the character at the pointer. Returns the character, the size via the `size` parameter
pub unsafe fn decode_code_point(s: *const u8, size: *mut usize) -> u32 {
    // 0xxxxxxx
    // 110xxxxx 10xxxxxx
    // 1110xxxx 10xxxxxx 10xxxxxx
    // 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx

    let (size, mut value) = {
        let leading_ones = (*s).leading_ones() as usize;
        if leading_ones == 0 {
            *size = 1;
            return *s as u32;
        } else {
            *size = leading_ones;
            (leading_ones, ((*s << leading_ones) >> leading_ones) as u32)
        }
    };

    for i in 1..size {
        value <<= 6;
        value += ((*s.add(i)) & 0b00111111) as u32;
    }

    value as u32
}

/// Allocate a text from a character
#[ic_mem_fn]
pub unsafe fn text_singleton<M: Memory>(mem: &mut M, char: u32) -> Value {
    let mut buf = [0u8; 4];
    let str_len = char::from_u32_unchecked(char).encode_utf8(&mut buf).len();

    let blob_ptr = alloc_text_blob(mem, Bytes(str_len));

    let blob = blob_ptr.as_blob_mut();

    for i in 0..str_len {
        blob.set(i, buf[i]);
    }

    allocation_barrier(blob_ptr)
}

/// Convert a Text value into lower case (generally a different length).
#[ic_mem_fn]
pub unsafe fn text_lowercase<M: Memory>(mem: &mut M, text: Value) -> Value {
    text_convert(mem, text, |s| s.to_lowercase())
}

/// Convert a Text value into upper case (generally a different length).
#[ic_mem_fn]
pub unsafe fn text_uppercase<M: Memory>(mem: &mut M, text: Value) -> Value {
    text_convert(mem, text, |s| s.to_uppercase())
}

/// Convert a Text value via given to_string function
unsafe fn text_convert<M: Memory, F>(mem: &mut M, text: Value, to_string: F) -> Value
where
    F: Fn(&str) -> String,
{
    let blob = blob_of_text(mem, text).as_blob_mut();
    let str = str::from_utf8_unchecked(slice::from_raw_parts(
        blob.payload_addr() as *const u8,
        blob.len().as_usize(),
    ));
    let string = to_string(&str);
    let bytes = string.as_bytes();
    let lowercase = alloc_blob(mem, TAG_BLOB_T, Bytes(bytes.len()));
    let mut i = 0;
    let target_ptr = lowercase.as_blob_mut().payload_addr();
    for b in bytes {
        *target_ptr.offset(i) = *b;
        i += 1;
    }
    allocation_barrier(lowercase)
}
