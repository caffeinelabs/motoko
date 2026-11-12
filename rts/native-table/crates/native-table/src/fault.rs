//! Errors, and the texts hosts trap with.
//!
//! The crate never traps. Every fallible function returns `Err(Fault)`; the host decides: a canister host
//! traps, so the message rolls back.

/// What invariant stored bytes break.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corruption {
    Header,
    Count,
    Flags,
    Layout,
    Bounds,
    Order,
    Encoding,
    Leaf,
    Range,
    Link,
    Depth,
}

/// Which image a corruption was met in. `Runtime`: a structure the runtime wrote (a crate bug).
///
/// Nothing constructs `Run` (an uploaded index segment) or `Loading` (an image being uploaded). They stay
/// because a variant with a byte before its `u64` keeps this enum's tag at one byte. Without them the tag
/// widens to eight bytes, which changes the layout of `Fault` and of every `Result<_, Fault>`, and the
/// compiled query loops (minimum and maximum reads, counts) run slower.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageId {
    Runtime,
    Group(u64),
    Run { index: u8, start_seq: u64 },
    Loading { slot: u8, image: u64 },
}

/// Where a corruption was met: the image, the kind of break and the byte offset in that image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CorruptAt {
    pub image: ImageId,
    pub kind: Corruption,
    pub offset: u64,
}

/// A refused schema image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemaError {
    /// A change of a column or leaf the table holds, a new leaf that is not nullable, a new index, or a
    /// version that does not follow the table's.
    ColumnChanged,
    /// A change of a key leaf or of the primary index.
    KeyChanged,
    /// A dropped leaf or column that is live again.
    RestoredDropped,
    /// A drop marked other than by the dropped kind: a dropped version, or a new leaf of the dropped kind.
    NewDropped,
    TooManyIndexes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The host does not know this reference, or a pair request names one twice.
    BadRef,
    /// Bytes break an invariant: runtime-written (`ImageId::Runtime`) or of a group image.
    Corrupt(CorruptAt),
    /// A search past the path stack's 64 levels (`PATH_STACK`).
    PathOverflow,
    /// A cell its leaf's type refuses (never refused for its size).
    BadValue { leaf: u32 },
    /// A refused schema image.
    Schema(SchemaError),
    /// Caller scratch below the stated minimum.
    Scratch { need: u64 },
    /// A token that is malformed, names another table, or names an index whose key spec changed.
    BadToken,
    /// A write that would give a unique index's value, with no null part, to a second row.
    Unique,
}

impl Fault {
    /// One fixed ASCII text per variant, at most 60 bytes. Inlined, so that a host trapping with the text
    /// of a fault the optimizer can see needs no stack slot for the fault.
    #[inline]
    pub fn text(self) -> &'static str {
        match self {
            Fault::Corrupt(CorruptAt { image: ImageId::Runtime, .. }) => "native table: corrupt structure",
            Fault::Corrupt(_) => "table image corrupt",
            Fault::BadRef => "native table: bad reference",
            Fault::PathOverflow => "native table: tree deeper than its path stack",
            Fault::BadValue { .. } => "native table: value refused by its column",
            Fault::Schema(_) => "native table: schema change refused",
            Fault::Scratch { .. } => "native table: scratch too small",
            Fault::BadToken => "native table: bad or stale token",
            Fault::Unique => "native table: a unique index already has this value",
        }
    }
}

/// A `Corrupt` fault at `offset` of `image`.
pub const fn corrupt(image: ImageId, kind: Corruption, offset: u64) -> Fault {
    Fault::Corrupt(CorruptAt { image, kind, offset })
}

/// A `Corrupt(Runtime, kind)` fault at `offset`: a structure the runtime wrote breaks an invariant.
pub const fn rt(kind: Corruption, offset: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, offset)
}

/// A `Corrupt(Runtime, Bounds)` fault at `offset`: a range outside a lent slice.
pub const fn bounds(offset: u64) -> Fault {
    corrupt(ImageId::Runtime, Corruption::Bounds, offset)
}
