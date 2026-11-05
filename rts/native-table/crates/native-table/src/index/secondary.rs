//! The secondary index: one B-tree in table memory per index, holding every row the index has taken on
//! chain, with a count per key, beside the segments its uploads brought.
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
//! Removing an entry hands its record back (`area_nodes`), so the log follows the rows the tree holds. A key
//! too long for a stack buffer is changed through a `LongKey`, a record of the key log written first, and
//! counted through a `KeyFeed`, which hands it over again for each comparison (`count_fed`).
//!
//! **Uploaded segments.** The index data an upload brings stays apart: one run per upload in the loaded-run
//! list of the index's run list, never merged into the tree or with another run. A count adds each run's count
//! of the key (`runprobe::count_key`); a group walk adds each run's keys and totals, and the sorted walk
//! (`sorted`) each run's keys and rows.
//!
//! **Descriptor.** A table's secondary index `id` keeps its state in its index descriptor (`directory`): kind
//! 2 @0, the tree's W u16 @2, the key spec hash u64 @8, the tree's root u64 @16, node count u32 @24, key-log
//! blocks u32 @28, key-log radix root u64 @32 and height u8 @40, the rid its build takes next u64 @48 and the
//! rid the build ends before u64 @56 (both 0 when the tree holds every row), the key-log ordinal of the state
//! block of the build's key being written over units u64 @64 (0: none), the key-log ordinal of the state block
//! of a build key whose blocks are being handed back over units u64 @72 (0: none), and the run list block u64
//! @80.
//! The table makes it when it is created or a schema change adds the index, and writes the tree's fields
//! back whenever a write changes them.

use core::cmp::Ordering;
use core::ops::Bound;

use crate::consts::{magic, FORMAT_TABLE};
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::host::NodeMem;
use crate::image::ImageReader;
use crate::index::area_insert;
use crate::index::area_nodes::{AreaNodes, AreaView, KeyLog};
use crate::index::directory;
use crate::index::nodes::Run;
use crate::index::runlist::LoadedRecord;
use crate::index::runprobe;
use crate::key::probe::{
    count_field, is_count, is_ovf, ovf_field, payload, payload_to_count, row_field, FeedProbe, IndexPending, KeyFeed,
    KeyProbe, Kind, RecProbe, ResolveKey,
};
use crate::key::{KeySpec, LenSink};
use crate::mem::{
    copy_bytes, put, put_u16, put_u32, put_u64, put_u8, rd_array, rd_u16, rd_u32, rd_u64, rd_u8, Owner, PageHost,
    Region,
};
use crate::radix::Radix;
use crate::tree::node::{count, is_leaf, ko};
use crate::tree::{self, Cursor, Dir, Fwd, IndexShape, Path, Probe, Search, Shape};

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
const D_BUILD_NEXT: u64 = 48;
const D_BUILD_END: u64 = 56;
const D_BUILD_KEY: u64 = 64;
const D_BUILD_BACK: u64 = 72;
const D_RUN_LIST: u64 = 80;

fn rt(kind: Corruption, off: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, off)
}

/// One secondary index: its tree's state, kept by the index's owner, and where its uploaded segments are
/// listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Secondary {
    /// The index's id, which its runs' image ids carry.
    pub index: u8,
    /// The tree's shape, (W, 0).
    pub shape: IndexShape,
    /// The tree's root (0: empty).
    pub root: u64,
    /// The tree's node count.
    pub nodes: u64,
    /// The tree's key log.
    pub log: KeyLog,
    /// The block of the index's run list, which lists its uploaded segments (0: none).
    pub run_list: u64,
    /// Whether a build's key is being written over units (the descriptor names its state block).
    pub pending: bool,
}

impl Secondary {
    /// An empty index `index` of key width `w` (a multiple of 8 in 8..=1,016), with no uploaded segment;
    /// `None` for any other width. It holds no block until its first row.
    pub fn create(index: u8, w: u64) -> Option<Secondary> {
        Some(Secondary {
            index,
            shape: IndexShape::new(w, 0)?,
            root: 0,
            nodes: 0,
            log: KeyLog::default(),
            run_list: 0,
            pending: false,
        })
    }

    /// Makes table `tbl`'s descriptor of index `index` (1 to 63), for an empty tree of key width `w` and the key
    /// spec hash `hash`, with no uploaded segment; its directory block is made when needed.
    pub fn declare<H: PageHost>(h: &mut H, tbl: u64, index: u8, w: u64, hash: u64) -> Result<Secondary, Fault> {
        let s = Secondary::create(index, w).ok_or(rt(Corruption::Layout, w))?;
        if index == 0 {
            return Err(rt(Corruption::Range, 0));
        }
        let (b, o) = directory::ensure(h, tbl, index)?;
        let w16 = u16::try_from(w).map_err(|_| rt(Corruption::Layout, w))?;
        let blk = h.get_mut(Region::Block(b), Owner(0))?;
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
            run_list: rd_u64(blk, at(D_RUN_LIST))?,
            pending: rd_u64(blk, at(D_BUILD_KEY))? != 0,
        })
    }

    /// Clears table `tbl`'s descriptor of index `index`, whose tree and key log its owner has handed on: the
    /// table has no such index any more.
    pub fn clear<H: PageHost>(h: &mut H, tbl: u64, index: u8) -> Result<(), Fault> {
        let (b, o) = directory::locate(&*h, tbl, index)?.ok_or(rt(Corruption::Link, u64::from(index)))?;
        crate::mem::fill(h.get_mut(Region::Block(b), Owner(0))?, o, directory::INDEX_DESC, 0)
    }

    /// Table `tbl`'s build of index `index`: the rid it takes next and the rid it ends before (both 0 when the
    /// tree holds every row).
    pub fn build_state<H: PageHost + ?Sized>(h: &H, tbl: u64, index: u8) -> Result<(u64, u64), Fault> {
        let (b, o) = directory::locate(h, tbl, index)?.ok_or(rt(Corruption::Link, u64::from(index)))?;
        let blk = h.get(Region::Block(b))?;
        Ok((rd_u64(blk, o.wrapping_add(D_BUILD_NEXT))?, rd_u64(blk, o.wrapping_add(D_BUILD_END))?))
    }

    /// Writes table `tbl`'s build of index `index` into its descriptor (`build_state`).
    pub fn set_build_state<H: PageHost>(h: &mut H, tbl: u64, index: u8, next: u64, end: u64) -> Result<(), Fault> {
        let (b, o) = directory::locate(&*h, tbl, index)?.ok_or(rt(Corruption::Link, u64::from(index)))?;
        let blk = h.get_mut(Region::Block(b), Owner(0))?;
        put_u64(blk, o.wrapping_add(D_BUILD_NEXT), next)?;
        put_u64(blk, o.wrapping_add(D_BUILD_END), end)
    }

    /// The key-log ordinal of the state block of table `tbl`'s build of index `index`'s key being written over
    /// units (0: none).
    pub fn build_key<H: PageHost + ?Sized>(h: &H, tbl: u64, index: u8) -> Result<u64, Fault> {
        let (b, o) = directory::locate(h, tbl, index)?.ok_or(rt(Corruption::Link, u64::from(index)))?;
        rd_u64(h.get(Region::Block(b))?, o.wrapping_add(D_BUILD_KEY))
    }

    /// Writes `build_key` into table `tbl`'s descriptor of index `index`.
    pub fn set_build_key<H: PageHost>(h: &mut H, tbl: u64, index: u8, k: u64) -> Result<(), Fault> {
        let (b, o) = directory::locate(&*h, tbl, index)?.ok_or(rt(Corruption::Link, u64::from(index)))?;
        put_u64(h.get_mut(Region::Block(b), Owner(0))?, o.wrapping_add(D_BUILD_KEY), k)
    }

    /// The key-log ordinal of the state block of table `tbl`'s build of index `index`'s key whose blocks are
    /// being handed back over units (0: none).
    pub fn build_back<H: PageHost + ?Sized>(h: &H, tbl: u64, index: u8) -> Result<u64, Fault> {
        let (b, o) = directory::locate(h, tbl, index)?.ok_or(rt(Corruption::Link, u64::from(index)))?;
        rd_u64(h.get(Region::Block(b))?, o.wrapping_add(D_BUILD_BACK))
    }

    /// Writes `build_back` into table `tbl`'s descriptor of index `index`.
    pub fn set_build_back<H: PageHost>(h: &mut H, tbl: u64, index: u8, k: u64) -> Result<(), Fault> {
        let (b, o) = directory::locate(&*h, tbl, index)?.ok_or(rt(Corruption::Link, u64::from(index)))?;
        put_u64(h.get_mut(Region::Block(b), Owner(0))?, o.wrapping_add(D_BUILD_BACK), k)
    }

    /// Runs `f` on the tree's nodes, keeping the key log it leaves (a key written over units).
    pub fn with_log<H: PageHost, R, F: FnOnce(&mut AreaNodes<'_, H>) -> Result<R, Fault>>(
        &mut self,
        h: &mut H,
        tbl: u64,
        f: F,
    ) -> Result<R, Fault> {
        let mut m = self.tree(h, tbl);
        let r = f(&mut m);
        self.log = m.log;
        r
    }

    /// Writes the tree's root, node count and key log into table `tbl`'s descriptor of the index.
    pub fn write_tree<H: PageHost>(&self, h: &mut H, tbl: u64) -> Result<(), Fault> {
        let (b, o) = directory::locate(&*h, tbl, self.index)?.ok_or(rt(Corruption::Link, u64::from(self.index)))?;
        let nodes = u32::try_from(self.nodes).map_err(|_| rt(Corruption::Count, self.nodes))?;
        let blocks = u32::try_from(self.log.blocks).map_err(|_| rt(Corruption::Count, self.log.blocks))?;
        let blk = h.get_mut(Region::Block(b), Owner(0))?;
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

    /// `insert` under a key longer than a stack buffer holds, in the tree's key log already.
    pub fn insert_long<H: PageHost>(
        &mut self,
        h: &mut H,
        tbl: u64,
        key: LongKey<'_>,
        rid: u64,
        path: &mut Path,
    ) -> Result<bool, Fault> {
        self.insert_by(h, tbl, key, rid, path).map(|r| r.is_some())
    }

    #[inline(always)]
    fn insert_by<'k, H: PageHost, K: EntryKey<'k>>(
        &mut self,
        h: &mut H,
        tbl: u64,
        key: K,
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

    /// `delete` under a key longer than a stack buffer holds, in the tree's key log already.
    pub fn delete_long<H: PageHost>(
        &mut self,
        h: &mut H,
        tbl: u64,
        key: LongKey<'_>,
        rid: u64,
        path: &mut Path,
    ) -> Result<bool, Fault> {
        self.delete_by(h, tbl, key, rid, path)
    }

    #[inline(always)]
    fn delete_by<'k, H: PageHost, K: EntryKey<'k>>(
        &mut self,
        h: &mut H,
        tbl: u64,
        key: K,
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

    /// The tree's height: the nodes from its root down its leftmost path (0 when it is empty).
    pub fn height<H: PageHost + ?Sized>(&self, h: &H) -> Result<u64, Fault> {
        let (v, mut n, mut k) = (self.view(h), self.root, 0u64);
        while n != 0 {
            k = k.wrapping_add(1);
            if is_leaf(&v, n)? {
                break;
            }
            n = tree::node::child(self.shape, &v, n, 0)?;
        }
        Ok(k)
    }

    /// The nodes an insert of the row `rid` under `key`, its full encoding, and of its key's count entry may
    /// split: none when the row's leaf has room for both entries, else the leaf and every full node right above
    /// it on the row's path (a node splits once at most: the next entry finds room in one of its halves). `path`
    /// is the caller's scratch.
    pub fn splits<H: PageHost + ?Sized>(&self, h: &H, key: &[u8], rid: u64, path: &mut Path) -> Result<u64, Fault> {
        Ok(self.full_run(h, key, rid, path)?.0)
    }

    /// `splits`, with the highest node of that run, and `path` holding the frames above it.
    fn full_run<H: PageHost + ?Sized>(
        &self,
        h: &H,
        key: &[u8],
        rid: u64,
        path: &mut Path,
    ) -> Result<(u64, u64), Fault> {
        if self.root == 0 {
            return Ok((0, 0));
        }
        let (s, v) = (self.shape, self.view(h));
        path.clear();
        let leaf = match key.descend(s, &v, self.root, Kind::Row(rid), path)? {
            Search::Found(..) => return Ok((0, 0)),
            Search::Vacant(n, _) => n,
        };
        let (k, top, above) = full_above(s, &v, leaf, path)?;
        path.truncate(above);
        Ok((k, top))
    }

    /// `insert` for a build's row, which the tree does not hold yet: when its descent finds a run of two or
    /// more full nodes on the row's path (`splits`: its leaf without room for the row's two entries and at least
    /// the node above it full), which the tree's general insert splits, `fits` is asked first whether that run
    /// fits, and when it does not, nothing changes and `Ok(None)` is returned. Else the row goes in: `Ok(Some(k))`
    /// with the run k, 0 when it is under two nodes. A row the tree holds already is `Corrupt(Runtime, Link)`.
    pub fn insert_fitting<H: PageHost, F: FnOnce(u64) -> bool>(
        &mut self,
        h: &mut H,
        tbl: u64,
        key: &[u8],
        rid: u64,
        path: &mut Path,
        fits: F,
    ) -> Result<Option<u64>, Fault> {
        let mut m = self.tree(h, tbl);
        let r = insert_fitting(&mut m, &mut self.root, key, rid, path, fits);
        self.nodes = m.nodes;
        self.log = m.log;
        r
    }

    /// Splits the highest node of the run `splits` counts for the row `rid` under `key` ahead of its insert
    /// (`tree::split_ahead`), so the run is one node shorter: the step a build takes when the insert's splits
    /// would pass a unit's pages. Returns whether it split: not when the run is empty or is a leaf of fewer
    /// than three entries, which a split would leave a half without one.
    pub fn split_ahead<H: PageHost>(
        &mut self,
        h: &mut H,
        tbl: u64,
        key: &[u8],
        rid: u64,
        path: &mut Path,
    ) -> Result<bool, Fault> {
        let (k, n) = self.full_run(&*h, key, rid, path)?;
        if k == 0 || count(self.shape, &self.view(&*h), n)? < 3 {
            return Ok(false);
        }
        let mut m = self.tree(h, tbl);
        let r = tree::split_ahead(self.shape, &mut m, &mut self.root, n, path);
        self.nodes = m.nodes;
        self.log = m.log;
        r.map(|()| true)
    }

    /// Whether a build's insert step from node `at` (0: the root) reaches a leaf (`build_step`).
    pub fn last_step<H: PageHost + ?Sized>(&self, h: &H, at: u64) -> Result<bool, Fault> {
        if self.root == 0 {
            return Ok(true);
        }
        let v = self.view(h);
        let n = if at == 0 { self.root } else { at };
        Ok(is_leaf(&v, n)? || is_leaf(&v, tree::node::child(self.shape, &v, n, 0)?)?)
    }

    /// One step of a build's insert of the row `rid`, above every rid the tree holds, under `key`, its full
    /// encoding: made top-down, a node at a time, so that it can stop between steps. From node `at` (0: the
    /// root), which has room for one more entry, the row's path goes one node down, to a child split ahead
    /// when full (`full_ahead`), which is returned while it is internal. On the way, an entry of the path that
    /// is the key's count entry is raised by one, and `RAISED` set in `flags`; one that is a row entry of the
    /// key sets `SEEN`. At a leaf, the row's entry goes in, with a new count entry of one beside it when the
    /// key is new: the leaf has room for both, or splits once into its parent, which has room; 0 is returned.
    #[allow(clippy::too_many_arguments)]
    pub fn build_step<H: PageHost>(
        &mut self,
        h: &mut H,
        tbl: u64,
        key: &[u8],
        rid: u64,
        at: u64,
        flags: &mut u64,
        path: &mut Path,
    ) -> Result<u64, Fault> {
        let mut m = self.tree(h, tbl);
        let r = build_step(&mut m, &mut self.root, key, (rid, at), flags, path);
        self.nodes = m.nodes;
        self.log = m.log;
        r
    }

    /// Lowers the count of `key`, its full encoding, by one: a build's insert stopped after raising it.
    pub fn lower_count<H: PageHost>(&mut self, h: &mut H, tbl: u64, key: &[u8], path: &mut Path) -> Result<(), Fault> {
        let mut m = self.tree(h, tbl);
        let r = add_count(&mut m, &mut self.root, key, -1, path);
        self.nodes = m.nodes;
        self.log = m.log;
        r.map(|_| ())
    }

    /// The rows that hold `key`, its full encoding: its count entry in the tree, found by one descent, plus
    /// its count in every uploaded segment.
    pub fn count<H: PageHost + ?Sized>(&self, h: &H, key: &[u8]) -> Result<i128, Fault> {
        let mut sum = i128::from(self.tree_count(h, key)?);
        if self.run_list != 0 {
            sum = sum.wrapping_add(runprobe::count_key(h, self.run_list, self.index, key)?);
        }
        Ok(sum)
    }

    /// The count of `key` in the tree alone.
    pub fn tree_count<H: PageHost + ?Sized>(&self, h: &H, key: &[u8]) -> Result<i64, Fault> {
        self.tree_count_by(h, &KeyProbe { key, kind: Kind::Count })
    }

    /// The next group of the index in key order: the smallest key above `after` (the smallest of all when
    /// `None`) that the tree or an uploaded segment holds, written whole at the start of `out`, with its
    /// length and its count over the tree and every segment; `None` past the last key. `spec` is the index's
    /// key spec, which gives each key's end; `scratch` takes each segment's key in turn. A step is two descents
    /// of the tree (to the key after `after`, then to its count entry) and one key step in each segment, so a
    /// walk of g groups over s segments costs g·(2 + s) descents, whatever the rows. It keeps no state: a walk
    /// resumes from the key it handed out last.
    pub fn group_next<H: PageHost + ?Sized>(
        &self,
        h: &H,
        spec: &KeySpec<'_>,
        after: Option<&[u8]>,
        out: &mut [u8],
        scratch: &mut [u8],
    ) -> Result<Option<(usize, i128)>, Fault> {
        let mut best = match self.tree_next(h, after, out)? {
            Some((len, c)) => Some((exact(spec, out, len)?, i128::from(c))),
            None => None,
        };
        let key_run_len = key_run_len(spec);
        self.segments(h, |run| {
            let Some((len, total)) = run.key_step(after, Dir::Fwd, key_run_len, scratch)? else { return Ok(()) };
            let len = exact(spec, scratch, len)?;
            let key = scratch.get(..len).ok_or(Fault::Scratch { need: len as u64 })?;
            let total = i128::from(total);
            best = match best {
                Some((bl, bt)) => match key.cmp(out.get(..bl).unwrap_or(&[])) {
                    Ordering::Less => Some((take(out, key)?, total)),
                    Ordering::Equal => Some((bl, bt.wrapping_add(total))),
                    Ordering::Greater => Some((bl, bt)),
                },
                None => Some((take(out, key)?, total)),
            };
            Ok(())
        })?;
        Ok(best)
    }

    /// Calls `f` with each uploaded segment, opened, in upload order.
    fn segments<'h, H: PageHost + ?Sized>(
        &self,
        h: &'h H,
        mut f: impl FnMut(&Run<ImageReader<'h, H>>) -> Result<(), Fault>,
    ) -> Result<(), Fault> {
        let Some(loaded) = self.loaded(h)? else { return Ok(()) };
        let mut j = 0u64;
        while j < loaded.runs {
            f(&self.segment(h, &loaded, j)?)?;
            j = j.wrapping_add(1);
        }
        Ok(())
    }

    /// The list of the uploaded segments; `None` when the index has none.
    fn loaded<H: PageHost + ?Sized>(&self, h: &H) -> Result<Option<LoadedRecord>, Fault> {
        if self.run_list == 0 {
            return Ok(None);
        }
        let b = h.get(Region::Block(self.run_list))?;
        if rd_u32(b, 0)? != magic::NTRL || rd_u16(b, 4)? != FORMAT_TABLE {
            return Err(rt(Corruption::Header, self.run_list));
        }
        LoadedRecord::read(b).map(Some)
    }

    /// Uploaded segment j of `loaded`, opened.
    fn segment<'h, H: PageHost + ?Sized>(
        &self,
        h: &'h H,
        loaded: &LoadedRecord,
        j: u64,
    ) -> Result<Run<ImageReader<'h, H>>, Fault> {
        let d = loaded.get(h, j)?;
        let id = ImageId::Run { index: self.index, start_seq: d.start_seq };
        Run::open(ImageReader::new(h, id, d.image(h)?), d.rid_base, d.rid_lo, d.rid_hi)
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

    /// The tree's nodes, writable.
    fn tree<'h, H: PageHost>(&self, h: &'h mut H, tbl: u64) -> AreaNodes<'h, H> {
        let mut m = AreaNodes::new(h, tbl, AREA, self.shape, self.log);
        m.nodes = self.nodes;
        m
    }

    /// The tree's nodes, read through a shared borrow.
    fn view<'h, H: PageHost + ?Sized>(&self, h: &'h H) -> AreaView<'h, H> {
        AreaView::new(h, self.shape, self.log)
    }
}

/// A key a change of the tree is made under: its full encoding as bytes, or a `LongKey`.
pub trait EntryKey<'k>: Copy {
    /// A descent to the entry of this key of kind `kind`.
    fn descend<M: NodeMem + ResolveKey + ?Sized>(
        self,
        s: IndexShape,
        m: &M,
        root: u64,
        kind: Kind,
        path: &mut Path,
    ) -> Result<Search, Fault>;
    /// The comparator of the entry at `koff` of node n against this key of kind `kind`.
    fn cmp_at<M: NodeMem + ResolveKey + ?Sized>(self, m: &M, n: u64, koff: u64, kind: Kind) -> Result<Ordering, Fault>;
    /// The bytes an entry's key area takes: the whole key, or its first W bytes.
    fn area(self) -> &'k [u8];
    /// The field an entry of this key stores for `field`: `field` itself when the key fits W, else an overflowed
    /// field naming a new key-log record whose payload is `rec`.
    fn field<H: PageHost>(self, m: &mut AreaNodes<'_, H>, field: u64, rec: u64) -> Result<u64, Fault>;
    /// `field`, for an entry that takes the key's own record when it has one, its payload set to `rec`.
    fn own<H: PageHost>(self, m: &mut AreaNodes<'_, H>, field: u64, rec: u64) -> Result<u64, Fault> {
        self.field(m, field, rec)
    }
}

impl<'k> EntryKey<'k> for &'k [u8] {
    #[inline(always)]
    fn descend<M: NodeMem + ResolveKey + ?Sized>(
        self,
        s: IndexShape,
        m: &M,
        root: u64,
        kind: Kind,
        path: &mut Path,
    ) -> Result<Search, Fault> {
        tree::descend(s, m, root, &KeyProbe { key: self, kind }, path)
    }
    #[inline(always)]
    fn cmp_at<M: NodeMem + ResolveKey + ?Sized>(self, m: &M, n: u64, koff: u64, kind: Kind) -> Result<Ordering, Fault> {
        KeyProbe { key: self, kind }.cmp_at(m, n, koff)
    }
    #[inline(always)]
    fn area(self) -> &'k [u8] {
        self
    }
    #[inline(always)]
    fn field<H: PageHost>(self, m: &mut AreaNodes<'_, H>, field: u64, rec: u64) -> Result<u64, Fault> {
        stored_field(m, field, rec, self)
    }
}

/// A key longer than a stack buffer holds, written into the tree's key log as a record first: its first W
/// bytes `head`, the record at `loc` holding the rest, and its length `len`. Descents compare the stored keys
/// with that record (`RecProbe`), and each entry made under it takes a copy of the record, or the record
/// itself (`Secondary::place_own`).
#[derive(Clone, Copy, Debug)]
pub struct LongKey<'k> {
    pub head: &'k [u8],
    pub loc: u64,
    pub len: u64,
}

impl LongKey<'_> {
    fn probe(&self, kind: Kind) -> RecProbe<'_> {
        RecProbe { head: self.head, field: ovf_field(row_field(0), self.loc), len: self.len, kind }
    }
}

impl<'k> EntryKey<'k> for LongKey<'k> {
    fn descend<M: NodeMem + ResolveKey + ?Sized>(
        self,
        s: IndexShape,
        m: &M,
        root: u64,
        kind: Kind,
        path: &mut Path,
    ) -> Result<Search, Fault> {
        tree::descend(s, m, root, &self.probe(kind), path)
    }
    fn cmp_at<M: NodeMem + ResolveKey + ?Sized>(self, m: &M, n: u64, koff: u64, kind: Kind) -> Result<Ordering, Fault> {
        self.probe(kind).cmp_at(m, n, koff)
    }
    fn area(self) -> &'k [u8] {
        self.head
    }
    fn field<H: PageHost>(self, m: &mut AreaNodes<'_, H>, field: u64, rec: u64) -> Result<u64, Fault> {
        Ok(ovf_field(field, m.copy_record(self.loc, rec)?))
    }
    fn own<H: PageHost>(self, m: &mut AreaNodes<'_, H>, field: u64, rec: u64) -> Result<u64, Fault> {
        m.set_record_payload(self.loc, rec)?;
        Ok(ovf_field(field, self.loc))
    }
}

/// A `LongKey` whose entries take records of the key made for them: the row entry the key's own record
/// (`EntryKey::own`), a count entry made `twin`, a second record of the key, or a copy when `twin` is 0.
#[derive(Clone, Copy, Debug)]
struct Owned<'k> {
    key: LongKey<'k>,
    twin: u64,
}

impl<'k> EntryKey<'k> for Owned<'k> {
    fn descend<M: NodeMem + ResolveKey + ?Sized>(
        self,
        s: IndexShape,
        m: &M,
        root: u64,
        kind: Kind,
        path: &mut Path,
    ) -> Result<Search, Fault> {
        self.key.descend(s, m, root, kind, path)
    }
    fn cmp_at<M: NodeMem + ResolveKey + ?Sized>(self, m: &M, n: u64, koff: u64, kind: Kind) -> Result<Ordering, Fault> {
        self.key.cmp_at(m, n, koff, kind)
    }
    fn area(self) -> &'k [u8] {
        self.key.head
    }
    fn field<H: PageHost>(self, m: &mut AreaNodes<'_, H>, field: u64, rec: u64) -> Result<u64, Fault> {
        if self.twin == 0 {
            return self.key.field(m, field, rec);
        }
        m.set_record_payload(self.twin, rec)?;
        Ok(ovf_field(field, self.twin))
    }
    fn own<H: PageHost>(self, m: &mut AreaNodes<'_, H>, field: u64, rec: u64) -> Result<u64, Fault> {
        self.key.own(m, field, rec)
    }
}

impl Secondary {
    /// A new record of the tree's key log for a key of `len` bytes, longer than W, whose bytes beyond W are
    /// written next (`write_key`); its locator. Its payload is 0: it stands for a `LongKey`, and is handed back
    /// (`free_key`) once the change made under it is done.
    pub fn new_key<H: PageHost>(&mut self, h: &mut H, tbl: u64, len: u64) -> Result<u64, Fault> {
        let mut m = self.tree(h, tbl);
        let loc = m.alloc_record(0, len)?;
        self.log = m.log;
        Ok(loc)
    }

    /// Writes bytes [at, at + bytes.len()) of the key of `len` bytes whose record `new_key` made at `loc`.
    pub fn write_key<H: PageHost>(
        &self,
        h: &mut H,
        tbl: u64,
        loc: u64,
        len: u64,
        at: u64,
        bytes: &[u8],
    ) -> Result<(), Fault> {
        self.tree(h, tbl).write_key(loc, len, at, bytes)
    }

    /// Hands back the record `new_key` made at `loc`.
    pub fn free_key<H: PageHost>(&mut self, h: &mut H, tbl: u64, loc: u64) -> Result<(), Fault> {
        let mut m = self.tree(h, tbl);
        m.free_record(loc)?;
        self.log = m.log;
        Ok(())
    }

    /// The rows that hold the key `key` hands over, never held whole: its count entry in the tree, found by one
    /// descent comparing each stored key with the key's bytes as they come, plus its count in every uploaded
    /// segment, counted the same way (`Run::count_fed`).
    pub fn count_fed<H: PageHost + ?Sized, F: KeyFeed + ?Sized>(&self, h: &H, key: &F) -> Result<i128, Fault> {
        let mut sum = i128::from(self.tree_count_by(h, &FeedProbe { key, skip: 0, kind: Kind::Count })?);
        if self.run_list != 0 {
            let mut len = LenSink(0);
            key.feed(&mut len)?;
            self.segments(h, |run| {
                sum = sum.wrapping_add(i128::from(run.count_fed(key, len.0)?));
                Ok(())
            })?;
        }
        Ok(sum)
    }

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
fn insert_row<'k, H: PageHost, K: EntryKey<'k>>(
    m: &mut AreaNodes<'_, H>,
    root: &mut u64,
    key: K,
    rid: u64,
    path: &mut Path,
) -> Result<Option<bool>, Fault> {
    let s = m.shape;
    let row = |m: &mut AreaNodes<'_, H>| key.field(m, row_field(rid), rid);
    if *root == 0 {
        let field = row(m)?;
        *root = tree::insert_first(s, m, &IndexPending::Borrowed { key: key.area(), field, value: &[], child: 0 })?;
        return add_count(m, root, key, 1, path).map(Some);
    }
    path.clear();
    let (n, j) = match key.descend(s, &*m, *root, Kind::Row(rid), path)? {
        Search::Found(..) => return Ok(None),
        Search::Vacant(n, j) => (n, j),
    };
    // The count entry the row lands before, raised first; the row then moves it up by one.
    let raised = count_next(m, n, j, key, 1)?.is_some();
    let field = row(m)?;
    area_insert::insert_vacant(m, root, n, j, key.area(), field, &[], path)?;
    if raised {
        return Ok(Some(false));
    }
    add_count(m, root, key, 1, path).map(Some)
}

/// The run of full nodes an insert into leaf `leaf` splits, `path` holding the frames above the leaf
/// (`Secondary::splits`): none when the leaf has room for two more entries, else the leaf and every full node
/// right above it. With the run's highest node and the frames above that node.
fn full_above<M: NodeMem + ?Sized>(s: IndexShape, m: &M, leaf: u64, path: &Path) -> Result<(u64, u64, usize), Fault> {
    if count(s, m, leaf)?.saturating_add(2) <= s.leaf.cap {
        return Ok((0, 0, path.len()));
    }
    let (mut k, mut top, mut above) = (1u64, leaf, path.len());
    for (d, &(n, _)) in path.frames().iter().enumerate().rev() {
        if count(s, m, n)? < s.internal.cap {
            break;
        }
        (k, top, above) = (k.wrapping_add(1), n, d);
    }
    Ok((k, top, above))
}

/// `Secondary::insert_fitting` on the tree `m` with root `root`.
fn insert_fitting<H: PageHost, F: FnOnce(u64) -> bool>(
    m: &mut AreaNodes<'_, H>,
    root: &mut u64,
    key: &[u8],
    rid: u64,
    path: &mut Path,
    fits: F,
) -> Result<Option<u64>, Fault> {
    if *root == 0 {
        // An empty tree's first entry makes its root leaf: no run.
        insert_row(m, root, key, rid, path)?;
        return Ok(Some(0));
    }
    let s = m.shape;
    path.clear();
    let (n, j) = match key.descend(s, &*m, *root, Kind::Row(rid), path)? {
        Search::Found(..) => return Err(rt(Corruption::Link, rid)),
        Search::Vacant(n, j) => (n, j),
    };
    let k = full_above(s, &*m, n, path)?.0;
    let k = if k >= 2 { k } else { 0 };
    if k != 0 && !fits(k) {
        return Ok(None);
    }
    // Then as `insert_row`.
    let raised = count_next(m, n, j, key, 1)?.is_some();
    let field = key.field(m, row_field(rid), rid)?;
    area_insert::insert_vacant(m, root, n, j, key.area(), field, &[], path)?;
    if !raised {
        add_count(m, root, key, 1, path)?;
    }
    Ok(Some(k))
}

/// The flag of a build's insert step that raised its key's count on the row's path (`build_step`).
pub const RAISED: u64 = 1 << 63;
/// The flag of a build's insert step that passed a row entry of its key after the row (`build_step`).
pub const SEEN: u64 = 1 << 62;

/// Whether node n splits ahead of a build's insert passing it: an internal node when full, a leaf when it has
/// no room for two more entries and holds three or more (its halves then have room for two).
fn full_ahead<M: NodeMem + ?Sized>(s: IndexShape, m: &M, n: u64) -> Result<bool, Fault> {
    let c = count(s, m, n)?;
    Ok(if is_leaf(m, n)? { c.saturating_add(2) > s.leaf.cap && c >= 3 } else { c >= s.internal.cap })
}

/// Entry j of node n, the next entry after the build's row there, against `key`: when it is the key's count
/// entry, its count raised by one and `RAISED`; `SEEN` when it is a row entry of the key (a row taken after the
/// build's end), the count entry then lying further on; 0 otherwise.
fn raise_at<H: PageHost>(m: &mut AreaNodes<'_, H>, n: u64, j: u64, key: &[u8]) -> Result<u64, Fault> {
    let s = m.shape;
    if j >= count(s, &*m, n)? {
        return Ok(0);
    }
    let koff = ko(s, &s.shape(is_leaf(&*m, n)?), j);
    match (KeyProbe { key, kind: Kind::Count }).cmp_at(&*m, n, koff)? {
        Ordering::Equal => {
            let c = count_at(s, &*m, n, j)?.checked_add(1).ok_or(rt(Corruption::Count, n))?;
            set_count(m, n, j, c)?;
            Ok(RAISED)
        }
        Ordering::Less => Ok(SEEN),
        Ordering::Greater => Ok(0),
    }
}

/// `Secondary::build_step` on the tree `m` with root `root`, for the row `rid` from node `at`.
fn build_step<H: PageHost>(
    m: &mut AreaNodes<'_, H>,
    root: &mut u64,
    key: &[u8],
    (rid, at): (u64, u64),
    flags: &mut u64,
    path: &mut Path,
) -> Result<u64, Fault> {
    let s = m.shape;
    if *root == 0 {
        insert_row(m, root, key, rid, path)?;
        return Ok(0);
    }
    path.clear();
    let mut n = at;
    if n == 0 {
        n = *root;
        if full_ahead(s, &*m, n)? {
            tree::split_ahead(s, m, root, n, path)?;
            n = *root;
        }
    }
    let p = KeyProbe { key, kind: Kind::Row(rid) };
    let mut parent = None;
    loop {
        let j = match tree::search(s, &*m, n, &p, &mut tree::Shared::default())? {
            Ok(_) => return Err(rt(Corruption::Link, rid)),
            Err(j) => j,
        };
        if *flags & RAISED == 0 {
            *flags |= raise_at(m, n, j, key)?;
        }
        if is_leaf(&*m, n)? {
            place(m, root, (n, j), key, rid, *flags, parent, path)?;
            return Ok(0);
        }
        let c = tree::node::child(s, &*m, n, j)?;
        if full_ahead(s, &*m, c)? {
            path.clear();
            path.push(n, j)?;
            tree::split_ahead(s, m, root, c, path)?;
            continue;
        }
        if !is_leaf(&*m, c)? {
            return Ok(c);
        }
        (parent, n) = (Some((n, j)), c);
    }
}

/// Places the row `rid` under `key` at the vacant position (n, j) of a leaf whose parent frame is `parent`,
/// its key's count raised through a descent when `flags` saw a row of the key after it, or a count entry of
/// one beside it when the key is new; nothing more when a step `RAISED` the count.
/// The row entry takes the key's record as its own (`EntryKey::own`). Returns whether the count entry was made.
#[allow(clippy::too_many_arguments)]
fn place<'k, H: PageHost, K: EntryKey<'k>>(
    m: &mut AreaNodes<'_, H>,
    root: &mut u64,
    (n, j): (u64, u64),
    key: K,
    rid: u64,
    flags: u64,
    parent: Option<(u64, u64)>,
    path: &mut Path,
) -> Result<bool, Fault> {
    let frame = |path: &mut Path| {
        path.clear();
        parent.map_or(Ok(()), |(p, i)| path.push(p, i))
    };
    let new = flags & (RAISED | SEEN) == 0;
    if new {
        let base = count_field(1);
        let field = key.field(m, base, payload(base))?;
        frame(path)?;
        area_insert::insert_vacant(m, root, n, j, key.area(), field, &[], path)?;
    }
    let field = key.own(m, row_field(rid), rid)?;
    frame(path)?;
    area_insert::insert_vacant(m, root, n, j, key.area(), field, &[], path)?;
    if flags & (RAISED | SEEN) == SEEN {
        add_count(m, root, key, 1, path)?;
    }
    Ok(new)
}

/// The flag of a build's insert under a long key whose node search is under way (`LongAt`).
pub const MID: u64 = 1 << 61;
/// What entry `hi` of a node search under a long key is to the key: its count entry, or a row entry of it.
const HI_COUNT: u64 = 1 << 60;
const HI_ROW: u64 = 1 << 59;
/// The flags a build's insert keeps beside its node.
pub const AT_FLAGS: u64 = RAISED | SEEN | MID | HI_COUNT | HI_ROW;

/// Where a build's insert under a `LongKey` made over units stands (`Secondary::long_step`): the node its
/// search is in (0: not begun), with `RAISED`, `SEEN` and, while that node's search is under way, `MID`; the
/// search's bounds `lo` and `hi`; the key's bytes known equal to those of the entries just outside them
/// (`lcp_lo`, `lcp_hi`), from which each comparison between them starts; the bytes of the entry between them
/// found equal so far (`off`); and the frame above the node (`parent`, `pidx`), which a leaf's split needs.
#[derive(Clone, Copy, Debug, Default)]
pub struct LongAt {
    pub at: u64,
    pub flags: u64,
    pub lo: u64,
    pub hi: u64,
    pub lcp_lo: u64,
    pub lcp_hi: u64,
    pub off: u64,
    pub parent: u64,
    pub pidx: u64,
}

impl Secondary {
    /// Whether a build's insert under a long key at `st` has its row's place in a leaf (or the tree is empty),
    /// so that the next step takes the row (`place_own`).
    pub fn long_ready<H: PageHost + ?Sized>(&self, h: &H, st: &LongAt) -> Result<bool, Fault> {
        if self.root == 0 {
            return Ok(true);
        }
        Ok(st.flags & MID != 0 && st.lo >= st.hi && st.at != 0 && is_leaf(&self.view(h), st.at)?)
    }

    /// One step of a build's insert of the row `rid`, above every rid the tree holds, under `key`, a key
    /// longer than a stack buffer holds: `build_step` with the comparisons of a node's search made from the
    /// bytes its bounds share with the key, at most `budget` of the key's bytes in all (lowered by those
    /// read), resumed from the byte they reached. Returns once the budget is spent, the path has gone one node
    /// down, or the row's place in its leaf is found (`long_ready`).
    #[allow(clippy::too_many_arguments)]
    pub fn long_step<H: PageHost>(
        &mut self,
        h: &mut H,
        tbl: u64,
        key: LongKey<'_>,
        rid: u64,
        st: &mut LongAt,
        budget: &mut u64,
        path: &mut Path,
    ) -> Result<(), Fault> {
        let mut m = self.tree(h, tbl);
        let r = long_step(&mut m, &mut self.root, key, rid, st, budget, path);
        self.nodes = m.nodes;
        self.log = m.log;
        r
    }

    /// Takes the build's row `rid` at the place `long_step` found, under `key`, whose record its row entry
    /// takes as its own; a new count entry of one takes `twin`, a second record of the key (a copy when 0).
    /// Returns whether `twin` was taken.
    #[allow(clippy::too_many_arguments)]
    pub fn place_own<H: PageHost>(
        &mut self,
        h: &mut H,
        tbl: u64,
        key: LongKey<'_>,
        twin: u64,
        rid: u64,
        st: &LongAt,
        path: &mut Path,
    ) -> Result<bool, Fault> {
        let mut m = self.tree(h, tbl);
        let k = Owned { key, twin };
        let r = if self.root == 0 {
            place_first(&mut m, &mut self.root, k, rid, path)
        } else {
            let parent = if st.parent == 0 { None } else { Some((st.parent, st.pidx)) };
            place(&mut m, &mut self.root, (st.at, st.lo), k, rid, st.flags, parent, path)
        };
        self.nodes = m.nodes;
        self.log = m.log;
        r
    }

    /// One step of a descent under `key`, a key longer than a stack buffer holds, to its count entry, changed
    /// by `d` once found: `long_step`'s node search, its comparisons made from the bytes its bounds share with
    /// the key, at most `budget` of the key's bytes in all (lowered by those read), resumed from the byte they
    /// reached. Only the count changes, so a row's place another descent found stays where it is. Returns
    /// whether the count is changed; `Corrupt` when the key has no count entry, or the count would fall to 0.
    pub fn count_step<H: PageHost>(
        &mut self,
        h: &mut H,
        tbl: u64,
        key: LongKey<'_>,
        d: i64,
        st: &mut LongAt,
        budget: &mut u64,
    ) -> Result<bool, Fault> {
        if self.root == 0 {
            return Err(rt(Corruption::Count, 0));
        }
        let mut m = self.tree(h, tbl);
        count_step(&mut m, self.root, key, d, st, budget)
    }
}

/// `place` into an empty tree: its root made with the key's count entry of one, the row's entry before it, so
/// no comparison reads the key.
fn place_first<'k, H: PageHost, K: EntryKey<'k>>(
    m: &mut AreaNodes<'_, H>,
    root: &mut u64,
    key: K,
    rid: u64,
    path: &mut Path,
) -> Result<bool, Fault> {
    let base = count_field(1);
    let field = key.field(m, base, payload(base))?;
    *root = tree::insert_first(m.shape, m, &IndexPending::Borrowed { key: key.area(), field, value: &[], child: 0 })?;
    let field = key.own(m, row_field(rid), rid)?;
    path.clear();
    let n = *root;
    area_insert::insert_vacant(m, root, n, 0, key.area(), field, &[], path)?;
    Ok(true)
}

/// `Secondary::long_step` on the tree `m` with root `root`, not empty.
#[allow(clippy::too_many_arguments)]
fn long_step<H: PageHost>(
    m: &mut AreaNodes<'_, H>,
    root: &mut u64,
    key: LongKey<'_>,
    rid: u64,
    st: &mut LongAt,
    budget: &mut u64,
    path: &mut Path,
) -> Result<(), Fault> {
    let s = m.shape;
    if st.flags & MID == 0 {
        if st.at == 0 {
            let mut n = *root;
            if full_ahead(s, &*m, n)? {
                path.clear();
                tree::split_ahead(s, m, root, n, path)?;
                n = *root;
            }
            (st.at, st.lcp_lo, st.lcp_hi, st.parent, st.pidx) = (n, 0, 0, 0, 0);
        }
        (st.lo, st.hi) = (0, count(s, &*m, st.at)?);
        st.off = st.lcp_lo.min(st.lcp_hi);
        st.flags = (st.flags & !(HI_COUNT | HI_ROW)) | MID;
    }
    let n = st.at;
    let leaf = is_leaf(&*m, n)?;
    let ns = s.shape(leaf);
    let p = key.probe(Kind::Row(rid));
    loop {
        while st.lo < st.hi {
            if *budget == 0 {
                return Ok(());
            }
            let mid = st.lo.wrapping_add(st.hi.wrapping_sub(st.lo) / 2);
            let r = p.cmp_part(&*m, n, ko(s, &ns, mid), st.off, *budget)?;
            let at = match r {
                Ok(c) => c.at,
                Err(at) => at,
            };
            *budget = budget.saturating_sub(at.saturating_sub(st.off));
            let c = match r {
                Ok(c) => c,
                Err(at) => {
                    st.off = at;
                    return Ok(());
                }
            };
            match c.ord {
                Ordering::Less => (st.lo, st.lcp_lo) = (mid.wrapping_add(1), c.shared),
                Ordering::Greater => {
                    (st.hi, st.lcp_hi) = (mid, c.shared);
                    st.flags &= !(HI_COUNT | HI_ROW);
                    st.flags |= match c.count {
                        Ordering::Equal => HI_COUNT,
                        Ordering::Less => HI_ROW,
                        Ordering::Greater => 0,
                    };
                }
                Ordering::Equal => return Err(rt(Corruption::Link, rid)),
            }
            st.off = st.lcp_lo.min(st.lcp_hi);
        }
        let j = st.lo;
        // Entry j, the next after the row's place, was compared: what it is to the key is known.
        if st.flags & RAISED == 0 && st.flags & HI_COUNT != 0 {
            let c = count_at(s, &*m, n, j)?.checked_add(1).ok_or(rt(Corruption::Count, n))?;
            set_count(m, n, j, c)?;
            st.flags |= RAISED;
        } else if st.flags & RAISED == 0 && st.flags & HI_ROW != 0 {
            st.flags |= SEEN;
        }
        st.flags &= !(HI_COUNT | HI_ROW);
        if leaf {
            return Ok(());
        }
        let c = tree::node::child(s, &*m, n, j)?;
        if full_ahead(s, &*m, c)? {
            // Its middle entry comes up to j, between the entries the search's bounds are: one comparison more.
            path.clear();
            path.push(n, j)?;
            tree::split_ahead(s, m, root, c, path)?;
            st.hi = j.wrapping_add(1);
            continue;
        }
        (st.parent, st.pidx, st.at) = (n, j, c);
        st.flags &= !MID;
        return Ok(());
    }
}

/// `Secondary::count_step` on the tree `m` with root `root`, not empty. `st` keeps the node searched (0: not
/// begun), `MID` while its search is under way, the search's bounds, the bytes they share with the key and
/// those of the entry between them found equal so far.
fn count_step<H: PageHost>(
    m: &mut AreaNodes<'_, H>,
    root: u64,
    key: LongKey<'_>,
    d: i64,
    st: &mut LongAt,
    budget: &mut u64,
) -> Result<bool, Fault> {
    let s = m.shape;
    if st.flags & MID == 0 {
        if st.at == 0 {
            (st.at, st.lcp_lo, st.lcp_hi) = (root, 0, 0);
        }
        (st.lo, st.hi) = (0, count(s, &*m, st.at)?);
        st.off = st.lcp_lo.min(st.lcp_hi);
        st.flags |= MID;
    }
    let n = st.at;
    let ns = s.shape(is_leaf(&*m, n)?);
    let p = key.probe(Kind::Count);
    while st.lo < st.hi {
        if *budget == 0 {
            return Ok(false);
        }
        let mid = st.lo.wrapping_add(st.hi.wrapping_sub(st.lo) / 2);
        let r = p.cmp_part(&*m, n, ko(s, &ns, mid), st.off, *budget)?;
        let at = match r {
            Ok(c) => c.at,
            Err(at) => at,
        };
        *budget = budget.saturating_sub(at.saturating_sub(st.off));
        let c = match r {
            Ok(c) => c,
            Err(at) => {
                st.off = at;
                return Ok(false);
            }
        };
        match c.ord {
            Ordering::Less => (st.lo, st.lcp_lo) = (mid.wrapping_add(1), c.shared),
            Ordering::Greater => (st.hi, st.lcp_hi) = (mid, c.shared),
            Ordering::Equal => {
                let v = count_at(s, &*m, n, mid)?.checked_add(d).filter(|&v| v > 0).ok_or(rt(Corruption::Count, n))?;
                set_count(m, n, mid, v)?;
                return Ok(true);
            }
        }
        st.off = st.lcp_lo.min(st.lcp_hi);
    }
    if is_leaf(&*m, n)? {
        return Err(rt(Corruption::Count, n));
    }
    st.at = tree::node::child(s, &*m, n, st.lo)?;
    st.flags &= !MID;
    Ok(false)
}

/// `Secondary::delete` on the tree `m` with root `root`, not empty.
fn delete_row<'k, H: PageHost, K: EntryKey<'k>>(
    m: &mut AreaNodes<'_, H>,
    root: &mut u64,
    key: K,
    rid: u64,
    path: &mut Path,
) -> Result<bool, Fault> {
    let s = m.shape;
    path.clear();
    let (n, i) = match key.descend(s, &*m, *root, Kind::Row(rid), path)? {
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
fn count_next<'k, H: PageHost, K: EntryKey<'k>>(
    m: &mut AreaNodes<'_, H>,
    n: u64,
    j: u64,
    key: K,
    d: i64,
) -> Result<Option<i64>, Fault> {
    let s = m.shape;
    if j >= count(s, &*m, n)? {
        return Ok(None);
    }
    let koff = ko(s, &s.leaf, j);
    if key.cmp_at(&*m, n, koff, Kind::Count)? != Ordering::Equal {
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
fn add_count<'k, H: PageHost, K: EntryKey<'k>>(
    m: &mut AreaNodes<'_, H>,
    root: &mut u64,
    key: K,
    d: i64,
    path: &mut Path,
) -> Result<bool, Fault> {
    let s = m.shape;
    if *root == 0 {
        return Err(rt(Corruption::Count, 0));
    }
    path.clear();
    match key.descend(s, &*m, *root, Kind::Count, path)? {
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
            let field = key.field(m, base, payload(base))?;
            area_insert::insert_vacant(m, root, n, j, key.area(), field, &[], path)?;
            Ok(true)
        }
        // A row the tree holds whose key has no count entry.
        Search::Vacant(n, _) => Err(rt(Corruption::Count, n)),
    }
}

/// The offset in node n of entry i's field, and the field.
fn field_at<M: NodeMem + ?Sized>(s: IndexShape, m: &M, n: u64, i: u64) -> Result<(u64, u64), Fault> {
    let at = ko(s, &s.shape(is_leaf(m, n)?), i).wrapping_add(s.w);
    let field = u64::from_be_bytes(rd_array::<8>(&m.bytes(n, at, 8)?, 0)?);
    Ok((at, field))
}

/// The count entry i of node n holds: from its field, or from its key-log record when its key is longer than
/// W. `Corrupt` when the entry is not a count entry.
fn count_at<M: NodeMem + ResolveKey + ?Sized>(s: IndexShape, m: &M, n: u64, i: u64) -> Result<i64, Fault> {
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
    put(m.host.get_mut(Region::Block(n), Owner(m.area))?, at, &new.to_be_bytes())
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

/// The length of the keys of key runs, which a run stores only for keys longer than its key area: the index's
/// one key length, when it has one.
fn key_run_len(spec: &KeySpec<'_>) -> u64 {
    spec.fixed_len().unwrap_or(0)
}

/// Copies `key` to the start of `out`; returns its length.
fn take(out: &mut [u8], key: &[u8]) -> Result<usize, Fault> {
    let dst = out.get_mut(..key.len()).ok_or(Fault::Scratch { need: key.len() as u64 })?;
    copy_bytes(dst, key);
    Ok(key.len())
}
