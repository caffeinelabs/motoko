//! The primary index's tree M: the index directory block that describes it, the point lookup, and
//! the insert and removal of a row's entry (E(k), rid).
//!
//! **Index directory block 0** (area 0): {magic `NTIX` u32 @0; `FORMAT_TABLE` u16 @4; first index id u8 @6;
//! descriptors u8 @7; zero to 64}, then 31 descriptors of 128 bytes; the primary index is descriptor 0. A
//! descriptor holds {kind u8 @0 (1 primary); state u8 @1; W u16 @2; flags u16 @4 (bit 2 keys unique); key
//! spec hash u64 @8; M root u64 @16; M nodes u32 @24; M key-log blocks u32 @28; M key-log radix root u64
//! @32; M key-log radix height u8 @40; …}. It changes only when M's root, node count or key log change.
//!
//! M's entries have no value part. A key longer than W keeps its first W bytes in the entry and the rest in
//! M's key log, whose record payload is the rid; removing the entry hands its record back.

use crate::area::alloc_zeroed;
use crate::consts::{magic, FORMAT_TABLE};
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::index::area_insert;
use crate::index::area_nodes::{AreaNodes, AreaView, KeyLog};
use crate::key::probe::{cmp_entry, is_ovf, ovf_field, payload, row_field, IndexPending, KeyProbe, Kind, ResolveKey};
use crate::mem::{put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u64, rd_u8, Owner, PageHost, Region};
use crate::radix::Radix;
use crate::tree::{self, IndexShape, Path, Search, Shape as _};
use crate::NodeMem;

use super::root;

/// Offsets in the index directory block.
const DESCRIPTORS: u64 = 64;
const D_KIND: u64 = 0;
const D_W: u64 = 2;
const D_FLAGS: u64 = 4;
const D_HASH: u64 = 8;
const D_M_ROOT: u64 = 16;
const D_M_NODES: u64 = 24;
const D_M_LOG_BLOCKS: u64 = 28;
const D_M_LOG_ROOT: u64 = 32;
const D_M_LOG_HEIGHT: u64 = 40;
/// Kind 1: the primary index.
const KIND_PRIMARY: u8 = 1;
/// Flags bit 2: keys unique.
const KEYS_UNIQUE: u16 = 4;

fn rt(kind: Corruption, at: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, at)
}

/// Depth faults over table memory name the runtime structure.
fn depth(f: Fault, root: u64) -> Fault {
    if f == Fault::PathOverflow {
        rt(Corruption::Depth, root)
    } else {
        f
    }
}

/// The primary index's M as its descriptor records it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Primary {
    pub dir: u64,
    pub shape: IndexShape,
    pub root: u64,
    pub nodes: u64,
    pub log: KeyLog,
}

impl Primary {
    /// The primary index, or `None` before the table's first index write made its directory block.
    pub fn read<H: PageHost + ?Sized>(h: &H, tbl: u64) -> Result<Option<Primary>, Fault> {
        let dir = root::u64_at(h, tbl, root::INDEX_DIRS)?;
        if dir == 0 {
            return Ok(None);
        }
        let b = h.get(Region::Block(dir))?;
        let at = DESCRIPTORS;
        if rd_u32(b, 0)? != magic::NTIX || rd_u8(b, at.wrapping_add(D_KIND))? != KIND_PRIMARY {
            return Err(rt(Corruption::Header, dir));
        }
        let w = u64::from(rd_u16(b, at.wrapping_add(D_W))?);
        let shape = IndexShape::new(w, 0).ok_or(rt(Corruption::Layout, dir))?;
        let log = KeyLog {
            radix: Radix {
                root: rd_u64(b, at.wrapping_add(D_M_LOG_ROOT))?,
                height: rd_u8(b, at.wrapping_add(D_M_LOG_HEIGHT))?,
            },
            blocks: u64::from(rd_u32(b, at.wrapping_add(D_M_LOG_BLOCKS))?),
        };
        Ok(Some(Primary {
            dir,
            shape,
            root: rd_u64(b, at.wrapping_add(D_M_ROOT))?,
            nodes: u64::from(rd_u32(b, at.wrapping_add(D_M_NODES))?),
            log,
        }))
    }

    /// The primary index, making index directory block 0 with an empty M of width `w` when it is not made yet.
    pub fn ensure<H: PageHost>(h: &mut H, tbl: u64, w: u64, hash: u64) -> Result<Primary, Fault> {
        if let Some(p) = Primary::read(h, tbl)? {
            return Ok(p);
        }
        let shape = IndexShape::new(w, 0).ok_or(rt(Corruption::Layout, w))?;
        let dir = alloc_zeroed(h, tbl, 0)?;
        {
            let b = h.get_mut(Region::Block(dir), Owner(0))?;
            put_u32(b, 0, magic::NTIX)?;
            put_u16(b, 4, FORMAT_TABLE)?;
            put_u8(b, 6, 0)?;
            put_u8(b, 7, 31)?;
            let at = DESCRIPTORS;
            put_u8(b, at.wrapping_add(D_KIND), KIND_PRIMARY)?;
            let w16 = u16::try_from(w).map_err(|_| rt(Corruption::Layout, w))?;
            put_u16(b, at.wrapping_add(D_W), w16)?;
            put_u16(b, at.wrapping_add(D_FLAGS), KEYS_UNIQUE)?;
            put_u64(b, at.wrapping_add(D_HASH), hash)?;
        }
        root::set_u64(h, tbl, root::INDEX_DIRS, dir)?;
        Ok(Primary { dir, shape, root: 0, nodes: 0, log: KeyLog::default() })
    }

    /// Writes M's root, node count and key log into the descriptor.
    fn write_m<H: PageHost>(&self, h: &mut H) -> Result<(), Fault> {
        let nodes = u32::try_from(self.nodes).map_err(|_| rt(Corruption::Count, self.nodes))?;
        let blocks = u32::try_from(self.log.blocks).map_err(|_| rt(Corruption::Count, self.log.blocks))?;
        let b = h.get_mut(Region::Block(self.dir), Owner(0))?;
        let at = DESCRIPTORS;
        put_u64(b, at.wrapping_add(D_M_ROOT), self.root)?;
        put_u32(b, at.wrapping_add(D_M_NODES), nodes)?;
        put_u32(b, at.wrapping_add(D_M_LOG_BLOCKS), blocks)?;
        put_u64(b, at.wrapping_add(D_M_LOG_ROOT), self.log.radix.root)?;
        put_u8(b, at.wrapping_add(D_M_LOG_HEIGHT), self.log.radix.height)
    }
}

/// The result of a primary point lookup in M; the path it found is in the caller's `Path`.
pub struct Lookup {
    /// The live row with key E(k), and its entry's node and index.
    pub found: Option<(u64, u64, u64)>,
    /// Where (E(k), rid) goes on a miss: the leaf and position, or `None` for an empty M.
    pub vacant: Option<(u64, u64)>,
}

/// The primary point lookup of E(k) in M: `descend(M, (E(k), Low))`, then the first entry at or after the
/// position. M's row entries are live rows. `path`, the caller's, reused, receives the path to `found`'s node
/// on a hit, to the vacant leaf on a miss.
pub fn lookup<H: PageHost + ?Sized>(h: &H, p: &Primary, e: &[u8], path: &mut Path) -> Result<Lookup, Fault> {
    let mut out = Lookup { found: None, vacant: None };
    path.clear();
    if p.root == 0 {
        return Ok(out);
    }
    let s = p.shape;
    let m = AreaView::new(h, s, p.log);
    let probe = KeyProbe { key: e, kind: Kind::Low };
    let (leaf, j) = match tree::descend(s, &m, p.root, &probe, path).map_err(|f| depth(f, p.root))? {
        Search::Vacant(n, j) => (n, j),
        Search::Found(n, _) => return Err(rt(Corruption::Order, n)),
    };
    out.vacant = Some((leaf, j));
    // The first entry at or after (leaf, j): in the leaf, else the separator of the deepest frame that has
    // one to its right.
    let mut at = None;
    if j < tree::node::count(s, &m, leaf)? {
        at = Some((leaf, j, path.len()));
    } else {
        let frames = path.frames();
        for (t, &(n, k)) in frames.iter().enumerate().rev() {
            if k < tree::node::count(s, &m, n)? {
                at = Some((n, k, t));
                break;
            }
        }
    }
    let Some((n, i, keep)) = at else { return Ok(out) };
    let leaf_n = tree::node::is_leaf(&m, n)?;
    let ns = s.shape(leaf_n);
    let w = s.w;
    let part = m.bytes(n, tree::node::ko(s, &ns, i), w.wrapping_add(8))?;
    let wu = usize::try_from(w).map_err(|_| rt(Corruption::Layout, w))?;
    let area = part.get(..wu).ok_or(rt(Corruption::Layout, n))?;
    let field = u64::from_be_bytes(crate::mem::rd_array::<8>(part, w)?);
    let rid = if is_ovf(field) { m.record(field)?.0 } else { payload(field) };
    let same = cmp_entry(&m, wu, area, field, &KeyProbe { key: e, kind: Kind::Row(rid) }, 0)?.0;
    if same == core::cmp::Ordering::Equal {
        path.truncate(keep);
        out.found = Some((rid, n, i));
    }
    Ok(out)
}

/// Inserts (E(k), `Row` rid) at the vacant position a lookup returned, with the lookup's path: in place when
/// the leaf, or the leaf's parent, has room (`area_insert::insert_vacant`).
#[inline]
pub fn insert<H: PageHost>(
    h: &mut H,
    tbl: u64,
    p: &mut Primary,
    e: &[u8],
    rid: u64,
    vacant: Option<(u64, u64)>,
    path: &mut Path,
) -> Result<(), Fault> {
    let s = p.shape;
    let before = *p;
    let mut m = AreaNodes::new(h, tbl, 1, s, p.log);
    m.nodes = p.nodes;
    let field = if e.len() as u64 > s.w { ovf_field(row_field(rid), m.new_record(rid, e)?) } else { row_field(rid) };
    match vacant {
        None if p.root == 0 => {
            let pending = IndexPending::Borrowed { key: e, field, value: &[], child: 0 };
            p.root = tree::insert_first(s, &mut m, &pending)?;
        }
        Some((n, j)) if p.root != 0 => {
            area_insert::insert_vacant(&mut m, &mut p.root, n, j, e, field, &[], path)
                .map_err(|f| depth(f, before.root))?;
        }
        _ => return Err(rt(Corruption::Link, p.root)),
    }
    p.log = m.log;
    p.nodes = m.nodes;
    if *p != before {
        p.write_m(h)?;
    }
    Ok(())
}

/// Removes the entry a lookup found, and hands back its key-log record when its key is longer than W.
pub fn remove<H: PageHost>(h: &mut H, tbl: u64, p: &mut Primary, n: u64, i: u64, path: &mut Path) -> Result<(), Fault> {
    let s = p.shape;
    let before = *p;
    let mut m = AreaNodes::new(h, tbl, 1, s, p.log);
    m.nodes = p.nodes;
    let ns = s.shape(tree::node::is_leaf(&m, n)?);
    let field = u64::from_be_bytes(crate::mem::rd_array::<8>(
        m.bytes(n, tree::node::ko(s, &ns, i), s.w.wrapping_add(8))?,
        s.w,
    )?);
    tree::remove_found(s, &mut m, &mut p.root, n, i, path).map_err(|f| depth(f, before.root))?;
    if is_ovf(field) {
        m.free_record(payload(field))?;
    }
    p.log = m.log;
    p.nodes = m.nodes;
    if *p != before {
        p.write_m(h)?;
    }
    Ok(())
}

/// Every (key, rid) entry of M in key order, through `f`; for checkers and tests.
pub fn for_each<H: PageHost + ?Sized, F>(h: &H, p: &Primary, mut f: F) -> Result<(), Fault>
where
    F: FnMut(u64) -> Result<(), Fault>,
{
    let s = p.shape;
    let m = AreaView::new(h, s, p.log);
    let mut c: tree::Cursor<tree::Fwd> =
        tree::Cursor::seek(s, &m, p.root, core::ops::Bound::<&KeyProbe<'_>>::Unbounded)?;
    while let Some((n, i)) = c.next(s, &m, core::ops::Bound::<&KeyProbe<'_>>::Unbounded)? {
        let ns = s.shape(tree::node::is_leaf(&m, n)?);
        let part = m.bytes(n, tree::node::ko(s, &ns, i), s.w.wrapping_add(8))?;
        let field = u64::from_be_bytes(crate::mem::rd_array::<8>(part, s.w)?);
        let rid = if is_ovf(field) { m.record(field)?.0 } else { payload(field) };
        f(rid)?;
    }
    Ok(())
}
