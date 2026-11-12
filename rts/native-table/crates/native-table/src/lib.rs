//! The shared table crate: the one B-tree, the key and value encodings, columnar row groups with
//! running totals, and indexes, used by Motoko's runtime.
//!
//! The crate never allocates, never traps and never panics: every fallible function returns
//! `Result<_, Fault>`, and every byte it reads or writes is reached through a host it is given.
#![no_std]
#![forbid(unsafe_code)]
#![deny(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::cast_possible_truncation
)]

pub mod area;
pub mod bytes;
pub mod consts;
pub mod fault;
pub mod hash;
pub mod host;
pub mod image;
pub mod index;
pub mod key;
pub mod mem;
pub mod radix;
pub mod schema;
pub mod stats;
pub mod table;
pub mod tree;
pub mod value;

pub use fault::{CorruptAt, Corruption, Fault, ImageId, SchemaError};
pub use host::{Budget, NodeMem, NodeRead, Pending, Progress, Scratch, Wake};
pub use mem::{PageHost, Region};

/// The version of each stored or exchanged format this crate writes. A runtime records the highest
/// version any of its tables has written, one u16 per format, and refuses a binary that reads a lower one.
pub mod format {
    pub use crate::consts::{FORMAT_IMAGE, FORMAT_KEY, FORMAT_NODE, FORMAT_TABLE, FORMAT_VALUE};
}
