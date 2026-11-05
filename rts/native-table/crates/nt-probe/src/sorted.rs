//! The sorted walk of a secondary index, for the panic scan and the self-test: walks of an index alone and
//! through a table, their ends and a table's top rows by a column.

use native_table::index::secondary::{Secondary, Start};
use native_table::key::{self, KeyKind, KeyPart, KeySpec, SliceSink};
use native_table::table::Table;
use native_table::tree::{Dir, Path};
use native_table::{Fault, PageHost};

use super::{code, index_decl, is_fault, probe_schema, ArrayHost, ArrayHostN, Gen, ProbeRow, PROBE_COLS, PROBE_POOL};

/// Keys of the index probe.
const KEYS: usize = 16;

/// Copies `k` to the start of `out`; its length.
fn take(out: &mut [u8], k: &[u8]) -> Result<usize, Fault> {
    native_table::mem::copy_bytes(out.get_mut(..k.len()).ok_or(Fault::BadRef)?, k);
    Ok(k.len())
}

/// A secondary index of `count % 300` rows under 16 text keys (key j: 2j letters, so some are longer than the
/// key area of 8 to 32 bytes by `w`), every third row deleted again, with `run_list` as its run list block (0:
/// none): its sorted walk forward and backward, whole and in pieces of 7 from key 5, against the rows kept, and
/// its smallest and largest keys. The tree's root and node count, a fault code, or `u64::MAX − 3` for a wrong
/// answer.
#[no_mangle]
pub extern "C" fn nt_sorted(seed: u64, count: u32, w: u64, run_list: u64) -> u64 {
    const WRONG: u64 = u64::MAX - 3;
    let mut h = ArrayHost::new();
    let mut g = Gen(seed | 1);
    let r = (|| -> Result<u64, Fault> {
        let tbl = native_table::area::new_root(&mut h)?;
        let mut ix = Secondary::create(1, 8u64.wrapping_mul(1 + w % 4)).ok_or(Fault::BadRef)?;
        ix.run_list = run_list;
        let mut path = Path::new();
        let mut keys = [[0u8; 40]; KEYS];
        let mut lens = [0usize; KEYS];
        let letters = [b'a'; 32];
        for (j, (k, l)) in keys.iter_mut().zip(lens.iter_mut()).enumerate() {
            let mut sink = SliceSink::new(k);
            key::enc_bytes(&mut sink, letters.get(..2 * j).unwrap_or(&[]))?;
            *l = sink.len;
        }
        let key_of = |j: usize| keys.get(j).and_then(|k| k.get(..lens.get(j).copied().unwrap_or(0))).unwrap_or(&[]);
        let mut kept = [0i128; KEYS];
        for i in 0..count % 300 {
            let j = (g.next() % KEYS as u64) as usize;
            let (k, rid) = (key_of(j), u64::from(i) + 1);
            let slot = kept.get_mut(j).ok_or(Fault::BadRef)?;
            ix.insert(&mut h, tbl, k, rid, &mut path)?;
            *slot += 1;
            if i % 3 == 0 {
                ix.delete(&mut h, tbl, k, rid, &mut path)?;
                *slot -= 1;
            }
        }
        let parts = [KeyPart { leaf: 0, kind: KeyKind::Text, nullable: false }];
        let spec = KeySpec { parts: &parts };
        // Keys rise with j.
        let (mut key, mut buf, mut out, mut prev) = ([0u8; 64], [0u8; 128], [0u8; 64], [0u8; 64]);
        for dir in [Dir::Fwd, Dir::Rev] {
            let mut rows = 0i128;
            ix.walk(&h, &spec, dir, Start::End, u64::MAX, &mut key, &mut buf, |_, _| {
                rows += 1;
                Ok(())
            })?;
            let (mut last, mut pieces) = (None, 0i128);
            loop {
                let mut at = (0usize, 0u64);
                let start = match last {
                    Some((n, rid)) => Start::After(prev.get(..n).unwrap_or(&[]), rid),
                    None => Start::Key(key_of(5)),
                };
                let n = ix.walk(&h, &spec, dir, start, 7, &mut key, &mut buf, |k, rid| {
                    at = (take(&mut out, k)?, rid);
                    Ok(())
                })?;
                if n == 0 {
                    break;
                }
                take(&mut prev, out.get(..at.0).unwrap_or(&[]))?;
                last = Some(at);
                pieces += i128::from(n);
            }
            let from5: i128 = match dir {
                Dir::Fwd => kept.iter().skip(5).sum(),
                Dir::Rev => kept.iter().take(6).sum(),
            };
            if rows != kept.iter().sum::<i128>() || pieces != from5 {
                return Ok(WRONG);
            }
        }
        let first = kept.iter().position(|&c| c > 0).map(key_of);
        let last = kept.iter().rposition(|&c| c > 0).map(key_of);
        let min = ix.min_key(&h, &spec, &mut out, &mut buf)?.map(|n| out.get(..n) == first);
        let max = ix.max_key(&h, &spec, &mut out, &mut buf)?.map(|n| out.get(..n) == last);
        if min != first.map(|_| true) || max != last.map(|_| true) {
            return Ok(WRONG);
        }
        Ok(ix.root ^ ix.nodes.rotate_left(32))
    })();
    code(r)
}

/// A table with two secondary indexes (the Nat8; the optional Bool and the text) and `n % 300` upserts of keys
/// from `seed`: each index walked from both ends, from a row's key and after the last row handed out, and its
/// smallest and largest keys; a digest of what they gave, or a fault code.
#[no_mangle]
pub extern "C" fn nt_table_sorted(seed: u64, n: u32) -> u64 {
    let mut h = ArrayHostN::<{ 6 * 65_536 }>::new();
    let mut g = Gen(seed | 1);
    let r = (|| -> Result<u64, Fault> {
        let mut img = [0u8; 1_024];
        let indexes = [index_decl(1, &[(4 << 16, false)]), index_decl(2, &[(3 << 16, true), (1 << 16, false)])];
        let len = probe_schema(&PROBE_POOL, &PROBE_COLS, &indexes, &mut img)?;
        let (mut t, _) = Table::create(&mut h, seed, img.get(..len).unwrap_or(&[]))?;
        let mut sc = crate::ProbeScratch { buf: [0; 81_920] };
        let mut digest = 0u64;
        for i in 0..n % 300 {
            let x = g.next();
            let mut row = ProbeRow {
                key: x % 500,
                text: [b'a' + (x % 26) as u8; 24],
                len: (x % 25) as usize,
                val: x as i64 >> 3,
                flag: if x % 3 == 0 { None } else { Some(x % 2 == 0) },
                small: x % 3 + 1,
            };
            t.upsert(&mut sc, &mut row)?;
            if i % 16 == 5 {
                digest ^= walks(&t, 1 + (i % 2) as u8, &mut row)?;
            }
        }
        Ok(digest ^ t.len()?)
    })();
    code(r)
}

/// Bytes of the buffer the table's walks hold their keys in: three keys of the probe's indexes, each at most
/// 1 KiB.
const WALK_BUF: usize = 3 * 1_024;

/// A digest of index `ix`'s sorted walks: from each end, from the key `row`'s cells give, and after the last
/// row those handed out; and its smallest and largest keys.
fn walks<H: PageHost>(t: &Table<'_, H>, ix: u8, row: &mut ProbeRow) -> Result<u64, Fault> {
    let mut digest = 0u64;
    let mut key = [0u8; 64];
    let n = t.index_key(ix, row, &mut key)?;
    let mut buf = [0u8; WALK_BUF];
    for dir in [Dir::Fwd, Dir::Rev] {
        let (mut out, mut last) = ([0u8; 64], (0usize, 0u64));
        {
            let mut f = |k: &[u8], rid: u64| -> Result<(), Fault> {
                digest = native_table::hash::mix(digest ^ rid ^ k.len() as u64);
                last = (take(&mut out, k)?, rid);
                Ok(())
            };
            t.walk(ix, dir, Start::End, 4, &mut buf, &mut f)?;
            t.walk(ix, dir, Start::Key(key.get(..n).unwrap_or(&[])), 4, &mut buf, &mut f)?;
        }
        let (len, rid) = last;
        t.walk(ix, dir, Start::After(out.get(..len).unwrap_or(&[]), rid), 4, &mut buf, |_, rid| {
            digest = native_table::hash::mix(digest ^ rid);
            Ok(())
        })?;
    }
    let lo = t.min_key(ix, &mut buf)?.unwrap_or(0) as u64;
    let hi = t.max_key(ix, &mut buf)?.unwrap_or(0) as u64;
    Ok(digest ^ lo ^ hi.rotate_left(16))
}

/// The sorted walks' known answers for the self-test: 0 when they hold.
pub fn sorted_selftest() -> u32 {
    // The walks agree with the rows the index keeps, at two key widths.
    if is_fault(nt_sorted(5, 250, 0, 0)) || is_fault(nt_sorted(9, 299, 2, 0)) {
        return 50;
    }
    // A table's walks and ends run without a fault, and give the same answers twice.
    if is_fault(nt_table_sorted(1, 150)) || nt_table_sorted(3, 90) != nt_table_sorted(3, 90) {
        return 51;
    }
    0
}
