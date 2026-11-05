//! The sorted walks of the table's secondary indexes, their group walks and their ends, over keys of any
//! length: the caller's buffer holds the keys a walk holds, and a key that does not fit is `Scratch` with the
//! bytes the buffer needs.

use core::cell::Cell;

use crate::fault::Fault;
use crate::index::secondary::{Secondary, Start};
use crate::mem::PageHost;
use crate::tree::Dir;

use super::indexes::find_built;
use super::layout::Shape;

/// `Scratch` of the walks' buffers: a key needing `need` bytes of each of `parts` equal parts of the caller's
/// buffer needs `parts` times as many of it. A fault the walk's caller gave (`given`) passes as it is.
fn room<T>(r: Result<T, Fault>, parts: u64, given: bool) -> Result<T, Fault> {
    match r {
        Err(Fault::Scratch { need }) if !given => Err(Fault::Scratch { need: need.saturating_mul(parts) }),
        r => r,
    }
}

/// Index `index`'s rows in key order, from `start` in direction `dir`, at most `limit` of them, handed to `row`
/// as each row's key and rid; returns how many were handed out. `buf` holds the walk's keys, in three equal
/// parts (the key walked, the next, and each segment's): `Scratch` with the bytes it needs when a key does not
/// fit, before any row of that key is handed out.
#[allow(clippy::too_many_arguments)]
pub fn walk<H: PageHost + ?Sized, F: FnMut(&[u8], u64) -> Result<(), Fault>>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    index: u8,
    dir: Dir,
    start: Start<'_>,
    limit: u64,
    buf: &mut [u8],
    mut row: F,
) -> Result<u64, Fault> {
    let d = find_built(shape, h, tbl, index)?;
    let parts = d.parts(shape, h)?;
    let third = buf.len() / 3;
    let (key, scratch) = buf.split_at_mut(third);
    let scratch = scratch.get_mut(..third.wrapping_mul(2)).unwrap_or(&mut []);
    let given = Cell::new(false);
    let r = Secondary::read(h, tbl, index)?.walk(h, &d.spec(&parts), dir, start, limit, key, scratch, |k, rid| {
        row(k, rid).inspect_err(|_| given.set(true))
    });
    room(r, 3, given.get())
}

/// Index `index`'s smallest key (`largest`: its largest) whose first part is not null, at the start of `out`;
/// its length. `out` takes the key in its first half and each segment's in its second: `Scratch` with the
/// bytes it needs when a key does not fit.
pub fn end_key<H: PageHost + ?Sized>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    index: u8,
    largest: bool,
    out: &mut [u8],
) -> Result<Option<usize>, Fault> {
    let d = find_built(shape, h, tbl, index)?;
    let parts = d.parts(shape, h)?;
    let (ix, spec) = (Secondary::read(h, tbl, index)?, d.spec(&parts));
    let half = out.len() / 2;
    let (out, scratch) = out.split_at_mut(half);
    let scratch = scratch.get_mut(..half).unwrap_or(&mut []);
    let r = if largest { ix.max_key(h, &spec, out, scratch) } else { ix.min_key(h, &spec, out, scratch) };
    room(r, 2, false)
}

/// Index `index`'s groups in key order, from the first key above `after` (the smallest when `None`), at most
/// `limit` of them, each handed to `group` as its key, the full encoding, and how many rows hold it; returns how
/// many were handed out. `buf` holds the walk's keys, in three equal parts (the key handed out last, the next,
/// and each segment's): `Scratch` with the bytes it needs when a key does not fit, before that key is handed out.
#[allow(clippy::too_many_arguments)]
pub fn groups<H: PageHost + ?Sized, F: FnMut(&[u8], i128) -> Result<(), Fault>>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    index: u8,
    after: Option<&[u8]>,
    limit: u64,
    buf: &mut [u8],
    mut group: F,
) -> Result<u64, Fault> {
    let d = find_built(shape, h, tbl, index)?;
    let parts = d.parts(shape, h)?;
    let (ix, spec) = (Secondary::read(h, tbl, index)?, d.spec(&parts));
    let third = buf.len() / 3;
    let (mut last, rest) = buf.split_at_mut(third);
    let (mut next, scratch) = rest.split_at_mut(third);
    let scratch = scratch.get_mut(..third).unwrap_or(&mut []);
    // The length of the key handed out last, which lies at the start of `last`.
    let mut held: Option<usize> = None;
    let mut n = 0u64;
    while n < limit {
        let from = match held {
            Some(len) => Some(last.get(..len).unwrap_or(&[])),
            None => after,
        };
        let Some((len, count)) = room(ix.group_next(h, &spec, from, next, scratch), 3, false)? else { break };
        group(next.get(..len).unwrap_or(&[]), count)?;
        core::mem::swap(&mut last, &mut next);
        held = Some(len);
        n = n.wrapping_add(1);
    }
    Ok(n)
}
