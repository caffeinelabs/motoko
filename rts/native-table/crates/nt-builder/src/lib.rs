//! The off-chain run builder: signed items in (key, rid) order become one run image, the read-only index
//! image the native-table crate reads (`native_table::index::Run`). It never runs inside a canister: the
//! runtime only reads uploaded images.
//!
//! Like the crate it builds for, it never allocates, never traps and never panics.
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

pub mod builder;
pub mod bulk;
pub mod plan;

pub use builder::{build_run, plan_run, Item, RunInput, RunParams, RunPlan};
pub use bulk::{BulkState, Level, MemRunSink, Plan, RunGeometry, RunSink};
