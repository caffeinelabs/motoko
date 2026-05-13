//! Principal ID encoding and decoding, with integrity checking

use crate::barriers::allocation_barrier;
use crate::mem_utils::memcpy_bytes;
use crate::memory::{Memory, alloc_blob};
use crate::rts_trap_with;
use crate::text::{blob_compare, blob_of_text};
use crate::types::{Bytes, TAG_BLOB_B, TAG_BLOB_T, Value};

use motoko_rts_macros::ic_mem_fn;

// CRC32 for blobs. Loosely based on https://rosettacode.org/wiki/CRC-32#Implementation_2

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_crc32(blob: Value) -> u32 {
    if !blob.is_blob() {
        panic!("compute_crc32: Blob expected");
    }

    let blob = blob.as_blob();
    let len = blob.len();

    let mut crc: u32 = !0;

    for i in 0..len.as_usize() {
        let octet = blob.get(i);
        crc = (crc >> 8) ^ CRC_TABLE[usize::from((crc & 0xFF) as u8 ^ octet)];
    }

    !crc
}

static CRC_TABLE: [u32; 256] = [
    0x0, 0x77073096, 0xee0e612c, 0x990951ba, 0x76dc419, 0x706af48f, 0xe963a535, 0x9e6495a3,
    0xedb8832, 0x79dcb8a4, 0xe0d5e91e, 0x97d2d988, 0x9b64c2b, 0x7eb17cbd, 0xe7b82d07, 0x90bf1d91,
    0x1db71064, 0x6ab020f2, 0xf3b97148, 0x84be41de, 0x1adad47d, 0x6ddde4eb, 0xf4d4b551, 0x83d385c7,
    0x136c9856, 0x646ba8c0, 0xfd62f97a, 0x8a65c9ec, 0x14015c4f, 0x63066cd9, 0xfa0f3d63, 0x8d080df5,
    0x3b6e20c8, 0x4c69105e, 0xd56041e4, 0xa2677172, 0x3c03e4d1, 0x4b04d447, 0xd20d85fd, 0xa50ab56b,
    0x35b5a8fa, 0x42b2986c, 0xdbbbc9d6, 0xacbcf940, 0x32d86ce3, 0x45df5c75, 0xdcd60dcf, 0xabd13d59,
    0x26d930ac, 0x51de003a, 0xc8d75180, 0xbfd06116, 0x21b4f4b5, 0x56b3c423, 0xcfba9599, 0xb8bda50f,
    0x2802b89e, 0x5f058808, 0xc60cd9b2, 0xb10be924, 0x2f6f7c87, 0x58684c11, 0xc1611dab, 0xb6662d3d,
    0x76dc4190, 0x1db7106, 0x98d220bc, 0xefd5102a, 0x71b18589, 0x6b6b51f, 0x9fbfe4a5, 0xe8b8d433,
    0x7807c9a2, 0xf00f934, 0x9609a88e, 0xe10e9818, 0x7f6a0dbb, 0x86d3d2d, 0x91646c97, 0xe6635c01,
    0x6b6b51f4, 0x1c6c6162, 0x856530d8, 0xf262004e, 0x6c0695ed, 0x1b01a57b, 0x8208f4c1, 0xf50fc457,
    0x65b0d9c6, 0x12b7e950, 0x8bbeb8ea, 0xfcb9887c, 0x62dd1ddf, 0x15da2d49, 0x8cd37cf3, 0xfbd44c65,
    0x4db26158, 0x3ab551ce, 0xa3bc0074, 0xd4bb30e2, 0x4adfa541, 0x3dd895d7, 0xa4d1c46d, 0xd3d6f4fb,
    0x4369e96a, 0x346ed9fc, 0xad678846, 0xda60b8d0, 0x44042d73, 0x33031de5, 0xaa0a4c5f, 0xdd0d7cc9,
    0x5005713c, 0x270241aa, 0xbe0b1010, 0xc90c2086, 0x5768b525, 0x206f85b3, 0xb966d409, 0xce61e49f,
    0x5edef90e, 0x29d9c998, 0xb0d09822, 0xc7d7a8b4, 0x59b33d17, 0x2eb40d81, 0xb7bd5c3b, 0xc0ba6cad,
    0xedb88320, 0x9abfb3b6, 0x3b6e20c, 0x74b1d29a, 0xead54739, 0x9dd277af, 0x4db2615, 0x73dc1683,
    0xe3630b12, 0x94643b84, 0xd6d6a3e, 0x7a6a5aa8, 0xe40ecf0b, 0x9309ff9d, 0xa00ae27, 0x7d079eb1,
    0xf00f9344, 0x8708a3d2, 0x1e01f268, 0x6906c2fe, 0xf762575d, 0x806567cb, 0x196c3671, 0x6e6b06e7,
    0xfed41b76, 0x89d32be0, 0x10da7a5a, 0x67dd4acc, 0xf9b9df6f, 0x8ebeeff9, 0x17b7be43, 0x60b08ed5,
    0xd6d6a3e8, 0xa1d1937e, 0x38d8c2c4, 0x4fdff252, 0xd1bb67f1, 0xa6bc5767, 0x3fb506dd, 0x48b2364b,
    0xd80d2bda, 0xaf0a1b4c, 0x36034af6, 0x41047a60, 0xdf60efc3, 0xa867df55, 0x316e8eef, 0x4669be79,
    0xcb61b38c, 0xbc66831a, 0x256fd2a0, 0x5268e236, 0xcc0c7795, 0xbb0b4703, 0x220216b9, 0x5505262f,
    0xc5ba3bbe, 0xb2bd0b28, 0x2bb45a92, 0x5cb36a04, 0xc2d7ffa7, 0xb5d0cf31, 0x2cd99e8b, 0x5bdeae1d,
    0x9b64c2b0, 0xec63f226, 0x756aa39c, 0x26d930a, 0x9c0906a9, 0xeb0e363f, 0x72076785, 0x5005713,
    0x95bf4a82, 0xe2b87a14, 0x7bb12bae, 0xcb61b38, 0x92d28e9b, 0xe5d5be0d, 0x7cdcefb7, 0xbdbdf21,
    0x86d3d2d4, 0xf1d4e242, 0x68ddb3f8, 0x1fda836e, 0x81be16cd, 0xf6b9265b, 0x6fb077e1, 0x18b74777,
    0x88085ae6, 0xff0f6a70, 0x66063bca, 0x11010b5c, 0x8f659eff, 0xf862ae69, 0x616bffd3, 0x166ccf45,
    0xa00ae278, 0xd70dd2ee, 0x4e048354, 0x3903b3c2, 0xa7672661, 0xd06016f7, 0x4969474d, 0x3e6e77db,
    0xaed16a4a, 0xd9d65adc, 0x40df0b66, 0x37d83bf0, 0xa9bcae53, 0xdebb9ec5, 0x47b2cf7f, 0x30b5ffe9,
    0xbdbdf21c, 0xcabac28a, 0x53b39330, 0x24b4a3a6, 0xbad03605, 0xcdd70693, 0x54de5729, 0x23d967bf,
    0xb3667a2e, 0xc4614ab8, 0x5d681b02, 0x2a6f2b94, 0xb40bbe37, 0xc30c8ea1, 0x5a05df1b, 0x2d02ef8d,
];

struct Pump {
    inp_gran: u32,
    out_gran: u32,
    dest: *mut u8,
    pending_data: u32,
    pending_bits: u32,
}

static BASE32_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

unsafe fn stash_enc_base32(d: u8, dest: *mut u8) {
    *dest = BASE32_CHARS[(d & 0b0001_1111) as usize];
}

unsafe fn enc_stash(pump: &mut Pump, data: u8) {
    pump.pending_data <<= pump.inp_gran;
    pump.pending_data |= data as u32;
    pump.pending_bits += pump.inp_gran;

    while pump.pending_bits >= pump.out_gran {
        pump.pending_bits -= pump.out_gran;
        stash_enc_base32((pump.pending_data >> pump.pending_bits) as u8, pump.dest);
        pump.dest = pump.dest.add(1);
        pump.pending_data &= (1 << pump.pending_bits) - 1;
    }
}

/// Encode a blob into an checksum-prepended base32 representation
pub unsafe fn base32_of_checksummed_blob<M: Memory>(mem: &mut M, b: Value) -> Value {
    let checksum = compute_crc32(b);
    let n = b.as_blob().len();
    let mut data = b.as_blob().payload_const();

    let r = alloc_blob(mem, TAG_BLOB_T, Bytes((n.as_usize() + 4 + 4) / 5 * 8)); // contains padding
    let blob = r.as_blob_mut();
    let dest = blob.payload_addr();

    let mut pump = Pump {
        inp_gran: 8,
        out_gran: 5,
        dest,
        pending_data: 0,
        pending_bits: 0,
    };
    enc_stash(&mut pump, (checksum >> 24) as u8); // checksum is serialized as big-endian
    enc_stash(&mut pump, (checksum >> 16) as u8);
    enc_stash(&mut pump, (checksum >> 8) as u8);
    enc_stash(&mut pump, checksum as u8);

    for _ in 0..n.as_usize() {
        enc_stash(&mut pump, *data);
        data = data.add(1);
    }

    if pump.pending_bits != 0 {
        // Flush odd bits
        pump.pending_data <<= pump.out_gran - pump.pending_bits;
        stash_enc_base32(pump.pending_data as u8, pump.dest);
        pump.dest = pump.dest.add(1);
        // Discount padding
        let new_len = Bytes(pump.dest.offset_from(dest) as usize);
        blob.shrink(new_len);
    }

    allocation_barrier(r)
}

// tolerant conversion
// accepts lower case and fillers/padding '-', '='
// values are one more than base32 value
// 0 elements signify invalid alphabet letter
// fillers/padding have upper bit set
// Returns in range [1,32]
fn conv(b: u8) -> u8 {
    if b == b'-' {
        0xF0
    } else if b == b'=' {
        0xF1
    } else if b >= b'A' && b <= b'Z' {
        b - b'A' + 1
    } else if b >= b'a' && b <= b'z' {
        b - b'a' + 1
    } else if b >= b'2' && b <= b'7' {
        b - b'2' + 27
    } else {
        0
    }
}

unsafe fn accum_base32(pump: &mut Pump, c: u8) {
    if c > b'z' {
        rts_trap_with("accum_base32: Base32 symbol out of range");
    }

    let v = conv(c & 0b0111_1111) - 1; // 0..31

    if v < 32 {
        // excludes '-' and '='
        pump.pending_bits += pump.inp_gran;
        pump.pending_data <<= pump.inp_gran;
        pump.pending_data |= v as u32;
    }
}

unsafe fn dec_stash(pump: &mut Pump, data: u8) {
    accum_base32(pump, data);

    while pump.pending_bits >= pump.out_gran {
        pump.pending_bits -= pump.out_gran;
        *pump.dest = (pump.pending_data >> pump.pending_bits) as u8;
        pump.dest = pump.dest.add(1);
        pump.pending_data &= (1 << pump.pending_bits) - 1;
    }
}

pub unsafe fn base32_to_blob<M: Memory>(mem: &mut M, b: Value) -> Value {
    let n = b.as_blob().len();
    let mut data = b.as_blob().payload_const();

    // Every group of 8 characters will yield 5 bytes
    let r = alloc_blob(mem, TAG_BLOB_B, Bytes(((n.as_usize() + 7) / 8) * 5)); // we deal with padding later
    let blob = r.as_blob_mut();
    let dest = blob.payload_addr();

    let mut pump = Pump {
        inp_gran: 5,
        out_gran: 8,
        dest,
        pending_bits: 0,
        pending_data: 0,
    };

    for _ in 0..n.as_usize() {
        dec_stash(&mut pump, *data);
        data = data.add(1);
    }

    // Adjust resulting blob len
    let new_len = Bytes(pump.dest.offset_from(dest) as usize);
    blob.shrink(new_len);

    allocation_barrier(r)
}

/// Encode a blob into its textual representation
#[ic_mem_fn]
pub unsafe fn principal_of_blob<M: Memory>(mem: &mut M, b: Value) -> Value {
    let base32 = base32_of_checksummed_blob(mem, b);
    base32_to_principal(mem, base32)
}

/// Convert a checksum-prepended base32 representation blob into the public principal name format
/// by hyphenating and lowercasing
unsafe fn base32_to_principal<M: Memory>(mem: &mut M, b: Value) -> Value {
    let blob = b.as_blob();

    let n = blob.len();
    let mut data = blob.payload_const();

    // Every group of 5 characters will yield 6 bytes (due to the hypen)
    let r = alloc_blob(mem, TAG_BLOB_T, Bytes(((n.as_usize() + 4) / 5) * 6));
    let blob = r.as_blob_mut();
    let mut dest = blob.payload_addr();

    let mut n_written = 0;
    for i in 0..n.as_usize() {
        let mut byte = *data;
        data = data.add(1);

        // If uppercase, convert to lowercase
        if byte >= b'A' && byte <= b'Z' {
            byte += b'a' - b'A'
        }

        *dest = byte;
        dest = dest.add(1);
        n_written += 1;

        // If quintet done, add hyphen
        if n_written % 5 == 0 && i + 1 < n.as_usize() {
            n_written = 0;
            *dest = b'-';
            dest = dest.add(1);
        }
    }

    // Adjust result length
    let new_len = Bytes(dest as usize - blob.payload_addr() as usize);
    blob.shrink(new_len);
    allocation_barrier(r)
}

// Decode an textual principal representation into a blob
#[ic_mem_fn]
pub unsafe fn blob_of_principal<M: Memory>(mem: &mut M, t: Value) -> Value {
    let b0 = blob_of_text(mem, t);
    let bytes = base32_to_blob(mem, b0);

    // Strip first four bytes
    let bytes_len = bytes.as_blob().len();
    if bytes_len < Bytes(4) {
        rts_trap_with("blob_of_principal: principal too short");
    }

    let stripped = alloc_blob(mem, TAG_BLOB_B, bytes_len - Bytes(4));
    memcpy_bytes(
        stripped.as_blob_mut().payload_addr() as usize,
        bytes.as_blob().payload_const().add(4) as usize,
        bytes_len - Bytes(4),
    );

    // Check encoding
    let expected = principal_of_blob(mem, stripped);
    if blob_compare(b0, expected) != 0 {
        rts_trap_with("blob_of_principal: invalid principal");
    }

    allocation_barrier(stripped)
}

// for testing
pub unsafe fn blob_of_ptr_size<M: Memory>(mem: &mut M, buf: *const u8, n: Bytes<usize>) -> Value {
    let blob = alloc_blob(mem, TAG_BLOB_B, n);
    let payload_addr = blob.as_blob_mut().payload_addr();
    memcpy_bytes(payload_addr as usize, buf as usize, n);
    allocation_barrier(blob)
}

pub unsafe fn blob_of_str<M: Memory>(mem: &mut M, s: &str) -> Value {
    blob_of_ptr_size(mem, s.as_ptr(), Bytes(s.len()))
}
