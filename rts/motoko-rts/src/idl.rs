#![allow(non_upper_case_globals)]

use crate::bitrel::BitRel;
use crate::buf::{Buf, read_byte, read_word, skip_leb128};
use crate::idl_trap_with;

use crate::memory::{Memory, alloc_blob};
use crate::types::{TAG_BLOB_B, Words};
use crate::utf8::utf8_validate;

use core::cmp::min;

use motoko_rts_macros::{enhanced_orthogonal_persistence, ic_mem_fn};

use crate::libc_declarations::{c_void, memcmp};

#[enhanced_orthogonal_persistence]
use crate::types::Value;

unsafe extern "C" {
    // check instruction decoding limit, exported by moc
    pub fn idl_limit_check(decrement: bool, value_count: u64);
}

//
// IDL constants
//

const IDL_PRIM_null: i32 = -1;
const IDL_PRIM_bool: i32 = -2;
const IDL_PRIM_nat: i32 = -3;
const IDL_PRIM_int: i32 = -4;
const IDL_PRIM_nat8: i32 = -5;
const IDL_PRIM_nat16: i32 = -6;
const IDL_PRIM_nat32: i32 = -7;
const IDL_PRIM_nat64: i32 = -8;
const IDL_PRIM_int8: i32 = -9;
const IDL_PRIM_int16: i32 = -10;
const IDL_PRIM_int32: i32 = -11;
const IDL_PRIM_int64: i32 = -12;
const IDL_PRIM_float32: i32 = -13;
const IDL_PRIM_float64: i32 = -14;
const IDL_PRIM_text: i32 = -15;
const IDL_PRIM_reserved: i32 = -16;
const IDL_PRIM_empty: i32 = -17;

const IDL_CON_opt: i32 = -18;
const IDL_CON_vec: i32 = -19;
const IDL_CON_record: i32 = -20;
const IDL_CON_variant: i32 = -21;
const IDL_CON_func: i32 = -22;
const IDL_CON_service: i32 = -23;

const IDL_REF_principal: i32 = -24;

// Extended Candid only
const IDL_EXT_region: i32 = -128;

// Extended Candid only
const IDL_CON_alias: i32 = 1;

const IDL_PRIM_lowest: i32 = -17;

// Only used for memory compatiblity checks for orthogonal persistence.
#[enhanced_orthogonal_persistence]
const IDL_EXT_blob: i32 = -129;
#[enhanced_orthogonal_persistence]
const IDL_EXT_tuple: i32 = -130;
#[enhanced_orthogonal_persistence]
const IDL_EXT_weak: i32 = -131;

unsafe fn leb128_decode(buf: *mut Buf) -> u32 {
    let value = crate::leb128::leb128_decode(buf);
    assert!(value <= u32::MAX as usize);
    value as u32
}

unsafe fn sleb128_decode(buf: *mut Buf) -> i32 {
    let value = crate::leb128::sleb128_decode(buf);
    assert!(value >= i32::MIN as isize && value <= i32::MAX as isize);
    value as i32
}

pub unsafe fn leb128_decode_ptr(buf: *mut Buf) -> (u32, *mut u8) {
    (leb128_decode(buf), (*buf).ptr)
}

#[derive(Copy, Clone, PartialEq)]
enum CompatibilityMode {
    /// Pure Candid used for IC message payloads.
    PureCandid,
    /// Candidish stabilization (old stabilization format).
    CandidishStabilization,
    /// Memory compatibility of orthogonal persistence (with or without graph copying).
    #[cfg(feature = "enhanced_orthogonal_persistence")]
    MemoryCompatibility,
}

unsafe fn is_primitive_type(mode: CompatibilityMode, ty: i32) -> bool {
    if ty >= 0 {
        return false;
    }
    if ty >= IDL_PRIM_lowest || ty == IDL_REF_principal {
        return true;
    }
    match mode {
        CompatibilityMode::PureCandid => false,
        CompatibilityMode::CandidishStabilization => ty == IDL_EXT_region,
        #[cfg(feature = "enhanced_orthogonal_persistence")]
        CompatibilityMode::MemoryCompatibility => ty == IDL_EXT_region || ty == IDL_EXT_blob,
    }
}

// TBR; based on Text.text_compare
unsafe fn utf8_cmp(len1: usize, p1: *mut u8, len2: usize, p2: *mut u8) -> i32 {
    let len = min(len1, len2);
    let cmp = memcmp(p1 as *mut c_void, p2 as *mut c_void, len);
    if cmp != 0 {
        return cmp;
    } else if len1 > len {
        return 1;
    } else if len2 > len {
        return -1;
    } else {
        return 0;
    }
}

unsafe fn check_typearg(mode: CompatibilityMode, ty: i32, n_types: u32) {
    // Arguments to type constructors can be primitive types or type indices
    if !(is_primitive_type(mode, ty) || (ty >= 0 && (ty as u32) < n_types)) {
        idl_trap_with("invalid type argument");
    }
}

unsafe fn parse_fields(mode: CompatibilityMode, buf: *mut Buf, n_types: u32) {
    let mut next_valid = 0;
    for n in (1..=leb128_decode(buf)).rev() {
        let tag = leb128_decode(buf);
        if (tag < next_valid) || (tag == 0xFFFFFFFF && n > 1) {
            idl_trap_with("variant or record tag out of order");
        }
        next_valid = tag + 1;
        let t = sleb128_decode(buf);
        check_typearg(mode, t, n_types);
    }
}

// NB. This function assumes the allocation does not need to survive GC
// Therefore, no post allocation barrier is applied.
unsafe fn alloc<M: Memory>(mem: &mut M, size: Words<usize>) -> *mut u8 {
    alloc_blob(mem, TAG_BLOB_B, size.to_bytes())
        .as_blob_mut()
        .payload_addr()
}

/// This function parses the IDL magic header and type description. It
///
/// * traps if the type description is not well-formed. In particular, it traps if any index into
///   the type description table is out of bounds, so that subsequent code can trust these values
///
/// * advances `buf` until the first byte after the IDL header
///
/// * allocates a type description table, and returns it
///   (via pointer argument, for lack of multi-value returns in C ABI)
///
/// * returns the size of that type description table
///   (again via pointer argument, for lack of multi-value returns in C ABI)
///
/// * returns a pointer to the beginning of the list of main types
///   (again via pointer argument, for lack of multi-value returns in C ABI)
///
/// `extended` denotes Candidish stabilization format, otherwise it assumes the pure Candid format.
#[ic_mem_fn]
unsafe fn parse_idl_header<M: Memory>(
    mem: &mut M,
    extended: bool,
    buf: *mut Buf,
    typtbl_max: usize,
    typtbl_out: *mut *mut *mut u8,
    typtbl_size_out: *mut usize,
    main_types_out: *mut *mut u8,
) {
    let mode = if extended {
        CompatibilityMode::CandidishStabilization
    } else {
        CompatibilityMode::PureCandid
    };

    if (*buf).ptr == (*buf).end {
        idl_trap_with(
            "empty input. Expected Candid-encoded argument, but received a zero-length argument",
        );
    }

    // Magic bytes (DIDL)
    if read_word(buf) != 0x4C444944 {
        idl_trap_with("missing magic bytes");
    }

    // Create a table for the type description
    let n_types = leb128_decode(buf);

    // Early sanity check
    if (*buf).ptr.add(n_types as usize) >= (*buf).end {
        idl_trap_with("too many types");
    }

    // Let the caller know about the table size
    *typtbl_size_out = n_types as usize;

    // Allocate the type table to be passed out
    let typtbl: *mut *mut u8 = alloc(mem, Words(n_types as usize)) as *mut _;

    // Set up a limited buffer to detect absurd type tables
    let full_end = if extended {
        (*buf).end
    } else {
        buf.limit_size(typtbl_max)
    };

    // Go through the table
    for i in 0..n_types {
        *typtbl.add(i as usize) = (*buf).ptr;

        let ty = sleb128_decode(buf);

        if extended && ty == IDL_CON_alias {
            // internal
            // See Note [mutable stable values] in codegen/compile.ml
            let t = sleb128_decode(buf);
            check_typearg(mode, t, n_types);
        } else if ty >= 0 {
            idl_trap_with("illegal type table"); // illegal
        } else if is_primitive_type(mode, ty) {
            // illegal
            idl_trap_with("primitive type in type table");
        } else if ty == IDL_CON_opt {
            let t = sleb128_decode(buf);
            check_typearg(mode, t, n_types);
        } else if ty == IDL_CON_vec {
            let t = sleb128_decode(buf);
            check_typearg(mode, t, n_types);
        } else if ty == IDL_CON_record {
            parse_fields(mode, buf, n_types);
        } else if ty == IDL_CON_variant {
            parse_fields(mode, buf, n_types);
        } else if ty == IDL_CON_func {
            // Arg types
            for _ in 0..leb128_decode(buf) {
                let t = sleb128_decode(buf);
                check_typearg(mode, t, n_types);
            }
            // Ret types
            for _ in 0..leb128_decode(buf) {
                let t = sleb128_decode(buf);
                check_typearg(mode, t, n_types);
            }
            // Annotations
            for _ in 0..leb128_decode(buf) {
                let a = read_byte(buf);
                if !(1 <= a && a <= 3) {
                    idl_trap_with("func annotation not within 1..3");
                }
                // TODO: shouldn't we also check
                // * 1 (query) or 2 (oneway), but not both
                // * 2 -> |Ret types| == 0
                // c.f. https://github.com/dfinity/candid/issues/318
                // NB: if this code changes, change sub type check below accordingly
            }
        } else if ty == IDL_CON_service {
            let mut last_len: u32 = 0 as u32;
            let mut last_p = core::ptr::null_mut();
            for _ in 0..leb128_decode(buf) {
                // Name
                let (len, p) = leb128_decode_ptr(buf);
                buf.advance(len as usize);
                // Method names must be valid unicode
                utf8_validate(p as *const _, len as usize);
                // Method names must be in order
                if last_p != core::ptr::null_mut() {
                    let cmp = memcmp(
                        last_p as *mut c_void,
                        p as *mut c_void,
                        min(last_len, len) as usize,
                    );
                    if cmp > 0 || (cmp == 0 && last_len >= len) {
                        idl_trap_with("service method names out of order");
                    }
                }
                last_len = len;
                last_p = p;

                // Type
                let t = sleb128_decode(buf);
                check_typearg(mode, t, n_types);
            }
        } else {
            // Future type
            let n = leb128_decode(buf);
            buf.advance(n as usize);
        }
    }

    // Restore limit
    (*buf).end = full_end;

    // Now that we have the indices, we can go through it again
    // and validate that all service method types are really function types
    // (We could not do that in the first run because of possible forward
    // references
    for i in 0..n_types {
        // do not modify the main buf
        let mut tmp_buf = Buf {
            end: (*buf).end,
            ptr: *typtbl.add(i as usize),
        };

        let ty = sleb128_decode(&mut tmp_buf);
        if ty == IDL_CON_service {
            for _ in 0..leb128_decode(&mut tmp_buf) {
                // Name
                let len = leb128_decode(&mut tmp_buf);
                Buf::advance(&mut tmp_buf, len as usize);
                // Type
                let t = sleb128_decode(&mut tmp_buf);
                if !(t >= 0 && (t as u32) < n_types) {
                    idl_trap_with("service method arg not a constructor type");
                }
                let mut tmp_buf2 = Buf {
                    end: (*buf).end,
                    ptr: *typtbl.add(t as usize),
                };
                let mty = sleb128_decode(&mut tmp_buf2);
                if mty != IDL_CON_func {
                    idl_trap_with("service method arg not a function type");
                }
            }
        }
    }

    // Now read the main types
    *main_types_out = (*buf).ptr;
    for _ in 0..leb128_decode(buf) {
        let t = sleb128_decode(buf);
        check_typearg(mode, t, n_types);
    }

    *typtbl_out = typtbl;
}

// used for opt, bool, references...
unsafe fn read_byte_tag(buf: *mut Buf) -> u8 {
    let b = read_byte(buf);
    if b > 1 {
        idl_trap_with("skip_any: byte tag not 0 or 1");
    }
    b
}

unsafe fn skip_blob(buf: *mut Buf) {
    let len = leb128_decode(buf);
    buf.advance(len as usize);
}

unsafe fn skip_text(buf: *mut Buf) {
    let (len, p) = leb128_decode_ptr(buf);
    buf.advance(len as usize); // advance first; does the bounds check
    utf8_validate(p as *const _, len as usize);
}

unsafe fn skip_any_vec(buf: *mut Buf, typtbl: *mut *mut u8, t: i32, count: u32) {
    if count == 0 {
        return;
    }
    idl_limit_check(false, count as u64);
    let ptr_before = (*buf).ptr;
    skip_any(buf, typtbl, t, 0);
    let ptr_after = (*buf).ptr;
    if ptr_after == ptr_before {
        // this looks like a vec null bomb, or equivalent, where skip_any
        // makes no progress. No point in calling it over and over again.
        // (This is easier to detect this way than by analyzing the type table,
        // where we’d have to chase single-field-records.)
        idl_limit_check(true, (count - 1) as u64);
        return;
    }
    for _ in 1..count {
        skip_any(buf, typtbl, t, 0);
    }
}

// Assumes buf is the encoding of type t, and fast-forwards past that
// Assumes all type references in the typtbl are already checked
//
// This is currently implemented recursively, but we could
// do this in a loop (by maintaining a stack of the t arguments)
#[unsafe(no_mangle)]
unsafe extern "C" fn skip_any(buf: *mut Buf, typtbl: *mut *mut u8, t: i32, depth: i32) {
    if depth > 100 {
        idl_trap_with("skip_any: too deeply nested record");
    }

    idl_limit_check(true, 1); // decrement and check quota

    if t < 0 {
        // Primitive type
        match t {
            IDL_PRIM_null | IDL_PRIM_reserved => {}
            IDL_PRIM_bool => {
                read_byte_tag(buf);
            }
            IDL_PRIM_nat | IDL_PRIM_int => {
                skip_leb128(buf);
            }
            IDL_PRIM_nat8 | IDL_PRIM_int8 => {
                buf.advance(1);
            }
            IDL_PRIM_nat16 | IDL_PRIM_int16 => {
                buf.advance(2);
            }
            IDL_PRIM_nat32 | IDL_PRIM_int32 | IDL_PRIM_float32 => {
                buf.advance(4);
            }
            IDL_PRIM_nat64 | IDL_PRIM_int64 | IDL_PRIM_float64 => {
                buf.advance(8);
            }
            IDL_PRIM_text => skip_text(buf),
            IDL_PRIM_empty => {
                idl_trap_with("skip_any: encountered empty");
            }
            IDL_REF_principal => {
                if read_byte_tag(buf) != 0 {
                    skip_blob(buf);
                }
            }
            IDL_EXT_region => {
                buf.advance(12); // id (u64) & page_count (u32)
                skip_blob(buf); // vec_pages
            }
            _ => {
                idl_trap_with("skip_any: unknown prim");
            }
        }
    } else {
        // t >= 0
        let mut tb = Buf {
            ptr: *typtbl.add(t as usize),
            end: (*buf).end,
        };
        let tc = sleb128_decode(&mut tb);
        match tc {
            IDL_CON_opt => {
                let it = sleb128_decode(&mut tb);
                if read_byte_tag(buf) != 0 {
                    skip_any(buf, typtbl, it, 0);
                }
            }
            IDL_CON_vec => {
                let it = sleb128_decode(&mut tb);
                let count = leb128_decode(buf);
                skip_any_vec(buf, typtbl, it, count);
            }
            IDL_CON_record => {
                for _ in 0..leb128_decode(&mut tb) {
                    skip_leb128(&mut tb);
                    let it = sleb128_decode(&mut tb);
                    // This is just a quick check; we should be keeping
                    // track of all enclosing records to detect larger loops
                    if it == t {
                        idl_trap_with("skip_any: recursive record");
                    }
                    skip_any(buf, typtbl, it, depth + 1);
                }
            }
            IDL_CON_variant => {
                let n = leb128_decode(&mut tb);
                let i = leb128_decode(buf);
                if i >= n {
                    idl_trap_with("skip_any: variant tag too large");
                }
                for _ in 0..i {
                    skip_leb128(&mut tb);
                    skip_leb128(&mut tb);
                }
                skip_leb128(&mut tb);
                let it = sleb128_decode(&mut tb);
                skip_any(buf, typtbl, it, 0);
            }
            IDL_CON_func => {
                if read_byte_tag(buf) == 0 {
                    idl_trap_with("skip_any: skipping references");
                } else {
                    if read_byte_tag(buf) == 0 {
                        idl_trap_with("skip_any: skipping references");
                    } else {
                        skip_blob(buf)
                    }
                    skip_text(buf)
                }
            }
            IDL_CON_service => {
                if read_byte_tag(buf) == 0 {
                    idl_trap_with("skip_any: skipping references");
                } else {
                    skip_blob(buf)
                }
            }
            IDL_CON_alias => {
                // See Note [mutable stable values] in codegen/compile.ml
                let it = sleb128_decode(&mut tb);
                let tag = read_byte_tag(buf);
                if tag == 0 {
                    buf.advance(8);
                    // this is the contents (not a reference)
                    skip_any(buf, typtbl, it, 0);
                } else {
                    buf.advance(4);
                }
            }
            _ => {
                // Future type
                let n_data = leb128_decode(buf);
                let n_ref = leb128_decode(buf);
                buf.advance(n_data as usize);
                if n_ref > 0 {
                    idl_trap_with("skip_any: skipping references");
                }
            }
        }
    }
}

/*
This finds a field in a record.

Preconditions:
  tb:     points into the type table,
          into the sequence of tags/types that are the argument of IDL_CON_record,
          at the tag
  b:      points into the data buffer, at value corresponding to the field
          pointed to by tb
  typtbl: the type table
  tag:    the desired tag
  n:      the number of fields left in the data

If the tag exists:
  return value: 1
  tb:    points at the type corresponding to the found field
  b:     points at the value corresponding to the found field
  n:     the number of fields left after the found field

If the tag does not exist:
  return value: 0
  tb:    points at the tag of the first field with a higher tag
         or at the end of the buffer
  b:     points at the value corresponding to that field
         or at the value past the record
  n:     the number of fields left, including the field pointed to by tb
*/
#[unsafe(no_mangle)]
unsafe extern "C" fn find_field(
    tb: *mut Buf,
    buf: *mut Buf,
    typtbl: *mut *mut u8,
    tag: u32,
    n: *mut u8,
) -> bool {
    while *n > 0 {
        let last_p = (*tb).ptr;
        let this_tag = leb128_decode(tb);
        if this_tag < tag {
            let it = sleb128_decode(tb);
            skip_any(buf, typtbl, it, 0);
            *n -= 1;
        } else if tag == this_tag {
            *n -= 1;
            return true;
        } else {
            // Rewind reading tag
            (*tb).ptr = last_p;
            return false;
        }
    }

    false
}

#[unsafe(no_mangle)]
unsafe extern "C" fn skip_fields(tb: *mut Buf, buf: *mut Buf, typtbl: *mut *mut u8, n: *mut u8) {
    while *n > 0 {
        skip_leb128(tb);
        let it = sleb128_decode(tb);
        skip_any(buf, typtbl, it, 0);
        *n -= 1;
    }
}

unsafe fn is_null_opt_reserved(typtbl: *mut *mut u8, end: *mut u8, t: i32) -> bool {
    if is_primitive_type(CompatibilityMode::PureCandid, t) {
        return t == IDL_PRIM_null || t == IDL_PRIM_reserved;
    }

    // unfold t
    let mut t = t;

    let mut tb = Buf {
        ptr: *typtbl.add(t as usize),
        end: end,
    };

    t = sleb128_decode(&mut tb);

    return t == IDL_CON_opt;
}

#[enhanced_orthogonal_persistence]
#[derive(PartialEq, Clone, Copy, Debug)]
pub(crate) enum TypeVariance {
    Covariance,
    Contravariance,
    Invariance,
}

#[enhanced_orthogonal_persistence]
impl TypeVariance {
    fn invert(self) -> TypeVariance {
        match self {
            TypeVariance::Covariance => TypeVariance::Contravariance,
            TypeVariance::Contravariance => TypeVariance::Covariance,
            TypeVariance::Invariance => TypeVariance::Invariance,
        }
    }
}

#[enhanced_orthogonal_persistence]
unsafe fn recurring_memory_check(
    cache: &BitRel,
    variance: TypeVariance,
    t1: usize,
    t2: usize,
) -> bool {
    match variance {
        TypeVariance::Covariance => cache.visited(true, t1, t2),
        TypeVariance::Contravariance => cache.visited(false, t1, t2),
        TypeVariance::Invariance => cache.visited(true, t1, t2) && cache.visited(false, t2, t1),
    }
}

#[enhanced_orthogonal_persistence]
unsafe fn remember_memory_check(cache: &BitRel, variance: TypeVariance, t1: usize, t2: usize) {
    match variance {
        TypeVariance::Covariance => cache.visit(true, t1, t2),
        TypeVariance::Contravariance => cache.visit(false, t1, t2),
        TypeVariance::Invariance => {
            cache.visit(true, t1, t2);
            cache.visit(false, t2, t1);
        }
    }
}

/// Memory compatibility check for orthogonal persistence (with or without graph copying).
/// Checks whether the new type (`typetbl2`) is compatible to the old type (`typetbl1`).
/// The implementation is similar to the Candid sub-type test `sub()` below, however,
/// with some relevant differences w.r.t. the permitted type relations:
/// * Support of variable (MutBox) with type invariance.
/// * Types cannot be made optional (no insertion of Option).
/// * Same arity for function parameters and function return types.
/// * Records cannot introduce additional optional fields.
/// * Same arity for tuple types.
/// * Records and tuples are distinct.
#[enhanced_orthogonal_persistence]
pub(crate) unsafe fn memory_compatible(
    rel: &BitRel,
    variance: TypeVariance,
    typtbl1: *mut *mut u8,
    typtbl2: *mut *mut u8,
    end1: *mut u8,
    end2: *mut u8,
    t1: i32,
    t2: i32,
    main_actor: bool,
) -> bool {
    // Do not use the cache for the main actor sub-type relation, as it does not follow the ordinary sub-type rules,
    // i.e. new actor fields can be inserted in new program versions.
    // The `main_actor` flag only occurs non-recursively at the top level of the memory compatibility check.
    if !main_actor && t1 >= 0 && t2 >= 0 {
        let t1 = t1 as usize;
        let t2 = t2 as usize;
        if recurring_memory_check(rel, variance, t1, t2) {
            return true;
        };
        remember_memory_check(rel, variance, t1, t2);
    };

    /* primitives reflexive */
    if is_primitive_type(CompatibilityMode::MemoryCompatibility, t1)
        && is_primitive_type(CompatibilityMode::MemoryCompatibility, t2)
        && t1 == t2
    {
        return true;
    }

    // unfold t1, if necessary
    let mut tb1 = Buf {
        ptr: if t1 < 0 {
            end1
        } else {
            *typtbl1.add(t1 as usize)
        },
        end: end1,
    };

    let u1 = if t1 >= 0 {
        sleb128_decode(&mut tb1)
    } else {
        t1
    };

    // unfold t2, if necessary
    let mut tb2 = Buf {
        ptr: if t2 < 0 {
            end2
        } else {
            *typtbl2.add(t2 as usize)
        },
        end: end2,
    };

    let u2 = if t2 >= 0 {
        sleb128_decode(&mut tb2)
    } else {
        t2
    };

    match (u1, u2) {
        (IDL_CON_alias, IDL_CON_alias) => {
            let t11 = sleb128_decode(&mut tb1);
            let t21 = sleb128_decode(&mut tb2);
            // invariance
            memory_compatible(
                rel,
                TypeVariance::Invariance,
                typtbl1,
                typtbl2,
                end1,
                end2,
                t11,
                t21,
                false,
            )
        }
        (IDL_PRIM_reserved, IDL_PRIM_reserved) | (IDL_PRIM_empty, IDL_PRIM_empty) => true,
        (_, IDL_PRIM_reserved) => false, // information lost
        (IDL_PRIM_empty, _) | (IDL_PRIM_nat, IDL_PRIM_int) => variance != TypeVariance::Invariance,
        (_, IDL_CON_alias) | (IDL_CON_alias, _) => false,
        (IDL_EXT_weak, IDL_EXT_weak) => {
            let t11 = sleb128_decode(&mut tb1);
            let t21 = sleb128_decode(&mut tb2);
            memory_compatible(rel, variance, typtbl1, typtbl2, end1, end2, t11, t21, false)
        }
        (IDL_CON_opt, IDL_CON_opt) => {
            let t11 = sleb128_decode(&mut tb1);
            let t21 = sleb128_decode(&mut tb2);
            memory_compatible(rel, variance, typtbl1, typtbl2, end1, end2, t11, t21, false)
        }
        (IDL_PRIM_null, IDL_CON_opt) => true,
        (_, IDL_CON_opt) => false,
        (IDL_CON_vec, IDL_CON_vec) => {
            let t11 = sleb128_decode(&mut tb1);
            let t21 = sleb128_decode(&mut tb2);
            memory_compatible(rel, variance, typtbl1, typtbl2, end1, end2, t11, t21, false)
        }
        (IDL_CON_func, IDL_CON_func) => {
            // contra in domain
            let in1 = leb128_decode(&mut tb1);
            let in2 = leb128_decode(&mut tb2);
            if in1 != in2 {
                return false;
            }
            for _ in 0..in1 {
                let t11 = sleb128_decode(&mut tb1);
                let t21 = sleb128_decode(&mut tb2);
                // NB: invert p and args!
                if !memory_compatible(
                    rel,
                    variance.invert(),
                    typtbl2,
                    typtbl1,
                    end2,
                    end1,
                    t21,
                    t11,
                    false,
                ) {
                    return false;
                }
            }
            // co in range
            let out1 = leb128_decode(&mut tb1);
            let out2 = leb128_decode(&mut tb2);
            if out1 != out2 {
                return false;
            }
            for _ in 0..out2 {
                let t21 = sleb128_decode(&mut tb2);
                let t11 = sleb128_decode(&mut tb1);
                if !memory_compatible(rel, variance, typtbl1, typtbl2, end1, end2, t11, t21, false)
                {
                    return false;
                }
            }
            // check annotations (that we care about)
            // TODO: more generally, we would check equality of 256-bit bit-vectors,
            // but validity ensures each entry is 1, 2 or 3 (for now)
            // c.f. https://github.com/dfinity/candid/issues/318
            let mut a11 = false;
            let mut a12 = false;
            let mut a13 = false;
            for _ in 0..leb128_decode(&mut tb1) {
                match read_byte(&mut tb1) {
                    1 => a11 = true,
                    2 => a12 = true,
                    3 => a13 = true,
                    _ => {}
                }
            }
            let mut a21 = false;
            let mut a22 = false;
            let mut a23 = false;
            for _ in 0..leb128_decode(&mut tb2) {
                match read_byte(&mut tb2) {
                    1 => a21 = true,
                    2 => a22 = true,
                    3 => a23 = true,
                    _ => {}
                }
            }
            a11 == a21 && a12 == a22 && a13 == a23
        }
        (IDL_EXT_tuple, IDL_EXT_tuple) => {
            let n1 = leb128_decode(&mut tb1);
            let n2 = leb128_decode(&mut tb2);
            if n1 != n2 {
                return false;
            }
            for _ in 0..n1 {
                let tag1 = leb128_decode(&mut tb1);
                let t11 = sleb128_decode(&mut tb1);
                let tag2 = leb128_decode(&mut tb2);
                let t21 = sleb128_decode(&mut tb2);
                if tag1 != tag2 {
                    return false;
                }
                if !memory_compatible(rel, variance, typtbl1, typtbl2, end1, end2, t11, t21, false)
                {
                    return false;
                }
            }
            true
        }
        (IDL_CON_record, IDL_CON_record) if !main_actor => {
            // field preserving object/record subtyping
            // see stable_sub in type.ml
            let mut n1 = leb128_decode(&mut tb1);
            let n2 = leb128_decode(&mut tb2);
            for _ in 0..n2 {
                let tag2 = leb128_decode(&mut tb2);
                let t21 = sleb128_decode(&mut tb2);
                if n1 == 0 {
                    return false;
                };
                let tag1 = leb128_decode(&mut tb1);
                let t11 = sleb128_decode(&mut tb1);
                n1 -= 1;
                if tag1 < tag2 {
                    return false; // discarded field
                }
                if tag1 > tag2 {
                    return false; // new field
                };
                if !memory_compatible(rel, variance, typtbl1, typtbl2, end1, end2, t11, t21, false)
                {
                    return false;
                }
            }
            return n1 == 0;
        }
        (IDL_CON_record, IDL_CON_record) if main_actor => {
            // memory compatibility
            assert!(variance == TypeVariance::Covariance);
            let mut n1 = leb128_decode(&mut tb1);
            let mut n2 = leb128_decode(&mut tb2);
            let mut tag1: u32;
            let mut t11: i32;
            let mut tag2: u32;
            let mut t21: i32;
            while n1 > 0 && n2 > 0 {
                tag1 = leb128_decode(&mut tb1);
                t11 = sleb128_decode(&mut tb1);
                tag2 = leb128_decode(&mut tb2);
                t21 = sleb128_decode(&mut tb2);
                n1 -= 1;
                n2 -= 1;
                while tag1 != tag2 {
                    if tag1 < tag2 {
                        return false; // discarded field
                    };
                    if tag1 > tag2 {
                        if n2 > 0 {
                            tag2 = leb128_decode(&mut tb2);
                            t21 = sleb128_decode(&mut tb2);
                            n2 -= 1;
                            continue;
                        };
                        return true;
                    };
                }
                if !memory_compatible(rel, variance, typtbl1, typtbl2, end1, end2, t11, t21, false)
                {
                    return false;
                }
            }
            return n1 == 0; // false if any remaining fields discarded
        }
        (IDL_CON_variant, IDL_CON_variant) => {
            let n1 = leb128_decode(&mut tb1);
            let mut n2 = leb128_decode(&mut tb2);
            for _ in 0..n1 {
                if n2 == 0 {
                    return false;
                };
                let tag1 = leb128_decode(&mut tb1);
                let t11 = sleb128_decode(&mut tb1);
                let mut tag2: u32;
                let mut t21: i32;
                loop {
                    tag2 = leb128_decode(&mut tb2);
                    t21 = sleb128_decode(&mut tb2);
                    n2 -= 1;
                    if variance == TypeVariance::Invariance || !(tag2 < tag1 && n2 > 0) {
                        break;
                    }
                }
                if tag1 != tag2 {
                    return false;
                }
                if !memory_compatible(rel, variance, typtbl1, typtbl2, end1, end2, t11, t21, false)
                {
                    return false;
                }
            }
            variance != TypeVariance::Invariance || n2 == 0
        }
        (IDL_CON_service, IDL_CON_service) => {
            let mut n1 = leb128_decode(&mut tb1);
            let n2 = leb128_decode(&mut tb2);
            for _ in 0..n2 {
                if n1 == 0 {
                    return false;
                };
                let (len2, p2) = leb128_decode_ptr(&mut tb2);
                Buf::advance(&mut tb2, len2 as usize);
                let t21 = sleb128_decode(&mut tb2);
                let mut len1: u32;
                let mut p1: *mut u8;
                let mut t11: i32;
                let mut cmp: i32;
                loop {
                    (len1, p1) = leb128_decode_ptr(&mut tb1);
                    Buf::advance(&mut tb1, len1 as usize);
                    t11 = sleb128_decode(&mut tb1);
                    n1 -= 1;
                    cmp = utf8_cmp(len1 as usize, p1, len2 as usize, p2);
                    if variance != TypeVariance::Invariance && cmp < 0 && n1 > 0 {
                        continue;
                    };
                    break;
                }
                if cmp != 0 {
                    return false;
                };
                if !memory_compatible(rel, variance, typtbl1, typtbl2, end1, end2, t11, t21, false)
                {
                    return false;
                }
            }
            variance != TypeVariance::Invariance || n1 == 0
        }
        // default
        (_, _) => false,
    }
}

// TODO: consider storing fixed args typtbl1...end2 in `rel` to use less stack.
pub(crate) unsafe fn sub(
    rel: &BitRel,
    p: bool,
    typtbl1: *mut *mut u8,
    typtbl2: *mut *mut u8,
    end1: *mut u8,
    end2: *mut u8,
    t1: i32,
    t2: i32,
) -> bool {
    if t1 >= 0 && t2 >= 0 {
        let t1 = t1 as usize;
        let t2 = t2 as usize;
        if rel.visited(p, t1, t2) {
            // visited? (bit 0)
            // return assumed or determined result
            return rel.related(p, t1, t2);
        };
        // cache and continue
        rel.visit(p, t1, t2); // mark visited
        rel.assume(p, t1, t2); // assume t1 <:/:> t2 true
    };

    /* primitives reflexive */
    if is_primitive_type(CompatibilityMode::PureCandid, t1)
        && is_primitive_type(CompatibilityMode::PureCandid, t2)
        && t1 == t2
    {
        return true;
    }

    // unfold t1, if necessary
    let mut tb1 = Buf {
        ptr: if t1 < 0 {
            end1
        } else {
            *typtbl1.add(t1 as usize)
        },
        end: end1,
    };

    let u1 = if t1 >= 0 {
        sleb128_decode(&mut tb1)
    } else {
        t1
    };

    // unfold t2, if necessary
    let mut tb2 = Buf {
        ptr: if t2 < 0 {
            end2
        } else {
            *typtbl2.add(t2 as usize)
        },
        end: end2,
    };

    let u2 = if t2 >= 0 {
        sleb128_decode(&mut tb2)
    } else {
        t2
    };

    // NB we use a trivial labelled loop so we can factor out the common failure continuation.
    // exit either via 'return true' or 'break 'return_false' to memoize the negative result
    'return_false: loop {
        match (u1, u2) {
            (_, IDL_CON_alias) | (IDL_CON_alias, _) => idl_trap_with("sub: unexpected alias"),
            (_, IDL_PRIM_reserved)
            | (IDL_PRIM_empty, _)
            | (IDL_PRIM_nat, IDL_PRIM_int)
            | (_, IDL_CON_opt) => return true, // apparently, this is admissable
            (IDL_CON_vec, IDL_CON_vec) => {
                let t11 = sleb128_decode(&mut tb1);
                let t21 = sleb128_decode(&mut tb2);
                if sub(rel, p, typtbl1, typtbl2, end1, end2, t11, t21) {
                    return true;
                } else {
                    break 'return_false;
                }
            }
            (IDL_CON_func, IDL_CON_func) => {
                // contra in domain
                let in1 = leb128_decode(&mut tb1);
                let mut in2 = leb128_decode(&mut tb2);
                for _ in 0..in1 {
                    let t11 = sleb128_decode(&mut tb1);
                    if in2 == 0 {
                        if !is_null_opt_reserved(typtbl1, end1, t11) {
                            break 'return_false;
                        }
                    } else {
                        let t21 = sleb128_decode(&mut tb2);
                        in2 -= 1;
                        // NB: invert p and args!
                        if !sub(rel, !p, typtbl2, typtbl1, end2, end1, t21, t11) {
                            break 'return_false;
                        }
                    }
                }
                while in2 > 0 {
                    let _ = sleb128_decode(&mut tb2);
                    in2 -= 1;
                }
                // co in range
                let mut out1 = leb128_decode(&mut tb1);
                let out2 = leb128_decode(&mut tb2);
                for _ in 0..out2 {
                    let t21 = sleb128_decode(&mut tb2);
                    if out1 == 0 {
                        if !is_null_opt_reserved(typtbl2, end2, t21) {
                            break 'return_false;
                        }
                    } else {
                        let t11 = sleb128_decode(&mut tb1);
                        out1 -= 1;
                        if !sub(rel, p, typtbl1, typtbl2, end1, end2, t11, t21) {
                            break 'return_false;
                        }
                    }
                }
                while out1 > 0 {
                    let _ = sleb128_decode(&mut tb1);
                    out1 -= 1;
                }
                // check annotations (that we care about)
                // TODO: more generally, we would check equality of 256-bit bit-vectors,
                // but validity ensures each entry is 1, 2 or 3 (for now)
                // c.f. https://github.com/dfinity/candid/issues/318
                let mut a11 = false;
                let mut a12 = false;
                let mut a13 = false;
                for _ in 0..leb128_decode(&mut tb1) {
                    match read_byte(&mut tb1) {
                        1 => a11 = true,
                        2 => a12 = true,
                        3 => a13 = true,
                        _ => {}
                    }
                }
                let mut a21 = false;
                let mut a22 = false;
                let mut a23 = false;
                for _ in 0..leb128_decode(&mut tb2) {
                    match read_byte(&mut tb2) {
                        1 => a21 = true,
                        2 => a22 = true,
                        3 => a23 = true,
                        _ => {}
                    }
                }
                if (a11 == a21) && (a12 == a22) && (a13 == a23) {
                    return true;
                } else {
                    break 'return_false;
                }
            }
            (IDL_CON_record, IDL_CON_record) => {
                let mut n1 = leb128_decode(&mut tb1);
                let n2 = leb128_decode(&mut tb2);
                let mut tag1 = 0;
                let mut t11 = 0;
                let mut advance = true;
                for _ in 0..n2 {
                    let tag2 = leb128_decode(&mut tb2);
                    let t21 = sleb128_decode(&mut tb2);
                    if n1 == 0 {
                        // check all remaining fields optional
                        if !is_null_opt_reserved(typtbl2, end2, t21) {
                            break 'return_false;
                        }
                        continue;
                    };
                    if advance {
                        loop {
                            tag1 = leb128_decode(&mut tb1);
                            t11 = sleb128_decode(&mut tb1);
                            n1 -= 1;
                            if !(tag1 < tag2 && n1 > 0) {
                                break;
                            }
                        }
                    };
                    if tag1 > tag2 {
                        if !is_null_opt_reserved(typtbl2, end2, t21) {
                            // missing, non_opt field
                            break 'return_false;
                        }
                        advance = false; // reconsider this field in next round
                        continue;
                    };
                    if !sub(rel, p, typtbl1, typtbl2, end1, end2, t11, t21) {
                        break 'return_false;
                    }
                    advance = true;
                }
                return true;
            }
            (IDL_CON_variant, IDL_CON_variant) => {
                let n1 = leb128_decode(&mut tb1);
                let mut n2 = leb128_decode(&mut tb2);
                for _ in 0..n1 {
                    if n2 == 0 {
                        break 'return_false;
                    };
                    let tag1 = leb128_decode(&mut tb1);
                    let t11 = sleb128_decode(&mut tb1);
                    let mut tag2: u32;
                    let mut t21: i32;
                    loop {
                        tag2 = leb128_decode(&mut tb2);
                        t21 = sleb128_decode(&mut tb2);
                        n2 -= 1;
                        if !(tag2 < tag1 && n2 > 0) {
                            break;
                        }
                    }
                    if tag1 != tag2 {
                        break 'return_false;
                    };
                    if !sub(rel, p, typtbl1, typtbl2, end1, end2, t11, t21) {
                        break 'return_false;
                    }
                }
                return true;
            }
            (IDL_CON_service, IDL_CON_service) => {
                let mut n1 = leb128_decode(&mut tb1);
                let n2 = leb128_decode(&mut tb2);
                for _ in 0..n2 {
                    if n1 == 0 {
                        break 'return_false;
                    };
                    let (len2, p2) = leb128_decode_ptr(&mut tb2);
                    Buf::advance(&mut tb2, len2 as usize);
                    let t21 = sleb128_decode(&mut tb2);
                    let mut len1: u32;
                    let mut p1: *mut u8;
                    let mut t11: i32;
                    let mut cmp: i32;
                    loop {
                        (len1, p1) = leb128_decode_ptr(&mut tb1);
                        Buf::advance(&mut tb1, len1 as usize);
                        t11 = sleb128_decode(&mut tb1);
                        n1 -= 1;
                        cmp = utf8_cmp(len1 as usize, p1, len2 as usize, p2);
                        if cmp < 0 && n1 > 0 {
                            continue;
                        };
                        break;
                    }
                    if cmp != 0 {
                        break 'return_false;
                    };
                    if !sub(rel, p, typtbl1, typtbl2, end1, end2, t11, t21) {
                        break 'return_false;
                    }
                }
                return true;
            }
            // default
            (_, _) => {
                break 'return_false;
            }
        }
    }
    // remember negative result ...
    if t1 >= 0 && t2 >= 0 {
        rel.disprove(p, t1 as usize, t2 as usize);
    }
    // .. only then return false
    return false;
}

#[unsafe(no_mangle)]
unsafe extern "C" fn idl_sub_buf_words(typtbl_size1: usize, typtbl_size2: usize) -> usize {
    BitRel::words(typtbl_size1, typtbl_size2)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn idl_sub_buf_init(
    rel_buf: *mut usize,
    typtbl_size1: usize,
    typtbl_size2: usize,
) {
    let rel = BitRel {
        ptr: rel_buf,
        end: rel_buf.add(idl_sub_buf_words(typtbl_size1, typtbl_size2) as usize),
        size1: typtbl_size1,
        size2: typtbl_size2,
    };
    rel.init();
}

#[enhanced_orthogonal_persistence]
#[ic_mem_fn]
unsafe fn idl_alloc_typtbl<M: Memory>(
    mem: &mut M,
    candid_data: Value,
    type_offsets: Value,
    typtbl_out: *mut *mut *mut u8,
    typtbl_end_out: *mut *mut u8,
    typtbl_size_out: *mut usize,
) {
    use crate::persistence::compatibility::TypeDescriptor;

    let mut type_descriptor = TypeDescriptor::new(candid_data, type_offsets);
    *typtbl_out = type_descriptor.build_type_table(mem);
    *typtbl_end_out = type_descriptor.type_table_end();
    *typtbl_size_out = type_descriptor.type_count();
}

#[unsafe(no_mangle)]
unsafe extern "C" fn idl_sub(
    rel_buf: *mut usize, // a buffer with at least 2 * typtbl_size1 * typtbl_size2 bits
    typtbl1: *mut *mut u8,
    typtbl2: *mut *mut u8,
    typtbl_end1: *mut u8,
    typtbl_end2: *mut u8,
    typtbl_size1: usize,
    typtbl_size2: usize,
    t1: i32,
    t2: i32,
) -> bool {
    debug_assert!(rel_buf != 0 as *mut usize);
    debug_assert!(typtbl1 != 0 as *mut *mut u8);
    debug_assert!(typtbl2 != 0 as *mut *mut u8);
    debug_assert!(typtbl_end1 != 0 as *mut u8);
    debug_assert!(typtbl_end2 != 0 as *mut u8);

    let rel = BitRel {
        ptr: rel_buf,
        end: rel_buf.add(idl_sub_buf_words(typtbl_size1, typtbl_size2) as usize),
        size1: typtbl_size1,
        size2: typtbl_size2,
    };
    debug_assert!(t1 < typtbl_size1 as i32 && t2 < typtbl_size2 as i32);
    return sub(
        &rel,
        true,
        typtbl1,
        typtbl2,
        typtbl_end1,
        typtbl_end2,
        t1,
        t2,
    );
}
