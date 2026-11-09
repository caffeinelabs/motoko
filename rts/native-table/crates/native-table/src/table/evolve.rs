//! Schema change and column release.
//!
//! **Schema change** (`change_schema`, O(columns + leaves) with a bounded rewrite of the hot state): the new
//! schema image keeps every leaf record of the old one at its ordinal (its id, kind, type, flags and the
//! version that added it), with these changes only: a column or record field dropped (its leaves marked
//! with the new version as their dropped version; a key leaf is never dropped, a dropped leaf never
//! returns); new leaves after the last ordinal, added by the new version, each with a default (a column's
//! declared default, null for an option, or the zero of its type). Columns keep
//! their ids and defaults. The primary index declarations stay as they are; a secondary index the new image
//! no longer declares byte for byte, or one with a part the image drops, is removed (`indexes::remove`: its
//! tree and key log go to the drain); a secondary index the image declares that the table does not hold is
//! added (`indexes::add`: built over the rows the table holds in maintenance steps). An id whose declaration
//! changes is a removal and an addition. An image of the table's schema hash is a change when its
//! declarations differ. An image may keep the table's version only when it changes the declarations alone (the
//! same row type, leaves and columns); one that adds or drops a leaf has a higher version. Every refusal comes
//! before the first write. The integer leaves' totals follow: a dropped leaf's total goes, and a new integer
//! leaf's total counts every live row, each of which reads its default; it lies in the hot state when its
//! record fits `HOT_MAX` (in leaf ordinal order like every record), else in the totals space. A new tail block
//! takes the hot state when it does not fit below the old one's first row byte.
//!
//! **Column release** (job slot 0 at table root 1,824: {kind u8 @0, 1; state u8 @1, 1 running; the dropped-
//! leaf generation u32 @4, the schema version of the latest drop the walk releases; cursor u64 @8, the next
//! group number to visit}). One step visits one group in group-number order, skipping absent directory
//! entries: a sealed group whose image holds a dropped leaf is rewritten whole without it (`seal::rewrite`); any
//! other group costs the check only; the open group drops its dropped leaves at its seal. When the walk passes
//! the last group: if the latest drop is still the slot's generation, `cleaned` := the dropped columns and the
//! release flag clears; otherwise the walk records the new generation and restarts at group 0.

use crate::consts::{BUFFER_BLOCKS_MAX, HOT_MAX, PAGE};
use crate::fault::{corrupt, rt, Corruption, Fault, ImageId, SchemaError};
use crate::mem::{fill, put, put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u8, PageHost, Region};
use crate::schema::SchemaView;
use crate::stats::running::LIST_AT;

use super::buffer::{self, DESC_BIT};
use super::group::{align8, GroupView};
use super::hot::RunCursor;
use super::indexes::{DECL, PRIMARY};
use super::layout::{Shape, Ty};
use super::root;
use super::seal;
use super::totals::{self, At, Totals};

const SLOT: u64 = root::JOB_SLOTS;
const S_KIND: u64 = SLOT;
const S_STATE: u64 = SLOT + 1;
const S_GEN: u64 = SLOT + 4;
const S_CURSOR: u64 = SLOT + 8;

/// The number of dropped columns of the table's schema.
pub fn dropped_columns<H: PageHost + ?Sized>(h: &H, shape: &Shape) -> Result<u32, Fault> {
    let v = SchemaView::open(h.get(Region::Page(shape.page))?)?;
    let mut n = 0u32;
    for i in 0..v.columns {
        if v.column(i)?.flags & 1 != 0 {
            n = n.wrapping_add(1);
        }
    }
    Ok(n)
}

/// Whether the image of group page `pg` holds a leaf the schema dropped.
fn holds_dropped<H: PageHost + ?Sized>(h: &H, shape: &Shape, pg: u64) -> Result<bool, Fault> {
    let gv = GroupView::open(h, pg)?;
    for i in 0..gv.l {
        let o = u64::from(gv.leaf(i)?.ordinal);
        if o >= shape.leaves {
            return Err(corrupt(ImageId::Group(gv.g), Corruption::Leaf, o));
        }
        if shape.span_of(h, o)?.dropped != 0 {
            return Ok(true);
        }
    }
    Ok(false)
}

/// What a piece of the walk for the next group found.
enum NextGroup {
    Group(u64, u64),
    /// None before this group number.
    From(u64),
    End,
}

/// Directory entries one piece of the walk examines.
const WALK_PIECE: u64 = 48;

/// The first directory entry at or after group number `from` among the next `WALK_PIECE` entries: (group,
/// entry).
fn next_group<H: PageHost>(h: &H, tbl: u64, from: u64) -> Result<NextGroup, Fault> {
    let dir = root::directory(h, tbl)?;
    let mut found = None;
    let w = dir.walk_max(h, from, WALK_PIECE, |g, v| {
        if found.is_none() {
            found = Some((g, v));
        }
        Ok(())
    })?;
    Ok(match (found, w.next) {
        (Some((g, v)), _) => NextGroup::Group(g, v),
        (None, Some(n)) => NextGroup::From(n),
        (None, None) => NextGroup::End,
    })
}

/// One step of the column release: the next group from the cursor, rewritten whole without its dropped leaves
/// when its image holds one; past the last group, the release ends, or starts again at group 0 when a later
/// drop came meanwhile.
pub fn release_step<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape) -> Result<(), Fault> {
    let latest = shape.dropped;
    if root::u8_at(&*h, tbl, S_KIND)? == 0 {
        let b = h.get_mut(Region::Block(tbl))?;
        fill(b, SLOT, 32, 0)?;
        put_u8(b, S_KIND, 1)?;
        put_u8(b, S_STATE, 1)?;
        put_u32(b, S_GEN, latest)?;
        put_u64(b, S_CURSOR, 0)?;
    }
    let mut from = root::u64_at(&*h, tbl, S_CURSOR)?;
    loop {
        match next_group(&*h, tbl, from)? {
            NextGroup::End => {
                if root::u32_at(&*h, tbl, S_GEN)? == latest {
                    root::set_u32(h, tbl, root::CLEANED, shape.dropped_columns)?;
                    let f = root::u8_at(&*h, tbl, root::FLAGS)?;
                    root::set_u8(h, tbl, root::FLAGS, f & !root::FLAG_RELEASE)?;
                    fill(h.get_mut(Region::Block(tbl))?, SLOT, 32, 0)?;
                } else {
                    root::set_u32(h, tbl, S_GEN, latest)?;
                    root::set_u64(h, tbl, S_CURSOR, 0)?;
                }
                return Ok(());
            }
            NextGroup::From(g) => from = g,
            NextGroup::Group(g, entry) => {
                root::set_u64(h, tbl, S_CURSOR, g.checked_add(1).ok_or(rt(Corruption::Range, g))?)?;
                // The open group drops its dropped leaves at its seal.
                if entry & DESC_BIT == 0 && holds_dropped(&*h, shape, entry)? {
                    seal::rewrite(h, tbl, shape, entry, seal::RELEASE)?;
                }
                return Ok(());
            }
        }
    }
}

fn refuse(e: SchemaError) -> Fault {
    Fault::Schema(e)
}

/// Declaration i of a schema image: its bytes, and whether it is a secondary one each of whose parts is a live
/// leaf of the image (one with a dropped part declares no index).
fn view_decl<'a>(v: &SchemaView<'a>, i: u64) -> Result<(&'a [u8], bool), Fault> {
    // i < 64.
    let b = crate::mem::sub(v.bytes, u64::from(v.layout.indexes).wrapping_add(i.wrapping_mul(DECL)), DECL)?;
    if rd_u8(b, 1)? & PRIMARY != 0 {
        return Ok((b, false));
    }
    for k in 0..u64::from(rd_u8(b, 4)?) {
        let leaf = rd_u32(b, 8u64.wrapping_add(k.wrapping_mul(4)))?;
        for o in 0..v.leaves {
            let l = v.leaf(o)?;
            if l.id == leaf && l.dropped != 0 {
                return Ok((b, false));
            }
        }
    }
    Ok((b, true))
}

/// Whether schema image `v` has a declaration of the same bytes as `d`, secondary and live when `live`.
fn view_has_decl(v: &SchemaView<'_>, d: &[u8], live: bool) -> Result<bool, Fault> {
    for i in 0..u64::from(v.indexes) {
        let (b, l) = view_decl(v, i)?;
        if b == d && l == live {
            return Ok(true);
        }
    }
    Ok(false)
}

/// What a schema change does beside its leaves: whether it drops leaves, and the ids of the secondary indexes
/// it removes and adds (bit i for id i).
struct Change {
    drops: bool,
    removed: u64,
    added: u64,
}

/// Checks a new schema image against the table's, reading the image alone, before anything is written.
fn check<H: PageHost + ?Sized>(h: &H, shape: &Shape, image: &[u8]) -> Result<Change, Fault> {
    let old_bytes = h.get(Region::Page(shape.page))?;
    let old = SchemaView::open(old_bytes)?;
    let new = SchemaView::open(image)?;
    // An image of the table's own version changes index declarations alone: its row type (the schema hash),
    // leaves and columns are the table's. Adding or dropping a leaf takes a higher version.
    let same = new.version == old.version;
    if new.version < old.version
        || new.leaves < old.leaves
        || new.columns < old.columns
        || (same && (new.hash != old.hash || new.leaves != old.leaves || new.columns != old.columns))
    {
        return Err(refuse(SchemaError::ColumnChanged));
    }
    let mut drops = false;
    for o in 0..old.leaves {
        let a = old.leaf(o)?;
        let b = new.leaf(o)?;
        let key = a.flags & crate::schema::LEAF_KEY != 0;
        if a.id != b.id
            || a.kind != b.kind
            || a.code != b.code
            || a.flags != b.flags
            || a.width != b.width
            || a.added != b.added
        {
            return Err(refuse(if key { SchemaError::KeyChanged } else { SchemaError::ColumnChanged }));
        }
        if a.dropped != 0 {
            if b.dropped != a.dropped {
                return Err(refuse(SchemaError::RestoredDropped));
            }
        } else if b.dropped != 0 {
            if key {
                return Err(refuse(SchemaError::KeyChanged));
            }
            if same {
                return Err(refuse(SchemaError::ColumnChanged));
            }
            if b.dropped != new.version {
                return Err(refuse(SchemaError::NewDropped));
            }
            drops = true;
        }
    }
    for o in old.leaves..new.leaves {
        let b = new.leaf(o)?;
        if b.added != new.version || b.dropped != 0 || b.flags & crate::schema::LEAF_KEY != 0 {
            return Err(refuse(SchemaError::ColumnChanged));
        }
    }
    for i in 0..old.columns {
        let a = old.column(i)?;
        let b = new.column(i)?;
        if a.id != b.id || a.first_leaf != b.first_leaf || a.default_len != b.default_len {
            return Err(refuse(SchemaError::ColumnChanged));
        }
        let at_a = u64::from(old.layout.defaults).wrapping_add(u64::from(a.default_off));
        let at_b = u64::from(new.layout.defaults).wrapping_add(u64::from(b.default_off));
        let da = crate::mem::sub(old.bytes, at_a, u64::from(a.default_len))?;
        let db = crate::mem::sub(new.bytes, at_b, u64::from(b.default_len))?;
        if da != db {
            return Err(refuse(SchemaError::ColumnChanged));
        }
        if a.flags & 1 != 0 && b.flags & 1 == 0 {
            return Err(refuse(SchemaError::RestoredDropped));
        }
    }
    // The indexes: the primary declarations stay as they are; a secondary index the new image no longer
    // declares as it was, or one holding a leaf the image drops, is removed; one the table does not hold yet is
    // added.
    super::indexes::check(&new)?;
    let (mut removed, mut added) = (0u64, 0u64);
    let mut primaries = 0u64;
    for i in 0..u64::from(old.indexes) {
        let (d, live) = view_decl(&old, i)?;
        if rd_u8(d, 1)? & PRIMARY != 0 {
            primaries = primaries.wrapping_add(1);
            if !view_has_decl(&new, d, false)? {
                return Err(refuse(SchemaError::KeyChanged));
            }
        } else if live && !view_has_decl(&new, d, true)? {
            removed |= 1u64.wrapping_shl(u32::from(rd_u8(d, 0)?));
        }
    }
    for i in 0..u64::from(new.indexes) {
        let (d, live) = view_decl(&new, i)?;
        if rd_u8(d, 1)? & PRIMARY != 0 {
            primaries = primaries.wrapping_sub(1);
        } else if live && !view_has_decl(&old, d, true)? {
            added |= 1u64.wrapping_shl(u32::from(rd_u8(d, 0)?));
        }
    }
    if primaries != 0 {
        return Err(refuse(SchemaError::KeyChanged));
    }
    Ok(Change { drops, removed, added })
}

/// Whether two schema images declare the same indexes, byte for byte.
fn same_decls(a: &SchemaView<'_>, b: &SchemaView<'_>) -> Result<bool, Fault> {
    let n = u64::from(a.indexes).wrapping_mul(DECL);
    Ok(a.indexes == b.indexes
        && crate::mem::sub(a.bytes, u64::from(a.layout.indexes), n)?
            == crate::mem::sub(b.bytes, u64::from(b.layout.indexes), n)?)
}

/// Adopts a new schema image: checked first (every refusal before the first write), then written to a new
/// page, the root's schema fields and the shape it holds set, the hot state's running records re-laid, and,
/// when leaves are dropped, the column release started.
pub fn change_schema<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, image: &[u8]) -> Result<Shape, Fault> {
    let v = SchemaView::open(image)?;
    // The schema hash covers the row type alone: an image of the same hash is a change only when it declares
    // other indexes.
    if v.hash == shape.hash && same_decls(&v, &SchemaView::open_stored(h.get(Region::Page(shape.page))?)?)? {
        return Ok(*shape);
    }
    if image.len() as u64 > PAGE {
        return Err(refuse(SchemaError::ColumnChanged));
    }
    for o in 0..v.leaves {
        let l = v.leaf(o)?;
        if Ty::of(&l).is_none() {
            return Err(refuse(SchemaError::ColumnChanged));
        }
    }
    // Every refusal of the change itself comes before the first write.
    let Change { drops, removed, added } = check(&*h, shape, image)?;
    // The new image goes to a page of its own, which a refusal of its layout frees at once: it is the
    // table's only once the root names it.
    let page = crate::area::image_page_alloc(h, tbl)?;
    put(h.get_mut(Region::Page(page))?, 0, image)?;
    let new_shape = match Shape::open(&*h, page) {
        Ok(s) => s,
        Err(e) => {
            crate::area::image_page_free(h, tbl, page)?;
            return Err(e);
        }
    };
    let mut buf = [0u8; 2_048];
    let (r, end) = build_hot(&*h, tbl, shape, &new_shape, &mut buf)?;
    relay_totals(h, tbl, shape, &new_shape, &mut buf, r)?;
    rewrite_hot(h, tbl, &new_shape, &buf, r, end)?;
    crate::area::image_page_free(h, tbl, shape.page)?;
    root::set_u64(h, tbl, root::SCHEMA_HASH, new_shape.hash)?;
    root::set_u32(h, tbl, root::SCHEMA_VERSION, new_shape.version)?;
    root::set_u64(h, tbl, root::SCHEMA_PAGE, page)?;
    new_shape.store(h, tbl)?;
    for id in 1..64u8 {
        if removed & 1u64.wrapping_shl(u32::from(id)) != 0 {
            super::indexes::remove(h, tbl, id)?;
        }
    }
    for id in 1..64u8 {
        if added & 1u64.wrapping_shl(u32::from(id)) != 0 {
            let d = super::indexes::find(&new_shape, &*h, id)?;
            super::indexes::add(h, tbl, &new_shape, &d)?;
        }
    }
    if drops {
        let f = root::u8_at(&*h, tbl, root::FLAGS)?;
        root::set_u8(h, tbl, root::FLAGS, f | root::FLAG_RELEASE)?;
    }
    Ok(new_shape)
}

/// The most running records a hot state holds: its list fits `HOT_MAX`.
const RUN_MAX: usize = 1_024;

/// The new hot state's list and records, built in `buf` from offset `LIST_AT`: the old records of leaves still
/// live, copied, then a zero record per new integer leaf that fits (`relay_totals` writes its total). Returns
/// the list's length R and the end of the records, which the room keeps within `HOT_MAX`.
fn build_hot<H: PageHost + ?Sized>(
    h: &H,
    tbl: u64,
    old: &Shape,
    new: &Shape,
    buf: &mut [u8; 2_048],
) -> Result<(u64, u64), Fault> {
    let tail = root::u64_at(h, tbl, root::TAIL)?;
    let tb = h.get(Region::Block(tail))?;
    let r_old = u64::from(rd_u16(tb, buffer::H_RUN_R)?);
    // Whether one more record of `bytes` and its list entry fit after n records of `recs` bytes.
    let fits = |n: usize, recs: u64, bytes: u64| {
        let list = align8((n as u64).wrapping_add(1).wrapping_mul(2));
        n < RUN_MAX && LIST_AT.wrapping_add(list).wrapping_add(recs).wrapping_add(bytes) <= HOT_MAX
    };
    // The ordinals kept, then the new ones, each with its record's old offset (0 for a new record), in
    // ordinal order. A record's size never changes, so every record of a live leaf fits again.
    let mut keep: [(u16, u64); RUN_MAX] = [(0, 0); RUN_MAX];
    let mut n = 0usize;
    let mut recs = 0u64;
    let mut off = LIST_AT.wrapping_add(align8(r_old.wrapping_mul(2)));
    for i in 0..r_old {
        let o = rd_u16(tb, LIST_AT.wrapping_add(i.wrapping_mul(2)))?;
        let at = off;
        let bytes = old.leaf(h, u64::from(o))?.ty.running().ok_or(rt(Corruption::Leaf, u64::from(o)))?.bytes();
        off = off.wrapping_add(bytes);
        if new.leaf(h, u64::from(o))?.live() && fits(n, recs, bytes) {
            let slot = keep.get_mut(n).ok_or(rt(Corruption::Count, i))?;
            *slot = (o, at);
            n = n.wrapping_add(1);
            recs = recs.wrapping_add(bytes);
        }
    }
    for o in old.leaves..new.leaves {
        let Some(k) = new.leaf(h, o)?.ty.running() else { continue };
        if fits(n, recs, k.bytes()) {
            let slot = keep.get_mut(n).ok_or(rt(Corruption::Count, o))?;
            *slot = (u16::try_from(o).map_err(|_| rt(Corruption::Range, o))?, 0);
            n = n.wrapping_add(1);
            recs = recs.wrapping_add(k.bytes());
        }
    }
    let r = n as u64;
    buf.fill(0);
    let mut at = LIST_AT.wrapping_add(align8(r.wrapping_mul(2)));
    for (i, (o, src)) in keep.iter().take(n).enumerate() {
        put_u16(buf, LIST_AT.wrapping_add((i as u64).wrapping_mul(2)), *o)?;
        let bytes = new.leaf(h, u64::from(*o))?.ty.running().ok_or(rt(Corruption::Leaf, u64::from(*o)))?.bytes();
        if *src != 0 {
            put(buf, at, crate::mem::sub(tb, *src, bytes)?)?;
        }
        at = at.wrapping_add(bytes);
    }
    Ok((r, at))
}

/// The integer leaves' totals for the new schema version. A new leaf's total counts every live row, each of
/// which reads its default, in the new hot state `buf` (whose list has `r` entries) or in the totals space.
/// When the leaves whose totals lie in the space change (a new leaf there, a dropped one, one the hot state no
/// longer has room for), the space's records are written anew at its end, each from its old place.
fn relay_totals<H: PageHost>(
    h: &mut H,
    tbl: u64,
    old: &Shape,
    new: &Shape,
    buf: &mut [u8; 2_048],
    r: u64,
) -> Result<(), Fault> {
    let tail = root::u64_at(&*h, tbl, root::TAIL)?;
    let live = buffer::u64_of(&*h, tail, buffer::H_LIVE)?;
    // Each leaf's old and new place; whether the space's records stay as they are.
    let (mut oc, mut nc) = (RunCursor::open(&*h, tail)?, RunCursor::of_list(r));
    let (mut old_end, mut new_end, mut same) = (0u64, 0u64, true);
    for o in 0..new.leaves {
        let ln = new.leaf(&*h, o)?;
        let was = place_old(&*h, old, &mut oc, tail, o)?;
        let now = nc.place(buf, &ln)?;
        let bytes = ln.ty.running().map_or(0, |k| k.bytes());
        if let At::Space(x) = was {
            old_end = x.wrapping_add(bytes);
        }
        match now {
            At::Space(x) => {
                new_end = x.wrapping_add(bytes);
                same &= was == now;
            }
            // A new leaf's record in the hot state.
            At::Hot(x) if o >= old.leaves => {
                let rec = sub_mut(buf, x, bytes)?;
                totals::of_default(h, tbl, tail, &ln, &super::defaults::of(&*h, new, &ln)?, live, rec)?;
            }
            _ => {}
        }
    }
    if same && old_end == new_end {
        return Ok(());
    }
    let mut sp = Totals::ensure(h, tbl, tail)?;
    let old_recs = sp.recs;
    let base = sp.append(h, tbl, tail, new_end)?;
    sp.set_recs(h, base)?;
    let (mut oc, mut nc) = (RunCursor::open(&*h, tail)?, RunCursor::of_list(r));
    for o in 0..new.leaves {
        let ln = new.leaf(&*h, o)?;
        let was = place_old(&*h, old, &mut oc, tail, o)?;
        let At::Space(x) = nc.place(buf, &ln)? else { continue };
        let bytes = ln.ty.running().map_or(0, |k| k.bytes());
        let mut r = [0u8; 32];
        let rec = sub_mut(&mut r, 0, bytes)?;
        match was {
            At::Hot(y) => put(rec, 0, crate::mem::sub(h.get(Region::Block(tail))?, y, bytes)?)?,
            At::Space(y) => sp.read_bytes(&*h, old_recs.wrapping_add(y), rec)?,
            At::None => {
                totals::of_default(h, tbl, tail, &ln, &super::defaults::of(&*h, new, &ln)?, live, rec)?;
                sp = Totals::read(&*h, tail)?.ok_or(rt(Corruption::Link, tail))?;
            }
        }
        sp.write_bytes(h, base.wrapping_add(x), rec)?;
    }
    Ok(())
}

/// Leaf ordinal `o`'s place in the old schema version's hot state (`oc`, in tail block `tail`); `None` for a
/// leaf the old version lacks.
fn place_old<H: PageHost + ?Sized>(h: &H, old: &Shape, oc: &mut RunCursor, tail: u64, o: u64) -> Result<At, Fault> {
    if o >= old.leaves {
        return Ok(At::None);
    }
    oc.place(h.get(Region::Block(tail))?, &old.leaf(h, o)?)
}

fn sub_mut(b: &mut [u8], at: u64, n: u64) -> Result<&mut [u8], Fault> {
    let s = usize::try_from(at).map_err(|_| rt(Corruption::Range, at))?;
    let e = usize::try_from(at.wrapping_add(n)).map_err(|_| rt(Corruption::Range, at))?;
    b.get_mut(s..e).ok_or(rt(Corruption::Bounds, at))
}

/// The hot state's rewrite for the new schema version, from `buf` (a list of `r` entries, records up to
/// `end`), in the tail block when the new layout fits below its first row byte, else in a new tail block of
/// the open group (after its seal when its descriptor is full).
fn rewrite_hot<H: PageHost>(
    h: &mut H,
    tbl: u64,
    new: &Shape,
    buf: &[u8; 2_048],
    r: u64,
    end: u64,
) -> Result<(), Fault> {
    let mut tail = root::u64_at(&*h, tbl, root::TAIL)?;
    if end > buffer::u16_of(&*h, tail, buffer::H_H)? {
        tail = new_tail_at(h, tbl, new, align8(end))?;
    }
    let hh = buffer::u16_of(&*h, tail, buffer::H_H)?;
    let s0 = usize::try_from(LIST_AT).unwrap_or(0);
    let s1 = usize::try_from(end).unwrap_or(0);
    let b = h.get_mut(Region::Block(tail))?;
    put(b, LIST_AT, buf.get(s0..s1).ok_or(rt(Corruption::Bounds, end))?)?;
    fill(b, end, hh.saturating_sub(end), 0)?;
    put_u16(b, buffer::H_RUN_R, u16::try_from(r).map_err(|_| rt(Corruption::Range, r))?)?;
    put_u32(b, buffer::H_RUN_VERSION, new.version)
}

/// A new tail block of the open group whose first row byte is `hh`: the hot state's fixed part carried over;
/// the open group is sealed first when its descriptor is full.
fn new_tail_at<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, hh: u64) -> Result<u64, Fault> {
    let (_, desc, _) = buffer::open_group(&*h, tbl)?;
    if buffer::u16_of(&*h, desc, buffer::D_BLOCKS)? >= BUFFER_BLOCKS_MAX {
        super::write::freeze(h, tbl, shape)?;
    }
    let (g, desc, tail) = buffer::open_group(&*h, tbl)?;
    let blocks = buffer::u16_of(&*h, desc, buffer::D_BLOCKS)?;
    let pos = buffer::u16_of(&*h, tail, buffer::H_OPEN_POS)?;
    let tail_slots = buffer::u16_of(&*h, tail, buffer::B_SLOTS)?;
    let nb = buffer::new_block_at(h, tbl, g, pos, desc, None, hh)?;
    // The hot state's fixed part [32, 192).
    crate::mem::copy(h, Region::Block(tail), 32, Region::Block(nb), 32, LIST_AT.wrapping_sub(32))?;
    let h16 = u16::try_from(hh).map_err(|_| rt(Corruption::Range, hh))?;
    put_u16(h.get_mut(Region::Block(nb))?, buffer::H_H, h16)?;
    {
        let d = h.get_mut(Region::Block(desc))?;
        let last = buffer::D_RECORDS.wrapping_add(blocks.saturating_sub(1).wrapping_mul(16));
        put_u16(d, last.wrapping_add(10), u16::try_from(tail_slots).map_err(|_| rt(Corruption::Range, tail_slots))?)?;
        let rec = buffer::D_RECORDS.wrapping_add(blocks.wrapping_mul(16));
        put_u64(d, rec, nb)?;
        put_u16(d, rec.wrapping_add(8), u16::try_from(pos).map_err(|_| rt(Corruption::Range, pos))?)?;
        put_u16(d, rec.wrapping_add(10), 0)?;
        put_u32(d, rec.wrapping_add(12), 0)?;
        put_u16(d, buffer::D_BLOCKS, u16::try_from(blocks.wrapping_add(1)).unwrap_or(u16::MAX))?;
    }
    root::set_u64(h, tbl, root::TAIL, nb)?;
    Ok(nb)
}
