//! Uploaded segments in the probe: a page host over a caller's slice, an index's loaded runs built and listed,
//! the count of one key over them for the measurement of its cost, the distinct keys of one run stepped
//! through in order, the key classes of OQL's text-key count bars, and the copy-heavy unit of the meter's
//! calibration.

use native_table::area::new_root;
use native_table::consts::{BLOCK, PAGE};
use native_table::image::{Image, ImageReader};
use native_table::index::runlist::{image_radix, RunDesc, RunList, D_BELIEVED, D_RELATIVE};
use native_table::index::secondary::Secondary;
use native_table::index::Run;
use native_table::key::probe::{KeyProbe, Kind};
use native_table::mem::{fill, put_u64, PageHost, Region};
use native_table::Fault;
use nt_builder::MemRunSink;
use nt_builder::{build_run, plan_run, Item, RunInput, RunParams};

use crate::{code, Gen};

/// Pages a slice host can return to its free list.
const FREE_MAX: usize = 4_096;

/// Main memory over a caller's slice: page k (k ≥ 1) is bytes [(k − 1)·65,536, k·65,536). Freed pages are
/// reused, newest first.
pub struct SliceHost<'a> {
    mem: &'a mut [u8],
    next: u64,
    free: [u64; FREE_MAX],
    nfree: usize,
}

impl<'a> SliceHost<'a> {
    pub fn new(mem: &'a mut [u8]) -> Self {
        SliceHost { mem, next: 1, free: [0; FREE_MAX], nfree: 0 }
    }

    fn span(&self, r: Region) -> Result<(usize, usize), Fault> {
        let (base, len) = match r {
            Region::Page(p) if p >= PAGE && p % PAGE == 0 => (p - PAGE, PAGE),
            Region::Block(b) if b >= PAGE && b % BLOCK == 0 => (b - PAGE, BLOCK),
            _ => return Err(Fault::BadRef),
        };
        let end = base.checked_add(len).ok_or(Fault::BadRef)?;
        let s = usize::try_from(base).map_err(|_| Fault::BadRef)?;
        let e = usize::try_from(end).map_err(|_| Fault::BadRef)?;
        if e > self.mem.len() {
            return Err(Fault::BadRef);
        }
        Ok((s, e))
    }
}

impl PageHost for SliceHost<'_> {
    fn block_of(&self, page: u64, i: u32) -> u64 {
        page.wrapping_add(u64::from(i).wrapping_mul(BLOCK))
    }
    fn alloc_changes(&self) -> u64 {
        1
    }
    fn get(&self, r: Region) -> Result<&[u8], Fault> {
        let (s, e) = self.span(r)?;
        self.mem.get(s..e).ok_or(Fault::BadRef)
    }
    fn get_mut(&mut self, r: Region) -> Result<&mut [u8], Fault> {
        let (s, e) = self.span(r)?;
        self.mem.get_mut(s..e).ok_or(Fault::BadRef)
    }
    fn pair_mut(&mut self, a: Region, b: Region) -> Result<(&mut [u8], &mut [u8]), Fault> {
        let (sa, ea) = self.span(a)?;
        let (sb, eb) = self.span(b)?;
        if sa < eb && sb < ea {
            return Err(Fault::BadRef);
        }
        if sa < sb {
            let (lo, hi) = self.mem.split_at_mut_checked(sb).ok_or(Fault::BadRef)?;
            Ok((lo.get_mut(sa..ea).ok_or(Fault::BadRef)?, hi.get_mut(..eb.wrapping_sub(sb)).ok_or(Fault::BadRef)?))
        } else {
            let (lo, hi) = self.mem.split_at_mut_checked(sa).ok_or(Fault::BadRef)?;
            let y = lo.get_mut(sb..eb).ok_or(Fault::BadRef)?;
            Ok((hi.get_mut(..ea.wrapping_sub(sa)).ok_or(Fault::BadRef)?, y))
        }
    }
    fn read_write(&mut self, r: Region, wr: Region) -> Result<(&[u8], &mut [u8]), Fault> {
        let (a, b) = self.pair_mut(r, wr)?;
        Ok((&*a, b))
    }
    fn page_alloc(&mut self) -> Result<u64, Fault> {
        if self.nfree > 0 {
            self.nfree = self.nfree.wrapping_sub(1);
            return self.free.get(self.nfree).copied().ok_or(Fault::OutOfMemory);
        }
        let p = self.next.wrapping_mul(PAGE);
        self.span(Region::Page(p)).map_err(|_| Fault::OutOfMemory)?;
        self.next = self.next.wrapping_add(1);
        Ok(p)
    }
    fn page_free(&mut self, p: u64) -> Result<(), Fault> {
        self.span(Region::Page(p))?;
        let slot = self.free.get_mut(self.nfree).ok_or(Fault::OutOfMemory)?;
        *slot = p;
        self.nfree = self.nfree.wrapping_add(1);
        Ok(())
    }
}

/// A table with secondary index 1 (W 8) and a run list for its uploaded segments, which the index names; the
/// root's other fields zero.
fn table<H: PageHost>(h: &mut H) -> Result<(u64, Secondary), Fault> {
    let tbl = new_root(h)?;
    {
        let s = h.get_mut(Region::Block(tbl))?;
        fill(s, 0, 160, 0)?;
        fill(s, 928, 4_096 - 928, 0)?;
        put_u64(s, 96, 1)?;
    }
    let mut ix = Secondary::declare(h, tbl, 1, 8, 9)?;
    ix.run_list = RunList::create(h, tbl)?.block;
    Ok((tbl, ix))
}

/// Items as two parallel arrays of 8-byte keys and rids in a caller's slices, in (key, rid) order.
pub struct SliceItems<'a> {
    keys: &'a [u8],
    rids: &'a [u8],
    tombs: u64,
    n: u64,
}

impl RunInput for SliceItems<'_> {
    fn items(&self) -> u64 {
        self.n
    }
    fn item(&self, i: u64) -> Result<Item<'_>, Fault> {
        if i >= self.n {
            return Err(Fault::BadRef);
        }
        let at = usize::try_from(i.wrapping_mul(8)).map_err(|_| Fault::BadRef)?;
        let key = self.keys.get(at..at.wrapping_add(8)).ok_or(Fault::BadRef)?;
        let rid = native_table::mem::rd_u64(self.rids, i.wrapping_mul(8))?;
        // Every `tombs`-th item is a tombstone (0: none).
        Ok(Item { key, rid, tomb: self.tombs != 0 && i % self.tombs == self.tombs.wrapping_sub(1) })
    }
}

/// Fills `n` items: keys from the generator below `keys`, sorted, each with a fresh rid from `rid0` on;
/// returns how many distinct (key, rid) items there are (all of them).
fn gen_items(
    g: &mut Gen,
    keys_out: &mut [u8],
    rids_out: &mut [u8],
    n: u64,
    keys: u64,
    rid0: u64,
) -> Result<u64, Fault> {
    let nu = usize::try_from(n).map_err(|_| Fault::BadRef)?;
    // Keys as u64 values in the rid array first, sorted, then written big-endian.
    for j in 0..n {
        put_u64(rids_out, j.wrapping_mul(8), g.next() % keys.max(1))?;
    }
    let words = rids_out.get_mut(..nu.wrapping_mul(8)).ok_or(Fault::BadRef)?;
    sort_words(words);
    for j in 0..n {
        let at = usize::try_from(j.wrapping_mul(8)).map_err(|_| Fault::BadRef)?;
        let k = native_table::mem::rd_u64(words, j.wrapping_mul(8))?;
        let dst = keys_out.get_mut(at..at.wrapping_add(8)).ok_or(Fault::BadRef)?;
        native_table::mem::copy_bytes(dst, &k.to_be_bytes());
        put_u64(words, j.wrapping_mul(8), rid0.wrapping_add(j))?;
    }
    Ok(n)
}

/// Sorts 8-byte little-endian words in place (an insertion of runs merged by a heap sort, no allocation).
fn sort_words(b: &mut [u8]) {
    let n = b.len() / 8;
    let get = |b: &[u8], i: usize| native_table::mem::rd_u64(b, (i as u64).wrapping_mul(8)).unwrap_or(0);
    let set = |b: &mut [u8], i: usize, v: u64| {
        let _ = put_u64(b, (i as u64).wrapping_mul(8), v);
    };
    // Heap sort.
    let sift = |b: &mut [u8], mut root: usize, end: usize| loop {
        let child = root.wrapping_mul(2).wrapping_add(1);
        if child >= end {
            break;
        }
        let mut c = child;
        if c.wrapping_add(1) < end && get(b, c) < get(b, c.wrapping_add(1)) {
            c = c.wrapping_add(1);
        }
        if get(b, root) >= get(b, c) {
            break;
        }
        let (x, y) = (get(b, root), get(b, c));
        set(b, root, y);
        set(b, c, x);
        root = c;
    };
    let mut start = n / 2;
    while start > 0 {
        start = start.wrapping_sub(1);
        sift(b, start, n);
    }
    let mut end = n;
    while end > 1 {
        end = end.wrapping_sub(1);
        let (x, y) = (get(b, 0), get(b, end));
        set(b, 0, y);
        set(b, end, x);
        sift(b, 0, end);
    }
}

/// Builds a run of `items` in `scratch` and copies it into table memory; returns its descriptor (absolute
/// rids) with the image in reference form.
fn install_run<H: PageHost, I: RunInput>(
    h: &mut H,
    tbl: u64,
    items: &I,
    relative: bool,
    scratch: &mut [u8],
) -> Result<RunDesc, Fault> {
    let p = RunParams {
        index: 1,
        relative,
        w0: 0,
        key_run_l: 0,
        table_uid: 0,
        schema_hash: 0,
        key_spec_hash: 9,
        force: None,
    };
    let plan = plan_run(items, &p)?;
    let len = usize::try_from(plan.layout.len).map_err(|_| Fault::OutOfMemory)?;
    let fl = usize::try_from(plan.filter_scratch()).map_err(|_| Fault::OutOfMemory)?;
    let (img_buf, rest) = scratch.split_at_mut_checked(len).ok_or(Fault::Scratch { need: plan.layout.len })?;
    let fbuf = rest.get_mut(..fl).ok_or(Fault::Scratch { need: plan.filter_scratch() })?;
    build_run(items, &p, &plan, &mut MemRunSink, img_buf, fbuf)?;
    let mut img = Image::EMPTY;
    for piece in img_buf.chunks(4_096) {
        img.append(h, tbl, piece)?;
    }
    let img = img.ref_form(h, tbl)?;
    let r = image_radix(&img)?;
    let hd = plan.header;
    Ok(RunDesc {
        root: r.root,
        items: hd.rows.wrapping_add(hd.tombs),
        rid_base: 0,
        rid_lo: hd.rid_lo,
        rid_hi: hd.rid_hi,
        start_seq: native_table::index::directory::take_run_seq(h, tbl)?,
        img_height: r.height,
        flags: 0,
        tree_height: u8::try_from(hd.height).unwrap_or(0),
        v: u16::try_from(hd.v).unwrap_or(0),
        w: u16::try_from(hd.w).unwrap_or(0),
    })
}

/// The bytes of the count probe's host and work space.
pub const COUNT_MEM: usize = 100 * 1_024 * 1_024;

/// The bytes of the calibration canister's host and work space: room for every workload.
pub const CALIB_MEM: usize = 320 * 1_024 * 1_024;

/// The count of one key over an index's uploaded segments, at 100,000 rows of 100,000 account keys: `loaded`
/// loaded runs holding the 100,000 rows between them, the index's tree empty. With `count` 0 only the runs are
/// built; with 1 the key's count is read too, as the index counts (its tree, then each run's probe record,
/// then its filter and its count tree or one descent), so the difference of the two calls' fuel is the
/// count's cost; 2 to 4 cut the count opening every run at a stage (`count_stage`); 5 counts with every run
/// opened from its header. `key_sel` is the counted key's position (`bar_value`).
#[no_mangle]
pub extern "C" fn nt_count_bar(count: u32, loaded: u32, key_sel: u64) -> u64 {
    let mut mem = [0u8; COUNT_MEM];
    code(count_case(&mut mem, count, loaded, key_sel))
}

/// The work of `nt_count_bar` over a caller's memory (the native tests run it on the heap).
pub fn count_case(mem: &mut [u8], count: u32, loaded: u32, key_sel: u64) -> Result<u64, Fault> {
    let spec = BarSpec { count, loaded, key_sel, options: 0, sweep: 0 };
    count_bar(mem, &spec, no_counter, &mut || {})
}

/// A key option of a run whose keys are stepped through: text keys, paths of 60 to 200 bytes sharing a
/// 40-byte prefix.
pub const BAR_PATHS: u64 = 4;
/// A key option: text keys of 2,000 bytes sharing a 1,500-byte prefix.
pub const BAR_LONG: u64 = 8;
/// A count bar's option: every run opened from its header (the count before probe records), for reference.
pub const BAR_OPENED: u64 = 16;
/// A key option: text keys of 18 bytes, `b`, the 16 hexadecimal digits of the key's value as the letters `a`
/// to `p`, and a zero byte.
pub const BAR_SHORT: u64 = 32;
/// A count bar's option: each count first does the crate's fixed part of an indexed count for its key (the
/// value encoded as the index's key, the index located: `Entry::run`), so that the figure holds that part
/// too.
pub const BAR_ENTRY: u64 = 128;

/// The most keys a sweep counts.
pub const SWEEP_MAX: usize = 512;

/// The values a count bar's keys are made from: 10^5 of them, as the account keys.
const BAR_KEYS: u64 = 100_000;

/// The value a count bar counts at key position `pos`: (`pos` × 7,919 mod 10^5) with its low bit cleared,
/// which the rows may hold; or, for a key that no row holds, ((1,000 + `pos`) × 7,919 mod 10^5) with its low
/// bit set (text keys' rows hold even values only). OQL's text-key count bars count the same values at the
/// same positions, so each key has OQL's figure for the same key.
pub fn bar_value(pos: u64, absent: bool) -> u64 {
    if absent {
        (pos.wrapping_add(1_000).wrapping_mul(7_919) % BAR_KEYS) | 1
    } else {
        (pos.wrapping_mul(7_919) % BAR_KEYS) & !1
    }
}

/// What a count bar builds and counts: `count`, `loaded` and `key_sel` as `count_case` takes them (`key_sel`
/// the counted key's position, `bar_value`); `options` the `BAR_` bits; `sweep` 0 to count the one key at
/// `key_sel`, else the number of keys counted one after another (those at `key_sel`, `key_sel` + 1, …), the
/// result then being the most instructions one count took, read with the caller's counter.
#[derive(Clone, Copy, Debug)]
pub struct BarSpec {
    pub count: u32,
    pub loaded: u32,
    pub key_sel: u64,
    pub options: u64,
    pub sweep: u64,
}

/// The count bars' work: an index's loaded runs as `count_case` builds them, with the options of `spec`, then
/// its count (or `spec.sweep` counts, each read with `counter`). `mark` is called once everything is built.
pub fn count_bar(mem: &mut [u8], spec: &BarSpec, counter: fn() -> u64, mark: &mut dyn FnMut()) -> Result<u64, Fault> {
    count_bar_keys(mem, spec, counter, mark, &mut [])
}

/// `count_bar`, writing each swept key's instructions, as `counter` read them, into `per_key` (as many as
/// it holds).
pub fn count_bar_keys(
    mem: &mut [u8],
    spec: &BarSpec,
    counter: fn() -> u64,
    mark: &mut dyn FnMut(),
    per_key: &mut [u64],
) -> Result<u64, Fault> {
    const ROWS: u64 = 100_000;
    let work = 4 * 1_024 * 1_024;
    let (hostmem, rest) = mem.split_at_mut_checked(mem.len().saturating_sub(work)).ok_or(Fault::BadRef)?;
    let (scratch, words) = rest.split_at_mut_checked(work / 2).ok_or(Fault::BadRef)?;
    let (kbuf, rbuf) = words.split_at_mut_checked(work / 4).ok_or(Fault::BadRef)?;
    let mut h = SliceHost::new(hostmem);
    let (tbl, ix) = table(&mut h)?;
    let mut g = Gen(0x5EED);
    let loaded = u64::from(spec.loaded);
    // Loaded runs: each over its own reservation.
    let mut base = 4_096u64;
    for j in 0..loaded {
        let rows = ROWS / loaded + u64::from(j < ROWS % loaded);
        let cnt = gen_items(&mut g, kbuf, rbuf, rows, BAR_KEYS, 0)?;
        let it = SliceItems { keys: kbuf, rids: rbuf, tombs: 0, n: cnt };
        let mut d = install_run(&mut h, tbl, &it, true, scratch)?;
        let groups = rows.div_ceil(4_096).max(1);
        d.rid_base = base;
        d.rid_lo = base;
        d.rid_hi = base + groups * 4_096 - 1;
        d.flags = D_BELIEVED | D_RELATIVE;
        let mut rl = RunList::load(&h, ix.run_list)?;
        rl.loaded_append(&mut h, tbl, &d)?;
        rl.store(&mut h)?;
        base = base.wrapping_add(groups * 4_096);
    }
    // The counted keys' values, in counting order.
    let n_keys = usize::try_from(spec.sweep.max(1)).unwrap_or(SWEEP_MAX).min(SWEEP_MAX);
    let count = if spec.options & BAR_OPENED != 0 && spec.count == 1 { COUNT_OPENED } else { spec.count };
    let one = |h: &SliceHost<'_>, key: &[u8]| -> Result<u64, Fault> {
        match count {
            0 => Ok(0),
            1 => Ok(u64::try_from(ix.count(h, key)?.max(0)).unwrap_or(u64::MAX)),
            COUNT_OPENED => count_runs_opened(h, &ix, key),
            stage => count_stage(h, &ix, key, stage),
        }
    };
    // With `BAR_ENTRY`, the crate's fixed part first: it holds the key's value; its encoded key goes to
    // `encoded`.
    let mut entry = if spec.options & BAR_ENTRY != 0 { Some(crate::costs::Entry::new(false)) } else { None };
    let mut encoded = [0u8; 16];
    let prepare = |s: u64, entry: &mut Option<crate::costs::Entry>| {
        let u = bar_value(spec.key_sel.wrapping_add(s), false);
        if let Some(e) = entry.as_mut() {
            e.plan(u, &[]);
        }
        u.to_be_bytes()
    };
    if spec.sweep == 0 {
        let key = prepare(0, &mut entry);
        mark();
        if let Some(e) = entry.as_ref() {
            e.run(&h, tbl, &mut encoded)?;
        }
        return one(&h, &key);
    }
    mark();
    let mut most = 0u64;
    for s in 0..n_keys {
        let key = prepare(s as u64, &mut entry);
        let t0 = counter();
        if let Some(e) = entry.as_ref() {
            e.run(&h, tbl, &mut encoded)?;
        }
        one(&h, &key)?;
        let spent = counter().saturating_sub(t0);
        if let Some(slot) = per_key.get_mut(s) {
            *slot = spent;
        }
        most = most.max(spent);
    }
    Ok(most)
}

/// `count_case`'s mode that counts with every run opened from its header.
pub const COUNT_OPENED: u32 = 5;

/// `count_case` with every run opened from its header: the reference for the count through probe records.
pub fn count_case_opened(mem: &mut [u8], loaded: u32, key_sel: u64) -> Result<u64, Fault> {
    count_case(mem, COUNT_OPENED, loaded, key_sel)
}

/// Calls `f` with each of the index's loaded runs, opened from its header.
fn each_run<H: PageHost + ?Sized>(
    h: &H,
    ix: &Secondary,
    mut f: impl FnMut(&Run<ImageReader<'_, H>>) -> Result<(), Fault>,
) -> Result<(), Fault> {
    let rl = RunList::load(h, ix.run_list)?;
    for i in 0..rl.loaded.runs {
        let r = rl.loaded_get(h, i)?;
        let id = native_table::ImageId::Run { index: ix.index, start_seq: r.start_seq };
        f(&Run::open(ImageReader::new(h, id, r.image(h)?), r.rid_base, r.rid_lo, r.rid_hi)?)?;
    }
    Ok(())
}

/// One stage of a count over every run, for the breakdown of its cost: 2 the run list and each run's image
/// and header; 3 also each run's fences; 4 also one descent of each run's main tree.
pub fn count_stage<H: PageHost + ?Sized>(h: &H, ix: &Secondary, key: &[u8], stage: u32) -> Result<u64, Fault> {
    let mut acc = 0u64;
    each_run(h, ix, |run| {
        acc = acc.wrapping_add(run.hdr.n);
        if stage >= 3 {
            let (_, f0, _) = run.fence(false)?;
            let (_, f1, _) = run.fence(true)?;
            acc = acc.wrapping_add(f0 ^ f1);
        }
        if stage >= 4 {
            let t = run.main()?;
            let found = t.find(&KeyProbe { key, kind: Kind::Count })?;
            acc = acc.wrapping_add(found.map_or(1, |x| x.0));
        }
        Ok(())
    })?;
    Ok(acc)
}

/// The count of `key` over the index's loaded runs with every run opened from its header, as counts read runs
/// before probe records: the reference the records' counts are checked against.
pub fn count_runs_opened<H: PageHost + ?Sized>(h: &H, ix: &Secondary, key: &[u8]) -> Result<u64, Fault> {
    let mut sum = 0i128;
    each_run(h, ix, |run| {
        sum = sum.saturating_add(i128::from(run.count_key(key)?));
        Ok(())
    })?;
    Ok(u64::try_from(sum.max(0)).unwrap_or(u64::MAX))
}

/// A counter that never moves, for callers without one.
pub fn no_counter() -> u64 {
    0
}

/// The keys of a run whose keys are stepped through, and of a count's fixed part: the classes of OQL's
/// text-key count bars.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Keys {
    /// 8-byte account keys: the value, big-endian.
    Account,
    /// Paths of 60 to 200 bytes sharing a 40-byte prefix.
    Paths,
    /// Keys of 2,000 bytes sharing a 1,500-byte prefix.
    Long,
    /// Keys of 18 bytes (`BAR_SHORT`).
    Short,
}

impl Keys {
    fn of(options: u64) -> Keys {
        if options & BAR_LONG != 0 {
            Keys::Long
        } else if options & BAR_PATHS != 0 {
            Keys::Paths
        } else if options & BAR_SHORT != 0 {
            Keys::Short
        } else {
            Keys::Account
        }
    }

    fn text(self) -> bool {
        self != Keys::Account
    }

    /// Bytes a key takes in the item buffer.
    fn stride(self) -> usize {
        match self {
            Keys::Account => 8,
            Keys::Paths => 200,
            Keys::Long => 2_000,
            Keys::Short => 18,
        }
    }

    /// Writes the key of value `u` into `out`; returns its length.
    fn key(self, u: u64, out: &mut [u8]) -> Result<usize, Fault> {
        match self {
            Keys::Account => {
                native_table::mem::put(out, 0, &u.to_be_bytes())?;
                Ok(8)
            }
            Keys::Paths => text_key(false, u, out),
            Keys::Long => text_key(true, u, out),
            Keys::Short => short_key(u, out),
        }
    }
}

/// Bytes of a text key's prefix and its length (paths: 60 to 200 by the key's value).
fn text_shape(long: bool, u: u64) -> (usize, usize) {
    if long {
        (1_500, 2_000)
    } else {
        // mix(u) % 141 < 141.
        (40, 60 + (native_table::hash::mix(u) % 141) as usize)
    }
}

/// Writes the text key of value `u` into `out`: the shared prefix, the 16 hexadecimal digits of u as letters
/// (so keys sort as their values), filler up to the key's length less one, and a terminating zero byte (no
/// other byte is zero, so the keys are prefix-free). Returns its length.
fn text_key(long: bool, u: u64, out: &mut [u8]) -> Result<usize, Fault> {
    let (prefix, len) = text_shape(long, u);
    let key = out.get_mut(..len).ok_or(Fault::BadRef)?;
    for (i, b) in key.iter_mut().enumerate() {
        *b = if i < prefix {
            b'a' + (i % 23) as u8
        } else if i < prefix + 16 {
            b'a' + ((u >> (4 * (15 - (i - prefix)))) & 15) as u8
        } else if i + 1 < len {
            b'q' + (i % 7) as u8
        } else {
            0
        };
    }
    Ok(len)
}

/// Writes the 18-byte key of value `u` into `out`: `b`, the 16 hexadecimal digits of u as letters, a zero
/// byte. Returns its length.
fn short_key(u: u64, out: &mut [u8]) -> Result<usize, Fault> {
    let key = out.get_mut(..18).ok_or(Fault::BadRef)?;
    for (i, b) in key.iter_mut().enumerate() {
        *b = if i == 0 {
            b'b'
        } else if i < 17 {
            b'a' + ((u >> (4 * (16 - i))) & 15) as u8
        } else {
            0
        };
    }
    Ok(18)
}

/// The key of value `u` of a count bar's text class (1: 18 bytes, 2: a path, 3: 2,000 bytes), its
/// terminating zero byte included, into `out`: its length.
pub fn bar_text_key(class: u64, u: u64, out: &mut [u8]) -> Result<usize, Fault> {
    match class {
        1 => short_key(u, out),
        2 => text_key(false, u, out),
        3 => text_key(true, u, out),
        _ => Err(Fault::BadRef),
    }
}

/// Items of text keys: their values (sorted), rids and keys written at the class's stride.
pub struct TextItems<'a> {
    keys_of: Keys,
    values: &'a [u8],
    rids: &'a [u8],
    keys: &'a [u8],
    n: u64,
}

impl RunInput for TextItems<'_> {
    fn items(&self) -> u64 {
        self.n
    }
    fn item(&self, i: u64) -> Result<Item<'_>, Fault> {
        if i >= self.n {
            return Err(Fault::BadRef);
        }
        let u = native_table::mem::rd_u64(self.values, i.wrapping_mul(8))?;
        let len = match self.keys_of {
            Keys::Paths => text_shape(false, u).1,
            Keys::Long => text_shape(true, u).1,
            Keys::Short => 18,
            Keys::Account => 8,
        };
        let at = usize::try_from(i).map_err(|_| Fault::BadRef)?.wrapping_mul(self.keys_of.stride());
        let key = self.keys.get(at..at.wrapping_add(len)).ok_or(Fault::BadRef)?;
        let rid = native_table::mem::rd_u64(self.rids, i.wrapping_mul(8))?;
        Ok(Item { key, rid, tomb: false })
    }
}

/// Draws `n` values below `keys` (even only, with `odd_free`) and sorts them into `values` with fresh rids from
/// `rid0` on in `rids`, both as 8-byte little-endian words.
fn gen_values(
    g: &mut Gen,
    values: &mut [u8],
    rids: &mut [u8],
    n: u64,
    keys: u64,
    rid0: u64,
    odd_free: bool,
) -> Result<(), Fault> {
    let nu = usize::try_from(n).map_err(|_| Fault::BadRef)?;
    for j in 0..n {
        let v = g.next() % keys.max(1);
        put_u64(values, j.wrapping_mul(8), if odd_free { v & !1 } else { v })?;
        put_u64(rids, j.wrapping_mul(8), rid0.wrapping_add(j))?;
    }
    sort_words(values.get_mut(..nu.wrapping_mul(8)).ok_or(Fault::BadRef)?);
    Ok(())
}

/// The key-emitting reader's work: one run of `rows` rows of the key class of `options` (its `BAR_` key bits),
/// values below 10^5, then the run's distinct keys in order through `Run::key_step` (backward when `rev`), each
/// handed out whole, a shared prefix re-attached.
/// `mark` starts the window once the run is open. Returns the keys stepped, less 1 when a step does not move.
pub fn key_steps(mem: &mut [u8], rows: u64, options: u64, rev: bool, mark: &mut dyn FnMut()) -> Result<u64, Fault> {
    let cls = Keys::of(options);
    let work = 48 * 1_024 * 1_024;
    let (hostmem, rest) = mem.split_at_mut_checked(mem.len().saturating_sub(work)).ok_or(Fault::BadRef)?;
    let (scratch, words) = rest.split_at_mut_checked(24 * 1_024 * 1_024).ok_or(Fault::BadRef)?;
    let (kbuf, rbuf) = words.split_at_mut_checked(2 * 1_024 * 1_024).ok_or(Fault::BadRef)?;
    let mut h = SliceHost::new(hostmem);
    let (tbl, _) = table(&mut h)?;
    let mut g = Gen(0x5EED);
    let (rids, keys) = rbuf.split_at_mut_checked(1_024 * 1_024).ok_or(Fault::BadRef)?;
    gen_values(&mut g, kbuf, rids, rows, BAR_KEYS, 1, cls.text())?;
    let d = if cls.text() {
        let stride = cls.stride();
        for i in 0..rows {
            let u = native_table::mem::rd_u64(kbuf, i.wrapping_mul(8))?;
            let at = usize::try_from(i).map_err(|_| Fault::BadRef)?.wrapping_mul(stride);
            cls.key(u, keys.get_mut(at..).ok_or(Fault::BadRef)?)?;
        }
        let it = TextItems { keys_of: cls, values: kbuf, rids, keys, n: rows };
        install_run(&mut h, tbl, &it, false, scratch)?
    } else {
        for i in 0..rows {
            let u = native_table::mem::rd_u64(kbuf, i.wrapping_mul(8))?;
            native_table::mem::put(kbuf, i.wrapping_mul(8), &u.to_be_bytes())?;
        }
        let it = SliceItems { keys: kbuf, rids, tombs: 0, n: rows };
        install_run(&mut h, tbl, &it, false, scratch)?
    };
    let img = d.image(&h)?;
    let id = native_table::ImageId::Run { index: 1, start_seq: d.start_seq };
    let run = Run::open(ImageReader::new(&h, id, img), d.rid_base, d.rid_lo, d.rid_hi)?;
    let (a, b) = scratch.split_at_mut_checked(4_096).ok_or(Fault::BadRef)?;
    let b = b.get_mut(..4_096).ok_or(Fault::BadRef)?;
    let dir = if rev { native_table::tree::Dir::Rev } else { native_table::tree::Dir::Fwd };
    mark();
    let mut n = 0u64;
    let mut have: Option<usize> = None;
    loop {
        let after = have.map(|l| a.get(..l).unwrap_or(&[]));
        let Some((len, _)) = run.key_step(after, dir, 0, b)? else { break };
        if after == b.get(..len) {
            return Ok(n.wrapping_sub(1));
        }
        native_table::mem::copy_bytes(a.get_mut(..len).ok_or(Fault::BadRef)?, b.get(..len).unwrap_or(&[]));
        have = Some(len);
        n = n.wrapping_add(1);
    }
    Ok(n)
}

/// A copy-heavy unit, for the cost meter's calibration: `rounds` copies of a page's bytes between two pages
/// through the crate's region copy. Returns two words of the last page written.
#[no_mangle]
pub extern "C" fn nt_copy_unit(rounds: u64) -> u64 {
    let mut h = crate::ArrayHost::new();
    code(copy_unit(&mut h, rounds))
}

fn copy_unit(h: &mut crate::ArrayHost, rounds: u64) -> Result<u64, Fault> {
    let a = h.page_alloc()?;
    let b = h.page_alloc()?;
    let mut g = Gen(rounds | 1);
    let mut k = 0u64;
    while k < PAGE {
        put_u64(h.get_mut(Region::Page(a))?, k, g.next())?;
        k = k.wrapping_add(8);
    }
    let mut r = 0u64;
    let mut last = a;
    while r < rounds {
        let (from, to) = if r % 2 == 0 { (a, b) } else { (b, a) };
        native_table::mem::copy(h, Region::Page(from), 0, Region::Page(to), 0, PAGE)?;
        last = to;
        r = r.wrapping_add(1);
    }
    // Two words of the page, so that the unit stays dominated by its copies.
    let page = h.get(Region::Page(last))?;
    Ok(native_table::mem::rd_u64(page, 0)? ^ native_table::mem::rd_u64(page, PAGE - 8)?.rotate_left(1))
}
