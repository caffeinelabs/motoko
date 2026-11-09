//! Workloads for the cost meter beside the count bars: the primary index's inserts as a table write makes
//! them, the fixed part of an indexed count on the crate's side, the secondary index's insert, delete and
//! count, a table's insert and delete with 0 to 3 secondary indexes, filtered scans against a scan that
//! reads every row whole, an index's group counts against a count of each of its keys, and an integer leaf's
//! minimum and maximum and a table's open.

use native_table::area::new_root;
use native_table::consts::GROUP_SLOTS;
use native_table::index::area_nodes::{AreaView, KeyLog};
use native_table::index::directory;
use native_table::index::secondary::Secondary;
use native_table::key::probe::KeyProbe;
use native_table::key::SliceSink;
use native_table::table::cell::{enc_key, Val};
use native_table::table::filter::{Cond, Filter};
use native_table::table::layout::{LeafInfo, Ty};
use native_table::table::primary::{self, Primary};
use native_table::table::totals::Total;
use native_table::table::{buffer, root, CellIn, CellOut, RowSink, RowSource, Table, Upsert};
use native_table::tree::Path;
use native_table::value::ty;
use native_table::{Fault, PageHost, Region};

use crate::segments::SliceHost;
use crate::Gen;

/// One insert of (key, rid) into the primary index's tree as a table write makes it: a path made for the
/// write, the lookup (a descent that records the path, then the first entry at or after the key), then the
/// insert at the vacant position the lookup found, with the lookup's path.
#[inline(never)]
fn primary_insert<H: PageHost>(h: &mut H, tbl: u64, p: &mut Primary, key: &[u8], rid: u64) -> Result<(), Fault> {
    let mut path = Path::new();
    let look = primary::lookup(&*h, p, key, &mut path)?;
    if look.found.is_some() {
        return Err(Fault::BadRef);
    }
    primary::insert(h, tbl, p, key, rid, look.vacant, &mut path)
}

/// A digest of every entry of the primary index's tree in order: key parts and value parts.
fn tree_digest<H: PageHost>(h: &H, p: &Primary) -> Result<u64, Fault> {
    use native_table::hash::Hasher64;
    use native_table::tree::{node, Cursor, Fwd, Shape};
    use native_table::NodeRead;
    let s = p.shape;
    let m = AreaView::new(h, s, KeyLog::default());
    let mut hs = Hasher64::new(0);
    let mut c: Cursor<Fwd> = Cursor::seek(s, &m, p.root, core::ops::Bound::<&KeyProbe<'_>>::Unbounded)?;
    let mut entries = 0u64;
    while let Some((n, i)) = c.next(s, &m, core::ops::Bound::<&KeyProbe<'_>>::Unbounded)? {
        let ns = s.shape(node::is_leaf(&m, n)?);
        hs.update(m.bytes(n, node::ko(s, &ns, i), s.ks())?);
        hs.update(m.bytes(n, node::vo(s, &ns, i), s.vs())?);
        entries = entries.wrapping_add(1);
    }
    Ok(hs.finish() ^ entries)
}

/// Inserts into the primary index's tree of shape (8, 0) at 10^5 rows, through its lookup and its insert, as
/// a table write reaches it. Words: w0 the entries inserted before the measured ones; w1 the key order (0
/// distinct random 64-bit values; 1 rising); w2 the measured inserts; w3 the result (0 the measured inserts,
/// the caller's counter read around them through `mark` and the return; 1 the most instructions one insert
/// took, read with `counter`; 2 a digest of the tree's entries after them; 3 its node count; 4 the measured
/// inserts' keys made and nothing inserted, the harness's own share of result 0's window); w4 the seed.
pub fn primary_inserts(
    mem: &mut [u8],
    w: [u64; 6],
    counter: fn() -> u64,
    mark: &mut dyn FnMut(),
) -> Result<u64, Fault> {
    let [pre, order, n, what, seed, _] = w;
    let mut h = SliceHost::new(mem);
    let tbl = new_root(&mut h)?;
    // The table root's word for the index directory blocks, which a new root leaves unwritten.
    root::set_u64(&mut h, tbl, root::INDEX_DIRS, 0)?;
    let mut p = Primary::ensure(&mut h, tbl, 8, 0)?;
    let mut g = Gen(seed | 1);
    // The primary index's keys are unique: random ones are distinct 64-bit values.
    let next_key = |_: &mut Gen, i: u64| -> [u8; 8] {
        let v = if order == 1 { i } else { i.wrapping_add(1).wrapping_mul(0x9E37_79B9_7F4A_7C15) };
        v.to_be_bytes()
    };
    for i in 0..pre {
        primary_insert(&mut h, tbl, &mut p, &next_key(&mut g, i), i.wrapping_add(1))?;
    }
    let mut most = 0u64;
    let mut acc = 0u64;
    mark();
    for i in pre..pre.saturating_add(n) {
        let key = next_key(&mut g, i);
        match what {
            1 => {
                let t0 = counter();
                primary_insert(&mut h, tbl, &mut p, &key, i.wrapping_add(1))?;
                most = most.max(counter().saturating_sub(t0));
            }
            4 => acc ^= u64::from_be_bytes(key),
            _ => primary_insert(&mut h, tbl, &mut p, &key, i.wrapping_add(1))?,
        }
    }
    match what {
        1 => Ok(most),
        2 => tree_digest(&h, &p),
        3 => Ok(p.nodes),
        4 => Ok(n ^ (acc & 0)),
        _ => Ok(n),
    }
}

/// Column `k`'s leaf in the count's table.
const K_LEAF: u32 = 65_536;
/// Column `k`'s index in the count's table.
const K_INDEX: u8 = 1;

/// The value an indexed count of one key counts, in a table whose column `k` (a Nat64 or a Text) has a hash
/// index 1.
pub struct Entry {
    text: bool,
    nat: u64,
    key: [u8; 2_000],
    len: usize,
}

impl Entry {
    /// A count of a column `k` that is a Nat64 (`text` false) or a Text.
    pub fn new(text: bool) -> Entry {
        Entry { text, nat: 0, key: [0; 2_000], len: 0 }
    }

    /// The value of `count(*) where k == <value>`: the Nat `nat`, or the text `key` (a key of the count bars
    /// less its terminating zero byte).
    pub fn plan(&mut self, nat: u64, key: &[u8]) {
        self.nat = nat;
        self.len = key.len().min(self.key.len());
        if let (Some(d), Some(k)) = (self.key.get_mut(..self.len), key.get(..self.len)) {
            native_table::mem::copy_bytes(d, k);
        }
    }

    /// The work a count of one key does on the crate's side before it reads the index's parts: the value
    /// encoded as the index's key into `key_out`, and the index's descriptor located in table `tbl`. The rest
    /// of a count's fixed part (OQL's preamble and its plan encoding, the runtime call, the reply) is the Motoko
    /// host's. Returns the encoded key's length.
    #[inline(never)]
    pub fn run<H: PageHost + ?Sized>(&self, h: &H, tbl: u64, key_out: &mut [u8]) -> Result<u64, Fault> {
        let (ty, value) = if self.text {
            (Ty::Text, Val::Bytes(self.key.get(..self.len).unwrap_or(&[])))
        } else {
            (Ty::U(8), Val::U(self.nat))
        };
        let l = LeafInfo { ordinal: 0, id: K_LEAF, ty, nullable: false, key: true, path: 0, added: 0, dropped: 0 };
        let mut s = SliceSink::new(key_out);
        enc_key(&mut s, &l, &value)?;
        let len = s.len as u64;
        let at = directory::locate(h, tbl, K_INDEX)?.map_or(0, |(b, o)| b ^ o);
        Ok(len.wrapping_add(at.rotate_left(16)))
    }
}

/// The fixed part of an indexed count on the crate's side (`Entry::run`), for the key at position w1 of the
/// count bars (`bar_value`). w0 the key class: 0 an 8-byte key (a Nat64), 1 an 18-byte text, 2 a path of 60
/// to 200 bytes, 3 a 2,000-byte text. w2: 0 the work once, the caller's counter read around it through
/// `mark` and the return; 1 nothing done after the mark, the counter's own share.
pub fn count_entry_case(mem: &mut [u8], w: [u64; 6], mark: &mut dyn FnMut()) -> Result<u64, Fault> {
    let [class, pos, what, ..] = w;
    let mut e = Entry::new(class != 0);
    // The key's text, without the terminating zero byte the count bars' keys end in.
    let u = crate::segments::bar_value(pos, false);
    let mut kb = [0u8; 2_000];
    let klen = match class {
        0 => 0,
        _ => crate::segments::bar_text_key(class, u, &mut kb)?.saturating_sub(1),
    };
    e.plan(u, kb.get(..klen).ok_or(Fault::BadRef)?);
    let mut h = SliceHost::new(mem);
    let tbl = new_root(&mut h)?;
    directory::ensure(&mut h, tbl, 1)?;
    let mut out = [0u8; 2_048];
    mark();
    if what == 1 {
        return Ok(0);
    }
    e.run(&h, tbl, &mut out)
}

/// The key of row i of the secondary-index workload: an 8-byte value, i's hash among `values` values.
#[inline(always)]
fn row_key(i: u64, values: u64, seed: u64) -> [u8; 8] {
    (native_table::hash::mix(i ^ seed) % values.max(1)).to_be_bytes()
}

/// Rows the delete workload steps over at a time, prime: deleting rows k · `DELETE_STRIDE` mod n for k
/// below n reaches each of n rows once when n is not a multiple of it.
const DELETE_STRIDE: u64 = 7_919;

/// One operation of the secondary-index workload on row i: inserts row i (rid i + 1), deletes it, or counts
/// its key.
#[inline(never)]
fn secondary_op<H: PageHost>(
    h: &mut H,
    tbl: u64,
    ix: &mut Secondary,
    path: &mut Path,
    op: u64,
    key: &[u8],
    i: u64,
) -> Result<u64, Fault> {
    match op {
        0 => Ok(u64::from(ix.insert(h, tbl, key, i.wrapping_add(1), path)?)),
        1 => Ok(u64::from(ix.delete(h, tbl, key, i.wrapping_add(1), path)?)),
        // A count is at most the rows inserted.
        _ => Ok(ix.count(&*h, key)? as u64),
    }
}

/// The secondary index (`index::secondary`) at its insert, delete and count, keys of 8 bytes (W 8) among w2
/// values: w1 rows inserted first, in rid order (row i's key `row_key(i)`, its rid i + 1); then w3 measured
/// operations by w0: 0 inserts of the next rows; 1 deletes of rows taken `DELETE_STRIDE` apart among the
/// first w1, each once; 2 counts of the keys of rows so taken. w4 the result: 0 the measured operations,
/// the caller's counter read around them through `mark` and the return; 1 the most instructions one
/// operation took, read with `counter`; 2 the sum of the operations' results (rows inserted, rows deleted,
/// or the counts), checked against what they must give; 3 the tree's node count after them; 4 the
/// operations' keys made and nothing done, the harness's own share of result 0's window. w5 the seed.
pub fn secondary_ops(mem: &mut [u8], w: [u64; 6], counter: fn() -> u64, mark: &mut dyn FnMut()) -> Result<u64, Fault> {
    let [op, pre, values, n, what, seed] = w;
    let mut h = SliceHost::new(mem);
    let tbl = new_root(&mut h)?;
    let mut ix = Secondary::create(1, 8).ok_or(Fault::BadRef)?;
    let mut path = Path::new();
    for i in 0..pre {
        if !ix.insert(&mut h, tbl, &row_key(i, values, seed), i.wrapping_add(1), &mut path)? {
            return Err(Fault::BadRef);
        }
    }
    let row = |k: u64| match op {
        0 => pre.wrapping_add(k),
        _ => k.wrapping_mul(DELETE_STRIDE) % pre.max(1),
    };
    let (mut most, mut sum, mut acc) = (0u64, 0u64, 0u64);
    mark();
    for k in 0..n {
        let i = row(k);
        let key = row_key(i, values, seed);
        match what {
            1 => {
                let t0 = counter();
                sum = sum.wrapping_add(secondary_op(&mut h, tbl, &mut ix, &mut path, op, &key, i)?);
                most = most.max(counter().saturating_sub(t0));
            }
            4 => acc ^= u64::from_be_bytes(key),
            _ => sum = sum.wrapping_add(secondary_op(&mut h, tbl, &mut ix, &mut path, op, &key, i)?),
        }
    }
    match what {
        1 => Ok(most),
        2 => Ok(sum),
        3 => Ok(ix.nodes),
        4 => Ok(n ^ (acc & 0)),
        _ => Ok(n),
    }
}

/// A row of the table workload {key Nat, amount Nat, kind Nat, ts Nat}: row i has key i, and amount, kind
/// and ts from i's hash, among 10^3 values, 3 values, and nearly all distinct.
struct OpsRow(u64, u64);

impl OpsRow {
    fn of(i: u64, seed: u64) -> OpsRow {
        OpsRow(i, native_table::hash::mix(i ^ seed))
    }
}

impl RowSource for OpsRow {
    fn cell(&mut self, leaf: u32) -> Result<CellIn<'_>, Fault> {
        Ok(CellIn::U64(match leaf >> 16 {
            0 => self.0,
            1 => (self.1 >> 16) % 1_000,
            2 => (self.1 >> 8) % 3,
            _ => self.1,
        }))
    }
    fn fill(&mut self, _leaf: u32, _at: u64, _dst: &mut [u8]) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
}

/// A row of the table workload shaped as the write benchmark's rows {key Nat, ts Int, amount Nat, kind Text,
/// owner Principal, note Text, flag Bool}: row i has key i, ts 1.76 x 10^18 + 1,000 i (10^12 more under an
/// odd seed), amount 7 i mod 1,000, kind "burn", "mint" or "xfer" by i mod 10 and 3, owner i mod 1,000 (29
/// bytes), a note of 10 + i mod 51 letters, and flag i even.
struct OpsRowR {
    i: u64,
    ts: i64,
    owner: [u8; 29],
    note: [u8; 60],
}

impl OpsRowR {
    fn of(i: u64, seed: u64) -> OpsRowR {
        let (k, o) = (i64::try_from(i).unwrap_or(i64::MAX), i % 1_000);
        let ts = 1_760_000_000_000_000_000i64.saturating_add(k.saturating_mul(1_000));
        let mut r = OpsRowR {
            i,
            ts: ts.saturating_add(if seed % 2 == 1 { 1_000_000_000_000 } else { 0 }),
            owner: [0; 29],
            note: [0; 60],
        };
        for (j, b) in (0u64..).zip(r.owner.iter_mut()) {
            *b = (o.wrapping_mul(31).wrapping_add(j.wrapping_mul(7)).wrapping_add(3) % 256) as u8;
        }
        for (j, b) in (0u64..).zip(r.note.iter_mut()) {
            *b = b'a'.wrapping_add((i.wrapping_add(j) % 26) as u8);
        }
        r
    }
}

impl RowSource for OpsRowR {
    fn cell(&mut self, leaf: u32) -> Result<CellIn<'_>, Fault> {
        let i = self.i;
        Ok(match leaf >> 16 {
            0 => CellIn::U64(i),
            1 => CellIn::I64(self.ts),
            2 => CellIn::U64(i.wrapping_mul(7) % 1_000),
            3 if i % 10 == 0 => CellIn::Bytes(b"burn"),
            3 if i % 3 == 0 => CellIn::Bytes(b"mint"),
            3 => CellIn::Bytes(b"xfer"),
            4 => CellIn::Bytes(&self.owner),
            5 => CellIn::Bytes(self.note.get(..usize::try_from(10 + i % 51).unwrap_or(0)).unwrap_or(&[])),
            _ => CellIn::Bool(i % 2 == 0),
        })
    }
    fn fill(&mut self, _leaf: u32, _at: u64, _dst: &mut [u8]) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
}

/// One operation of the table workload on row i (shaped as `OpsRowR` when `r`): inserts it (0), removes it
/// (1), or updates it to the values of another seed (2 and 3); 1 when it did.
#[inline(never)]
fn table_op<H: PageHost, S: native_table::Scratch>(
    t: &mut Table<'_, H>,
    sc: &mut S,
    op: u64,
    i: u64,
    seed: u64,
    r: bool,
) -> Result<u64, Fault> {
    let seed = if matches!(op, 2 | 3) { !seed } else { seed };
    if r {
        row_op(t, sc, op, &mut OpsRowR::of(i, seed))
    } else {
        row_op(t, sc, op, &mut OpsRow::of(i, seed))
    }
}

/// `table_op` with its row made.
fn row_op<H: PageHost, S: native_table::Scratch, R: RowSource>(
    t: &mut Table<'_, H>,
    sc: &mut S,
    op: u64,
    row: &mut R,
) -> Result<u64, Fault> {
    match op {
        0 => Ok(u64::from(matches!(t.upsert(sc, row)?.0, Upsert::Inserted(_)))),
        2 | 3 => Ok(u64::from(matches!(t.upsert(sc, row)?.0, Upsert::Replaced(_)))),
        _ => Ok(u64::from(t.remove(sc, row)?.0.is_some())),
    }
}

/// A table {key Nat, amount Nat, kind Nat, ts Nat} with w2 secondary indexes (0 to 3, W 16: on amount, 10^3
/// values; then on kind, 3 values; then on ts, nearly every key one row) at its insert and delete, or with w2
/// = 16 + x the rows of `OpsRowR` and x indexes (on ts, then kind, then amount): w1 rows
/// inserted first, in key order (`OpsRow`), and every full group sealed; then w3 measured operations by
/// w0: 0 inserts of the next rows; 1 removes of rows taken `DELETE_STRIDE` apart among the first w1, each
/// once; 2 updates of the last w3 rows of the first w1, which the open group still buffers; 3 updates of rows
/// taken `DELETE_STRIDE` apart among the first w1 / 2, sealed, each once. None runs the maintenance its write
/// leaves. w4 the result: 0 the operations that
/// took, removed or replaced their row, the caller's counter read around them through `mark` and the return;
/// 1 the most instructions one operation took, read with `counter`; 2 the rows after them. w5 the seed.
///
/// w0 = 5 instead fills the table from empty as a canister runs its writes, the write-first step (metered by
/// `counter`) before each insert of the next row, w3 inserts, then the maintenance left: w4 0 the operations'
/// window as above, 1 the most instructions one message (a step and its insert) took, 2 the rows after them.
///
/// w0 = 4 instead adds index w2 + 1 (of the three, after the w2 the table is made with) by a schema change
/// after the w1 rows are sealed, and runs maintenance steps, metered by `counter`, until the index is built
/// (w3 unused): w4 0 the steps, 1 the most instructions one step took, 2 the rows when the built index counts
/// the first row's key, else `u64::MAX`.
pub fn table_ops(mem: &mut [u8], w: [u64; 6], counter: fn() -> u64, mark: &mut dyn FnMut()) -> Result<u64, Fault> {
    let [op, pre, k, n, what, seed] = w;
    if op == 4 {
        return index_build(mem, pre, k, what, seed, counter, mark);
    }
    let mut h = SliceHost::new(mem);
    let all = [
        crate::index_decl(1, &[(1 << 16, false)]),
        crate::index_decl(2, &[(2 << 16, false)]),
        crate::index_decl(3, &[(3 << 16, false)]),
    ];
    let r = k >= 16;
    let (pool, cols): (&[u8], &[u32]) = if r {
        (&[ty::NAT, ty::INT, ty::TEXT, ty::PRINCIPAL, ty::BOOL], &[0, 1, 0, 2, 3, 2, 4])
    } else {
        (&[ty::NAT], &[0; 4])
    };
    let r_all = [
        crate::index_decl(1, &[(1 << 16, false)]),
        crate::index_decl(2, &[(3 << 16, false)]),
        crate::index_decl(3, &[(2 << 16, false)]),
    ];
    let (all, k) = if r { (&r_all, k - 16) } else { (&all, k) };
    let indexes = all.get(..usize::try_from(k).unwrap_or(usize::MAX)).ok_or(Fault::BadRef)?;
    let mut img = [0u8; 1_024];
    let len = crate::probe_schema(pool, cols, indexes, &mut img)?;
    let (mut t, _) = Table::create(&mut h, 1, img.get(..len).unwrap_or(&[]))?;
    let mut sc = crate::ProbeScratch { buf: [0; 81_920] };
    if op == 5 {
        let mut most = 0u64;
        mark();
        for i in 0..n {
            let t0 = if what == 1 { counter() } else { 0 };
            t.write_step(&crate::ProbeBudget)?;
            table_op(&mut t, &mut sc, 0, i, seed, r)?;
            if what == 1 {
                most = most.max(counter().saturating_sub(t0));
            }
        }
        t.maintain(&crate::ProbeBudget)?;
        return match what {
            1 => Ok(most),
            2 => t.len(),
            _ => Ok(0),
        };
    }
    for i in 0..pre {
        table_op(&mut t, &mut sc, 0, i, seed, r)?;
    }
    t.maintain(&crate::ProbeBudget)?;
    let row = |j: u64| match op {
        0 => pre.wrapping_add(j),
        2 => pre.saturating_sub(n).wrapping_add(j),
        3 => j.wrapping_mul(DELETE_STRIDE) % (pre / 2).max(1),
        _ => j.wrapping_mul(DELETE_STRIDE) % pre.max(1),
    };
    let (mut most, mut sum) = (0u64, 0u64);
    mark();
    for j in 0..n {
        if what == 1 {
            let t0 = counter();
            sum = sum.wrapping_add(table_op(&mut t, &mut sc, op, row(j), seed, r)?);
            most = most.max(counter().saturating_sub(t0));
        } else {
            sum = sum.wrapping_add(table_op(&mut t, &mut sc, op, row(j), seed, r)?);
        }
    }
    match what {
        1 => Ok(most),
        2 => t.len(),
        _ => Ok(sum),
    }
}

/// `table_ops`' workload 4: the build of an index a schema change adds over `pre` sealed rows.
fn index_build(
    mem: &mut [u8],
    pre: u64,
    k: u64,
    what: u64,
    seed: u64,
    counter: fn() -> u64,
    mark: &mut dyn FnMut(),
) -> Result<u64, Fault> {
    let mut h = SliceHost::new(mem);
    let all = [
        crate::index_decl(1, &[(1 << 16, false)]),
        crate::index_decl(2, &[(2 << 16, false)]),
        crate::index_decl(3, &[(3 << 16, false)]),
    ];
    let k = usize::try_from(k).map_err(|_| Fault::BadRef)?;
    let before = all.get(..k).ok_or(Fault::BadRef)?;
    let after = all.get(..k.wrapping_add(1)).ok_or(Fault::BadRef)?;
    let (mut img, mut img2) = ([0u8; 1_024], [0u8; 1_024]);
    let len = crate::probe_schema_at(&[0x0A], &[0, 0, 0, 0], 1, before, &mut img)?;
    let len2 = crate::probe_schema_at(&[0x0A], &[0, 0, 0, 0], 2, after, &mut img2)?;
    let (mut t, _) = Table::create(&mut h, 1, img.get(..len).unwrap_or(&[]))?;
    let mut sc = crate::ProbeScratch { buf: [0; 81_920] };
    for i in 0..pre {
        t.upsert(&mut sc, &mut OpsRow::of(i, seed))?;
    }
    t.maintain(&crate::ProbeBudget)?;
    t.change_schema(img2.get(..len2).unwrap_or(&[]))?;
    let (mut steps, mut most) = (0u64, 0u64);
    mark();
    while native_table::table::indexes::building(t.host(), t.root())? != 0 {
        let t0 = counter();
        t.step()?;
        most = most.max(counter().saturating_sub(t0));
        steps = steps.wrapping_add(1);
    }
    match what {
        1 => Ok(most),
        2 => {
            let id = u8::try_from(k.wrapping_add(1)).map_err(|_| Fault::BadRef)?;
            Ok(if t.count(&mut sc, id, &mut OpsRow::of(0, seed))? >= 1 { t.len()? } else { u64::MAX })
        }
        _ => Ok(steps),
    }
}

/// A row of the filtered-scan workload {key Nat, amount Nat, cat Nat, ts Nat}: row i has key i, amount among
/// 10^3 values and cat among 33 from i's hash (one cat value holds about 3 % of the rows), and ts i.
struct ScanRow(u64, u64);

impl RowSource for ScanRow {
    fn cell(&mut self, leaf: u32) -> Result<CellIn<'_>, Fault> {
        Ok(CellIn::U64(match leaf >> 16 {
            0 | 3 => self.0,
            1 => (self.1 >> 16) % 1_000,
            _ => (self.1 >> 8) % 33,
        }))
    }
    fn fill(&mut self, _leaf: u32, _at: u64, _dst: &mut [u8]) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
}

/// A sink folding every cell and rid it is handed into a digest, and counting the rows.
struct Digest(u64, u64);

impl RowSink for Digest {
    fn cell(&mut self, leaf: u32, v: CellOut<'_>) -> Result<(), Fault> {
        let x = match v {
            CellOut::U64(x) => x,
            _ => u64::MAX,
        };
        self.0 = self.0.rotate_left(7) ^ x ^ u64::from(leaf);
        Ok(())
    }
    fn row_end(&mut self, rid: u64) -> Result<(), Fault> {
        self.0 = self.0.rotate_left(3) ^ rid;
        self.1 = self.1.wrapping_add(1);
        Ok(())
    }
}

/// The filtered scans of a table of w0 rows `ScanRow` (inserted in key order, every full group sealed, so
/// all but the open group's rows sealed), counted from the end of the build, by w1: 0 the count of cat = 7
/// (`count_where`); 1 the count of ts in [0.47 w0, 0.53 w0), a contiguous 6 % of the rows; 2 the count of ts
/// at least 0, every row; 3 every row through `scan_where` (ts at least 0) in batches of w2 rows, into a
/// digest; 4 every row read whole by rid (`get_rid`, a group the directory has no entry for skipped, the open
/// group up to its next free position), into the same digest; 5 the count of cat = 7 by reading every row
/// whole and testing its cat cell; 6 every row's key and amount through `scan_cells` with no filter, in batches
/// of w2 rows, into the digest; 7 every row's key, amount, cat and ts so, into the digest; 8 as 7 under the
/// filter ts at least 0; 9 every row read whole by rid as 4, only its key and amount cells into the digest; 10
/// every row's amount alone through `scan_cells` as 6; 11 every row read whole by rid as 4, only its amount cell
/// into the digest. Results: a count, or the digest of 3, 4 and 6 to 11. w5 the seed.
pub fn filter_scans(mem: &mut [u8], w: [u64; 6], mark: &mut dyn FnMut()) -> Result<u64, Fault> {
    let [n, what, batch, _, _, seed] = w;
    let mut h = SliceHost::new(mem);
    let mut img = [0u8; 1_024];
    let len = crate::probe_schema(&[ty::NAT], &[0; 4], &[], &mut img)?;
    let (mut t, _) = Table::create(&mut h, 1, img.get(..len).unwrap_or(&[]))?;
    let mut sc = crate::ProbeScratch { buf: [0; 81_920] };
    for i in 0..n {
        t.upsert(&mut sc, &mut ScanRow(i, native_table::hash::mix(i ^ seed)))?;
    }
    t.maintain(&crate::ProbeBudget)?;
    let at = |x: u64| CellIn::U64(x);
    let ts = |lo, hi| Filter { leaf: 3 << 16, cond: Cond::Range(lo, hi) };
    let cat = Filter { leaf: 2 << 16, cond: Cond::Eq(at(7)) };
    let all = ts(core::ops::Bound::Included(at(0)), core::ops::Bound::Unbounded);
    let band = ts(core::ops::Bound::Included(at(n / 100 * 47)), core::ops::Bound::Excluded(at(n / 100 * 53)));
    mark();
    match what {
        0 => t.count_where(&cat),
        1 => t.count_where(&band),
        2 => t.count_where(&all),
        3 => {
            let mut d = Digest(0, 0);
            let mut next = Some(0u64);
            while let Some(from) = next {
                next = t.scan_where(&all, from, batch, &mut d)?;
            }
            Ok(d.0)
        }
        6..=8 | 10 => {
            let leaves: &[u32] = match what {
                6 => &[0, 1 << 16],
                10 => &[1 << 16],
                _ => &[0, 1 << 16, 2 << 16, 3 << 16],
            };
            let f = if what == 8 { Some(&all) } else { None };
            let mut d = Digest(0, 0);
            let mut next = Some(0u64);
            while let Some(from) = next {
                next = t.scan_cells(leaves, f, from, batch, &mut d)?;
            }
            Ok(d.0)
        }
        4 | 5 | 9 | 11 => {
            let mut d = Digest(0, 0);
            let mut matched = 0u64;
            let h = t.host();
            let (open, _, tail) = buffer::open_group(h, t.root())?;
            let end = open.wrapping_mul(GROUP_SLOTS).wrapping_add(buffer::u16_of(h, tail, buffer::H_OPEN_POS)?);
            let mut rid = 0u64;
            while rid < end {
                let g_end = (rid / GROUP_SLOTS).wrapping_add(1).wrapping_mul(GROUP_SLOTS).min(end);
                if root::group(h, t.root(), rid / GROUP_SLOTS)? == 0 {
                    rid = g_end;
                    continue;
                }
                while rid < g_end {
                    if what == 4 {
                        t.get_rid(rid, &mut d)?;
                    } else if what == 9 || what == 11 {
                        t.get_rid(rid, &mut Amount(&mut d, what == 9))?;
                    } else {
                        let mut row = CatCell(None);
                        if t.get_rid(rid, &mut row)? && row.0 == Some(7) {
                            matched = matched.wrapping_add(1);
                        }
                    }
                    rid = rid.wrapping_add(1);
                }
            }
            Ok(if what == 5 { matched } else { d.0 })
        }
        _ => Ok(0),
    }
}

/// A sink handing a row's amount cell alone to a digest, after its key cell when `.1`.
struct Amount<'d>(&'d mut Digest, bool);

impl RowSink for Amount<'_> {
    fn cell(&mut self, leaf: u32, v: CellOut<'_>) -> Result<(), Fault> {
        if leaf >> 16 == 1 || (self.1 && leaf >> 16 == 0) {
            self.0.cell(leaf, v)?;
        }
        Ok(())
    }
    fn row_end(&mut self, rid: u64) -> Result<(), Fault> {
        self.0.row_end(rid)
    }
}

/// A sink keeping a row's cat cell.
struct CatCell(Option<u64>);

impl RowSink for CatCell {
    fn cell(&mut self, leaf: u32, v: CellOut<'_>) -> Result<(), Fault> {
        if let (2, CellOut::U64(x)) = (leaf >> 16, v) {
            self.0 = Some(x);
        }
        Ok(())
    }
    fn row_end(&mut self, _rid: u64) -> Result<(), Fault> {
        Ok(())
    }
}

/// A key of one Nat cell, whatever the leaf.
struct NatKey(u64);

impl RowSource for NatKey {
    fn cell(&mut self, _leaf: u32) -> Result<CellIn<'_>, Fault> {
        Ok(CellIn::U64(self.0))
    }
    fn fill(&mut self, _leaf: u32, _at: u64, _dst: &mut [u8]) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
}

/// The group counts of a table {key Nat, amount Nat, kind Nat, ts Nat} of w0 rows `OpsRow` (amount among 10^3
/// values, kind among 3) with indexes on amount (1) and kind (2), W 16, inserted in key order and every frozen
/// group sealed; counted from the end of the build, on index w1: w2 0 every group through `group_counts`, w3
/// groups a call, each call going on after the last key handed out; w2 1 the count (`count`) of each value
/// below w3, one call a value. Result: the groups (the values with rows) when their counts add up to the rows,
/// else `u64::MAX`. w5 the seed.
pub fn group_counts(mem: &mut [u8], w: [u64; 6], mark: &mut dyn FnMut()) -> Result<u64, Fault> {
    let [n, index, what, per, _, seed] = w;
    let mut h = SliceHost::new(mem);
    let decls = [crate::index_decl(1, &[(1 << 16, false)]), crate::index_decl(2, &[(2 << 16, false)])];
    let mut img = [0u8; 1_024];
    let len = crate::probe_schema(&[ty::NAT], &[0; 4], &decls, &mut img)?;
    let (mut t, _) = Table::create(&mut h, 1, img.get(..len).unwrap_or(&[]))?;
    let mut sc = crate::ProbeScratch { buf: [0; 81_920] };
    for i in 0..n {
        t.upsert(&mut sc, &mut OpsRow::of(i, seed))?;
    }
    t.maintain(&crate::ProbeBudget)?;
    let ix = u8::try_from(index).map_err(|_| Fault::BadRef)?;
    let (mut groups, mut rows) = (0u64, 0i128);
    mark();
    if what == 0 {
        let mut buf = [0u8; 3 * 64];
        // The key the call goes on after, and the last key the call hands out.
        let (mut after, mut last, mut held) = ([0u8; 64], [0u8; 64], None::<usize>);
        loop {
            let mut got = None;
            let from = held.map(|l| after.get(..l).unwrap_or(&[]));
            let k = t.group_counts(ix, from, per, &mut buf, |key, c| {
                groups = groups.wrapping_add(1);
                rows = rows.wrapping_add(c);
                got = Some(key.len());
                native_table::mem::copy_bytes(last.get_mut(..key.len()).ok_or(Fault::BadRef)?, key);
                Ok(())
            })?;
            if k < per {
                break;
            }
            core::mem::swap(&mut after, &mut last);
            held = got;
        }
    } else {
        for v in 0..per {
            let c = t.count(&mut sc, ix, &mut NatKey(v))?;
            groups = groups.wrapping_add(u64::from(c > 0));
            rows = rows.wrapping_add(c);
        }
    }
    Ok(if rows == i128::from(n) { groups } else { u64::MAX })
}

/// A host's memory through a shared borrow, every write refused, so that `Table::open` can be handed the
/// table's own schema page in place, as the runtime hands it.
struct InPlace<'s, 'a>(&'s SliceHost<'a>);

impl PageHost for InPlace<'_, '_> {
    fn block_of(&self, page: u64, i: u32) -> u64 {
        self.0.block_of(page, i)
    }
    fn alloc_changes(&self) -> u64 {
        self.0.alloc_changes()
    }
    fn get(&self, r: Region) -> Result<&[u8], Fault> {
        self.0.get(r)
    }
    fn get_mut(&mut self, _r: Region) -> Result<&mut [u8], Fault> {
        Err(Fault::BadRef)
    }
    fn pair_mut(&mut self, _a: Region, _b: Region) -> Result<(&mut [u8], &mut [u8]), Fault> {
        Err(Fault::BadRef)
    }
    fn read_write(&mut self, _r: Region, _wr: Region) -> Result<(&[u8], &mut [u8]), Fault> {
        Err(Fault::BadRef)
    }
    fn page_alloc(&mut self) -> Result<u64, Fault> {
        Err(Fault::BadRef)
    }
    fn page_free(&mut self, _p: u64) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
}

/// The table of `filter_scans`, w0 rows `ScanRow` inserted in key order and every full group sealed, then,
/// counted from the end of the build, by w1: 0 the minimum of amount (`extreme`); 1 its maximum; 2 the
/// minimum of ts; 3 its maximum; 4 the table opened again (`Table::open`) with its own schema page in place,
/// as the runtime opens it, and nothing else. Results: the extreme, or for 4 the table's leaves when the open
/// took the stored shape and the table is not stale; else `u64::MAX`. w5 the seed.
pub fn extremes_open(mem: &mut [u8], w: [u64; 6], mark: &mut dyn FnMut()) -> Result<u64, Fault> {
    let [n, what, _, _, _, seed] = w;
    let mut h = SliceHost::new(mem);
    let mut img = [0u8; 1_024];
    let len = crate::probe_schema(&[ty::NAT], &[0; 4], &[], &mut img)?;
    let (mut t, tbl) = Table::create(&mut h, 1, img.get(..len).unwrap_or(&[]))?;
    let mut sc = crate::ProbeScratch { buf: [0; 81_920] };
    for i in 0..n {
        t.upsert(&mut sc, &mut ScanRow(i, native_table::hash::mix(i ^ seed)))?;
    }
    t.maintain(&crate::ProbeBudget)?;
    if what == 4 {
        let page = h.get(Region::Page(root::u64_at(&h, tbl, root::SCHEMA_PAGE)?))?;
        let mut shared = InPlace(&h);
        mark();
        let t = Table::open(&mut shared, tbl, page)?;
        return Ok(if t.stale() { u64::MAX } else { t.shape().leaves });
    }
    let leaf = if what < 2 { 1 << 16 } else { 3 << 16 };
    let mut buf = [0u8; 64];
    mark();
    Ok(match t.extreme(leaf, what % 2 == 1, &mut buf)? {
        Some(Total::Small(x)) => u64::try_from(x).unwrap_or(u64::MAX),
        _ => u64::MAX,
    })
}
