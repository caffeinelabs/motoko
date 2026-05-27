#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn char_to_upper(c: u32) -> u32 {
    let mut upper_chars = core::char::from_u32_unchecked(c).to_uppercase();
    if upper_chars.len() == 1 {
        upper_chars.next().unwrap() as u32
    } else {
        c
    }
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn char_to_lower(c: u32) -> u32 {
    let mut lower_chars = core::char::from_u32_unchecked(c).to_lowercase();
    if lower_chars.len() == 1 {
        lower_chars.next().unwrap() as u32
    } else {
        c
    }
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn char_is_whitespace(c: u32) -> u32 {
    core::char::from_u32_unchecked(c).is_whitespace().into()
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn char_is_uppercase(c: u32) -> u32 {
    core::char::from_u32_unchecked(c).is_uppercase().into()
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn char_is_lowercase(c: u32) -> u32 {
    core::char::from_u32_unchecked(c).is_lowercase().into()
}

#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
unsafe extern "C" fn char_is_alphabetic(c: u32) -> u32 {
    core::char::from_u32_unchecked(c).is_alphabetic().into()
}
