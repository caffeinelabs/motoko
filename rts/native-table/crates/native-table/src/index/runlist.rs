//! The run list of an index: one area-0 block (`NTRL`) that lists the runs its uploads brought, its loaded
//! runs. They are listed in the loaded-run list, a radix array of area-0 blocks of 64 descriptors each,
//! appended in ascending start sequence, with no cap, which the loaded-run record names. A loaded run is never
//! merged or rewritten.
//!
//! **Run list block** (little-endian): magic `NTRL` u32 @0; format u16 @4; zero to 4,032; the loaded-run
//! record @4,032.
//!
//! **Run descriptor** (64 bytes): image root u64 @0 (the image reference's root); items u64 @8 (rows and
//! tombstones); rid base u64 @16 (0 for absolute rids); smallest absolute rid u64 @24; largest u64 @32;
//! start sequence u64 @40; zero u64 @48; image height u8 @56; zero u8 @57; flags u8 @58 (bit 0 relative rids,
//! bit 1 filter, bit 2 key runs, bit 3 believed: a loaded run, bit 6 count tree); tree height u8 @59; V u16
//! @60; W u16 @62.
//!
//! **Loaded-run record** (64 bytes at 4,032): radix root u64 @0; radix height u8 @8; loaded runs u64 @16;
//! their items u64 @24; the probe store's radix height u8 @49 and root u64 @56 (0: no record yet), the facts a
//! count reads of each run, kept in step with the descriptors (`runprobe`); zero elsewhere.

use crate::area::alloc_zeroed;
use crate::consts::{magic, BLOCK, FORMAT_TABLE, PAGE};
use crate::fault::{rt, Corruption, Fault};
use crate::image::{Blocks, Image};
use crate::index::runprobe;
use crate::mem::{fill, put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u64, rd_u8, PageHost, Region};
use crate::radix::{Radix, HEIGHT_MAX};

/// Bytes of a run descriptor.
pub const DESC: u64 = 64;
/// Offset of the loaded-run record.
pub const LOADED_RECORD: u64 = 4_032;
/// Descriptors per block of the loaded-run list.
pub const LOADED_PER_BLOCK: u64 = BLOCK / DESC;

/// Descriptor flag bit 0: rids are relative to the rid base.
pub const D_RELATIVE: u8 = 1;
/// Descriptor flag bit 1: the run carries a filter.
pub const D_FILTER: u8 = 2;
/// Descriptor flag bit 2: the run holds key runs.
pub const D_KEY_RUNS: u8 = 4;
/// Descriptor flag bit 3: believed, installed by a load and listed in the loaded-run list.
pub const D_BELIEVED: u8 = 8;
/// Descriptor flag bit 6: the run carries a count tree.
pub const D_COUNT_TREE: u8 = 64;

/// One run descriptor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RunDesc {
    pub root: u64,
    pub items: u64,
    pub rid_base: u64,
    pub rid_lo: u64,
    pub rid_hi: u64,
    pub start_seq: u64,
    pub img_height: u8,
    pub flags: u8,
    pub tree_height: u8,
    pub v: u16,
    pub w: u16,
}

impl RunDesc {
    /// The descriptor at `off` of `b`.
    pub fn read(b: &[u8], off: u64) -> Result<RunDesc, Fault> {
        let at = |k: u64| off.checked_add(k).ok_or(rt(Corruption::Bounds, off));
        Ok(RunDesc {
            root: rd_u64(b, at(0)?)?,
            items: rd_u64(b, at(8)?)?,
            rid_base: rd_u64(b, at(16)?)?,
            rid_lo: rd_u64(b, at(24)?)?,
            rid_hi: rd_u64(b, at(32)?)?,
            start_seq: rd_u64(b, at(40)?)?,
            img_height: rd_u8(b, at(56)?)?,
            flags: rd_u8(b, at(58)?)?,
            tree_height: rd_u8(b, at(59)?)?,
            v: rd_u16(b, at(60)?)?,
            w: rd_u16(b, at(62)?)?,
        })
    }

    /// Writes all 64 bytes of the descriptor at `off` of `b`.
    pub fn write(&self, b: &mut [u8], off: u64) -> Result<(), Fault> {
        let at = |k: u64| off.checked_add(k).ok_or(rt(Corruption::Bounds, off));
        put_u64(b, at(0)?, self.root)?;
        put_u64(b, at(8)?, self.items)?;
        put_u64(b, at(16)?, self.rid_base)?;
        put_u64(b, at(24)?, self.rid_lo)?;
        put_u64(b, at(32)?, self.rid_hi)?;
        put_u64(b, at(40)?, self.start_seq)?;
        put_u64(b, at(48)?, 0)?;
        put_u8(b, at(56)?, self.img_height)?;
        put_u8(b, at(57)?, 0)?;
        put_u8(b, at(58)?, self.flags)?;
        put_u8(b, at(59)?, self.tree_height)?;
        put_u16(b, at(60)?, self.v)?;
        put_u16(b, at(62)?, self.w)
    }

    pub fn is_believed(&self) -> bool {
        self.flags & D_BELIEVED != 0
    }

    /// The run's image: its block entries from the descriptor, its length from the run header's bytes
    /// field (image offset 8), read through the image's first block.
    #[inline]
    pub fn image<H: PageHost + ?Sized>(&self, h: &H) -> Result<Image, Fault> {
        if self.img_height > HEIGHT_MAX {
            return Err(rt(Corruption::Depth, self.root));
        }
        let blocks = if self.img_height == 0 {
            let mut entries = [0u64; 8];
            if let Some(e) = entries.first_mut() {
                *e = self.root;
            }
            Blocks::Direct { count: u8::from(self.root != 0), entries }
        } else {
            Blocks::Radix(Radix { root: self.root, height: self.img_height })
        };
        let probe = Image { length: PAGE, blocks, believed: self.is_believed() };
        let first = probe.entry(h, 0)?;
        if first == 0 {
            return Err(rt(Corruption::Link, self.root));
        }
        let length = rd_u64(h.get(Region::Block(first))?, 8)?;
        Ok(Image { length, blocks, believed: self.is_believed() })
    }
}

/// The radix array of an image in reference form, as a run descriptor names it (its root and image height):
/// one block (height 0), or a radix.
pub fn image_radix(img: &Image) -> Result<Radix, Fault> {
    match img.blocks {
        Blocks::Radix(r) => Ok(r),
        Blocks::Direct { count, entries } if count <= 1 => {
            Ok(Radix { root: if count == 1 { entries.first().copied().unwrap_or(0) } else { 0 }, height: 0 })
        }
        Blocks::Direct { .. } => Err(rt(Corruption::Layout, img.length)),
    }
}

/// The loaded-run record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoadedRecord {
    pub list: Radix,
    pub runs: u64,
    pub items: u64,
    /// The probe store: the records a count reads, in step with the descriptors.
    pub probes: Radix,
}

impl LoadedRecord {
    /// The record of a run list block, its radix arrays checked.
    pub fn read(b: &[u8]) -> Result<LoadedRecord, Fault> {
        let r = LOADED_RECORD;
        let loaded = LoadedRecord {
            list: Radix { root: rd_u64(b, r)?, height: rd_u8(b, r.wrapping_add(8))? },
            runs: rd_u64(b, r.wrapping_add(16))?,
            items: rd_u64(b, r.wrapping_add(24))?,
            probes: Radix { root: rd_u64(b, r.wrapping_add(56))?, height: rd_u8(b, r.wrapping_add(49))? },
        };
        loaded.list.check()?;
        loaded.probes.check()?;
        Ok(loaded)
    }

    /// The block and offset of loaded run i.
    fn slot<H: PageHost + ?Sized>(&self, h: &H, i: u64) -> Result<(u64, u64), Fault> {
        if i >= self.runs {
            return Err(rt(Corruption::Range, i));
        }
        let b = self.list.get(h, i / LOADED_PER_BLOCK)?;
        if b == 0 {
            return Err(rt(Corruption::Link, i));
        }
        Ok((b, (i % LOADED_PER_BLOCK).wrapping_mul(DESC)))
    }

    /// Loaded run i, in ascending start-sequence order.
    pub fn get<H: PageHost + ?Sized>(&self, h: &H, i: u64) -> Result<RunDesc, Fault> {
        let (b, off) = self.slot(h, i)?;
        RunDesc::read(h.get(Region::Block(b))?, off)
    }
}

/// A run list block and its loaded-run record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunList {
    pub block: u64,
    pub loaded: LoadedRecord,
}

impl RunList {
    /// An empty run list in a new area-0 block.
    pub fn create<H: PageHost>(h: &mut H, tbl: u64) -> Result<RunList, Fault> {
        let rl = RunList { block: alloc_zeroed(h, tbl, 0)?, loaded: LoadedRecord::default() };
        rl.store(h)?;
        Ok(rl)
    }

    /// Reads the run list block `block`. A bad magic or format is `Corrupt(Runtime)`.
    pub fn load<H: PageHost + ?Sized>(h: &H, block: u64) -> Result<RunList, Fault> {
        let b = h.get(Region::Block(block))?;
        if rd_u32(b, 0)? != magic::NTRL || rd_u16(b, 4)? != FORMAT_TABLE {
            return Err(rt(Corruption::Header, block));
        }
        Ok(RunList { block, loaded: LoadedRecord::read(b)? })
    }

    /// Writes the whole block: the header, zero up to the loaded-run record, and the record.
    pub fn store<H: PageHost>(&self, h: &mut H) -> Result<(), Fault> {
        let b = h.get_mut(Region::Block(self.block))?;
        fill(b, 0, BLOCK, 0)?;
        put_u32(b, 0, magic::NTRL)?;
        put_u16(b, 4, FORMAT_TABLE)?;
        let r = LOADED_RECORD;
        let l = &self.loaded;
        put_u64(b, r, l.list.root)?;
        put_u8(b, r.wrapping_add(8), l.list.height)?;
        put_u64(b, r.wrapping_add(16), l.runs)?;
        put_u64(b, r.wrapping_add(24), l.items)?;
        put_u8(b, r.wrapping_add(49), l.probes.height)?;
        put_u64(b, r.wrapping_add(56), l.probes.root)
    }

    /// Loaded run i, in ascending start-sequence order.
    pub fn loaded_get<H: PageHost + ?Sized>(&self, h: &H, i: u64) -> Result<RunDesc, Fault> {
        self.loaded.get(h, i)
    }

    /// Appends a loaded run; its start sequence must lie above every listed one's. One changed leaf block
    /// per append (a new zero-filled block every 64 runs, and the radix blocks its listing needs), and one
    /// changed block of probe records (likewise). The record's totals take its items. The caller stores the
    /// run list block.
    pub fn loaded_append<H: PageHost>(&mut self, h: &mut H, tbl: u64, d: &RunDesc) -> Result<u64, Fault> {
        let i = self.loaded.runs;
        if let Some(last) = i.checked_sub(1) {
            if self.loaded_get(h, last)?.start_seq >= d.start_seq {
                return Err(rt(Corruption::Order, d.start_seq));
            }
        }
        if i % LOADED_PER_BLOCK == 0 {
            let b = alloc_zeroed(h, tbl, 0)?;
            let mut list = self.loaded.list;
            list.set(h, tbl, i / LOADED_PER_BLOCK, b)?;
            self.loaded.list = list;
        }
        self.loaded.runs = i.checked_add(1).ok_or(rt(Corruption::Count, i))?;
        self.loaded.items = self.loaded.items.checked_add(d.items).ok_or(rt(Corruption::Count, i))?;
        let (b, off) = self.loaded.slot(h, i)?;
        d.write(h.get_mut(Region::Block(b))?, off)?;
        runprobe::set_loaded(h, tbl, &mut self.loaded.probes, i, d)?;
        Ok(i)
    }
}
