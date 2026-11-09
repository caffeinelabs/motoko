//! A rewrite's work space: one byte space made of its work page's area (page bytes 656–4,087) followed by
//! bytes 16–4,095 of each of its work space blocks, in list order. A record may cross from the area into a
//! block, or from one block into the next.
//!
//! The area starts with a 16-byte header {locator capacity D u16 @0; block list height u8 @2; zero u8 @3;
//! used bytes u32 @4; block list root u64 @8}, then D u32 locators by leaf ordinal (a record's offset in the
//! space, 0: none), then the records, each 8-aligned. A work space block is {magic `NTSX` u32 @0;
//! `FORMAT_TABLE` u16 @4; zero u16 @6; group number u64 @8; 4,080 bytes of the space}.

use crate::area::alloc_block;
use crate::consts::{magic, BLOCK, FORMAT_TABLE, WORK_AREA};
use crate::fault::{rt, Corruption, Fault};
use crate::mem::{copy_bytes, fill, put, put_u16, put_u32, put_u64, put_u8, sub, PageHost, Region};
use crate::radix::Radix;

/// Work page offset of the area.
pub const AREA_AT: u64 = 656;
/// Bytes before a work space block's share of the space.
pub const BLOCK_HEAD: u64 = 16;
/// Bytes of the space a work space block holds.
pub const BLOCK_ROOM: u64 = BLOCK - BLOCK_HEAD;
/// Bytes of the area header.
pub const HEADER: u64 = 16;

const _: () = assert!(AREA_AT + WORK_AREA == 4_088);

/// One rewrite's work space: the table it belongs to, its work page, the header fields, and the block list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Space {
    pub tbl: u64,
    pub page: u64,
    pub group: u64,
    pub d: u64,
    pub used: u64,
    pub blocks: Radix,
}

impl Space {
    /// A new space of capacity d whose locators are zeroed later, in steps, by `zero_step`: its header with
    /// the used bytes at the header's end. Its locators are not valid until then.
    pub fn begin<H: PageHost>(h: &mut H, tbl: u64, page: u64, group: u64, d: u64) -> Result<Space, Fault> {
        let s = Space { tbl, page, group, d, used: HEADER, blocks: Radix::EMPTY };
        s.write_header(h)?;
        Ok(s)
    }

    /// Zeroes at most `max` more locator bytes; `true` once every locator is zero.
    pub fn zero_step<H: PageHost>(&mut self, h: &mut H, max: u64) -> Result<bool, Fault> {
        let end = HEADER
            .checked_add(self.d.checked_mul(4).ok_or(rt(Corruption::Range, self.d))?)
            .ok_or(rt(Corruption::Range, self.d))?;
        if self.used >= end {
            return Ok(true);
        }
        let to = end.min(self.used.saturating_add(max));
        let from = self.used;
        self.grow_to(h, to)?;
        self.zero(h, from, to.wrapping_sub(from))?;
        self.write_header(h)?;
        Ok(to >= end)
    }

    /// Writes the header fields from this value.
    pub fn write_header<H: PageHost>(&self, h: &mut H) -> Result<(), Fault> {
        let d = u16::try_from(self.d).map_err(|_| rt(Corruption::Range, self.d))?;
        let used = u32::try_from(self.used).map_err(|_| rt(Corruption::Range, self.used))?;
        let b = h.get_mut(Region::Block(self.page))?;
        put_u16(b, AREA_AT, d)?;
        put_u8(b, AREA_AT.wrapping_add(2), self.blocks.height)?;
        put_u8(b, AREA_AT.wrapping_add(3), 0)?;
        put_u32(b, AREA_AT.wrapping_add(4), used)?;
        put_u64(b, AREA_AT.wrapping_add(8), self.blocks.root)
    }

    /// The region and offset of space byte `off`, and the bytes of the space contiguous from there.
    fn locate<H: PageHost + ?Sized>(&self, h: &H, off: u64) -> Result<(Region, u64, u64), Fault> {
        if off < WORK_AREA {
            return Ok((Region::Block(self.page), AREA_AT.wrapping_add(off), WORK_AREA.wrapping_sub(off)));
        }
        let rest = off.wrapping_sub(WORK_AREA);
        let k = rest / BLOCK_ROOM;
        let b = self.blocks.get(h, k)?;
        if b == 0 {
            return Err(rt(Corruption::Link, off));
        }
        let o = rest % BLOCK_ROOM;
        Ok((Region::Block(b), BLOCK_HEAD.wrapping_add(o), BLOCK_ROOM.wrapping_sub(o)))
    }

    /// Copies space bytes [off, off + out.len()) into `out`.
    pub fn read_bytes<H: PageHost + ?Sized>(&self, h: &H, off: u64, out: &mut [u8]) -> Result<(), Fault> {
        let mut at = off;
        let mut rest = out;
        while !rest.is_empty() {
            let (r, o, room) = self.locate(h, at)?;
            let n = usize::try_from(room).map_err(|_| rt(Corruption::Range, at))?.min(rest.len());
            let (now, later) = rest.split_at_mut_checked(n).ok_or(rt(Corruption::Range, at))?;
            copy_bytes(now, sub(h.get(r)?, o, n as u64)?);
            at = at.checked_add(n as u64).ok_or(rt(Corruption::Range, at))?;
            rest = later;
        }
        Ok(())
    }

    /// Writes `src` at space offset `off`, inside the used bytes.
    pub fn write_bytes<H: PageHost>(&self, h: &mut H, off: u64, src: &[u8]) -> Result<(), Fault> {
        let end = off.checked_add(src.len() as u64).ok_or(rt(Corruption::Range, off))?;
        if end > self.used {
            return Err(rt(Corruption::Bounds, off));
        }
        let mut at = off;
        let mut rest = src;
        while !rest.is_empty() {
            let (r, o, room) = self.locate(h, at)?;
            let n = usize::try_from(room).map_err(|_| rt(Corruption::Range, at))?.min(rest.len());
            let (now, later) = rest.split_at_checked(n).ok_or(rt(Corruption::Range, at))?;
            put(h.get_mut(r)?, o, now)?;
            at = at.wrapping_add(n as u64);
            rest = later;
        }
        Ok(())
    }

    /// Zeroes space bytes [off, off + len), a fill per block.
    pub fn zero<H: PageHost>(&self, h: &mut H, off: u64, len: u64) -> Result<(), Fault> {
        let mut at = off;
        let end = off.checked_add(len).ok_or(rt(Corruption::Range, off))?;
        while at < end {
            let (r, o, room) = self.locate(h, at)?;
            let n = room.min(end.wrapping_sub(at));
            fill(h.get_mut(r)?, o, n, 0)?;
            at = at.wrapping_add(n);
        }
        Ok(())
    }

    /// The little-endian u32 at space offset `off`.
    pub fn u32_at<H: PageHost + ?Sized>(&self, h: &H, off: u64) -> Result<u32, Fault> {
        let mut b = [0u8; 4];
        self.read_bytes(h, off, &mut b)?;
        Ok(u32::from_le_bytes(b))
    }

    pub fn put_u32_at<H: PageHost>(&self, h: &mut H, off: u64, v: u32) -> Result<(), Fault> {
        self.write_bytes(h, off, &v.to_le_bytes())
    }

    /// Leaf `ordinal`'s record offset, 0 when it has none (an ordinal at or past D has none).
    pub fn locator<H: PageHost + ?Sized>(&self, h: &H, ordinal: u64) -> Result<u64, Fault> {
        if ordinal >= self.d {
            return Ok(0);
        }
        // ordinal < d ≤ 65,535.
        Ok(u64::from(self.u32_at(h, HEADER.wrapping_add(ordinal.wrapping_mul(4)))?))
    }

    pub fn set_locator<H: PageHost>(&self, h: &mut H, ordinal: u64, at: u64) -> Result<(), Fault> {
        if ordinal >= self.d {
            return Err(rt(Corruption::Range, ordinal));
        }
        let v = u32::try_from(at).map_err(|_| rt(Corruption::Range, at))?;
        self.put_u32_at(h, HEADER.wrapping_add(ordinal.wrapping_mul(4)), v)
    }

    /// Extends the used bytes to `end`, allocating the work space blocks that end needs from area 0 (block
    /// headers written; the room unspecified until written).
    fn grow_to<H: PageHost>(&mut self, h: &mut H, end: u64) -> Result<(), Fault> {
        let blocks = |u: u64| if u > WORK_AREA { u.wrapping_sub(WORK_AREA).div_ceil(BLOCK_ROOM) } else { 0 };
        let need = blocks(end);
        // From the block that holds the last byte in use: room `reserve` made may have left it unallocated.
        let mut k = blocks(self.used).saturating_sub(1);
        while k < need {
            if self.blocks.get(h, k)? != 0 {
                k = k.wrapping_add(1);
                continue;
            }
            let b = alloc_block(h, self.tbl, 0)?;
            {
                let s = h.get_mut(Region::Block(b))?;
                put_u32(s, 0, magic::NTSX)?;
                put_u16(s, 4, FORMAT_TABLE)?;
                put_u16(s, 6, 0)?;
                put_u64(s, 8, self.group)?;
            }
            let mut list = self.blocks;
            list.set(h, self.tbl, k, b)?;
            self.blocks = list;
            // k < need, which a u64 holds.
            k = k.wrapping_add(1);
        }
        self.used = end;
        Ok(())
    }

    /// Appends `bytes` of room at the end of the space and returns its offset; the header is written. The
    /// room's bytes are unspecified until written.
    pub fn append<H: PageHost>(&mut self, h: &mut H, bytes: u64) -> Result<u64, Fault> {
        let at = self.used;
        let end = at.checked_add(bytes).ok_or(rt(Corruption::Range, at))?;
        if end > u64::from(u32::MAX) {
            return Err(rt(Corruption::Range, at));
        }
        self.grow_to(h, end)?;
        self.write_header(h)?;
        Ok(at)
    }
}
