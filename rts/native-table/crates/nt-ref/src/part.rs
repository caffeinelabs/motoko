//! Keys and values as `PBTreeMap` stores them: the types' sizes and alignments, the key order through a
//! lookup form, and `Part<T>`, inline up to `INLINE_LIMIT` bytes and boxed above, as today's map has them;
//! and `PString`, today's string as a key: a 16-byte header whose comparisons read the string's buffer.

use core::cmp::Ordering;
use core::marker::PhantomData;
use core::ops::Deref;

use crate::mem::{at, box_alloc, box_hand_back};

/// Parts up to this size are stored inline (`ic-eop-schema`'s `INLINE_LIMIT`).
pub const INLINE_LIMIT: u32 = 1012;

/// A type a map stores: its Canonical ABI size and alignment, which equal Rust's.
pub trait Persistent: Sized {
    const SIZE: u32;
    const ALIGN: u32;
}

/// A key type, ordered through a borrowed lookup form.
///
/// # Safety
///
/// `cmp_lookup` is a total order and stored keys compare through `as_lookup`.
pub unsafe trait PKey: Persistent {
    type Lookup<'a>: Copy
    where
        Self: 'a;
    fn as_lookup(&self) -> Self::Lookup<'_>;
    fn cmp_lookup<'a, 'b>(a: Self::Lookup<'a>, b: Self::Lookup<'b>) -> Ordering
    where
        Self: 'a + 'b;
    /// An owned key from its lookup form (the entry API's insert); a string allocates its buffer.
    fn from_lookup(k: Self::Lookup<'_>) -> Self;
}

impl Persistent for u64 {
    const SIZE: u32 = 8;
    const ALIGN: u32 = 8;
}

// SAFETY: the lookup form is the value itself, ordered by its `Ord`.
unsafe impl PKey for u64 {
    type Lookup<'a> = u64;

    #[inline(always)]
    fn as_lookup(&self) -> u64 {
        *self
    }

    #[inline(always)]
    fn cmp_lookup<'a, 'b>(a: u64, b: u64) -> Ordering
    where
        Self: 'a + 'b,
    {
        a.cmp(&b)
    }

    #[inline(always)]
    fn from_lookup(k: u64) -> Self {
        k
    }
}

impl<const N: usize> Persistent for [u8; N] {
    const SIZE: u32 = N as u32;
    const ALIGN: u32 = 1;
}

// SAFETY: the array itself, ordered bytewise as `[u8; N]`'s `Ord` does.
unsafe impl<const N: usize> PKey for [u8; N] {
    type Lookup<'a> = &'a [u8; N];

    #[inline(always)]
    fn as_lookup(&self) -> &[u8; N] {
        self
    }

    #[inline(always)]
    fn cmp_lookup<'a, 'b>(a: &'a [u8; N], b: &'b [u8; N]) -> Ordering
    where
        Self: 'a + 'b,
    {
        a.cmp(b)
    }

    #[inline(always)]
    fn from_lookup(k: &[u8; N]) -> Self {
        *k
    }
}

/// Today's persistent string: a header {buffer address, length} over UTF-8 bytes in a buffer of exactly
/// that length; address 0 for the empty string. Its buffer is the string's own: dropping the string hands it
/// back (today it goes to the freed list).
#[repr(C)]
pub struct PString {
    ptr: u64,
    len: u64,
}

impl Persistent for PString {
    const SIZE: u32 = 16;
    const ALIGN: u32 = 8;
}

impl PString {
    /// A string of the bytes of `s`, in a buffer of exactly their number from the selected arena.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        let n = s.len() as u64;
        if n == 0 {
            return PString { ptr: 0, len: 0 };
        }
        let ptr = box_alloc(n);
        // SAFETY: the new buffer holds `n` bytes; `s` lies in other memory.
        unsafe { core::ptr::copy_nonoverlapping(s.as_ptr(), at::<u8>(ptr), n as usize) };
        PString { ptr, len: n }
    }
}

impl Deref for PString {
    type Target = str;

    fn deref(&self) -> &str {
        if self.ptr == 0 {
            return "";
        }
        // SAFETY: the buffer holds `len` bytes of UTF-8, written by `from_str`.
        unsafe { core::str::from_utf8_unchecked(core::slice::from_raw_parts(at::<u8>(self.ptr), self.len as usize)) }
    }
}

impl Drop for PString {
    fn drop(&mut self) {
        if self.ptr != 0 {
            // SAFETY: the string's own buffer, which nothing else references.
            unsafe { box_hand_back(self.ptr) }
        }
    }
}

// SAFETY: the string's bytes, ordered bytewise as `str`'s `Ord` does.
unsafe impl PKey for PString {
    type Lookup<'a> = &'a str;

    #[inline(always)]
    fn as_lookup(&self) -> &str {
        self
    }

    #[inline(always)]
    fn cmp_lookup<'a, 'b>(a: &'a str, b: &'b str) -> Ordering
    where
        Self: 'a + 'b,
    {
        a.cmp(b)
    }

    fn from_lookup(k: &str) -> Self {
        PString::from_str(k)
    }
}

/// How a part of type `T` lies in a node: inline, or, above `INLINE_LIMIT`, as a box's payload address.
pub struct Part<T>(PhantomData<fn() -> T>);

impl<T: Persistent> Part<T> {
    /// Stored boxed.
    pub const BOXED: bool = T::SIZE > INLINE_LIMIT;
    /// Bytes the part takes in a node.
    pub const SIZE: u64 = {
        assert!(T::SIZE as usize == core::mem::size_of::<T>() && T::ALIGN as usize == core::mem::align_of::<T>());
        if Self::BOXED {
            8
        } else {
            T::SIZE as u64
        }
    };
    /// Alignment of the part in a node.
    pub const ALIGN: u64 = if Self::BOXED { 8 } else { T::ALIGN as u64 };

    /// The address of the value stored at `p`.
    ///
    /// # Safety
    ///
    /// `p` holds a stored part of `T`.
    #[inline(always)]
    pub unsafe fn value_ptr(p: *mut u8) -> *mut T {
        if Self::BOXED {
            // SAFETY: a boxed part is the box's payload address, aligned to 8.
            at(unsafe { p.cast::<u64>().read() })
        } else {
            p.cast()
        }
    }

    /// Stores `v` at `p`, which holds no stored part, boxing it when the part is boxed.
    ///
    /// # Safety
    ///
    /// `p` is writable for `SIZE` bytes at alignment `ALIGN`.
    pub unsafe fn put(p: *mut u8, v: T) {
        if Self::BOXED {
            let b = box_alloc(T::SIZE as u64);
            // SAFETY: a new box of `T`'s size at a payload address aligned to 8; then the part's 8 bytes.
            unsafe {
                at::<T>(b).write(v);
                p.cast::<u64>().write(b);
            }
        } else {
            // SAFETY: the caller's contract.
            unsafe { p.cast::<T>().write(v) }
        }
    }

    /// Moves the value out of `p`, which then holds no stored part; a box is handed back at once.
    ///
    /// # Safety
    ///
    /// `p` holds a stored part of `T`, which nothing references.
    pub unsafe fn take(p: *mut u8) -> T {
        if Self::BOXED {
            // SAFETY: the box's value, moved out once; then its slot, which nothing references.
            unsafe {
                let b = p.cast::<u64>().read();
                let v = at::<T>(b).read();
                box_hand_back(b);
                v
            }
        } else {
            // SAFETY: the caller's contract.
            unsafe { p.cast::<T>().read() }
        }
    }

    /// Replaces the value stored at `p` by `v` and returns the old value; a box is allocated before the
    /// old one is handed back.
    ///
    /// # Safety
    ///
    /// As for `put` and `take`.
    pub unsafe fn replace(p: *mut u8, v: T) -> T {
        if Self::BOXED {
            let mut new = [0u64; 1];
            // SAFETY: an 8-byte buffer receives the new box's address; then the old part moves out and
            // the new one takes its place.
            unsafe {
                Self::put(new.as_mut_ptr().cast(), v);
                let old = Self::take(p);
                p.cast::<u64>().write(new[0]);
                old
            }
        } else {
            // SAFETY: the caller's contract.
            unsafe { p.cast::<T>().replace(v) }
        }
    }

    /// Drops the part stored at `p`: a box is handed back (today it goes to the freed list), an inline
    /// value is dropped in place.
    ///
    /// # Safety
    ///
    /// `p` holds a stored part of `T`, which nothing references and which is not used again.
    pub unsafe fn drop_at(p: *mut u8) {
        if Self::BOXED {
            // SAFETY: a boxed part is the box's payload address.
            unsafe { box_hand_back(p.cast::<u64>().read()) }
        } else {
            // SAFETY: the caller's contract.
            unsafe { core::ptr::drop_in_place(p.cast::<T>()) }
        }
    }
}
