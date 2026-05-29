// Declarations adopted from https://github.com/rust-lang/libc/blob/main/src/wasi.rs.
#![allow(non_camel_case_types)]

pub(crate) type c_void = core::ffi::c_void;
pub(crate) type size_t = usize;
pub(crate) type c_char = i8;
pub(crate) type c_int = i32;

unsafe extern "C" {
    pub(crate) fn memcpy(dest: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    pub(crate) fn memset(dest: *mut c_void, c: c_int, n: size_t) -> *mut c_void;
    pub(crate) fn memcmp(cx: *const c_void, ct: *const c_void, n: size_t) -> c_int;
}
