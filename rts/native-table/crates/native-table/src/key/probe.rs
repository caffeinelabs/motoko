//! Index entries, probes and the comparator.
//!
//! Every entry of every index structure is a key part of W + 8 bytes, the key area (E zero-padded to W, or
//! its first W bytes: *overflow*) then the big-endian entry field, then, when V > 0, the value part. The
//! order is: E by memcmp (a proper prefix first); the rank (row entries 1, count entries 2, `Low` 0, which
//! occurs only in probes); at rank 1 the rid.

use crate::consts::{F_COUNT, F_OVF, PART_MAX, PAYLOAD, RID_PAYLOAD};
use crate::fault::{bounds, Fault};
use crate::host::{NodeRead, Pending};
use crate::key::words::{self, KeyWords};
use crate::mem::{copy_bytes, rd_array};
use crate::tree::Probe;
use core::cmp::Ordering;

// ---------------------------------------------------------------- the entry field

/// A row entry's field for a rid (bits 0–60).
#[inline(always)]
pub const fn row_field(rid: u64) -> u64 {
    rid & RID_PAYLOAD
}

/// A count entry's field: the count as a 62-bit two's-complement payload.
#[inline(always)]
pub const fn count_field(count: i64) -> u64 {
    F_COUNT | ((count as u64) & PAYLOAD)
}

/// The count a count entry's payload holds, sign-extended from 62 bits.
#[inline(always)]
pub const fn payload_to_count(field: u64) -> i64 {
    (((field & PAYLOAD) << 2) as i64) >> 2
}

#[inline(always)]
pub const fn is_ovf(field: u64) -> bool {
    field & F_OVF != 0
}

#[inline(always)]
pub const fn is_count(field: u64) -> bool {
    field & F_COUNT != 0
}

/// The payload bits of a field that does not overflow: 0–61 for a count entry, 0–60 otherwise.
#[inline(always)]
pub const fn payload(field: u64) -> u64 {
    if is_count(field) {
        field & PAYLOAD
    } else {
        field & RID_PAYLOAD
    }
}

/// An overflowed entry's field: `F_OVF` with the kind bits of `field` and `locator` as its payload.
#[inline(always)]
pub const fn ovf_field(field: u64, locator: u64) -> u64 {
    // A count entry's bit 61 belongs to its payload (a negative count sets it), so only bit 62 is its kind.
    let mask = if field & F_COUNT != 0 { PAYLOAD } else { RID_PAYLOAD };
    F_OVF | (field & F_COUNT) | (locator & mask)
}

/// `PART_MAX` = 1,024 as a usize.
#[allow(clippy::cast_possible_truncation)]
pub const PART: usize = PART_MAX as usize;

// ---------------------------------------------------------------- probes

/// The kind of a probe. `Row` carries a rid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Low,
    Row(u64),
    Count,
}

/// A search key of any length and its kind.
#[derive(Clone, Copy, Debug)]
pub struct KeyProbe<'k> {
    pub key: &'k [u8],
    pub kind: Kind,
}

/// Index trees: resolution of overflowed keys in pieces, and the width W.
pub trait ResolveKey {
    /// The key-area width W of this structure's entries.
    fn width(&self) -> u64;
    /// For a field with `F_OVF`: (the record's payload, the full key's length in bytes).
    fn record(&self, field: u64) -> Result<(u64, u64), Fault>;
    /// Copies bytes [at, at + out.len()) of that key, at ≥ W; fewer at its end; returns how many.
    fn key_piece(&self, field: u64, at: u64, out: &mut [u8]) -> Result<usize, Fault>;
    /// Bytes [at, …) of that key, W ≤ at < `len` (its length, from its record), where the structure holds
    /// them: as many as lie together, at least one.
    fn key_bytes(&self, field: u64, at: u64, len: u64) -> Result<&[u8], Fault>;
}

/// The index comparator over one stored entry (its W-byte key area and its field), against probe p,
/// starting at byte `skip` of the comparator's byte sequence; returns the ordering (stored on the left) and
/// the shared length of the two sequences, at least `skip`.
pub fn cmp_entry<R: ResolveKey + ?Sized>(
    r: &R,
    w: usize,
    area: &[u8],
    field: u64,
    p: &KeyProbe<'_>,
    skip: u64,
) -> Result<(Ordering, u64), Fault> {
    let f = p.key;
    // 1. The key areas, eight bytes at a time.
    // skip / 8 · 8 ≤ skip.
    let start = usize::try_from((skip / 8).wrapping_mul(8)).unwrap_or(usize::MAX).min(w);
    if let Some((o, at)) = words::area_key_cmp(area, f, start, w) {
        return Ok((o, at as u64));
    }
    let w64 = w as u64;
    // 2. Overflow.
    let ox = is_ovf(field);
    let oy = f.len() > w;
    let mut shared = w64.max(skip);
    let x_payload = match (ox, oy) {
        (true, false) => return Ok((Ordering::Greater, w64)),
        (false, true) => return Ok((Ordering::Less, w64)),
        (false, false) => payload(field),
        (true, true) => {
            let (rec_payload, xlen) = r.record(field)?;
            let mut at = w64.max(skip);
            loop {
                let x_end = at >= xlen;
                let y_end = at >= f.len() as u64;
                if x_end || y_end {
                    match (x_end, y_end) {
                        (true, true) => break,
                        (true, false) => return Ok((Ordering::Less, at)),
                        _ => return Ok((Ordering::Greater, at)),
                    }
                }
                // The stored key's bytes where the structure holds them.
                let xs = r.key_bytes(field, at, xlen)?;
                if xs.is_empty() {
                    return Err(bounds(at));
                }
                let ys = f.get(usize::try_from(at).unwrap_or(usize::MAX)..).unwrap_or(&[]);
                let n = xs.len().min(ys.len());
                let (o, j) = words::bytes_cmp_any(xs.get(..n).unwrap_or(&[]), ys.get(..n).unwrap_or(&[]));
                if o != Ordering::Equal {
                    return Ok((o, at.wrapping_add(j as u64)));
                }
                at = at.wrapping_add(n as u64);
            }
            shared = at;
            rec_payload
        }
    };
    Ok((rank(field, x_payload, p.kind), shared))
}

/// Steps 3 and 4 of the comparator, for a stored entry whose key equals the probe's: the ranks, then the rids
/// (`payload` the entry's rid or record payload).
#[inline(always)]
fn rank(field: u64, payload: u64, kind: Kind) -> Ordering {
    // 3. Ranks.
    let xr: u8 = if is_count(field) { 2 } else { 1 };
    let yr: u8 = match kind {
        Kind::Low => 0,
        Kind::Row(_) => 1,
        Kind::Count => 2,
    };
    if xr != yr {
        return xr.cmp(&yr);
    }
    // 4. Rids.
    match kind {
        Kind::Row(rid) => (payload & RID_PAYLOAD).cmp(&rid),
        _ => Ordering::Equal,
    }
}

/// The comparator for a probe whose key fits W, against a stored entry with an equal key area: an
/// overflowed stored key is the longer, else its rank and rid decide. `f` is the field's 8 bytes as a
/// little-endian word.
#[inline(always)]
pub(crate) fn rank_fits(f: u64, kind: Kind) -> Ordering {
    let field = f.swap_bytes();
    if is_ovf(field) {
        return Ordering::Greater;
    }
    rank(field, payload(field), kind)
}

/// Index nodes: a tree of W-byte key areas, through a host that lends key parts.
impl<M: NodeRead + ResolveKey + ?Sized> Probe<M> for KeyProbe<'_> {
    const PREFIX: bool = true;

    #[inline]
    fn cmp_at(&self, mem: &M, n: u64, koff: u64) -> Result<Ordering, Fault> {
        self.cmp_from(mem, n, koff, 0).map(|(o, _)| o)
    }

    #[inline]
    fn cmp_from(&self, mem: &M, n: u64, koff: u64, skip: u64) -> Result<(Ordering, u64), Fault> {
        let w = mem.width();
        let part = mem.bytes(n, koff, w.wrapping_add(8))?;
        let wu = usize::try_from(w).map_err(|_| bounds(koff))?;
        let area = part.get(..wu).ok_or(bounds(koff))?;
        let field = u64::from_be_bytes(rd_array::<8>(part, w)?);
        cmp_entry(mem, wu, area, field, self, skip)
    }

    #[inline]
    fn cmp_in(&self, mem: &M, node: &[u8], n: u64, koff: u64, skip: u64) -> Result<(Ordering, u64), Fault> {
        // A part outside the lent bytes: the memory's own read, with its own fault.
        let Some((area, field)) = entry_part(node, koff, mem.width()) else { return self.cmp_from(mem, n, koff, skip) };
        cmp_entry(mem, area.len(), area, field, self, skip)
    }

    /// A key that fits W, for W = 8 to 32: its words made once, every entry's key area compared with them where
    /// it lies, the search monomorphized by W; the field decides between equal key areas as `cmp_entry`'s
    /// steps 2 to 4 do. Any other key or W, or an entry read outside the lent bytes, compares entry by entry,
    /// each comparison resuming where the descent's bounds leave off.
    #[inline(always)]
    fn search_node(&self, mem: &M, node: &[u8], ks: u64, c: u64) -> Option<Result<u64, u64>> {
        let w = mem.width();
        if w > WORDS_W_MAX || self.key.len() as u64 > w || ks != w.wrapping_add(8) {
            return None;
        }
        search_fitting(self.key, self.kind, w, node, c)
    }
}

/// The widest key area a node search by words takes: four words.
const WORDS_W_MAX: u64 = 32;

/// `search_node` for a key that fits W ≤ 32, out of line: a descent for any other key or W carries only the
/// test above.
#[inline(never)]
fn search_fitting(key: &[u8], kind: Kind, w: u64, node: &[u8], c: u64) -> Option<Result<u64, u64>> {
    let c = usize::try_from(c).ok()?;
    let rank = move |f: u64| rank_fits(f, kind);
    let r = match w {
        8 => words::search_fits::<1, 16, _>(node, c, &KeyWords::first(key, 1), rank),
        16 => words::search_fits::<2, 24, _>(node, c, &KeyWords::first(key, 2), rank),
        24 => words::search_fits::<3, 32, _>(node, c, &KeyWords::first(key, 3), rank),
        32 => words::search_fits::<4, 40, _>(node, c, &KeyWords::first(key, 4), rank),
        _ => return None,
    }?;
    // Positions are below a node's capacity.
    Some(r.map(|i| i as u64).map_err(|j| j as u64))
}

/// The key area of W bytes at `koff` of a lent node and the field after it, or `None` outside the node.
#[inline]
fn entry_part(node: &[u8], koff: u64, w: u64) -> Option<(&[u8], u64)> {
    let a = usize::try_from(koff).ok()?;
    let b = a.checked_add(usize::try_from(w).ok()?)?;
    Some((node.get(a..b)?, u64::from_be_bytes(*node.get(b..)?.first_chunk::<8>()?)))
}

// ---------------------------------------------------------------- keys too long for a stack buffer

/// The comparator of a stored entry, its W-byte key area and its field, against a key of any length that is
/// handed over in order through `put` (a `Sink`), the stored key on the left; `finish` gives the order once
/// the whole key is handed over. Bytes after the first that differs are only counted.
pub struct StoredCmp<'a, R: ?Sized> {
    r: &'a R,
    area: &'a [u8],
    field: u64,
    /// The stored key's record payload and length, when it is longer than W.
    rec: Option<(u64, u64)>,
    /// The bytes handed over so far.
    pub at: u64,
    /// The order so far: `Equal` until a byte differs.
    ord: Ordering,
}

impl<'a, R: ResolveKey + ?Sized> StoredCmp<'a, R> {
    pub fn new(r: &'a R, area: &'a [u8], field: u64) -> Result<Self, Fault> {
        let rec = if is_ovf(field) { Some(r.record(field)?) } else { None };
        Ok(StoredCmp { r, area, field, rec, at: 0, ord: Ordering::Equal })
    }

    /// Whether a byte differed.
    pub fn decided(&self) -> bool {
        self.ord != Ordering::Equal
    }

    /// Whether the key handed over equals the stored one.
    pub fn same(&self) -> bool {
        !self.decided() && self.rec.is_some_and(|(_, len)| len == self.at)
    }

    /// The comparator's order, against a probe of kind `kind`, once the whole key is handed over, of any length.
    pub fn finish(&self, kind: Kind) -> Result<Ordering, Fault> {
        if self.decided() {
            return Ok(self.ord);
        }
        let w = self.area.len() as u64;
        if self.at <= w {
            // A key that fits W: the comparator pads it with zero bytes to the key area's end.
            let from = usize::try_from(self.at).map_err(|_| bounds(self.at))?;
            if self.area.get(from..).unwrap_or(&[]).iter().any(|&b| b != 0) {
                return Ok(Ordering::Greater);
            }
            return Ok(match self.rec {
                // The stored key overflows: it is the longer.
                Some(_) => Ordering::Greater,
                None => rank(self.field, payload(self.field), kind),
            });
        }
        match self.rec {
            Some((p, len)) => Ok(match len.cmp(&self.at) {
                Ordering::Equal => rank(self.field, p, kind),
                o => o,
            }),
            // A key that fits W ends before the bytes beyond it.
            None => Ok(Ordering::Less),
        }
    }
}

impl<R: ResolveKey + ?Sized> crate::key::Sink for StoredCmp<'_, R> {
    fn put(&mut self, bytes: &[u8]) -> Result<(), Fault> {
        let mut rest = bytes;
        let w = self.area.len() as u64;
        while !self.decided() && !rest.is_empty() {
            let xs = if self.at < w {
                self.area.get(usize::try_from(self.at).unwrap_or(usize::MAX)..).unwrap_or(&[])
            } else {
                match self.rec {
                    Some((_, len)) if self.at < len => self.r.key_bytes(self.field, self.at, len)?,
                    // The stored key ends first: a key that fits W, or a shorter one.
                    _ => &[],
                }
            };
            if xs.is_empty() {
                self.ord = Ordering::Less;
                break;
            }
            let n = xs.len().min(rest.len());
            let (now, later) = rest.split_at_checked(n).ok_or(bounds(self.at))?;
            self.ord = words::bytes_cmp_any(xs.get(..n).unwrap_or(&[]), now).0;
            if !self.decided() {
                (rest, self.at) = (later, self.at.wrapping_add(n as u64));
            }
        }
        self.at = self.at.saturating_add(rest.len() as u64);
        Ok(())
    }
}

/// The comparator of entry `koff` of node n against a key of any length that `feed` hands over.
pub fn cmp_stored<M: NodeRead + ResolveKey + ?Sized>(
    mem: &M,
    n: u64,
    koff: u64,
    kind: Kind,
    feed: impl FnOnce(&mut StoredCmp<'_, M>) -> Result<(), Fault>,
) -> Result<Ordering, Fault> {
    let w = mem.width();
    let part = mem.bytes(n, koff, w.wrapping_add(8))?;
    let wu = usize::try_from(w).map_err(|_| bounds(koff))?;
    let area = part.get(..wu).ok_or(bounds(koff))?;
    let field = u64::from_be_bytes(rd_array::<8>(part, w)?);
    let mut c = StoredCmp::new(mem, area, field)?;
    feed(&mut c)?;
    c.finish(kind)
}

/// A probe whose key, longer than W, lies in the structure's key log: its first W bytes `head`, and the record
/// that the overflowed field `field` names, which holds the rest; `len` is its length. A key too long for a
/// stack buffer is written into the log first and probed this way.
#[derive(Clone, Copy, Debug)]
pub struct RecProbe<'k> {
    pub head: &'k [u8],
    pub field: u64,
    pub len: u64,
    pub kind: Kind,
}

impl<M: NodeRead + ResolveKey + ?Sized> Probe<M> for RecProbe<'_> {
    fn cmp_at(&self, mem: &M, n: u64, koff: u64) -> Result<Ordering, Fault> {
        use crate::key::Sink;
        cmp_stored(mem, n, koff, self.kind, |c| {
            c.put(self.head)?;
            while !c.decided() && c.at < self.len {
                let ys = mem.key_bytes(self.field, c.at, self.len)?;
                if ys.is_empty() {
                    return Err(bounds(c.at));
                }
                c.put(ys)?;
            }
            Ok(())
        })
    }
}

// ---------------------------------------------------------------- pending entries

/// An index entry on its way into a node, and a split's `up` entry. `Owned` is large by design: the
/// split's `up` entry lives on the stack, as `PBTreeMap`'s two part buffers do.
#[allow(clippy::large_enum_variant)]
pub enum IndexPending<'k> {
    /// Key bytes (written zero-padded to W, or the first W bytes when the key overflows), the field, and
    /// the value part (empty when V = 0).
    Borrowed { key: &'k [u8], field: u64, value: &'k [u8], child: u64 },
    /// A split's `up` entry: the whole key part and value part copied from a node.
    Owned { bytes: [u8; PART], len: u16, child: u64 },
}

impl Pending for IndexPending<'_> {
    fn empty() -> Self {
        IndexPending::Owned { bytes: [0; PART], len: 0, child: 0 }
    }
    fn child(&self) -> u64 {
        match self {
            IndexPending::Borrowed { child, .. } | IndexPending::Owned { child, .. } => *child,
        }
    }
    fn set_child(&mut self, c: u64) {
        match self {
            IndexPending::Borrowed { child, .. } | IndexPending::Owned { child, .. } => *child = c,
        }
    }
}

impl IndexPending<'_> {
    /// Writes this entry's key part (W + 8 bytes) into `kpart` and its value part into `vpart`.
    pub fn write(&self, w: u64, kpart: &mut [u8], vpart: &mut [u8]) -> Result<(), Fault> {
        let wu = usize::try_from(w).map_err(|_| bounds(0))?;
        match self {
            IndexPending::Borrowed { key, field, value, .. } => {
                let (area, fld) = kpart.split_at_mut_checked(wu).ok_or(bounds(0))?;
                let n = key.len().min(wu);
                copy_bytes(area, key.get(..n).unwrap_or(&[]));
                area.get_mut(n..).unwrap_or(&mut []).fill(0);
                copy_bytes(fld, &field.to_be_bytes());
                copy_bytes(vpart, value);
                vpart.get_mut(value.len()..).unwrap_or(&mut []).fill(0);
            }
            IndexPending::Owned { bytes, len, .. } => {
                let ks = kpart.len();
                let total = usize::from(*len);
                copy_bytes(kpart, bytes.get(..ks.min(total)).unwrap_or(&[]));
                copy_bytes(vpart, bytes.get(ks.min(total)..total).unwrap_or(&[]));
            }
        }
        Ok(())
    }

    /// Takes a key part and a value part from a node into an `Owned` entry.
    pub fn take(&mut self, kpart: &[u8], vpart: &[u8]) -> Result<(), Fault> {
        let child = self.child();
        let mut bytes = [0u8; PART];
        let total = kpart.len().checked_add(vpart.len()).ok_or(bounds(0))?;
        let (a, b) = bytes.split_at_mut_checked(kpart.len()).ok_or(bounds(0))?;
        copy_bytes(a, kpart);
        copy_bytes(b, vpart);
        let len = u16::try_from(total).map_err(|_| bounds(0))?;
        if total > PART {
            return Err(bounds(0));
        }
        *self = IndexPending::Owned { bytes, len, child };
        Ok(())
    }
}
