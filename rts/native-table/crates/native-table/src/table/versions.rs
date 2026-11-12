//! The row layouts a seal reads its group's rows in. A buffered row keeps the layout of the schema version it
//! was written at, and finding a leaf's cell in it walks the leaf list, O(leaves). A seal reads every value of
//! every row, so it builds, for each schema version its rows were written at, a table of every leaf's offsets
//! in a row of that version, the first time it reads a row of that version; a value read then costs one table
//! read whatever the number of leaves.
//!
//! **Index**, in memory: up to `CAP` entries, one per version. A version that finds the index full takes the
//! entries in turn, and the room of the table it replaces: every table of one seal has the shape's leaves.
//!
//! **Table**, in the rewrite's work space: 8 bytes per ordinal {the leaf's cell offset from the fixed region's
//! start u32; its item table index u16; its bitmap bit u16}.

use crate::fault::{rt, Corruption, Fault};
use crate::mem::{put_u16, put_u32, rd_u16, rd_u32, PageHost};
use crate::stats::space::Space;

use super::layout::{RowHead, Shape};

/// The index's capacity.
pub const CAP: usize = 64;
/// Leaves one piece of a build walks, and the bytes of their table entries.
const PIECE: u64 = 64;
const ROW: u64 = 8;

/// One version's row layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    pub version: u32,
    pub leaves: u64,
    pub r: u64,
    pub vars: u64,
    pub fixed: u64,
    pub table: u64,
}

impl Entry {
    pub fn head(&self) -> RowHead {
        RowHead::of(self.version, self.r, self.fixed, self.vars)
    }
}

/// The versions whose row layouts a rewrite has built.
pub struct Versions {
    n: usize,
    next: usize,
    e: [Entry; CAP],
}

impl Default for Versions {
    fn default() -> Self {
        Self::new()
    }
}

impl Versions {
    pub const fn new() -> Self {
        Versions { n: 0, next: 0, e: [Entry { version: 0, leaves: 0, r: 0, vars: 0, fixed: 0, table: 0 }; CAP] }
    }

    /// The entry of version v, if built.
    pub fn find(&self, v: u32) -> Option<Entry> {
        self.e.iter().take(self.n).find(|e| e.version == v).copied()
    }

    /// Builds version v's table and entry: in a new entry while the index has room, else in the next entry in
    /// turn and its table's room.
    pub fn build<H: PageHost>(&mut self, h: &mut H, shape: &Shape, ws: &mut Space, v: u32) -> Result<Entry, Fault> {
        let i = if self.n < CAP {
            self.n
        } else {
            let i = self.next;
            self.next = i.wrapping_add(1) % CAP;
            i
        };
        let table = match self.e.get(i) {
            Some(e) if i < self.n => e.table,
            _ => ws.append(h, shape.leaves.checked_mul(ROW).ok_or(rt(Corruption::Range, shape.leaves))?)?,
        };
        let (mut r, mut vars, mut fixed) = (0u64, 0u64, 0u64);
        let mut from = 0u64;
        while from < shape.leaves {
            let to = from.saturating_add(PIECE).min(shape.leaves);
            let mut rows = [0u8; 512];
            shape.spans(&*h, from, to, |o, s| {
                let k = o.wrapping_sub(from).wrapping_mul(ROW);
                put_u32(&mut rows, k, u32::try_from(fixed).map_err(|_| rt(Corruption::Range, fixed))?)?;
                put_u16(&mut rows, k.wrapping_add(4), u16::try_from(vars).map_err(|_| rt(Corruption::Range, vars))?)?;
                put_u16(&mut rows, k.wrapping_add(6), u16::try_from(r).map_err(|_| rt(Corruption::Range, r))?)?;
                r = r.wrapping_add(1);
                fixed = fixed.wrapping_add(s.width);
                if s.variable {
                    vars = vars.wrapping_add(1);
                }
                Ok(())
            })?;
            let n = to.wrapping_sub(from).wrapping_mul(ROW);
            let piece = rows
                .get(..usize::try_from(n).map_err(|_| rt(Corruption::Range, n))?)
                .ok_or(rt(Corruption::Range, n))?;
            ws.write_bytes(h, table.wrapping_add(from.wrapping_mul(ROW)), piece)?;
            from = to;
        }
        let e = Entry { version: v, leaves: shape.leaves, r, vars, fixed, table };
        let slot = self.e.get_mut(i).ok_or(rt(Corruption::Range, i as u64))?;
        *slot = e;
        if self.n < CAP {
            self.n = self.n.wrapping_add(1);
        }
        Ok(e)
    }
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
