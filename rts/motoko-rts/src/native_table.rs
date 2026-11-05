//! Tables kept by the native-table crate.

pub mod layout;
#[cfg(feature = "ic")]
mod row;
pub mod store;
#[cfg(feature = "ic")]
mod table;
