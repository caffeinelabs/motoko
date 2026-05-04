//! Implements Motoko runtime system

// See: https://doc.rust-lang.org/nightly/edition-guide/rust-2024/static-mut-references.html
#![allow(static_mut_refs)]
#![no_std]
#![feature(
    arbitrary_self_types,
    proc_macro_hygiene,
    // // We do not need simd but this flag enables `core::arch:wasm64`.
    // // See https://github.com/rust-lang/rust/issues/90599
    simd_wasm64,
    arbitrary_self_types_pointers
)]
#![allow(internal_features)]

// c.f. https://os.phil-opp.com/heap-allocation/#dynamic-memory
extern crate alloc;
#[cfg(feature = "ic")]
pub mod allocator;

pub mod stable_option;

#[macro_use]
mod print;

#[cfg(debug_assertions)]
pub mod debug;

mod barriers;
pub mod bigint;
pub mod bitrel;
#[cfg(feature = "ic")]
mod blob_iter;
pub mod buf;
mod char;
pub mod constants;
pub mod continuation_table;
#[cfg(feature = "ic")]
mod float;
pub mod gc;
#[cfg(feature = "ic")]
mod idl;
pub mod leb128;
mod libc_declarations;
pub mod mem_utils;
pub mod memory;
#[cfg(feature = "ic")]
#[enhanced_orthogonal_persistence]
pub mod persistence;
pub mod principal_id;
#[cfg(feature = "ic")]
pub mod region;
#[enhanced_orthogonal_persistence]
pub mod stabilization;
pub mod stable_mem;
mod static_checks;
#[classical_persistence]
pub mod stream;
pub mod text;
pub mod text_iter;
mod tommath_bindings;
pub mod types;
pub mod utf8;
mod visitor;

use motoko_rts_macros::*;

#[ic_mem_fn(ic_only)]
fn version<M: memory::Memory>(mem: &mut M) -> types::Value {
    unsafe { text::text_of_str(mem, "0.1") }
}

#[non_incremental_gc]
#[ic_mem_fn(ic_only)]
fn alloc_words<M: memory::Memory>(mem: &mut M, n: types::Words<usize>) -> types::Value {
    unsafe { mem.alloc_words(n) }
}

#[incremental_gc]
#[ic_mem_fn(ic_only)]
unsafe fn alloc_words<M: memory::Memory>(mem: &mut M, n: types::Words<usize>) -> types::Value {
    crate::gc::incremental::get_partitioned_heap().allocate(mem, n)
}

extern "C" {
    fn rts_trap(msg: *const u8, len: u32) -> !;
}

pub(crate) fn trap_with_prefix(prefix: &str, msg: &str) -> ! {
    // Rust currently doesn't support stack-allocated dynamically-sized arrays or alloca, so we
    // have a max bound to the message size here.
    //
    // Messages longer than this are cut off.
    //
    // Relevant RFC: https://github.com/rust-lang/rust/issues/48055
    const BUF_LEN: usize = 512;

    let mut c_str = [0u8; BUF_LEN];
    let mut b_idx = 0;

    for b in prefix.as_bytes() {
        if b_idx == BUF_LEN {
            break;
        }
        c_str[b_idx] = *b;
        b_idx += 1;
    }

    for b in msg.as_bytes() {
        if b_idx == BUF_LEN {
            break;
        }
        c_str[b_idx] = *b;
        b_idx += 1;
    }

    assert!(b_idx <= u32::MAX as usize);
    unsafe { rts_trap(c_str.as_ptr(), b_idx as u32) }
}

pub(crate) fn idl_trap_with(msg: &str) -> ! {
    trap_with_prefix("IDL error: ", msg)
}

pub(crate) fn rts_trap_with(msg: &str) -> ! {
    trap_with_prefix("RTS error: ", msg)
}

#[cfg(feature = "ic")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        let message = info.message();
        if let Some(location) = info.location() {
            println!(
                1000,
                "panic occurred in file '{}' at line {}: {}",
                location.file(),
                location.line(),
                message,
            );
        } else {
            println!(1000, "RTS panic: {message}");
        }

        rts_trap_with("RTS panicked");
    }
}
