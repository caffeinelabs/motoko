//! Running records in the hot state: one per integer leaf that has room there, in the order of the
//! running-record leaf list (u16 ordinals, ascending, from hot-state offset 192, padded to 8), each 8-aligned.
//! Each holds the leaf's total over every live row: its non-null count and the exact sum of its values.
//!
//! | Record | Leaves | Fields |
//! |---|---|---|
//! | `R_INT` | integers of at most 64 bits | non-null u64; sum i128 |
//! | `R_NUM` | Nat, Int | non-null u64; sum i128 of values within 64 bits; big-form sum: length u32, room u32 |
//! | `R_WIDE` | u128, i128 | non-null u64; sum i192 |
//!
//! Counts are kept modulo 2^64. The sums are exact: a table holds fewer than 2^61 rows, so an i128 holds any
//! sum of values within 64 bits and an i192 any sum of u128 or i128 values. A leaf whose total has no room in
//! the hot state keeps it in the totals space (`table::totals`).

use crate::fault::Fault;
use crate::mem::{put, put_u64, rd_array, rd_u64};

/// Offset of the running-record leaf list in the hot state's block.
pub const LIST_AT: u64 = 192;

/// The running record an integer leaf gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RKind {
    Int,
    Num,
    Wide,
}

impl RKind {
    /// Bytes of the record.
    pub fn bytes(self) -> u64 {
        match self {
            RKind::Int => 24,
            RKind::Num | RKind::Wide => 32,
        }
    }
}

/// The non-null word.
pub const R_NONNULL: u64 = 0;
/// The sum.
pub const R_SUM: u64 = 8;
/// `R_NUM`: the big-form sum's length and its room's offset in the totals space (0: none).
pub const R_BIG_LEN: u64 = 24;
pub const R_BIG_ROOM: u64 = 28;

/// Adds `delta` to the u64 at `off`, modulo 2^64.
pub fn add_count(b: &mut [u8], off: u64, delta: i64) -> Result<(), Fault> {
    let v = rd_u64(b, off)?;
    put_u64(b, off, v.wrapping_add_signed(delta))
}

/// Adds `delta` to the i128 at `off`, modulo 2^128.
pub fn add_sum(b: &mut [u8], off: u64, delta: i128) -> Result<(), Fault> {
    let v = i128::from_le_bytes(rd_array::<16>(b, off)?);
    put(b, off, &v.wrapping_add(delta).to_le_bytes())
}
