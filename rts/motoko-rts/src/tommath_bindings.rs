#![allow(unused, non_camel_case_types)]

use motoko_rts_macros::enhanced_orthogonal_persistence;

#[enhanced_orthogonal_persistence]
include!("../../_build/wasm64/tommath_bindings.rs");
