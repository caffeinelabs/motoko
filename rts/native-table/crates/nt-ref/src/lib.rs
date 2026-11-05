//! Tests only: the reference the crate's typed tree is measured and checked against.
//!
//! - `map`: an unchecked transcription of the `PBTreeMap` that Rust EOP (a Rust library for upgrades that keep
//!   the heap in place) ships today: search, insert with split,
//!   remove with merge and rotation, and the cursor's seek and next, over nodes in raw memory reached by
//!   address, exactly as that map does it.
//! - `typed`: `RawTypedNodes<K, V>`, the crate's `NodeMem` over the same nodes, implemented the way Rust
//!   EOP's map will implement it once it runs on the crate's tree (today's raw field accessors), with
//!   `LookupProbe` and `PartsPending`; and `CrateMap`, a `PBTreeMap`-shaped front end over the crate's tree.
//! - `mem`: addresses, the raw field accessors and the node and box arenas both use.
//! - `walk`: tree digests and invariant checks for tests.
//!
//! The crate's typed instance with `CHECKED` false must cost what today's map costs; `cost_vs_ref`
//! (`scripts/cost_vs_ref.sh`) measures both side by side, and `typed_alias_miri` (`scripts/miri.sh`) checks
//! that the values the crate's cursor hands out stay valid under Miri.
//!
//! `unsafe` lives here and in Rust EOP's host, never in the shared crate.

pub mod map;
pub mod mem;
pub mod part;
pub mod typed;
pub mod walk;

pub use map::RefMap;
pub use mem::Region;
pub use part::{PKey, PString, Part, Persistent};
pub use typed::{CrateMap, LookupProbe, MapShape, MapShapeChecked, PartsPending, RawTypedNodes};
pub use walk::TestKey;
