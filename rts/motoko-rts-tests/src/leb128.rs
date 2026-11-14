use motoko_rts::buf::Buf;
use motoko_rts::leb128::{
    leb128_decode_checked, leb128_encode, sleb128_decode_checked, sleb128_encode,
};

use proptest::test_runner::{Config, TestCaseError, TestCaseResult, TestRunner};

pub unsafe fn test() {
    println!("Testing (s)leb128 encode-decode roundtrip ...");

    let mut proptest_runner = TestRunner::new(Config {
        cases: 10_000,
        failure_persistence: None,
        ..Default::default()
    });

    roundtrip_signed(1).unwrap();
    roundtrip_signed(0).unwrap();
    roundtrip_signed(-1).unwrap();
    roundtrip_signed(isize::MIN).unwrap();
    roundtrip_signed(isize::MAX).unwrap();

    proptest_runner
        .run(&proptest::num::isize::ANY, roundtrip_signed)
        .unwrap();

    roundtrip_unsigned(1).unwrap();
    roundtrip_unsigned(0).unwrap();
    roundtrip_unsigned(usize::MIN).unwrap();
    roundtrip_unsigned(usize::MAX).unwrap();

    proptest_runner
        .run(&proptest::num::usize::ANY, roundtrip_unsigned)
        .unwrap();

    check_overflows();
}

unsafe fn check_overflows() {
    check_signed_decode_overflow(&[
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b0001_0001,
    ]); // isize::MIN - 1

    check_signed_decode_overflow(&[
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b0000_0001,
    ]); // isize::MAX + 1

    // An 11th byte is never allowed, even when all extra bits are zero
    // (0) or all equal the sign bit (-1)
    check_signed_decode_overflow(
        &[0b1000_0000; 10]
            .iter()
            .chain(&[0])
            .copied()
            .collect::<Vec<_>>(),
    );
    check_signed_decode_overflow(
        &[0b1111_1111; 10]
            .iter()
            .chain(&[0b0111_1111])
            .copied()
            .collect::<Vec<_>>(),
    );
    check_signed_decode_overflow(
        &[0b1000_0000; 10]
            .iter()
            .chain(&[0b0000_0001])
            .copied()
            .collect::<Vec<_>>(),
    );

    check_unsigned_decode_overflow(
        &[0b1000_0000; 10]
            .iter()
            .chain(&[0])
            .copied()
            .collect::<Vec<_>>(),
    );

    // Non-minimal (padded) encodings of up to ten bytes stay valid
    check_signed_decode(&[0b1000_0000, 0], 0);
    check_signed_decode(
        &[0b1000_0000; 9]
            .iter()
            .chain(&[0])
            .copied()
            .collect::<Vec<_>>(),
        0,
    );
    check_signed_decode(&[0b1111_1111, 0b0111_1111], -1);
    check_signed_decode(
        &[0b1111_1111; 9]
            .iter()
            .chain(&[0b0111_1111])
            .copied()
            .collect::<Vec<_>>(),
        -1,
    );
    check_signed_decode(&[0b1011_1111, 0b1000_0000, 0], 63);
    check_signed_decode(
        &[0b1000_0000; 9]
            .iter()
            .chain(&[0b0111_1111])
            .copied()
            .collect::<Vec<_>>(),
        isize::MIN,
    );
    check_signed_decode(
        &[0b1111_1111; 9]
            .iter()
            .chain(&[0])
            .copied()
            .collect::<Vec<_>>(),
        isize::MAX,
    );

    check_unsigned_decode_overflow(&[
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b0000_0010,
    ]); // usize::MAX + 1
}

fn roundtrip_signed(val: isize) -> TestCaseResult {
    unsafe {
        let mut buf = [0u8; 100];
        sleb128_encode(val, buf.as_mut_ptr());

        let mut buf_ = Buf {
            ptr: buf.as_mut_ptr(),
            end: buf.as_mut_ptr().add(100),
        };

        match sleb128_decode_checked(&mut buf_) {
            None => Err(TestCaseError::Fail(
                format!("sleb128 decoding of {} overflowed", val).into(),
            )),
            Some(val_) => {
                if val_ == val {
                    Ok(())
                } else {
                    Err(TestCaseError::Fail(
                        format!("Encode-decode roundtrip gives different value for {}", val).into(),
                    ))
                }
            }
        }
    }
}

fn roundtrip_unsigned(val: usize) -> TestCaseResult {
    unsafe {
        let mut buf = [0u8; 100];
        leb128_encode(val, buf.as_mut_ptr());

        let mut buf_ = Buf {
            ptr: buf.as_mut_ptr(),
            end: buf.as_mut_ptr().add(100),
        };

        match leb128_decode_checked(&mut buf_) {
            None => Err(TestCaseError::Fail(
                format!("leb128 decoding of {} overflowed", val).into(),
            )),
            Some(val_) => {
                if val_ == val {
                    Ok(())
                } else {
                    Err(TestCaseError::Fail(
                        format!("Encode-decode roundtrip gives different value for {}", val).into(),
                    ))
                }
            }
        }
    }
}

unsafe fn check_signed_decode_overflow(buf: &[u8]) {
    let mut buf_ = Buf {
        ptr: buf.as_ptr() as *mut _,
        end: buf.as_ptr().add(buf.len()) as *mut _,
    };

    assert_eq!(sleb128_decode_checked(&mut buf_), None);
}

unsafe fn check_signed_decode(buf: &[u8], expected: isize) {
    let mut buf_ = Buf {
        ptr: buf.as_ptr() as *mut _,
        end: buf.as_ptr().add(buf.len()) as *mut _,
    };

    assert_eq!(sleb128_decode_checked(&mut buf_), Some(expected));
    // `Buf` is packed: copy the fields out instead of borrowing them
    let (ptr, end) = (buf_.ptr, buf_.end);
    assert_eq!(ptr, end, "decoder must consume all {} bytes", buf.len());
}

unsafe fn check_unsigned_decode_overflow(buf: &[u8]) {
    let mut buf_ = Buf {
        ptr: buf.as_ptr() as *mut _,
        end: buf.as_ptr().add(buf.len()) as *mut _,
    };

    assert_eq!(leb128_decode_checked(&mut buf_), None);
}
