use crate::libc_declarations::c_char;

/// Panics if the string is not valid UTF-8
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn utf8_validate(str: *const c_char, len: usize) {
    if !utf8_valid(str, len) {
        crate::rts_trap_with("utf8_validate: string is not UTF-8");
    }
}

/// Returns whether the string is valid UTF-8
#[unsafe(no_mangle)]
pub unsafe extern "C" fn utf8_valid(str: *const c_char, len: usize) -> bool {
    core::str::from_utf8(core::slice::from_raw_parts(str as *const _, len)).is_ok()
}
