//! The table root block: the first block area 0 allocates, never moved.

use crate::consts::{magic, BLOCK, FORMAT_IMAGE, FORMAT_KEY, FORMAT_TABLE, FORMAT_VALUE};
use crate::fault::{rt, Corruption, Fault};
use crate::mem::{fill, put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u64, rd_u8, PageHost, Region};

pub const MAGIC: u64 = 0;
pub const FORMAT: u64 = 4;
pub const KIND: u64 = 6;
pub const FLAGS: u64 = 7;
pub const UID: u64 = 8;
pub const SCHEMA_HASH: u64 = 16;
pub const SCHEMA_VERSION: u64 = 24;
pub const SCHEMA_PAGE: u64 = 32;
pub const NEXT_GROUP: u64 = 40;
pub const OPEN_GROUP: u64 = 48;
pub const TAIL: u64 = 56;
pub const DIR_HEIGHT: u64 = 84;
pub const DIR_ROOT: u64 = 88;
/// The compaction queue's head and tail blocks.
pub const COMPACT_HEAD: u64 = 112;
pub const COMPACT_TAIL: u64 = 120;
/// The group to queue (group number + 1; 0: none): a group a write's 255th patched unit had to queue while it was
/// not queued, recorded here in the root the write changes anyway instead of pushed onto the compaction queue,
/// whose blocks the write would change besides; the next maintenance step pushes it first.
pub const GROUP_TO_QUEUE: u64 = 128;
/// Index directory block references 0–2.
pub const INDEX_DIRS: u64 = 928;
pub const FMT_IMAGE: u64 = 2_128;
pub const FMT_KEY: u64 = 2_130;
pub const FMT_VALUE: u64 = 2_132;
/// The maintenance state byte: bit 0 pending.
pub const STATE: u64 = 2_449;
/// The table's shape, what every call needs of the schema image, derived when the image is written
/// (`layout::Shape::store`, `layout::STORED_BYTES` bytes).
pub const SHAPE: u64 = 2_464;
/// Host words: never read or written by the crate.
pub const HOST_WORDS: u64 = 3_968;
const HOST_WORDS_END: u64 = 4_032;

/// Kind byte: a table, or a set (rows that hold only their key).
pub const KIND_TABLE: u8 = 1;
pub const KIND_SET: u8 = 2;

/// Writes every byte of a new root the crate owns, except the area records (160–927, written by
/// `area::new_root`): zeros, then the header fields; the host words are left as they are.
pub fn init<H: PageHost>(
    h: &mut H,
    tbl: u64,
    kind: u8,
    uid: u64,
    hash: u64,
    version: u32,
    schema_page: u64,
) -> Result<(), Fault> {
    let b = h.get_mut(Region::Block(tbl))?;
    fill(b, 0, 160, 0)?;
    fill(b, INDEX_DIRS, HOST_WORDS.wrapping_sub(INDEX_DIRS), 0)?;
    fill(b, HOST_WORDS_END, BLOCK.wrapping_sub(HOST_WORDS_END), 0)?;
    put_u32(b, MAGIC, magic::NTTR)?;
    put_u16(b, FORMAT, FORMAT_TABLE)?;
    put_u8(b, KIND, kind)?;
    put_u8(b, FLAGS, 0)?;
    put_u64(b, UID, uid)?;
    put_u64(b, SCHEMA_HASH, hash)?;
    put_u32(b, SCHEMA_VERSION, version)?;
    put_u64(b, SCHEMA_PAGE, schema_page)?;
    put_u16(b, FMT_IMAGE, FORMAT_IMAGE)?;
    put_u16(b, FMT_KEY, FORMAT_KEY)?;
    put_u16(b, FMT_VALUE, FORMAT_VALUE)
}

/// Checks the root's magic and format version.
pub fn check<H: PageHost + ?Sized>(h: &H, tbl: u64) -> Result<(), Fault> {
    let b = h.get(Region::Block(tbl))?;
    if rd_u32(b, MAGIC)? != magic::NTTR || rd_u16(b, FORMAT)? > FORMAT_TABLE {
        return Err(rt(Corruption::Header, tbl));
    }
    Ok(())
}

pub fn u64_at<H: PageHost + ?Sized>(h: &H, tbl: u64, off: u64) -> Result<u64, Fault> {
    rd_u64(h.get(Region::Block(tbl))?, off)
}

pub fn set_u64<H: PageHost>(h: &mut H, tbl: u64, off: u64, v: u64) -> Result<(), Fault> {
    put_u64(h.get_mut(Region::Block(tbl))?, off, v)
}

pub fn u32_at<H: PageHost + ?Sized>(h: &H, tbl: u64, off: u64) -> Result<u32, Fault> {
    rd_u32(h.get(Region::Block(tbl))?, off)
}

pub fn set_u32<H: PageHost>(h: &mut H, tbl: u64, off: u64, v: u32) -> Result<(), Fault> {
    put_u32(h.get_mut(Region::Block(tbl))?, off, v)
}

pub fn u8_at<H: PageHost + ?Sized>(h: &H, tbl: u64, off: u64) -> Result<u8, Fault> {
    rd_u8(h.get(Region::Block(tbl))?, off)
}

pub fn set_u8<H: PageHost>(h: &mut H, tbl: u64, off: u64, v: u8) -> Result<(), Fault> {
    put_u8(h.get_mut(Region::Block(tbl))?, off, v)
}

/// The group directory: a radix array whose entry g names group g's page, or, with bit 63 set, its buffer
/// descriptor.
pub fn directory<H: PageHost + ?Sized>(h: &H, tbl: u64) -> Result<crate::radix::Radix, Fault> {
    let b = h.get(Region::Block(tbl))?;
    Ok(crate::radix::Radix { root: rd_u64(b, DIR_ROOT)?, height: rd_u8(b, DIR_HEIGHT)? })
}

pub fn set_directory<H: PageHost>(h: &mut H, tbl: u64, r: crate::radix::Radix) -> Result<(), Fault> {
    let b = h.get_mut(Region::Block(tbl))?;
    put_u64(b, DIR_ROOT, r.root)?;
    put_u8(b, DIR_HEIGHT, r.height)
}

/// Sets directory entry g to `v` (0 clears it).
pub fn set_group<H: PageHost>(h: &mut H, tbl: u64, g: u64, v: u64) -> Result<(), Fault> {
    let mut d = directory(h, tbl)?;
    let before = d;
    d.set(h, tbl, g, v)?;
    if d != before {
        set_directory(h, tbl, d)?;
    }
    Ok(())
}

/// Directory entry g (0: none).
pub fn group<H: PageHost + ?Sized>(h: &H, tbl: u64, g: u64) -> Result<u64, Fault> {
    directory(h, tbl)?.get(h, g)
}
