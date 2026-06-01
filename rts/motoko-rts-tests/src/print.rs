//! Support for standard output in wasmtime.

use std::ptr::addr_of_mut;

#[link(wasm_import_module = "wasi_snapshot_preview1")]
unsafe extern "C" {
    fn fd_write(fd: i32, iovs: i32, iovs_len: i32, nwritten: i32) -> i32;
}

#[repr(C)]
struct iov {
    base: i32,
    length: i32,
}

const BUFFER_LENGTH: usize = 1024;

static mut WRITTEN: i32 = 0;
static mut TEXT_BUFFER: [u8; BUFFER_LENGTH] = [0; BUFFER_LENGTH];
static mut IO_VECTOR: iov = iov { base: 0, length: 0 };

/// Truncates the text if it is longer than `BUFFER_LENGTH`.
pub(crate) fn wasmtime_println(text: &str) {
    unsafe {
        let mut length = 0;
        for byte in text.as_bytes() {
            TEXT_BUFFER[length] = *byte;
            length += 1;
            if length + 1 >= TEXT_BUFFER.len() {
                break;
            }
        }
        TEXT_BUFFER[length] = '\n' as u8; // new-line is required for a working output
        length += 1;

        IO_VECTOR.base = &mut TEXT_BUFFER[0] as *mut u8 as i32;
        IO_VECTOR.length = length as i32;

        let io_vector_array = addr_of_mut!(IO_VECTOR) as *mut iov;
        const STANDARDD_OUTPUT: i32 = 1;
        const IO_VECTOR_ARRAY_LENGTH: i32 = 1;
        let written_pointer = addr_of_mut!(WRITTEN) as *mut i32;

        fd_write(
            STANDARDD_OUTPUT,
            io_vector_array as i32,
            IO_VECTOR_ARRAY_LENGTH,
            written_pointer as i32,
        );
    }
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ({
        {
            use core::fmt::Write;
            let mut output = String::new();
            write!(&mut output, $($arg)*).unwrap();
            crate::print::wasmtime_println(&output);
        }
    })
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        println!($($arg)*);
    })
}

#[macro_export]
macro_rules! panic {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let mut output = String::from("[PANIC] ");
        write!(&mut output, $($arg)*).unwrap();
        crate::print::wasmtime_println(&output);
        std::process::exit(1);
    })
}
