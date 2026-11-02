//! LEB1128 encoding. Reference: https://en.wikipedia.org/wiki/LEB128

use crate::buf::{Buf, read_byte};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn leb128_encode(mut val: usize, mut buf: *mut u8) {
    loop {
        let byte = (val & 0b0111_1111) as u8;
        val >>= 7;
        if val == 0 {
            *buf = byte;
            break;
        } else {
            *buf = byte | 0b1000_0000;
            buf = buf.add(1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sleb128_encode(mut val: isize, mut buf: *mut u8) {
    loop {
        let byte = (val & 0b0111_1111) as u8;
        val >>= 7;
        if (val == 0 && byte & 0b0100_0000 == 0) || (val == -1 && byte & 0b0100_0000 == 0b0100_0000)
        {
            *buf = byte;
            break;
        } else {
            *buf = byte | 0b1000_0000;
            buf = buf.add(1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn leb128_decode(buf: *mut Buf) -> usize {
    leb128_decode_checked(buf).expect("leb128_decode: overflow")
}

/// Returns `None` on overflow
pub unsafe fn leb128_decode_checked(buf: *mut Buf) -> Option<usize> {
    let mut result = 0;
    let mut shift = 0;

    loop {
        let byte = read_byte(buf);

        result |= ((byte & 0b0111_1111) as usize) << shift;

        let overflow = match usize::BITS {
            u64::BITS => {
                // The 10th byte needs to be the last, and it must contribute at most 1 bit, otherwise we
                // have an overflow.
                shift == 63 && (byte & 0b1111_1110) != 0
            }
            u32::BITS => {
                // The 5th byte needs to be the last, and it must contribute at most 4 bits, otherwise we
                // have an overflow.
                shift == 28 && (byte & 0b1111_0000) != 0
            }
            _ => unreachable!(),
        };

        // The 10th byte needs to be the last, and it must contribute at most 1 bit, otherwise we
        // have an overflow
        if overflow {
            return None;
        }

        shift += 7;

        if byte & 0b1000_0000 == 0 {
            break;
        }
    }

    Some(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sleb128_decode(buf: *mut Buf) -> isize {
    sleb128_decode_checked(buf).expect("sleb128_decode: overflow")
}

/// Returns `None` on overflow
pub unsafe fn sleb128_decode_checked(buf: *mut Buf) -> Option<isize> {
    let mut result = 0;
    let mut shift = 0;

    let last_byte = loop {
        let byte = read_byte(buf);

        result |= ((byte & 0b0111_1111) as isize) << shift;

        // Overflow check ported from Wasm reference implementation:
        // https://github.com/WebAssembly/spec/blob/f9770eb75117cac0c878feaa5eaf4a4d9dda61f5/interpreter/binary/decode.ml#L89-L98
        let overflow = match usize::BITS {
            u64::BITS => {
                shift == 63 && (byte & 0b0111_1111 != 0 && byte & 0b0111_1111 != 0b0111_1111)
            }
            u32::BITS => {
                shift == 28 && (byte & 0b0111_1000 != 0 && byte & 0b0111_1000 != 0b0111_1000)
            }
            _ => unreachable!(),
        };

        if overflow {
            return None;
        }

        shift += 7;

        if byte & 0b1000_0000 == 0 {
            break byte;
        }
    };

    // Sign extend
    if shift < usize::BITS && last_byte & 0b0100_0000 != 0 {
        result |= !0 << shift;
    }

    Some(result)
}
