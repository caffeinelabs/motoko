//! Constants.
//!
//! **Format** constants are part of stored or exchanged bytes: changing one needs a new format version.
//! **Policy** constants may change between crate versions without touching stored bytes. Every size, offset
//! and count is a `u64`.

// ---- The one B-tree and index entries ----

/// Format. Bytes of a node.
pub const NODE_SIZE: u64 = 4_080;
/// Format. Flags bit 0: leaf node.
pub const FLAG_LEAF: u16 = 1;
/// Format. Flags bit 1: index-form node. Index nodes carry exactly 3 (leaf) or 2 (internal).
pub const FLAG_INDEX: u16 = 2;
/// Format. Frames of a path, which bounds a tree's depth.
pub const PATH_STACK: u64 = 64;
/// Format. An area block; a node slot in areas; one main-memory page-fee page.
pub const BLOCK: u64 = 4_096;
/// Format. A host page, an image block: 16 blocks.
pub const PAGE: u64 = 65_536;
/// Format. Blocks per page.
pub const BLOCKS_PER_PAGE: u64 = PAGE / BLOCK;
/// Format. Smallest index key-area width W, a multiple of 8.
pub const W_MIN: u64 = 8;
/// Format. Largest index key-area width W, a multiple of 8.
pub const W_MAX: u64 = 1_016;
/// Format. Value-part sizes V an index-form tree may use.
pub const V_CHOICES: [u64; 7] = [0, 8, 16, 32, 64, 128, 256];
/// Format. W + 8 + V never exceeds it, so every node has room for at least 3 entries.
pub const PART_MAX: u64 = 1_024;
/// Format. Entry field bit 63: the key is longer than the key area.
pub const F_OVF: u64 = 1 << 63;
/// Format. Entry field bit 62: a count entry.
pub const F_COUNT: u64 = 1 << 62;
/// Format. A count entry's payload, bits 0–61.
pub const PAYLOAD: u64 = (1 << 62) - 1;
/// Format. A row entry's payload, bits 0–60; bit 61 of a row entry is zero.
pub const RID_PAYLOAD: u64 = (1 << 61) - 1;
/// Format. Index-form node rules, in the layout word.
pub const FORMAT_NODE: u16 = 1;

// ---- Rows, groups and images ----

/// Format. Positions per group; rid = group · 4,096 + position.
pub const GROUP_SLOTS: u64 = 4_096;
/// Format. Largest group number, so every rid is below 2^61 and fits `RID_PAYLOAD`.
pub const GROUP_MAX: u64 = (1 << 49) - 1;
/// Policy. Inline bytes that fill the open group: the insert that would pass them seals it first.
pub const GROUP_BYTES: u64 = 1_048_576;
/// Policy. Medium out-of-line bytes that fill the open group.
pub const GROUP_OOL_BYTES: u64 = 4_194_304;
/// Policy. Out-of-line values (medium and huge) that fill the open group.
pub const GROUP_LARGE: u64 = 1_024;
/// Format. Buffer blocks per group (the buffer descriptor's capacity).
pub const BUFFER_BLOCKS_MAX: u64 = 240;
/// Policy. A longer row image is stored as a long value.
pub const ROW_INLINE_MAX: u64 = 3_072;
/// Format. A value's VE longer than this is out of line when the crate writes it.
pub const LONG_VALUE: u64 = 2_048;
/// Format. An out-of-line value longer than this is a long value of its own when the crate writes it.
pub const HUGE_VALUE: u64 = 65_536;
/// Format. `[u8; N]` with N at most this is form `Fixed`, else `Binary`.
pub const FIXED_MAX: u64 = 1_016;
/// Format. Block entries a group image record holds inline; a longer image keeps a radix.
pub const INLINE_ENTRIES: u64 = 8;
/// Format. Bytes of a rewrite's work block area, where its work space begins.
pub const WORK_AREA: u64 = 3_432;
/// Format. The most bytes from a tail block's start to its first row.
pub const HOT_MAX: u64 = 2_048;
/// Format. Bytes a big-form total's room in the totals space holds at least.
pub const BIGSUM_INLINE: u64 = 64;
/// Policy. Dead positions (25%) that queue a compaction.
pub const COMPACT_DEAD: u64 = 1_024;
/// Policy. Patch blocks of one leaf that queue a compaction.
pub const PATCH_FOLD_BLOCKS: u64 = 4;
/// Format. Patch unit records per patch directory block; record 255 links the next directory block.
pub const PATCH_UNITS_PER_DIR: u64 = 254;
/// Format. Areas per table.
pub const AREAS: u64 = 16;
/// Format. u64 references per radix block.
pub const RADIX_FAN: u64 = 512;
/// Format. The key encoding's version.
pub const FORMAT_KEY: u16 = 1;
/// Format. The value encoding's (VE) version.
pub const FORMAT_VALUE: u16 = 1;
/// Format. The highest image header version this crate writes and reads. Version 2: a run image stores the
/// bytes all its keys share once, in its header, and each entry holds only the rest of its key. An image
/// that uses nothing version 2 added (every image but a run with a shared prefix) is written as version 1,
/// which a reader of version 1 reads as before.
pub const FORMAT_IMAGE: u16 = 2;
/// Format. The version of an image that uses nothing version 2 added.
pub const IMAGE_V1: u16 = 1;
/// Format. The version of the table root, buffer, group page, patch, run-list, queue, drain and load-session
/// layouts. Version 2: a run's probe record holds its filter's lines in 4 bytes and its shared prefix in
/// the 4 beside them, where version 1 held the lines in 8. Version 3: those 4 bytes hold the length of the
/// prefix the run's header stores (image version 2), where version 2 held the bytes the run's fences share.
/// Version 4: an index tree's key log keeps its records in chunks it allocates and frees, under a head block at
/// ordinal 0, where version 3 appended them through blocks with a header of their own. Version 5: an integer
/// leaf's running record holds its total over every live row, in the hot state or the totals space, where
/// version 4 held the sum of the buffered rows alone. Version 6: sealed rows take updates (a group page's
/// patch directories, a rewrite's state, pending page and work space, the compaction queue), the schema has
/// versions (the hot state's running-record list written for the current one), and the table keeps its
/// buffered row count. Version 7: the table keeps no statistics beside the integer leaves' totals (no block
/// statistics or group extremes in buffer blocks, hot states and descriptors, no statistics area on group
/// pages, no statistics section in group images, running records for integer leaves only), and an index
/// declaration of the schema image is 40 bytes, without the rendered flag of each part. Version 8: no leaf is
/// a variant, so group images have no dictionary forms or segments, a leaf directory entry's bytes 14–15 are
/// zero, a rewrite's plan is 80 bytes and its state block holds no dictionary state or working-state blocks.
/// Version 9: no leaf nests in another, so a leaf record of the schema image is 28 bytes, without its depth,
/// parent, case and case count, and the schema's type pool has no variant, list, recursive, table or set
/// types. Version 10: the table root holds the table's shape at 2,464, derived when the schema image is written,
/// which an open reads instead of deriving it from the image again. Version 11: a buffer descriptor (at 48) and
/// a group page (at 656) name their group's extremes block (`table::extremes`). Version 12: the write that fills
/// a group seals it, and a compaction or column release rewrites a group whole in one call, so no group waits
/// frozen and no rewrite spans calls: the root's frozen queue (64–83) and job slots 2–7, a buffer descriptor's
/// bytes 24–31 and 56–63 and a group page's bytes 640–655 are zero, no rewrite state block exists, and the drain
/// holds only removed indexes' trees and key logs. An index directory of another version is refused with a
/// header fault, so no count reads a record at another layout's offsets or meaning.
pub const FORMAT_TABLE: u16 = 12;

// ---- Indexes ----

/// Format. The primary index and up to 63 secondary indexes.
pub const INDEXES_MAX: u64 = 64;

// ---- Images ----

/// Format. The longest image of any kind.
pub const IMAGE_MAX: u64 = 1 << 60;

// ---- Derived values ----

/// Whether `v` is one of `V_CHOICES`.
pub const fn v_choice(v: u64) -> bool {
    let mut i = 0;
    while i < V_CHOICES.len() {
        // `i` is below the array's length.
        #[allow(clippy::indexing_slicing)]
        if V_CHOICES[i] == v {
            return true;
        }
        // i < V_CHOICES.len().
        i = i.wrapping_add(1);
    }
    false
}

/// `SEAL_SCRATCH(L, X, S)` = 64·L + 32·(X + S) + 24,576 bytes for L leaves, X out-of-line values and S
/// segments; `None` on overflow.
pub const fn seal_scratch(leaves: u64, ool_values: u64, segments: u64) -> Option<u64> {
    let Some(l) = leaves.checked_mul(64) else { return None };
    let Some(xs) = ool_values.checked_add(segments) else { return None };
    let Some(xs) = xs.checked_mul(32) else { return None };
    let Some(t) = l.checked_add(xs) else { return None };
    t.checked_add(24_576)
}

/// Magic numbers, stored little-endian as u32: the four ASCII bytes in order.
pub mod magic {
    const fn m(s: &[u8; 4]) -> u32 {
        u32::from_le_bytes(*s)
    }
    /// Image.
    pub const NTIM: u32 = m(b"NTIM");
    /// Table root.
    pub const NTTR: u32 = m(b"NTTR");
    /// Buffer block.
    pub const NTBF: u32 = m(b"NTBF");
    /// Buffer descriptor.
    pub const NTBD: u32 = m(b"NTBD");
    /// Group page.
    pub const NTGP: u32 = m(b"NTGP");
    /// Patch directory.
    pub const NTPD: u32 = m(b"NTPD");
    /// Patch block.
    pub const NTPB: u32 = m(b"NTPB");
    /// Long-value list.
    pub const NTLL: u32 = m(b"NTLL");
    /// Index directory.
    pub const NTIX: u32 = m(b"NTIX");
    /// Key log.
    pub const NTKL: u32 = m(b"NTKL");
    /// Queue block.
    pub const NTQB: u32 = m(b"NTQB");
    /// Work space block.
    pub const NTSX: u32 = m(b"NTSX");
    /// Totals block.
    pub const NTTS: u32 = m(b"NTTS");
    /// Extremes block.
    pub const NTEX: u32 = m(b"NTEX");
    /// Park block.
    pub const NTPK: u32 = m(b"NTPK");
}

// ---- Compile-time checks of the stated relations ----

const _: () = assert!((GROUP_MAX + 1) * GROUP_SLOTS == 1 << 61);
const _: () = assert!(GROUP_MAX * GROUP_SLOTS + (GROUP_SLOTS - 1) <= RID_PAYLOAD);
const _: () = assert!(W_MAX + 8 == PART_MAX);
const _: () = assert!(BLOCKS_PER_PAGE == 16);
const _: () = assert!(magic::NTIM == 0x4D49_544E);
const _: () = assert!(matches!(seal_scratch(6, 0, 12), Some(25_344)));
