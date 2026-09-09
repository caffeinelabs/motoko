// Note [struct representation]
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~
//
// TLDR: Add `#[repr(C)]` in types used by both the runtime system and generated code, and add
// assertions to `static_assertions` module to check that the object size and field offsets are as
// expected.
//
// Rust compiler is free to reorder fields[1]. To avoid this in types that are used by both the
// runtime system and generated code we need a `repr` attribute like `repr(C)` or `repr(packed)`.
//
// We can't use `repr(packed)` because it potentially introduces undefined behavior as getting
// address (reference or pointer) of an unaligned field (or using the address) is an undefined
// behavior in Rust. See Motoko PR #2764.
//
// So to avoid reordering fields without introducing undefined behaviors we use `repr(C)`. See [2]
// for details on how `repr(C)` works. In short: it does not reorder fields. It can add padding,
// but in our case all fields are word-sized so that's not a problem.
//
// [1]: https://github.com/rust-lang/reference/blob/master/src/types/struct.md
// [2]: https://doc.rust-lang.org/stable/reference/type-layout.html#the-c-representation

use motoko_rts_macros::{enhanced_orthogonal_persistence, incremental_gc};

use crate::barriers::{init_with_barrier, write_with_barrier};
use crate::memory::Memory;
use crate::tommath_bindings::{mp_digit, mp_int};
use core::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};
use core::ptr::null;

use crate::constants::{MAX_ARRAY_LENGTH_FOR_ITERATOR, WORD_SIZE};
use crate::rts_trap_with;

pub fn size_of<T>() -> Words<usize> {
    Bytes(::core::mem::size_of::<T>()).to_words()
}

// TODO: Refactor by removing the generic type from `Words` and `Bytes`.

/// The unit "words": `Words(123)` means 123 words.
#[repr(transparent)]
#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub struct Words<A>(pub A);

impl Words<usize> {
    pub fn to_bytes(self) -> Bytes<usize> {
        Bytes(self.0 * WORD_SIZE)
    }

    pub fn as_usize(self) -> usize {
        self.0
    }
}

impl<A: Add<Output = A>> Add for Words<A> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Words(self.0 + rhs.0)
    }
}

impl<A: Sub<Output = A>> Sub for Words<A> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Words(self.0 - rhs.0)
    }
}

impl<A: Mul<Output = A>> Mul<A> for Words<A> {
    type Output = Self;

    fn mul(self, rhs: A) -> Self::Output {
        Words(self.0 * rhs)
    }
}

impl<A: Div<Output = A>> Div<A> for Words<A> {
    type Output = Self;

    fn div(self, rhs: A) -> Self::Output {
        Words(self.0 / rhs)
    }
}

impl<A: AddAssign> AddAssign for Words<A> {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl<A: SubAssign> SubAssign for Words<A> {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl From<Bytes<usize>> for Words<usize> {
    fn from(bytes: Bytes<usize>) -> Words<usize> {
        bytes.to_words()
    }
}

/// The unit "bytes": `Bytes(123)` means 123 bytes.
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub struct Bytes<A>(pub A);

impl Bytes<usize> {
    // Rounds up
    pub fn to_words(self) -> Words<usize> {
        // Rust issue for adding ceiling_div: https://github.com/rust-lang/rfcs/issues/2844
        Words((self.0 + WORD_SIZE - 1) / WORD_SIZE)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }

    /// Rust `next_multiple_of` is unstable, see https://github.com/rust-lang/rust/issues/88581.
    pub fn next_multiple_of(self, multiple: usize) -> Self {
        Bytes((self.0 + multiple - 1) / multiple * multiple)
    }
}

impl<A: Add<Output = A>> Add for Bytes<A> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Bytes(self.0 + rhs.0)
    }
}

impl<A: Sub<Output = A>> Sub for Bytes<A> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Bytes(self.0 - rhs.0)
    }
}

impl<A: AddAssign> AddAssign for Bytes<A> {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl<A: SubAssign> SubAssign for Bytes<A> {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl From<Words<usize>> for Bytes<usize> {
    fn from(words: Words<usize>) -> Bytes<usize> {
        words.to_bytes()
    }
}

// The `true` value. The only scalar value that has the lowest bit set.
pub const TRUE_VALUE: usize = 0x1;

/// Constant sentinel pointer value for fast null tests.
/// Points to the last unallocated Wasm page.
/// See also `compile.ml` for other reserved sentinel values.
#[enhanced_orthogonal_persistence]
pub const NULL_POINTER: Value = Value::from_raw(0xffff_ffff_ffff_fffb);

/// A value in a heap slot
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Value(usize);

/// A view of `Value` for analyzing the slot contents.
pub enum PtrOrScalar {
    /// Slot is a pointer to a boxed object
    Ptr(usize),

    /// Slot is an unboxed scalar value
    Scalar(usize),
}

impl PtrOrScalar {
    pub fn is_ptr(&self) -> bool {
        matches!(self, PtrOrScalar::Ptr(_))
    }

    pub fn is_scalar(&self) -> bool {
        matches!(self, PtrOrScalar::Scalar(_))
    }
}

impl Value {
    /// Create a value from a pointer
    pub const fn from_ptr(ptr: usize) -> Self {
        // Cannot use `debug_assert_eq` in const yet, so using `debug_assert`
        debug_assert!(ptr & 0b1 == 0b0);
        Value(skew(ptr))
    }

    /// Create a value from a scalar
    pub const fn from_scalar(value: usize) -> Self {
        // Cannot use `debug_assert_eq` in const yet, so using `debug_assert`
        debug_assert!(value >> (usize::BITS - 1) == 0);
        Value(value << 1)
    }

    /// Create a value from a signed scalar. The scalar must be obtained with `get_signed_scalar`.
    /// Using `get_scalar` will return an incorrect scalar.
    pub fn from_signed_scalar(value: isize) -> Self {
        debug_assert_eq!(value, value << 1 >> 1);
        Value((value << 1) as usize)
    }

    /// Create a value from raw representation. Useful when e.g. temporarily writing invalid values
    /// to object fields in garbage collection.
    pub const fn from_raw(raw: usize) -> Self {
        Value(raw)
    }

    /// Analyzes the value.
    ///
    /// Note: when using this function in performance critical code make sure to check the
    /// generated Wasm and see if it can be improved by using `Value::get_raw`, `unskew`, etc.
    /// rustc/LLVM generates slightly more inefficient code (compared to using functions like
    /// `Value::get_raw` and `unskew`) in our cost model where every Wasm instruction costs 1
    /// cycle.
    pub fn get(&self) -> PtrOrScalar {
        if is_ptr(self.0) {
            PtrOrScalar::Ptr(unskew(self.0))
        } else {
            PtrOrScalar::Scalar(self.0 >> 1)
        }
    }

    /// Get the raw value
    #[inline]
    pub const fn get_raw(&self) -> usize {
        self.0
    }

    /// Is the value a scalar?
    pub fn is_scalar(&self) -> bool {
        self.get().is_scalar()
    }

    /// Is the value a non-null pointer?
    #[enhanced_orthogonal_persistence]
    pub fn is_non_null_ptr(&self) -> bool {
        self.get().is_ptr() && *self != NULL_POINTER
    }

    /// Assumes that the value is a scalar and returns the scalar value. In debug mode panics if
    /// the value is not a scalar.
    pub fn get_scalar(&self) -> usize {
        debug_assert!(self.get().is_scalar());
        self.0 >> 1
    }

    /// Assumes that the value is a signed scalar and returns the scalar value. In debug mode
    /// panics if the value is not a scalar.
    pub fn get_signed_scalar(&self) -> isize {
        debug_assert!(self.get().is_scalar());
        self.0 as isize >> 1
    }

    /// Assumes that the value is a pointer and returns the pointer value. In debug mode panics if
    /// the value is not a pointer.
    pub fn get_ptr(self) -> usize {
        debug_assert!(self.get().is_ptr());
        unskew(self.0)
    }

    /// Check that the forwarding pointer is valid.
    #[inline]
    pub unsafe fn check_forwarding_pointer(self) {
        debug_assert!(
            self.forward().get_ptr() == self.get_ptr()
                || self.forward().forward().get_ptr() == self.forward().get_ptr()
        );
    }

    /// Check whether the object's forwarding pointer refers to a different location.
    pub unsafe fn is_forwarded(self) -> bool {
        self.check_forwarding_pointer();
        self.forward().get_ptr() != self.get_ptr()
    }

    /// Get the object tag. No forwarding. Can be applied to any block, regular objects
    /// with a header as well as `OneWordFiller`, `FwdPtr`, and `FreeSpace`.
    /// In debug mode panics if the value is not a pointer.
    pub unsafe fn tag(self) -> Tag {
        #[enhanced_orthogonal_persistence]
        debug_assert_ne!(self, NULL_POINTER);

        *(self.get_ptr() as *const Tag)
    }

    /// Get the forwarding pointer. Used by the incremental GC.
    #[incremental_gc]
    pub unsafe fn forward(self) -> Value {
        debug_assert!(self.is_obj());
        debug_assert!(self.get_ptr() as *const Obj != null());
        let obj = self.get_ptr() as *const Obj;
        (*obj).forward
    }

    /// Resolve forwarding if the value is a pointer. Otherwise, return the same value.
    #[enhanced_orthogonal_persistence]
    pub unsafe fn forward_if_possible(self) -> Value {
        // Second condition: Ignore raw null addresses used in `text_iter`.
        if self.is_non_null_ptr() && self.get_ptr() as *const Obj != null() {
            self.forward()
        } else {
            self
        }
    }

    /// Determines whether the value refers to an object with a regular header that contains a forwarding pointer.
    /// Returns `false` for pointers to special `OneWordFiller`, `FwdPtr`, and `FreeSpace` blocks that do not have
    /// a regular object header.
    pub unsafe fn is_obj(self) -> bool {
        let tag = self.tag();
        tag != TAG_FWD_PTR && tag != TAG_ONE_WORD_FILLER && tag != TAG_FREE_SPACE
    }

    pub unsafe fn is_blob(self) -> bool {
        let tag = self.tag();
        is_blob_tag(tag)
    }

    pub unsafe fn is_array(self) -> bool {
        let tag = self.tag();
        is_array_or_slice_tag(tag)
    }

    /// Get the pointer as `Obj` using forwarding. In debug mode panics if the value is not a pointer.
    pub unsafe fn as_obj(self) -> *mut Obj {
        debug_assert!(self.get().is_ptr());
        self.check_forwarding_pointer();
        self.forward().get_ptr() as *mut Obj
    }

    /// Get the pointer as `MutBox` using forwarding. In debug mode panics if the value is not a pointer.
    pub unsafe fn as_mutbox(self) -> *mut MutBox {
        debug_assert_eq!(self.tag(), TAG_MUTBOX);
        self.check_forwarding_pointer();
        self.forward().get_ptr() as *mut MutBox
    }

    /// Get the pointer as `Array` using forwarding. In debug mode panics if the value is not a pointer or the
    /// pointed object is not an `Array`.
    pub unsafe fn as_array(self) -> *mut Array {
        debug_assert!(self.is_array());
        self.check_forwarding_pointer();
        self.forward().get_ptr() as *mut Array
    }

    /// Get the pointer as `Object` using forwarding. In debug mode panics if the value is not a pointer.
    pub unsafe fn as_object(self) -> *mut Object {
        debug_assert!(self.tag() == TAG_OBJECT);
        self.check_forwarding_pointer();
        self.forward().get_ptr() as *mut Object
    }

    /// Get the pointer as `Region` using forwarding.
    pub unsafe fn as_region(self) -> *mut Region {
        debug_assert!(self.tag() == TAG_REGION);
        self.check_forwarding_pointer();
        self.forward().get_ptr() as *mut Region
    }

    /// Get the pointer as `Region` using forwarding, without checking the tag.
    /// NB: One cannot check the tag during stabilization.
    pub unsafe fn as_untagged_region(self) -> *mut Region {
        self.check_forwarding_pointer();
        self.forward().get_ptr() as *mut Region
    }

    /// Get the pointer as `Concat` using forwarding. In debug mode panics if the value is not a pointer or the
    /// pointed object is not a `Concat`.
    pub unsafe fn as_concat(self) -> *const Concat {
        debug_assert_eq!(self.tag(), TAG_CONCAT);
        self.check_forwarding_pointer();
        self.forward().get_ptr() as *const Concat
    }

    /// Get the pointer as `Blob` using forwarding. In debug mode panics if the value is not a pointer or the
    /// pointed object is not a `Blob`.
    pub unsafe fn as_blob(self) -> *const Blob {
        debug_assert!(self.is_blob());
        self.check_forwarding_pointer();
        self.forward().get_ptr() as *const Blob
    }

    /// Get the pointer as mutable `Blob` using forwarding.
    pub unsafe fn as_blob_mut(self) -> *mut Blob {
        debug_assert!(self.is_blob());
        self.check_forwarding_pointer();
        self.forward().get_ptr() as *mut Blob
    }

    /// Get the pointer as `BigInt` using forwarding. In debug mode panics if the value is not a pointer or the
    /// pointed object is not a `BigInt`.
    pub unsafe fn as_bigint(self) -> *mut BigInt {
        debug_assert_eq!(self.tag(), TAG_BIGINT);
        self.check_forwarding_pointer();
        self.forward().get_ptr() as *mut BigInt
    }

    pub fn as_tiny(self) -> isize {
        debug_assert!(self.is_scalar());
        self.0 as isize >> 1
    }

    // optimized version of `value.is_non_null_ptr() && value.get_ptr() >= address`
    // value is a non-null pointer equal or greater than the unskewed address > 1
    #[inline]
    #[enhanced_orthogonal_persistence]
    pub fn points_to_or_beyond(&self, address: usize) -> bool {
        debug_assert!(address > TRUE_VALUE);
        let raw = self.get_raw();
        is_skewed(raw) && unskew(raw) >= address && *self != NULL_POINTER
    }
}

#[inline]
/// Returns whether a raw value is representing a pointer. Useful when using `Value::get_raw`.
pub fn is_ptr(value: usize) -> bool {
    is_skewed(value) && value != TRUE_VALUE
}

#[inline]
pub const fn is_skewed(value: usize) -> bool {
    value & 0b1 != 0
}

#[inline]
pub const fn skew(ptr: usize) -> usize {
    ptr.wrapping_sub(1)
}

#[inline]
pub const fn unskew(value: usize) -> usize {
    value.wrapping_add(1)
}

// NOTE: We don't create an enum for tags as we can never assume to do exhaustive pattern match on
// tags, because of heap corruptions and other bugs (in the code generator or RTS, or maybe because
// of an unsafe API usage).
pub type Tag = usize;

// Tags need to have the lowest bit set, to allow distinguishing a header (tag) from object
// locations in mark-compact GC. (Reminder: objects and fields are word aligned).
// Odd tag numbers are expected by the mark-compact GC (for pointer threading).
pub const TAG_OBJECT: Tag = 1;
pub const TAG_ARRAY_I: Tag = 3; // Immutable Array ([T])
pub const TAG_ARRAY_M: Tag = 5; // Mutable Array ([var T])
pub const TAG_ARRAY_T: Tag = 7; // Non-nullary Tuple ((T,+))
pub const TAG_ARRAY_S: Tag = 9; // Shared function pairing TAG_BLOB_A with TAG_BLOB_T (shared ... -> ...)
pub const TAG_BITS64_U: Tag = 11; // Unsigned (Nat64)
pub const TAG_BITS64_S: Tag = 13; // Signed (Int64)
pub const TAG_BITS64_F: Tag = 15; // Float
pub const TAG_MUTBOX: Tag = 17;
pub const TAG_CLOSURE: Tag = 19;
pub const TAG_SOME: Tag = 21;
pub const TAG_VARIANT: Tag = 23;
pub const TAG_BLOB_B: Tag = 25; // Blob of Bytes (Blob)
pub const TAG_BLOB_T: Tag = 27; // Blob of Utf8 (Text)
pub const TAG_BLOB_P: Tag = 29; // Principal (Principal)
pub const TAG_BLOB_A: Tag = 31; // Actor (actor {})
pub const TAG_FWD_PTR: Tag = 33; // Used by graph copy stabilization and the copying GC - not to be confused with the incremental GC's forwarding pointer.
pub const TAG_BIGINT: Tag = 35;
pub const TAG_CONCAT: Tag = 37;
pub const TAG_REGION: Tag = 39;

#[enhanced_orthogonal_persistence]
pub const TAG_ONE_WORD_FILLER: Tag = 41;
#[enhanced_orthogonal_persistence]
pub const TAG_FREE_SPACE: Tag = 43;

#[enhanced_orthogonal_persistence]
pub const TAG_WEAK_REF: Tag = 45;

// Special value to visit only a range of array fields.
// This and all values above it are reserved and mean
// a slice of an array object (i.e. compressed array tag + start index) for
// purposes of `visit_pointer_fields`.
// The top two bits encode the original array tag, the remaining bits are the start index of the slice.
// Invariant: the value of this (pseudo-)tag must be
//            higher than all other tags defined above.
// Note: The minimum value can be even, as it only denotes
// a lower boundary to distinguish slice information from
// the actual tag values.
#[enhanced_orthogonal_persistence]
pub const TAG_ARRAY_SLICE_MIN: Tag = 46;

pub const TAG_SPACING: Tag = 2;

#[enhanced_orthogonal_persistence]
pub fn is_object_tag(tag: Tag) -> bool {
    tag >= TAG_OBJECT && tag <= TAG_REGION || tag == TAG_WEAK_REF
}

#[enhanced_orthogonal_persistence]
pub fn is_weak_ref_tag(tag: Tag) -> bool {
    tag == TAG_WEAK_REF
}

pub fn is_blob_tag(tag: Tag) -> bool {
    tag == TAG_BLOB_B || tag == TAG_BLOB_T || tag == TAG_BLOB_P || tag == TAG_BLOB_A
}

pub fn is_base_array_tag(tag: Tag) -> bool {
    tag == TAG_ARRAY_I || tag == TAG_ARRAY_M || tag == TAG_ARRAY_T || tag == TAG_ARRAY_S
}

pub fn is_array_or_slice_tag(tag: Tag) -> bool {
    is_base_array_tag(tag) || tag >= TAG_ARRAY_SLICE_MIN
}

#[inline]
pub fn start_of_slice(tag: Tag) -> usize {
    tag << 2 >> 2
}

#[inline]
pub fn tag_of_slice(tag: Tag) -> Tag {
    TAG_ARRAY_I + (tag >> (usize::BITS - 2)) * TAG_SPACING
}

pub fn slice_tag(array_tag: Tag, slice_start: usize) -> Tag {
    debug_assert!(is_base_array_tag(array_tag));
    debug_assert!(
        slice_start >= TAG_ARRAY_SLICE_MIN && slice_start <= MAX_ARRAY_LENGTH_FOR_ITERATOR
    );
    debug_assert!((array_tag - TAG_ARRAY_I) % TAG_SPACING == 0);
    (((array_tag - TAG_ARRAY_I) / TAG_SPACING) << (usize::BITS - 2)) | slice_start
}

pub fn slice_start(tag: Tag) -> (Tag, usize) {
    debug_assert!(is_array_or_slice_tag(tag));
    if tag >= TAG_ARRAY_SLICE_MIN {
        (tag_of_slice(tag), start_of_slice(tag))
    } else {
        (tag, 0)
    }
}

pub fn base_array_tag(tag: Tag) -> Tag {
    debug_assert!(is_array_or_slice_tag(tag));
    let base = if tag >= TAG_ARRAY_SLICE_MIN {
        tag_of_slice(tag)
    } else {
        tag
    };
    debug_assert!(is_base_array_tag(base));
    base
}

// Common parts of any object. Other object pointers can be coerced into a pointer to this.
#[repr(C)] // See the note at the beginning of this module
pub struct Obj {
    pub tag: Tag,
    // Cannot use `#[incremental_gc]` as Rust only allows non-macro attributes for fields.
    #[cfg(feature = "incremental_gc")]
    /// Forwarding pointer to support object moving in the incremental GC.
    pub forward: Value,
}

impl Obj {
    #[enhanced_orthogonal_persistence]
    pub fn new(tag: Tag, forward: Value) -> Obj {
        Obj { tag, forward }
    }

    #[incremental_gc]
    pub fn init_forward(&mut self, value: Value) {
        self.forward = value;
    }

    /// Check whether the object's forwarding pointer refers to a different location.
    #[incremental_gc]
    pub unsafe fn is_forwarded(self: *const Self) -> bool {
        (*self).forward.get_ptr() != self as usize
    }

    pub unsafe fn tag(self: *const Self) -> Tag {
        (*self).tag
    }

    pub unsafe fn as_blob(self: *mut Self) -> *mut Blob {
        debug_assert!(is_blob_tag(self.tag()));
        self as *mut Blob
    }

    pub unsafe fn as_concat(self: *mut Self) -> *const Concat {
        debug_assert_eq!(self.tag(), TAG_CONCAT);
        self as *const Concat
    }
}

#[rustfmt::skip]
#[repr(C)] // See the note at the beginning of this module
pub struct Array {
    pub header: Obj,
    pub len: usize, // number of elements

    // Array elements follow, each of `usize` width. We can't have variable-sized structs in Rust so we
    // can't add a field here for the elements.
    // https://doc.rust-lang.org/nomicon/exotic-sizes.html
}

impl Array {
    pub unsafe fn payload_addr(self: *const Self) -> *mut Value {
        self.offset(1) as *mut Value // skip array header
    }

    pub unsafe fn get(self: *mut Self, idx: usize) -> Value {
        let slot_addr = self.element_address(idx);
        *(slot_addr as *const Value)
    }

    /// Initialize the element of a newly created array.
    /// Uses a generational post-update barrier on pointer writes.
    /// No incremental pre-update barrier as the previous value is undefined.
    /// Resolve pointer forwarding for the written value if necessary.
    pub unsafe fn initialize<M: Memory>(self: *mut Self, idx: usize, value: Value, mem: &mut M) {
        let slot_addr = self.element_address(idx) as *mut Value;
        init_with_barrier(mem, slot_addr, value);
    }

    /// Write a value to an array element.
    /// The written and overwritten value can be a scalar or a pointer.
    /// Applies an incremental pre-update barrier when needed.
    /// Resolves pointer forwarding for the written value.
    pub unsafe fn set<M: Memory>(self: *mut Self, idx: usize, value: Value, mem: &mut M) {
        let slot_addr = self.element_address(idx) as *mut Value;
        write_with_barrier(mem, slot_addr, value);
    }

    /// Note: Only directly used by graph destabilization. No write barrier is applied.
    pub unsafe fn set_raw(self: *mut Self, idx: usize, value: Value) {
        let slot_addr = self.element_address(idx);
        *(slot_addr as *mut Value) = value;
    }

    #[inline]
    unsafe fn element_address(self: *const Self, idx: usize) -> usize {
        debug_assert!(self.len() > idx);
        self.payload_addr() as usize + idx * WORD_SIZE
    }

    pub unsafe fn len(self: *const Self) -> usize {
        (*self).len
    }

    pub unsafe fn base_tag(self: *const Self) -> Tag {
        base_array_tag((*self).header.tag)
    }

    pub unsafe fn get_slice_start(self: *const Self) -> (Tag, usize) {
        slice_start((*self).header.tag)
    }

    pub unsafe fn set_slice_start(self: *mut Self, array_tag: Tag, start: usize) {
        debug_assert!(is_base_array_tag(array_tag));
        (*self).header.tag = slice_tag(array_tag, start)
    }

    pub unsafe fn restore_tag(self: *mut Self, array_tag: Tag) {
        debug_assert!(is_base_array_tag(array_tag));
        (*self).header.tag = array_tag;
    }
}

#[rustfmt::skip]
#[repr(C)] // See the note at the beginning of this module
#[enhanced_orthogonal_persistence]
pub struct Region {
    pub header: Obj,
    pub id: u64,
    pub page_count: usize,
    pub vec_pages: Value, // Blob of u16's (each a page block ID).
}

#[enhanced_orthogonal_persistence]
impl Region {
    pub unsafe fn write_id64(self: *mut Self, value: u64) {
        (*self).id = value;
    }

    pub unsafe fn read_id64(self: *mut Self) -> u64 {
        (*self).id
    }
}

#[repr(C)] // See the note at the beginning of this module
pub struct Object {
    pub header: Obj,
    pub hash_blob: Value, // Pointer to a blob containing the hashes of the object field labels.
}

impl Object {
    pub unsafe fn hash_blob_addr(self: *mut Self) -> *mut Value {
        &mut (*self).hash_blob
    }

    pub unsafe fn payload_addr(self: *mut Self) -> *mut Value {
        self.add(1) as *mut Value // skip object header
    }

    /// Number of fields in the object.
    pub(crate) unsafe fn size(self: *mut Self) -> usize {
        let hash_blob_length = (*self).hash_blob.as_blob().len().as_usize();
        debug_assert_eq!(hash_blob_length % WORD_SIZE, 0);
        hash_blob_length / WORD_SIZE
    }

    #[allow(unused)]
    pub(crate) unsafe fn get(self: *mut Self, idx: usize) -> Value {
        *self.payload_addr().add(idx)
    }
}

#[repr(C)] // See the note at the beginning of this module
pub struct Closure {
    pub header: Obj,
    pub funid: usize,
    pub size: usize, // number of elements
                     // other stuff follows ...
}

impl Closure {
    pub unsafe fn payload_addr(self: *mut Self) -> *mut Value {
        self.offset(1) as *mut Value // skip closure header
    }

    pub(crate) unsafe fn size(self: *mut Self) -> usize {
        (*self).size
    }
}

#[repr(C)] // See the note at the beginning of this module
pub struct Blob {
    pub header: Obj,
    pub len: Bytes<usize>,
    // data follows ..
}

impl Blob {
    pub unsafe fn payload_addr(self: *mut Self) -> *mut u8 {
        self.add(1) as *mut u8 // skip blob header
    }

    pub unsafe fn payload_const(self: *const Self) -> *const u8 {
        self.add(1) as *mut u8 // skip blob header
    }

    pub unsafe fn len(self: *const Self) -> Bytes<usize> {
        (*self).len
    }

    pub unsafe fn get(self: *const Self, idx: usize) -> u8 {
        *self.payload_const().add(idx)
    }

    pub unsafe fn set(self: *mut Self, idx: usize, byte: u8) {
        *self.payload_addr().add(idx) = byte;
    }

    pub unsafe fn payload_addr_u16(self: *mut Self) -> *mut u16 {
        self.add(1) as *mut u16 // skip blob header
    }

    pub unsafe fn payload_const_u16(self: *const Self) -> *const u16 {
        self.add(1) as *mut u16 // skip blob header
    }

    pub unsafe fn get_u16(self: *const Self, idx: usize) -> u16 {
        *self.payload_const_u16().add(idx)
    }

    pub unsafe fn set_u16(self: *mut Self, idx: usize, value: u16) {
        *self.payload_addr_u16().add(idx) = value;
    }

    /// Shrink blob to the given size. Slop after the new size is filled with filler objects.
    pub unsafe fn shrink(self: *mut Self, new_len: Bytes<usize>) {
        let current_len_words = self.len().to_words();
        let new_len_words = new_len.to_words();

        debug_assert!(new_len_words <= current_len_words);

        let slop = current_len_words - new_len_words;

        if slop == Words(1) {
            let filler = (self.payload_addr() as *mut usize).add(new_len_words.as_usize())
                as *mut OneWordFiller;
            (*filler).tag = TAG_ONE_WORD_FILLER;
        } else if slop != Words(0) {
            debug_assert!(slop >= size_of::<FreeSpace>());
            let filler =
                (self.payload_addr() as *mut usize).add(new_len_words.as_usize()) as *mut FreeSpace;
            (*filler).tag = TAG_FREE_SPACE;
            (*filler).words = slop - size_of::<FreeSpace>();
        }

        (*self).len = new_len;
    }
}

/// Only used by the copying GC - not to be confused with the forwarding pointer in the general object header
/// that is used by the incremental GC.
/// A forwarding pointer placed by the copying GC in place of an evacuated object.
#[repr(C)] // See the note at the beginning of this module
pub struct FwdPtr {
    pub tag: Tag,
    pub fwd: Value,
}

#[repr(C)] // See the note at the beginning of this module
pub struct BigInt {
    pub header: Obj,
    /// The data following now must describe is the `mp_int` struct.
    /// The data pointer (mp_int.dp) is irrelevant, and will be changed to point to
    /// the data within this object before it is used.
    /// (NB: If we have a non-moving GC, we can make this an invariant)
    /// NOTE: `mp_int` originates from Tom's math library implementation.
    /// Layout in 64-bit memory (with enhanced orthogonal persistence):
    /// ```
    /// pub struct mp_int { // Total size 24
    ///   pub used: c_int, // Offset 0, size 4
    ///   pub alloc: c_int, // Offset 4, size 4
    ///   pub sign: mp_sign, // Offset 8, size 4
    ///   _padding: u32, // Implicit padding to align subsequent 64-bit pointer
    ///   pub dp: *mut mp_digit, // Offset 16, size 8
    /// }
    /// ```
    /// Layout in 32-bit memory (with classical persistence):
    /// ```
    /// pub struct mp_int { // Total size 24
    ///   pub used: c_int, // Offset 0, size 4
    ///   pub alloc: c_int, // Offset 4, size 4
    ///   pub sign: mp_sign, // Offset 8, size 4
    ///   pub dp: *mut mp_digit, // Offset 12, size 4
    /// }
    /// ```
    pub mp_int: mp_int,
    // data follows ..
    // Array of `mp_int` with length `alloc`.
    // Each `mp_int` has byte size `size_of<usize>()`.
}

impl BigInt {
    pub unsafe fn len(self: *mut Self) -> Bytes<usize> {
        Self::data_length(&(*self).mp_int)
    }

    pub unsafe fn data_length(mp_int: *const mp_int) -> Bytes<usize> {
        Bytes((*mp_int).alloc as usize * core::mem::size_of::<mp_digit>())
    }

    pub unsafe fn payload_addr(self: *mut Self) -> *mut mp_digit {
        self.add(1) as *mut mp_digit // skip closure header
    }

    #[incremental_gc]
    pub unsafe fn forward(self: *mut Self) -> *mut Self {
        (*self).header.forward.as_bigint()
    }

    pub unsafe fn from_payload(ptr: *mut mp_digit) -> *mut Self {
        let bigint = (ptr as *mut usize).sub(size_of::<BigInt>().as_usize()) as *mut BigInt;
        bigint.forward()
    }

    /// Returns pointer to the `mp_int` struct
    ///
    /// It fixes up the dp pointer. Instead of doing it here
    /// this could be done on allocation and every object move.
    ///
    /// Note that this returns a `const` pointer. This is very nice, as together with the const
    /// annotation on the libtommath API, this should prevent us from passing this pointer to a
    /// libtommath function that tries to change it. For example, we cannot confuse input and
    /// output parameters of mp_add() this way.
    pub unsafe fn mp_int_ptr(self: *mut BigInt) -> *const mp_int {
        (*self).mp_int.dp = self.payload_addr();
        &(*self).mp_int
    }
}

#[repr(C)] // See the note at the beginning of this module
pub struct MutBox {
    pub header: Obj,
    pub field: Value,
}

#[repr(C)] // See the note at the beginning of this module
pub struct Some {
    pub header: Obj,
    pub field: Value,
}

#[repr(C)] // See the note at the beginning of this module
pub struct Variant {
    pub header: Obj,
    pub tag: usize,
    pub field: Value,
}

#[repr(C)] // See the note at the beginning of this module
pub struct Concat {
    pub header: Obj,
    pub n_bytes: Bytes<usize>,
    pub text1: Value,
    pub text2: Value,
}

impl Concat {
    pub unsafe fn text1(self: *const Self) -> Value {
        (*self).text1
    }

    pub unsafe fn text2(self: *const Self) -> Value {
        (*self).text2
    }
}

#[repr(C)] // See the note at the beginning of this module
pub struct Bits64 {
    pub header: Obj,
    pub bits: u64,
}

impl Bits64 {
    pub fn bits(&self) -> u64 {
        self.bits
    }
}

/// Marks one word empty space in heap
#[repr(C)] // See the note at the beginning of this module
pub struct OneWordFiller {
    pub tag: Tag,
}

/// Marks arbitrary sized emtpy space in heap
#[repr(C)] // See the note at the beginning of this module
pub struct FreeSpace {
    pub tag: Tag,
    pub words: Words<usize>,
}

impl FreeSpace {
    /// Size of the free space (includes object header)
    pub unsafe fn size(self: *mut Self) -> Words<usize> {
        (*self).words + size_of::<FreeSpace>()
    }
}

/// Marks a weak reference to an object.
#[repr(C)] // See the note at the beginning of this module
pub struct WeakRef {
    pub header: Obj,
    pub field: Value,
}

#[cfg(feature = "enhanced_orthogonal_persistence")]
impl WeakRef {
    pub unsafe fn is_live(&self) -> bool {
        let field_tag = self.header.tag;
        match field_tag {
            TAG_WEAK_REF => {
                return self.field.is_non_null_ptr();
            }
            _ => {
                crate::rts_trap_with("weak_ref_is_live: Called on a non-weak reference.");
            }
        }
    }
}

/// Returns the heap block size in words.
/// Handles both objects with header and forwarding pointer
/// and special blocks such as `OneWordFiller`, `FwdPtr`, and `FreeSpace`
/// that do not have a forwarding pointer.
pub(crate) unsafe fn block_size(address: usize) -> Words<usize> {
    let tag = *(address as *mut Tag);
    match tag {
        TAG_OBJECT => {
            let object = address as *mut Object;
            let size = object.size();
            size_of::<Object>() + Words(size)
        }

        #[cfg(feature = "enhanced_orthogonal_persistence")]
        TAG_WEAK_REF => size_of::<WeakRef>(),

        // `block_size` is not used during the incremental mark phase and
        // therefore, does not support array slicing.
        TAG_ARRAY_I | TAG_ARRAY_M | TAG_ARRAY_T | TAG_ARRAY_S => {
            let array = address as *mut Array;
            let size = array.len();
            size_of::<Array>() + Words(size)
        }

        TAG_BITS64_U | TAG_BITS64_S | TAG_BITS64_F => size_of::<Bits64>(),

        TAG_MUTBOX => size_of::<MutBox>(),

        TAG_CLOSURE => {
            let closure = address as *mut Closure;
            let size = closure.size();
            size_of::<Closure>() + Words(size)
        }

        TAG_SOME => size_of::<Some>(),

        TAG_VARIANT => size_of::<Variant>(),

        TAG_BLOB_B | TAG_BLOB_T | TAG_BLOB_P | TAG_BLOB_A => {
            let blob = address as *mut Blob;
            size_of::<Blob>() + blob.len().to_words()
        }

        TAG_FWD_PTR => {
            rts_trap_with("object_size: forwarding pointer");
        }

        TAG_BIGINT => {
            let bigint = address as *mut BigInt;
            size_of::<BigInt>() + bigint.len().to_words()
        }

        TAG_CONCAT => size_of::<Concat>(),

        TAG_ONE_WORD_FILLER => size_of::<OneWordFiller>(),

        TAG_FREE_SPACE => {
            let free_space = address as *mut FreeSpace;
            free_space.size()
        }

        TAG_REGION => size_of::<Region>(),

        _ => {
            rts_trap_with("object_size: invalid object tag");
        }
    }
}
