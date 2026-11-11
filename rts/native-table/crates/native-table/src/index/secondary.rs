//! The secondary index: one B-tree in table memory per index, holding every row of the table, with a count per
//! key.
//!
//! **The tree.** An index-form tree of shape (W, 0) in area 1, reached through the area node host
//! (`area_nodes`): one row entry (key, rid) per row, and one count entry per distinct key, which sorts just
//! after the key's last row entry (the order of every index structure) and carries how many rows hold the
//! key. Every insert and delete changes the row entry and then the count in place: a key's first row makes
//! its count entry, its last row removes it. So a key's count in the tree is one descent, and the keys and
//! their counts are the count entries in order.
//!
//! Rows arrive in rising rid order, so an insert's row lands just before its key's count entry, usually in
//! the same leaf: the count is then raised where the insert's own descent ends, and only a key's first row, or
//! a count entry beyond the leaf, takes a second descent. A delete lowers a count of two or more found next to
//! its row the same way.
//!
//! A key longer than W keeps its bytes beyond W in the tree's key log: each of its entries has a record there,
//! a row entry's carrying its rid, a count entry's its count, which a change of the count rewrites in place.
//! Removing an entry hands its record back (`area_nodes`), so the log follows the rows the tree holds.
//!
//! **Descriptor.** A table's secondary index `id` keeps its state in its index descriptor (`directory`): kind
//! 2 @0, the tree's W u16 @2, the key spec hash u64 @8, the tree's root u64 @16, node count u32 @24, key-log
//! blocks u32 @28, key-log radix root u64 @32 and height u8 @40; bytes @48 to @88 are zero. The table makes it
//! when it is created, and writes the tree's fields back whenever a write changes them.

use core::cmp::Ordering;
use core::ops::Bound;

use crate::fault::{rt, Corruption, Fault, ImageId};
use crate::host::NodeRead;
use crate::index::area_insert;
use crate::index::area_nodes::{AreaNodes, AreaView, KeyLog};
use crate::index::directory;
use crate::key::probe::{
    count_field, is_count, is_ovf, ovf_field, payload, payload_to_count, row_field, IndexPending, KeyProbe, Kind,
    ResolveKey,
};
use crate::key::KeySpec;
use crate::mem::{
    copy_bytes, put, put_u16, put_u32, put_u64, put_u8, rd_array, rd_u16, rd_u32, rd_u64, rd_u8, PageHost, Region,
};
use crate::radix::Radix;
use crate::tree::node::{count, is_leaf, ko};
use crate::tree::{self, Cursor, Fwd, IndexShape, Path, Probe, Search, Shape};

mod sorted;
pub use sorted::Start;

/// The area the tree's nodes and key log take.
const AREA: u32 = 1;

/// Descriptor kind 2: a secondary index.
pub const KIND_SECONDARY: u8 = 2;
/// Offsets in an index descriptor.
const D_KIND: u64 = 0;
const D_W: u64 = 2;
const D_HASH: u64 = 8;
const D_ROOT: u64 = 16;
const D_NODES: u64 = 24;
const D_LOG_BLOCKS: u64 = 28;
const D_LOG_ROOT: u64 = 32;
const D_LOG_HEIGHT: u64 = 40;

/// One secondary index: its tree's state, kept by the index's owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Secondary {
    /// The index's id.
    pub index: u8,
    /// The tree's shape, (W, 0).
    pub shape: IndexShape,
    /// The tree's root (0: empty).
    pub root: u64,
    /// The tree's node count.
    pub nodes: u64,
    /// The tree's key log.
    pub log: KeyLog,
}

impl Secondary {
    /// An empty index `index` of key width `w` (a multiple of 8 in 8..=1,016); `None` for any other width. It
    /// holds no block until its first row.
    pub fn create(index: u8, w: u64) -> Option<Secondary> {
        Some(Secondary { index, shape: IndexShape::new(w, 0)?, root: 0, nodes: 0, log: KeyLog::default() })
    }

    /// Makes table `tbl`'s descriptor of index `index` (1 to 63), for an empty tree of key width `w` and the key
    /// spec hash `hash`; its directory block is made when needed.
    pub fn declare<H: PageHost>(h: &mut H, tbl: u64, index: u8, w: u64, hash: u64) -> Result<Secondary, Fault> {
        let s = Secondary::create(index, w).ok_or(rt(Corruption::Layout, w))?;
        if index == 0 {
            return Err(rt(Corruption::Range, 0));
        }
        let (b, o) = directory::ensure(h, tbl, index)?;
        let w16 = u16::try_from(w).map_err(|_| rt(Corruption::Layout, w))?;
        let blk = h.get_mut(Region::Block(b))?;
        // o ≤ 64 + 30 · 128: every field below lies inside the block.
        put_u8(blk, o.wrapping_add(D_KIND), KIND_SECONDARY)?;
        put_u16(blk, o.wrapping_add(D_W), w16)?;
        put_u64(blk, o.wrapping_add(D_HASH), hash)?;
        Ok(s)
    }

    /// Table `tbl`'s index `index` as its descriptor records it.
    pub fn read<H: PageHost + ?Sized>(h: &H, tbl: u64, index: u8) -> Result<Secondary, Fault> {
        let (b, o) = directory::locate(h, tbl, index)?.ok_or(rt(Corruption::Link, u64::from(index)))?;
        let blk = h.get(Region::Block(b))?;
        let at = |k: u64| o.wrapping_add(k);
        if rd_u8(blk, at(D_KIND))? != KIND_SECONDARY {
            return Err(rt(Corruption::Header, b));
        }
        let w = u64::from(rd_u16(blk, at(D_W))?);
        Ok(Secondary {
            index,
            shape: IndexShape::new(w, 0).ok_or(rt(Corruption::Layout, b))?,
            root: rd_u64(blk, at(D_ROOT))?,
            nodes: u64::from(rd_u32(blk, at(D_NODES))?),
            log: KeyLog {
                radix: Radix { root: rd_u64(blk, at(D_LOG_ROOT))?, height: rd_u8(blk, at(D_LOG_HEIGHT))? },
                blocks: u64::from(rd_u32(blk, at(D_LOG_BLOCKS))?),
            },
        })
    }

    /// Clears table `tbl`'s descriptor of index `index`: the table has no such index any more. Its tree and key
    /// log stay allocated, and nothing names them.
    pub fn clear<H: PageHost>(h: &mut H, tbl: u64, index: u8) -> Result<(), Fault> {
        let (b, o) = directory::locate(&*h, tbl, index)?.ok_or(rt(Corruption::Link, u64::from(index)))?;
        crate::mem::fill(h.get_mut(Region::Block(b))?, o, directory::INDEX_DESC, 0)
    }

    /// Writes the tree's root, node count and key log into table `tbl`'s descriptor of the index.
    pub fn write_tree<H: PageHost>(&self, h: &mut H, tbl: u64) -> Result<(), Fault> {
        let (b, o) = directory::locate(&*h, tbl, self.index)?.ok_or(rt(Corruption::Link, u64::from(self.index)))?;
        let nodes = u32::try_from(self.nodes).map_err(|_| rt(Corruption::Count, self.nodes))?;
        let blocks = u32::try_from(self.log.blocks).map_err(|_| rt(Corruption::Count, self.log.blocks))?;
        let blk = h.get_mut(Region::Block(b))?;
        let at = |k: u64| o.wrapping_add(k);
        put_u64(blk, at(D_ROOT), self.root)?;
        put_u32(blk, at(D_NODES), nodes)?;
        put_u32(blk, at(D_LOG_BLOCKS), blocks)?;
        put_u64(blk, at(D_LOG_ROOT), self.log.radix.root)?;
        put_u8(blk, at(D_LOG_HEIGHT), self.log.radix.height)
    }

    /// Takes the row `rid` (below 2^61, as every rid is) under `key`, its full encoding: its row entry, and
    /// its key's count raised by one. `Ok(false)` when the tree holds the row already, and nothing changes.
    /// `path` is the caller's scratch.
    pub fn insert<H: PageHost>(
        &mut self,
        h: &mut H,
        tbl: u64,
        key: &[u8],
        rid: u64,
        path: &mut Path,
    ) -> Result<bool, Fault> {
        self.insert_by(h, tbl, key, rid, path).map(|r| r.is_some())
    }

    #[inline(always)]
    fn insert_by<H: PageHost>(
        &mut self,
        h: &mut H,
        tbl: u64,
        key: &[u8],
        rid: u64,
        path: &mut Path,
    ) -> Result<Option<bool>, Fault> {
        let mut m = self.tree(h, tbl);
        let r = insert_row(&mut m, &mut self.root, key, rid, path);
        self.nodes = m.nodes;
        self.log = m.log;
        r
    }

    /// Drops the row `rid` under `key`: its row entry, and its key's count lowered by one. `Ok(false)` when
    /// the tree does not hold the row, and nothing changes. `path` is the caller's scratch.
    pub fn delete<H: PageHost>(
        &mut self,
        h: &mut H,
        tbl: u64,
        key: &[u8],
        rid: u64,
        path: &mut Path,
    ) -> Result<bool, Fault> {
        self.delete_by(h, tbl, key, rid, path)
    }

    #[inline(always)]
    fn delete_by<H: PageHost>(
        &mut self,
        h: &mut H,
        tbl: u64,
        key: &[u8],
        rid: u64,
        path: &mut Path,
    ) -> Result<bool, Fault> {
        if self.root == 0 {
            return Ok(false);
        }
        let mut m = self.tree(h, tbl);
        let r = delete_row(&mut m, &mut self.root, key, rid, path);
        self.nodes = m.nodes;
        self.log = m.log;
        r
    }

    /// The rows that hold `key`, its full encoding: its count entry in the tree, found by one descent.
    pub fn count<H: PageHost + ?Sized>(&self, h: &H, key: &[u8]) -> Result<i128, Fault> {
        Ok(i128::from(self.tree_count_by(h, &KeyProbe { key, kind: Kind::Count })?))
    }

    /// Whether the tree holds the row `rid` under `key`, its full encoding: one descent.
    pub fn holds<H: PageHost + ?Sized>(&self, h: &H, key: &[u8], rid: u64) -> Result<bool, Fault> {
        let p = KeyProbe { key, kind: Kind::Row(rid) };
        Ok(self.root != 0 && tree::find(self.shape, &self.view(h), self.root, &p)?.is_some())
    }

    /// The next group of the index in key order: the smallest key above `after` (the smallest of all when
    /// `None`), written whole at the start of `out`, with its length and its count; `None` past the last key.
    /// `spec` is the index's key spec, which gives each key's end. A step is two descents of the tree (to the
    /// key after `after`, then to its count entry), so a walk of g groups costs 2g descents, whatever the rows.
    /// It keeps no state: a walk resumes from the key it handed out last.
    pub fn group_next<H: PageHost + ?Sized>(
        &self,
        h: &H,
        spec: &KeySpec<'_>,
        after: Option<&[u8]>,
        out: &mut [u8],
    ) -> Result<Option<(usize, i128)>, Fault> {
        match self.tree_next(h, after, out)? {
            Some((len, c)) => Ok(Some((exact(spec, out, len)?, i128::from(c)))),
            None => Ok(None),
        }
    }
    /// The tree's first key above `after` (its first key when `None`), as the tree holds it, at the start of
    /// `out`: its key area of W bytes when it fits W, else the whole key; with its length so written and its
    /// count.
    fn tree_next<H: PageHost + ?Sized>(
        &self,
        h: &H,
        after: Option<&[u8]>,
        out: &mut [u8],
    ) -> Result<Option<(usize, i64)>, Fault> {
        if self.root == 0 {
            return Ok(None);
        }
        let (s, v) = (self.shape, self.view(h));
        // Every entry of `after` lies at or before its count entry: the next key's entries follow it.
        let mut c: Cursor<Fwd> = match after {
            Some(a) => Cursor::seek(s, &v, self.root, Bound::Excluded(&KeyProbe { key: a, kind: Kind::Count }))?,
            None => Cursor::seek(s, &v, self.root, Bound::<&KeyProbe<'_>>::Unbounded)?,
        };
        let Some((n, i)) = c.peek(s, &v)? else { return Ok(None) };
        let len = entry_key(&v, n, i, out)?;
        let key = out.get(..len).unwrap_or(&[]);
        match tree::find(s, &v, self.root, &KeyProbe { key, kind: Kind::Count })? {
            Some((n, i)) => Ok(Some((len, count_at(s, &v, n, i)?))),
            // Every key the tree holds has its count entry.
            None => Err(rt(Corruption::Count, n)),
        }
    }

    fn tree<'h, H: PageHost>(&self, h: &'h mut H, tbl: u64) -> AreaNodes<'h, H> {
        let mut m = AreaNodes::new(h, tbl, AREA, self.shape, self.log);
        m.nodes = self.nodes;
        m
    }

    fn view<'h, H: PageHost + ?Sized>(&self, h: &'h H) -> AreaView<'h, H> {
        AreaView::new(h, self.shape, self.log)
    }
}

impl Secondary {
    /// The count of the key of the count probe `p` in the tree alone: one descent.
    fn tree_count_by<'h, H: PageHost + ?Sized, P: Probe<AreaView<'h, H>>>(
        &self,
        h: &'h H,
        p: &P,
    ) -> Result<i64, Fault> {
        if self.root == 0 {
            return Ok(0);
        }
        let v = self.view(h);
        match tree::find(self.shape, &v, self.root, p)? {
            Some((n, i)) => count_at(self.shape, &v, n, i),
            None => Ok(0),
        }
    }
}

/// `Secondary::insert` on the tree `m` with root `root`. `None` when the tree holds the row; else whether the
/// key's count entry was made.
fn insert_row<H: PageHost>(
    m: &mut AreaNodes<'_, H>,
    root: &mut u64,
    key: &[u8],
    rid: u64,
    path: &mut Path,
) -> Result<Option<bool>, Fault> {
    let s = m.shape;
    let row = |m: &mut AreaNodes<'_, H>| stored_field(m, row_field(rid), rid, key);
    if *root == 0 {
        let field = row(m)?;
        *root = tree::insert_first(s, m, &IndexPending::Borrowed { key, field, value: &[], child: 0 })?;
        return add_count(m, root, key, 1, path).map(Some);
    }
    path.clear();
    let (n, j) = match tree::descend(s, &*m, *root, &KeyProbe { key, kind: Kind::Row(rid) }, path)? {
        Search::Found(..) => return Ok(None),
        Search::Vacant(n, j) => (n, j),
    };
    // The count entry the row lands before, raised first; the row then moves it up by one.
    let raised = count_next(m, n, j, key, 1)?.is_some();
    let field = row(m)?;
    area_insert::insert_vacant(m, root, n, j, key, field, &[], path)?;
    if raised {
        return Ok(Some(false));
    }
    add_count(m, root, key, 1, path).map(Some)
}

/// `Secondary::delete` on the tree `m` with root `root`, not empty.
fn delete_row<H: PageHost>(
    m: &mut AreaNodes<'_, H>,
    root: &mut u64,
    key: &[u8],
    rid: u64,
    path: &mut Path,
) -> Result<bool, Fault> {
    let s = m.shape;
    path.clear();
    let (n, i) = match tree::descend(s, &*m, *root, &KeyProbe { key, kind: Kind::Row(rid) }, path)? {
        Search::Found(n, i) => (n, i),
        Search::Vacant(..) => return Ok(false),
    };
    // A count of two or more next to the row in its leaf is lowered there; a count of one goes with its
    // key's last row, and a count beyond the leaf is reached by its own descent.
    let lowered = is_leaf(&*m, n)? && count_next(m, n, i.wrapping_add(1), key, -1)?.is_some();
    remove_entry(m, root, n, i, path)?;
    if !lowered {
        add_count(m, root, key, -1, path)?;
    }
    Ok(true)
}

/// Removes entry i of node n, found by the descent in `path`, and hands back its key-log record when its key
/// is longer than W.
fn remove_entry<H: PageHost>(
    m: &mut AreaNodes<'_, H>,
    root: &mut u64,
    n: u64,
    i: u64,
    path: &mut Path,
) -> Result<(), Fault> {
    let s = m.shape;
    let (_, field) = field_at(s, &*m, n, i)?;
    tree::remove_found(s, m, root, n, i, path)?;
    if is_ovf(field) {
        m.free_record(payload(field))?;
    }
    Ok(())
}

/// When entry j of leaf n is the count entry of `key` and its count changed by `d` stays positive: the count
/// so changed in place, and returned. `None`, with nothing changed, otherwise.
fn count_next<H: PageHost>(m: &mut AreaNodes<'_, H>, n: u64, j: u64, key: &[u8], d: i64) -> Result<Option<i64>, Fault> {
    let s = m.shape;
    if j >= count(s, &*m, n)? {
        return Ok(None);
    }
    let koff = ko(s, &s.leaf, j);
    if (KeyProbe { key, kind: Kind::Count }).cmp_at(&*m, n, koff)? != Ordering::Equal {
        return Ok(None);
    }
    let c = count_at(s, &*m, n, j)?.checked_add(d).ok_or(rt(Corruption::Count, n))?;
    if c <= 0 {
        return Ok(None);
    }
    set_count(m, n, j, c)?;
    Ok(Some(c))
}

/// Changes the count of `key` in the tree by `d`, 1 or −1, through a descent: in place, or with a count
/// entry made for the key's first row or removed with its last. Returns whether a count entry was made.
fn add_count<H: PageHost>(
    m: &mut AreaNodes<'_, H>,
    root: &mut u64,
    key: &[u8],
    d: i64,
    path: &mut Path,
) -> Result<bool, Fault> {
    let s = m.shape;
    if *root == 0 {
        return Err(rt(Corruption::Count, 0));
    }
    path.clear();
    match tree::descend(s, &*m, *root, &KeyProbe { key, kind: Kind::Count }, path)? {
        Search::Found(n, i) => {
            let c = count_at(s, &*m, n, i)?.checked_add(d).ok_or(rt(Corruption::Count, n))?;
            match c.cmp(&0) {
                Ordering::Greater => set_count(m, n, i, c)?,
                Ordering::Equal => remove_entry(m, root, n, i, path)?,
                Ordering::Less => return Err(rt(Corruption::Count, n)),
            }
            Ok(false)
        }
        // A key's first row: its count entry, of one.
        Search::Vacant(n, j) if d == 1 => {
            let base = count_field(1);
            let field = stored_field(m, base, payload(base), key)?;
            area_insert::insert_vacant(m, root, n, j, key, field, &[], path)?;
            Ok(true)
        }
        // A row the tree holds whose key has no count entry.
        Search::Vacant(n, _) => Err(rt(Corruption::Count, n)),
    }
}

/// The offset in node n of entry i's field, and the field.
fn field_at<M: NodeRead + ?Sized>(s: IndexShape, m: &M, n: u64, i: u64) -> Result<(u64, u64), Fault> {
    let at = ko(s, &s.shape(is_leaf(m, n)?), i).wrapping_add(s.w);
    let field = u64::from_be_bytes(rd_array::<8>(m.bytes(n, at, 8)?, 0)?);
    Ok((at, field))
}

/// The count entry i of node n holds: from its field, or from its key-log record when its key is longer than
/// W. `Corrupt` when the entry is not a count entry.
fn count_at<M: NodeRead + ResolveKey + ?Sized>(s: IndexShape, m: &M, n: u64, i: u64) -> Result<i64, Fault> {
    let (_, field) = field_at(s, m, n, i)?;
    if !is_count(field) {
        return Err(rt(Corruption::Order, n));
    }
    Ok(if is_ovf(field) { payload_to_count(m.record(field)?.0) } else { payload_to_count(field) })
}

/// Sets the count of count entry i of node n to c: in its field, or in its key-log record.
fn set_count<H: PageHost>(m: &mut AreaNodes<'_, H>, n: u64, i: u64, c: i64) -> Result<(), Fault> {
    let (at, field) = field_at(m.shape, &*m, n, i)?;
    let new = count_field(c);
    if is_ovf(field) {
        return m.set_record_payload(payload(field), payload(new));
    }
    put(m.host.get_mut(Region::Block(n))?, at, &new.to_be_bytes())
}

/// The field an entry of `key` stores for `field`: `field` itself when the key fits W, else an overflowed
/// field naming the key's new key-log record, whose payload is `rec`.
fn stored_field<H: PageHost>(m: &mut AreaNodes<'_, H>, field: u64, rec: u64, key: &[u8]) -> Result<u64, Fault> {
    if key.len() as u64 > m.shape.w {
        Ok(ovf_field(field, m.new_record(rec, key)?))
    } else {
        Ok(field)
    }
}

/// Writes the key of entry i of node n at the start of `out`: its key area, then, when it is longer than W,
/// its bytes beyond W from the key log; returns the bytes written.
fn entry_key<H: PageHost + ?Sized>(v: &AreaView<'_, H>, n: u64, i: u64, out: &mut [u8]) -> Result<usize, Fault> {
    let s = v.shape;
    let (at, field) = field_at(s, v, n, i)?;
    let w = usize::try_from(s.w).map_err(|_| rt(Corruption::Layout, n))?;
    let area = v.bytes(n, at.wrapping_sub(s.w), s.w)?;
    let len = if is_ovf(field) { v.record(field)?.1 } else { s.w };
    let len = usize::try_from(len).map_err(|_| rt(Corruption::Range, n))?;
    let dst = out.get_mut(..len).ok_or(Fault::Scratch { need: len as u64 })?;
    let (head, tail) = dst.split_at_mut_checked(w.min(len)).ok_or(rt(Corruption::Bounds, n))?;
    copy_bytes(head, area.get(..head.len()).unwrap_or(&[]));
    let mut done = 0usize;
    while done < tail.len() {
        let got = v.key_piece(field, s.w.wrapping_add(done as u64), tail.get_mut(done..).unwrap_or(&mut []))?;
        if got == 0 {
            return Err(rt(Corruption::Bounds, n));
        }
        done = done.wrapping_add(got);
    }
    Ok(len)
}

/// The length of the key at the start of `buf`, of which `written` bytes were written (a key area's padding
/// included): its end by the key spec, at most `written`.
fn exact(spec: &KeySpec<'_>, buf: &[u8], written: usize) -> Result<usize, Fault> {
    let key = buf.get(..written).ok_or(rt(Corruption::Bounds, written as u64))?;
    let end = spec.end(key, ImageId::Runtime)?;
    usize::try_from(end).ok().filter(|&e| e <= written).ok_or(rt(Corruption::Encoding, end))
}

/// Copies `key` to the start of `out`; returns its length.
fn take(out: &mut [u8], key: &[u8]) -> Result<usize, Fault> {
    let dst = out.get_mut(..key.len()).ok_or(Fault::Scratch { need: key.len() as u64 })?;
    copy_bytes(dst, key);
    Ok(key.len())
}
