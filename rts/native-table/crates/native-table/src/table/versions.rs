//! The row layouts a seal reads its frozen group's rows in. A buffered row keeps the layout of the schema
//! version it was written at, and finding a leaf's cell in it walks the leaf list, O(leaves). A seal reads
//! every value of every row, so it builds, for each schema version its rows were written at, a table of every
//! leaf's offsets in a row of that version, in pieces of `PIECE` leaves, the first time it reads a row of that
//! version; a value read then costs one table read whatever the number of leaves.
//!
//! **Index**, in the rewrite's work space at the offset `RW_VINDEX` names: `CAP` entries of 32 bytes {version u32 @0; ordinals
//! the table covers u32 @4; ordinals built u32 @8; the version's leaves R u32 @12; its variable leaves u32
//! @16; its fixed region's bytes u32 @20; the table's offset in the work space u32 @24; the rewrite's unit
//! count when its build began u32 @28}, the entries in use counted at @392 and the version whose table is to be
//! built at @396 (0: none). A version that finds the index full takes the entry built first, and its table's
//! room when the room is large enough.
//!
//! **Table**, in the rewrite's work space: 8 bytes per ordinal {the leaf's cell offset from the fixed region's
//! start u32; its item table index u16; its bitmap bit u16}, written for the ordinals the version holds.

use crate::fault::{rt, Corruption, Fault};
use crate::mem::{put_u16, put_u32, put_u64, rd_u16, rd_u32, rd_u64, PageHost, Region};
use crate::stats::space::Space;

use super::layout::{RowHead, Shape};

/// Entries in use, the version waiting for its table, and the index's offset in the work space (0: not yet
/// made), in the state block's control region.
const COUNT: u64 = 392;
const PENDING: u64 = 396;
const AT: u64 = 736;
/// The index's entry size and its capacity.
const ENTRY: u64 = 32;
pub const CAP: u64 = 64;
/// Leaves one piece of a build walks, and the bytes of their table entries.
pub const PIECE: u64 = 64;
const ROW: u64 = 8;

const E_VERSION: u64 = 0;
const E_LEAVES: u64 = 4;
const E_BUILT: u64 = 8;
const E_R: u64 = 12;
const E_VARS: u64 = 16;
const E_FIXED: u64 = 20;
const E_TABLE: u64 = 24;
const E_USED: u64 = 28;

/// One index entry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    pub version: u32,
    pub leaves: u64,
    pub built: u64,
    pub r: u64,
    pub vars: u64,
    pub fixed: u64,
    pub table: u64,
}

impl Entry {
    fn read(b: &[u8; 32]) -> Result<Entry, Fault> {
        let u = |o: u64| -> Result<u64, Fault> { Ok(u64::from(rd_u32(b, o)?)) };
        Ok(Entry {
            version: rd_u32(b, E_VERSION)?,
            leaves: u(E_LEAVES)?,
            built: u(E_BUILT)?,
            r: u(E_R)?,
            vars: u(E_VARS)?,
            fixed: u(E_FIXED)?,
            table: u(E_TABLE)?,
        })
    }

    fn bytes(&self, used: u64) -> Result<[u8; 32], Fault> {
        let mut b = [0u8; 32];
        let w = |x: u64| u32::try_from(x).map_err(|_| rt(Corruption::Range, x));
        put_u32(&mut b, E_VERSION, self.version)?;
        put_u32(&mut b, E_LEAVES, w(self.leaves)?)?;
        put_u32(&mut b, E_BUILT, w(self.built)?)?;
        put_u32(&mut b, E_R, w(self.r)?)?;
        put_u32(&mut b, E_VARS, w(self.vars)?)?;
        put_u32(&mut b, E_FIXED, w(self.fixed)?)?;
        put_u32(&mut b, E_TABLE, w(self.table)?)?;
        put_u32(&mut b, E_USED, u32::try_from(used).unwrap_or(u32::MAX))?;
        Ok(b)
    }

    pub fn done(&self) -> bool {
        self.built >= self.leaves
    }

    pub fn head(&self) -> RowHead {
        RowHead::of(self.version, self.r, self.fixed, self.vars)
    }
}

/// The index's entries in use and its offset in the work space.
fn index<H: PageHost + ?Sized>(h: &H, st: u64) -> Result<(u64, u64), Fault> {
    let b = h.get(Region::Block(st))?;
    Ok((u64::from(rd_u32(b, COUNT)?).min(CAP), rd_u64(b, AT)?))
}

fn entry_bytes<H: PageHost + ?Sized>(h: &H, ws: &Space, at: u64, i: u64) -> Result<[u8; 32], Fault> {
    let mut b = [0u8; 32];
    ws.read_bytes(h, at.wrapping_add(i.wrapping_mul(ENTRY)), &mut b)?;
    Ok(b)
}

/// The complete entry of version v and its index, if the index holds one.
pub fn find<H: PageHost + ?Sized>(h: &H, st: u64, ws: &Space, v: u32) -> Result<Option<(u64, Entry)>, Fault> {
    let (n, at) = index(h, st)?;
    for i in 0..n {
        let b = entry_bytes(h, ws, at, i)?;
        if rd_u32(&b, E_VERSION)? == v {
            let e = Entry::read(&b)?;
            return Ok(if e.done() { Some((i, e)) } else { None });
        }
    }
    Ok(None)
}

/// Leaf `ordinal`'s offsets in a row of entry `e`'s version: (cell offset, item index, bitmap bit), the cell
/// offset from the row image's start.
pub fn offsets<H: PageHost + ?Sized>(h: &H, ws: &Space, e: &Entry, ordinal: u64) -> Result<(u64, u64, u64), Fault> {
    if ordinal >= e.leaves {
        return Err(rt(Corruption::Leaf, ordinal));
    }
    let mut b = [0u8; 8];
    ws.read_bytes(h, e.table.wrapping_add(ordinal.wrapping_mul(ROW)), &mut b)?;
    let fixed = u64::from(rd_u32(&b, 0)?);
    let head = e.head();
    Ok((head.fixed_at.wrapping_add(fixed), u64::from(rd_u16(&b, 4)?), u64::from(rd_u16(&b, 6)?)))
}

/// Records that version v needs its table; the next unit builds it first.
pub fn need<H: PageHost>(h: &mut H, st: u64, v: u32) -> Result<(), Fault> {
    put_u32(h.get_mut(Region::Block(st))?, PENDING, v)
}

/// The version waiting for its table (0: none).
pub fn pending<H: PageHost + ?Sized>(h: &H, st: u64) -> Result<u32, Fault> {
    rd_u32(h.get(Region::Block(st))?, PENDING)
}

/// One piece of the build of the pending version's table: the index made first when the work space has
/// none, an entry for the version (a free one, or the one used least recently), then `PIECE` more leaves.
/// Returns the blocks the piece may have changed besides the state block, and whether the table is complete
/// (the pending version is then cleared). `stamp` is the rewrite's unit count, which ages the entries.
pub fn build_piece<H: PageHost>(
    h: &mut H,
    shape: &Shape,
    st: u64,
    ws: &mut Space,
    stamp: u64,
) -> Result<(u64, bool), Fault> {
    let v = pending(&*h, st)?;
    if v == 0 {
        return Ok((0, true));
    }
    let mut touched = 0u64;
    let (_, mut at) = index(&*h, st)?;
    if at == 0 {
        let bytes = CAP.wrapping_mul(ENTRY);
        at = ws.append(h, bytes)?;
        ws.zero(h, at, bytes)?;
        put_u64(h.get_mut(Region::Block(st))?, AT, at)?;
        touched = touched.wrapping_add(2);
        return Ok((touched, false));
    }
    let (i, mut e, new) = slot_for(&*h, st, ws, v, shape.leaves)?;
    if e.built == 0 && e.table == 0 {
        // Room for the table: appended a piece at a time, so it is contiguous only while one build runs,
        // which `need` keeps to one version at a time.
        e.table = ws.used;
    }
    let from = e.built;
    let to = from.saturating_add(PIECE).min(e.leaves);
    let room_end = e.table.wrapping_add(to.wrapping_mul(ROW));
    if room_end > ws.used {
        let grow = room_end.wrapping_sub(ws.used);
        let got = ws.append(h, grow)?;
        if got.wrapping_add(grow) != room_end {
            return Err(rt(Corruption::Layout, got));
        }
        touched = touched.wrapping_add(2);
    }
    let mut rows = [0u8; 1_024];
    let (mut r, mut vars, mut fixed) = (e.r, e.vars, e.fixed);
    shape.spans(&*h, from, to, |o, s| {
        let k = o.wrapping_sub(from).wrapping_mul(ROW);
        if s.at(v) {
            put_u32(&mut rows, k, u32::try_from(fixed).map_err(|_| rt(Corruption::Range, fixed))?)?;
            put_u16(&mut rows, k.wrapping_add(4), u16::try_from(vars).map_err(|_| rt(Corruption::Range, vars))?)?;
            put_u16(&mut rows, k.wrapping_add(6), u16::try_from(r).map_err(|_| rt(Corruption::Range, r))?)?;
            r = r.wrapping_add(1);
            fixed = fixed.wrapping_add(s.width);
            if s.variable {
                vars = vars.wrapping_add(1);
            }
        } else {
            put_u32(&mut rows, k, 0)?;
            put_u32(&mut rows, k.wrapping_add(4), 0)?;
        }
        Ok(true)
    })?;
    let n = to.wrapping_sub(from).wrapping_mul(ROW);
    let piece =
        rows.get(..usize::try_from(n).map_err(|_| rt(Corruption::Range, n))?).ok_or(rt(Corruption::Range, n))?;
    ws.write_bytes(h, e.table.wrapping_add(from.wrapping_mul(ROW)), piece)?;
    touched = touched.wrapping_add(2);
    e.built = to;
    e.r = r;
    e.vars = vars;
    e.fixed = fixed;
    let began = if from == 0 { stamp } else { u64::from(rd_u32(&entry_bytes(&*h, ws, at, i)?, E_USED)?) };
    ws.write_bytes(h, at.wrapping_add(i.wrapping_mul(ENTRY)), &e.bytes(began)?)?;
    let b = h.get_mut(Region::Block(st))?;
    if new {
        put_u32(b, COUNT, u32::try_from(i.wrapping_add(1)).map_err(|_| rt(Corruption::Range, i))?)?;
    }
    let done = e.done();
    if done {
        put_u32(b, PENDING, 0)?;
    }
    Ok((touched, done))
}

/// The index entry version v builds into, and whether it is a new one: its own when it has one; else a free
/// one; else the one built first, its table kept as the new table's room when that holds `leaves` ordinals.
fn slot_for<H: PageHost + ?Sized>(
    h: &H,
    st: u64,
    ws: &Space,
    v: u32,
    leaves: u64,
) -> Result<(u64, Entry, bool), Fault> {
    let (n, at) = index(h, st)?;
    let mut oldest = (u64::MAX, 0u64);
    for i in 0..n {
        let b = entry_bytes(h, ws, at, i)?;
        if rd_u32(&b, E_VERSION)? == v {
            return Ok((i, Entry::read(&b)?, false));
        }
        let used = u64::from(rd_u32(&b, E_USED)?);
        if used < oldest.0 {
            oldest = (used, i);
        }
    }
    if n < CAP {
        return Ok((n, Entry { version: v, leaves, ..Entry::default() }, true));
    }
    let old = Entry::read(&entry_bytes(h, ws, at, oldest.1)?)?;
    let table = if old.leaves >= leaves { old.table } else { 0 };
    Ok((oldest.1, Entry { version: v, leaves, table, ..Entry::default() }, false))
}
