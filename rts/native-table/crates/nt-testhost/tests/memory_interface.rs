//! The memory interface's checked reads and writes, the byte
//! helpers, faults, constants and the wake state, each run once per fill pattern.

use native_table::bytes::{put_be_u64, put_uleb, rd_be_u16, rd_be_u64, rd_uleb, uleb_len};
use native_table::consts::{self, form_code, seal_scratch, v_choice, BLOCK, PAGE};
use native_table::fault::{bounds, corrupt};
use native_table::host::{wake_from_state, STATE_PENDING, STATE_URGENT};
use native_table::mem::{copy, fill, put, put_u16, put_u32, put_u64, rd_u16, rd_u32, rd_u64, rd_u8, shift, sub};
use native_table::{CorruptAt, Corruption, Fault, ImageId, Owner, PageHost, Region, SchemaError, Scratch, Wake};
use nt_testhost::{Call, VecHost, VecScratch, XorShift, FILLS};

const W: Owner = Owner(0);

#[test]
fn reads_and_writes_in_bounds() {
    for f in FILLS {
        let mut h = VecHost::new(f);
        let p = h.page_alloc(W).unwrap();
        let b = h.block_of(p, 3);
        let s = h.get_mut(Region::Block(b), W).unwrap();
        put_u64(s, 4088, 0x0102_0304_0506_0708).unwrap();
        assert_eq!(rd_u64(s, 4088).unwrap(), 0x0102_0304_0506_0708);
        assert_eq!(rd_u32(s, 4088).unwrap(), 0x0506_0708);
        assert_eq!(rd_u16(s, 4094).unwrap(), 0x0102);
        assert_eq!(rd_u8(s, 4095).unwrap(), 0x01);
        assert!(matches!(rd_u64(s, 4089), Err(Fault::Corrupt(CorruptAt { kind: Corruption::Bounds, .. }))));
        assert_eq!(rd_u8(s, 4096), Err(bounds(4096)));
        assert!(matches!(sub(s, u64::MAX, 2), Err(Fault::Corrupt(_))));
        assert!(put(s, 4095, &[1, 2]).is_err());
        put_u16(s, 0, 0xBEEF).unwrap();
        put_u32(s, 2, 0xDEAD_BEEF).unwrap();
        assert_eq!(rd_u16(s, 0).unwrap(), 0xBEEF);
        assert_eq!(rd_u32(s, 2).unwrap(), 0xDEAD_BEEF);
        fill(s, 8, 8, 0).unwrap();
        assert_eq!(rd_u64(s, 8).unwrap(), 0);
        assert!(fill(s, 4090, 8, 0).is_err());
        // A fresh page carries the fill pattern where nothing was written.
        assert_eq!(rd_u8(s, 100).unwrap(), f);
    }
}

#[test]
fn shift_moves_overlapping_ranges() {
    for f in FILLS {
        let mut h = VecHost::new(f);
        let p = h.page_alloc(W).unwrap();
        let s = h.get_mut(Region::Block(p), W).unwrap();
        for (i, x) in s.iter_mut().enumerate().take(16) {
            *x = i as u8;
        }
        shift(s, 0, 4, 8).unwrap();
        assert_eq!(&s[4..12], &[0, 1, 2, 3, 4, 5, 6, 7]);
        shift(s, 4, 0, 8).unwrap();
        assert_eq!(&s[0..8], &[0, 1, 2, 3, 4, 5, 6, 7]);
        assert!(shift(s, 4090, 0, 8).is_err());
        assert!(shift(s, 0, 4090, 8).is_err());
        assert!(shift(s, u64::MAX, 0, 2).is_err());
    }
}

#[test]
fn copy_between_regions_and_pairs() {
    for f in FILLS {
        let mut h = VecHost::new(f);
        let p = h.page_alloc(W).unwrap();
        let q = h.page_alloc(W).unwrap();
        {
            let s = h.get_mut(Region::Page(p), W).unwrap();
            for (i, x) in s.iter_mut().enumerate() {
                *x = (i % 251) as u8;
            }
        }
        let qb = h.block_of(q, 2);
        copy(&mut h, Region::Page(p), 100, Region::Block(qb), 0, BLOCK, W).unwrap();
        let d = h.get(Region::Block(qb)).unwrap();
        assert_eq!(d[0], (100 % 251) as u8);
        assert_eq!(d[4095], ((100 + 4095) % 251) as u8);
        // An overlapping pair is refused; a disjoint pair is accepted in both orders.
        assert_eq!(h.pair_mut(Region::Page(p), Region::Block(p + BLOCK), W).err(), Some(Fault::BadRef));
        assert!(h.pair_mut(Region::Block(q), Region::Block(p), W).is_ok());
        assert!(h.pair_mut(Region::Block(p), Region::Block(q), W).is_ok());
        assert!(copy(&mut h, Region::Page(p), 1, Region::Page(q), 0, PAGE, W).is_err());
        assert!(copy(&mut h, Region::Page(p), 0, Region::Page(q), 0, PAGE + 1, W).is_err());
        assert_eq!(h.read_write(Region::Block(p), Region::Page(p), W).err(), Some(Fault::BadRef));
    }
}

#[test]
fn freed_pages_come_back_filled() {
    for f in FILLS {
        let mut h = VecHost::new(f);
        let p = h.page_alloc(W).unwrap();
        h.get_mut(Region::Page(p), W).unwrap().fill(0);
        h.page_free(p, W).unwrap();
        assert_eq!(h.get(Region::Page(p)).err(), Some(Fault::BadRef));
        assert_eq!(h.page_free(p, W).err(), Some(Fault::BadRef));
        let p2 = h.page_alloc(W).unwrap();
        assert_eq!(p2, p);
        assert!(h.get(Region::Page(p2)).unwrap().iter().all(|&x| x == f));
        assert_eq!(h.get(Region::Block(15)).err(), Some(Fault::BadRef));
        assert_eq!(h.get(Region::Block(0)).err(), Some(Fault::BadRef));
        assert_eq!(h.get(Region::Page(PAGE + 1)).err(), Some(Fault::BadRef));
    }
}

#[test]
fn host_logs_calls_and_counts_changed_blocks() {
    let mut h = VecHost::new(FILLS[0]);
    let p = h.page_alloc(W).unwrap();
    h.end_message();
    h.get_mut(Region::Block(p), W).unwrap();
    h.get_mut(Region::Block(p), W).unwrap();
    h.get_mut(Region::Block(p + BLOCK), W).unwrap();
    let _ = h.get(Region::Block(p + 2 * BLOCK)).unwrap();
    assert_eq!(h.end_message(), 2);
    let q = h.page_alloc(W).unwrap();
    copy(&mut h, Region::Block(p), 0, Region::Page(q), 0, 8, W).unwrap();
    assert_eq!(h.end_message(), 16);
    let log = h.take_log();
    assert_eq!(log[0], Call::PageAlloc(p));
    assert!(log.iter().any(|c| matches!(c, Call::ReadWrite(..))));
}

#[test]
fn scratch_grows_with_the_fill_pattern() {
    for f in FILLS {
        let mut s = VecScratch::new(f);
        assert_eq!(s.get(16).unwrap().len(), 16);
        assert!(s.get(32).unwrap().iter().all(|&x| x == f));
        assert_eq!(s.grows, 2);
        s.get(8).unwrap();
        assert_eq!(s.grows, 2);
    }
}

#[test]
fn big_endian_and_uleb() {
    let mut b = [0u8; 32];
    put_be_u64(&mut b, 3, 0x0102_0304_0506_0708).unwrap();
    assert_eq!(rd_be_u64(&b, 3).unwrap(), 0x0102_0304_0506_0708);
    assert_eq!(rd_be_u16(&b, 3).unwrap(), 0x0102);
    assert!(rd_be_u64(&b, 25).is_err());
    let mut r = XorShift(0x9E37_79B9_7F4A_7C15);
    let mut values = vec![0u64, 1, 127, 128, 16_383, 16_384, u64::MAX, u64::MAX - 1, 1 << 63];
    values.extend((0..2_000).map(|_| {
        let v = r.next_u64();
        v >> r.below(64)
    }));
    for v in values {
        let mut buf = [0xEEu8; 12];
        let n = put_uleb(&mut buf, 1, v).unwrap();
        assert_eq!(n, uleb_len(v), "{v}");
        assert_eq!(rd_uleb(&buf, 1, ImageId::Runtime).unwrap(), (v, n));
        // A form cut short runs past the slice.
        assert_eq!(rd_uleb(&buf[..n as usize], 1, ImageId::Runtime).err(), Some(bounds(1)).filter(|_| n > 0));
    }
    // Eleven continuation bytes, and a 10th byte above 1, are encoding faults.
    let long = [0x80u8; 11];
    assert_eq!(rd_uleb(&long, 0, ImageId::Group(7)).err(), Some(corrupt(ImageId::Group(7), Corruption::Encoding, 0)));
    let mut over = [0xFFu8; 10];
    over[9] = 0x02;
    assert_eq!(rd_uleb(&over, 0, ImageId::Runtime).err(), Some(corrupt(ImageId::Runtime, Corruption::Encoding, 0)));
    assert!(put_uleb(&mut [0u8; 9], 0, u64::MAX).is_err());
}

#[test]
fn fault_texts() {
    let all = [
        Fault::BadRef,
        Fault::NotOwner,
        Fault::OutOfMemory,
        bounds(3),
        corrupt(ImageId::Group(1), Corruption::Utf8, 9),
        Fault::PathOverflow,
        Fault::BadValue { leaf: 3 },
        Fault::Schema(SchemaError::KeyChanged),
        Fault::Scratch { need: 10 },
        Fault::Busy,
        Fault::BadToken,
    ];
    for f in all {
        let t = f.text();
        assert!(!t.is_empty() && t.len() <= 60 && t.is_ascii(), "{t}");
    }
    assert_eq!(bounds(3).text(), "native table: corrupt structure");
    assert_eq!(corrupt(ImageId::Group(1), Corruption::Utf8, 9).text(), "table image corrupt");
}

#[test]
fn constants_and_form_codes() {
    assert_eq!(form_code(8, 0), Some(0x8100_0010));
    assert_eq!(form_code(16, 8), Some(0x8100_8018));
    assert_eq!(form_code(1_016, 0), Some(0x8100_0400));
    assert_eq!(form_code(1_016, 8), None);
    assert_eq!(form_code(12, 0), None);
    assert_eq!(form_code(0, 0), None);
    assert_eq!(form_code(8, 24), None);
    assert_eq!(form_code(8, 32), Some(0x8102_0010));
    for v in consts::V_CHOICES {
        assert!(v_choice(v));
    }
    assert!(!v_choice(4) && !v_choice(512));
    assert_eq!(seal_scratch(6, 0, 12), Some(25_344));
    assert_eq!(seal_scratch(u64::MAX, 0, 0), None);
    let magics = [
        (consts::magic::NTIM, *b"NTIM"),
        (consts::magic::NTTR, *b"NTTR"),
        (consts::magic::NTGP, *b"NTGP"),
        (consts::magic::NTRL, *b"NTRL"),
    ];
    for (m, s) in magics {
        assert_eq!(m.to_le_bytes(), s);
    }
}

#[test]
fn wake_from_the_state_byte() {
    assert_eq!(wake_from_state(0), Wake::Idle);
    assert_eq!(wake_from_state(STATE_PENDING), Wake::Later);
    assert_eq!(wake_from_state(STATE_URGENT), Wake::Now);
    assert_eq!(wake_from_state(STATE_PENDING | STATE_URGENT), Wake::Now);
    assert_eq!(wake_from_state(0xFC), Wake::Idle);
}
