//! Errors, and the texts hosts trap with.
//!
//! The crate never traps. Every fallible function returns `Err(Fault)`; the host decides: a canister host
//! traps, so the message rolls back, and the builder discards its output.

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

/// A refused schema change. Dropping an indexed column drops every index that covers it, so it is never
/// refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemaError {
    ColumnChanged,
    KeyChanged,
    RestoredDropped,
    NewDropped,
    TooManyIndexes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The host does not know this reference, or a pair request names one twice.
    BadRef,
    /// The host could not provide a page.
    OutOfMemory,
    /// Bytes break an invariant: runtime-written (`ImageId::Runtime`) or believed.
    Corrupt(CorruptAt),
    /// A 65th path frame.
    PathOverflow,
    /// A cell its leaf's type refuses (never refused for its size).
    BadValue { leaf: u32 },
    /// A refused schema change.
    Schema(SchemaError),
    /// Caller scratch below the stated minimum.
    Scratch { need: u64 },
    /// Refused while a load holds the table.
    Busy,
    /// A token that is malformed, names another table, or names an index whose key spec changed.
    BadToken,
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
            Fault::OutOfMemory => "native table: out of memory",
            Fault::PathOverflow => "native table: tree deeper than its path stack",
            Fault::BadValue { .. } => "native table: value refused by its column",
            Fault::Schema(_) => "native table: schema change refused",
            Fault::Scratch { .. } => "native table: scratch too small",
            Fault::Busy => "native table: busy",
            Fault::BadToken => "native table: bad or stale token",
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

/// Upload refusals are values, not faults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum RefusalCode {
    BadMagic = 1,
    BadVersion = 2,
    BadKind = 3,
    WrongTable = 4,
    SchemaChanged = 5,
    WrongImage = 6,
    WrongIndex = 7,
    BadSizes = 8,
    BlockMismatch = 9,
    CountMismatch = 10,
    PlanMismatch = 11,
    GroupSpace = 12,
    /// The structure check.
    BadStructure = 13,
}

/// A refused upload: the code, the image and the field it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub code: RefusalCode,
    pub image: u64,
    pub field: u64,
}
