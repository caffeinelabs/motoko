//! Counts of one key over an index's uploaded segments through the runs' probe records: equal to the counts of
//! the runs opened from their headers and to an independent model, for every kind of run and key; records
//! written as runs are listed; records that name no run, a missing store, faults, and run headers left unread.

use std::collections::BTreeMap;

use native_table::consts::{BLOCK, PAGE};
use native_table::image::ImageReader;
use native_table::index::runlist::{RunDesc, LOADED_PER_BLOCK, LOADED_RECORD};
use native_table::index::runprobe::{self, RunProbe, PROBE};
use native_table::index::Run;
use native_table::key::enc_bytes;
use native_table::mem::{Owner, PageHost, Region};
use native_table::{Fault, ImageId};
use nt_testhost::runs::VecItems;
use nt_testhost::uploads::TTable;
use nt_testhost::{VecSink, XorShift, FILLS};

type Items = Vec<(Vec<u8>, u64, bool)>;

fn k8(x: u64) -> Vec<u8> {
    x.to_be_bytes().to_vec()
}

fn text(s: &[u8]) -> Vec<u8> {
    let mut v = VecSink::default();
    enc_bytes(&mut v, s).unwrap();
    v.0
}

/// The count of `key` with every run opened from its header, as counts read runs before probe records.
fn opened(t: &TTable, key: &[u8]) -> Result<i128, Fault> {
    let mut sum = 0i128;
    for d in t.runs() {
        let img = d.image(&t.h)?;
        let id = ImageId::Run { index: t.ix, start_seq: d.start_seq };
        let run = Run::open(ImageReader::new(&t.h, id, img), d.rid_base, d.rid_lo, d.rid_hi)?;
        sum += i128::from(run.count_key(key)?);
    }
    Ok(sum)
}

/// The count of `key` through the probe records.
fn probed(t: &TTable, key: &[u8]) -> Result<i128, Fault> {
    runprobe::count_key(&t.h, t.run_list, t.ix, key)
}

/// Rows less tombstones per key over every item list.
fn model(lists: &[Items]) -> BTreeMap<Vec<u8>, i128> {
    let mut m = BTreeMap::new();
    for l in lists {
        for (k, _, tomb) in l {
            *m.entry(k.clone()).or_default() += if *tomb { -1 } else { 1 };
        }
    }
    m
}

/// Probe keys: every key held, its neighbours one up and one down in its last byte, the empty key, keys
/// below and above everything, and random keys of 1 to 70 bytes.
fn probe_keys(m: &BTreeMap<Vec<u8>, i128>, g: &mut XorShift) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = vec![vec![], vec![0], vec![0; 8], vec![0xFF; 8], vec![0xFF; 70]];
    for k in m.keys() {
        out.push(k.clone());
        let mut up = k.clone();
        if let Some(b) = up.last_mut() {
            *b = b.wrapping_add(1);
        }
        out.push(up);
        let mut down = k.clone();
        if let Some(b) = down.last_mut() {
            *b = b.wrapping_sub(1);
        }
        out.push(down);
    }
    for _ in 0..500 {
        let n = 1 + g.below(70) as usize;
        out.push((0..n).map(|_| g.below(256) as u8).collect());
    }
    out
}

/// Every probe key (every `step`-th) counts the same through the records, through the opened runs and in the
/// model.
fn check_counts(t: &TTable, m: &BTreeMap<Vec<u8>, i128>, g: &mut XorShift, step: usize, what: &str) {
    for k in probe_keys(m, g).into_iter().step_by(step) {
        let want = m.get(&k).copied().unwrap_or(0);
        assert_eq!(opened(t, &k).unwrap(), want, "{what}: the opened runs, key {k:02x?}");
        assert_eq!(probed(t, &k).unwrap(), want, "{what}: the probe records, key {k:02x?}");
    }
}

/// Items of `keys` distinct 8-byte keys below `space`: most keys one row, some up to 7 rows, a few 8 to 40
/// (count entries), tombstones at one item in `tomb_every` (0: none); rids from `rid0`.
fn mixed_items(g: &mut XorShift, keys: u64, space: u64, rid0: u64, tomb_every: u64) -> Items {
    let mut seen = std::collections::BTreeSet::new();
    let mut v = Items::new();
    let mut rid = rid0;
    while (seen.len() as u64) < keys {
        let k = g.below(space);
        if !seen.insert(k) {
            continue;
        }
        let r = g.below(100);
        let n = if r < 70 {
            1
        } else if r < 95 {
            2 + g.below(6)
        } else {
            8 + g.below(33)
        };
        for _ in 0..n {
            let tomb = tomb_every != 0 && g.below(tomb_every) == 0;
            v.push((k8(k), rid, tomb));
            rid += 1;
        }
    }
    v.sort();
    v
}

/// Items of text keys of 1 to 40 bytes (encoded, so prefix-free): some longer than the run's W.
fn text_items(g: &mut XorShift, keys: u64, rid0: u64) -> Items {
    let mut v = Items::new();
    let mut rid = rid0;
    let mut seen = std::collections::BTreeSet::new();
    while (seen.len() as u64) < keys {
        let n = 1 + g.below(40) as usize;
        let s: Vec<u8> = (0..n).map(|_| b'a' + g.below(4) as u8).collect();
        if !seen.insert(s.clone()) {
            continue;
        }
        for _ in 0..1 + g.below(3) {
            v.push((text(&s), rid, false));
            rid += 1;
        }
    }
    v.sort();
    v
}

#[test]
fn counts_through_probe_records_equal_opened_runs_and_the_model() {
    let mut g = XorShift(0xC0_17);
    // Runs of every shape, of absolute rids: (items, forced (W, V)).
    let mut t = TTable::new(FILLS[0]);
    let mut lists = Vec::new();
    let shapes: Vec<(Items, Option<(u64, u64)>)> = vec![
        (mixed_items(&mut g, 6_000, 40_000, 1, 0), None),
        (mixed_items(&mut g, 3_000, 40_000, 100_000, 7), Some((8, 0))),
        (mixed_items(&mut g, 3_000, 40_000, 200_000, 0), Some((8, 8))),
        (mixed_items(&mut g, 2_000, 40_000, 300_000, 5), Some((16, 32))),
        (mixed_items(&mut g, 400, 40_000, 400_000, 0), Some((8, 16))),
        // Few distinct keys of many rows: a count tree.
        ((0..6_000u64).map(|i| (k8(i % 300 * 97), 500_000 + i, false)).collect::<Items>(), Some((8, 8))),
    ];
    for (items, force) in shapes {
        let mut items = items;
        items.sort();
        let d = t.make_run(&VecItems(items.clone()), force, 0);
        t.list_run(d);
        lists.push(items);
    }
    assert!(t.runs().iter().any(|d| d.flags & native_table::index::runlist::D_COUNT_TREE != 0));
    let m = model(&lists);
    check_counts(&t, &m, &mut g, 1, "runs of every shape");
    // Runs as uploads bring them, of relative rids: more than one block of probe records, some of one node.
    let mut base = 4_096 * 1_000;
    for i in 0..70u64 {
        let n = if i % 9 == 0 { 3_000 } else { 20 + i };
        let items: Items = mixed_items(&mut g, n, 40_000, 0, 0).into_iter().map(|(k, _, t)| (k, 0, t)).collect();
        let items: Items = items.into_iter().enumerate().map(|(r, (k, _, t))| (k, r as u64, t)).collect();
        let groups = (items.len() as u64).div_ceil(4_096).max(1);
        t.add_loaded(&VecItems(items.clone()), base, groups);
        base += groups * 4_096;
        lists.push(items);
    }
    assert!(t.list().loaded.runs > LOADED_PER_BLOCK);
    let m = model(&lists);
    check_counts(&t, &m, &mut g, 5, "every run");
}

/// Items of `keys` text keys sharing a prefix of `prefix` bytes, each `min` to `max` bytes before encoding: with
/// `dense`, most keys one row, some up to 7 rows, a few 8 to 30 rows (count entries, and a count tree); else
/// 85% one row and the rest 2 to 7 (no count tree); one item in 13 a tombstone.
fn long_items(g: &mut XorShift, keys: u64, prefix: usize, min: usize, max: usize, rid0: u64, dense: bool) -> Items {
    let head: Vec<u8> = (0..prefix).map(|i| b'a' + (i % 23) as u8).collect();
    let mut v = Items::new();
    let mut rid = rid0;
    let mut seen = std::collections::BTreeSet::new();
    while (seen.len() as u64) < keys {
        let n = min + g.below((max - min + 1) as u64) as usize;
        let mut s = head.clone();
        s.extend((prefix..n).map(|_| b'a' + g.below(3) as u8));
        if !seen.insert(s.clone()) {
            continue;
        }
        let r = g.below(100);
        let rows = if !dense {
            if r < 85 {
                1
            } else {
                2 + g.below(6)
            }
        } else if r < 75 {
            1
        } else if r < 96 {
            2 + g.below(6)
        } else {
            8 + g.below(23)
        };
        for _ in 0..rows {
            v.push((text(&s), rid, g.below(13) == 0));
            rid += 1;
        }
    }
    v.sort();
    v
}

/// Probe keys for text keys: `probe_keys`, and the encodings of each held key's text cut short and extended
/// by a byte (encodings, as every probe a count is given: a text without its terminator would equal its
/// encoding once both are zero-padded to W).
fn long_probe_keys(m: &BTreeMap<Vec<u8>, i128>, g: &mut XorShift) -> Vec<Vec<u8>> {
    let mut out = probe_keys(m, g);
    for k in m.keys().step_by(7) {
        // The held keys are encoded texts without zero bytes: the text is the key less its 2-byte terminator.
        let s = &k[..k.len().saturating_sub(2)];
        out.push(text(&s[..s.len() / 2]));
        out.push(text(&s[..s.len().saturating_sub(1)]));
        let mut more = s.to_vec();
        more.push(0x41);
        out.push(text(&more));
    }
    out
}

#[test]
fn keys_longer_than_w_or_64_bytes_are_counted_through_the_records() {
    let mut g = XorShift(0x10_96);
    // Paths of 60 to 200 bytes sharing 40-byte prefixes, and keys of 2,000 bytes sharing 1,500-byte prefixes,
    // in runs of every width: all overflowed (W 8, 16), some overflowed (W 64, 128), none (W 256, for the
    // paths: keys longer than 64 bytes that fit W).
    for (what, prefix, min, max, keys) in
        [("paths", 40, 60, 200, 1_200u64), ("2,000-byte keys", 1_500, 2_000, 2_000, 300)]
    {
        let mut t = TTable::new(FILLS[0]);
        let mut lists = Vec::new();
        let shapes: &[Option<(u64, u64)>] = if max <= 200 {
            &[None, Some((8, 0)), Some((16, 8)), Some((64, 0)), Some((128, 16)), Some((256, 0))]
        } else {
            &[None, Some((8, 0)), Some((16, 16)), Some((64, 8))]
        };
        for (i, &force) in shapes.iter().enumerate() {
            for dense in [false, true] {
                let rid0 = 1_000_000 * (2 * i as u64 + u64::from(dense));
                let items = long_items(&mut g, keys, prefix, min, max, rid0, dense);
                let d = t.make_run(&VecItems(items.clone()), force, 0);
                t.list_run(d);
                lists.push(items);
            }
        }
        // Runs of relative rids of the same keys.
        let mut base = 4_096 * 1_000;
        for dense in [false, true, false] {
            let items = long_items(&mut g, keys / 2, prefix, min, max, 0, dense);
            let items: Items = items.into_iter().enumerate().map(|(r, (k, _, _))| (k, r as u64, false)).collect();
            t.add_loaded(&VecItems(items.clone()), base, 1);
            base += 4_096;
            lists.push(items);
        }
        let all = t.runs();
        let ct = all.iter().filter(|d| d.flags & native_table::index::runlist::D_COUNT_TREE != 0).count();
        assert!(ct > 0 && ct < all.len(), "{what}: runs with and without count trees ({ct} of {})", all.len());
        let m = model(&lists);
        let keys = long_probe_keys(&m, &mut g);
        let want: Vec<i128> = keys.iter().map(|k| m.get(k).copied().unwrap_or(0)).collect();
        for (k, w) in keys.iter().zip(&want) {
            assert_eq!(opened(&t, k).unwrap(), *w, "{what}: the opened runs, key {k:02x?}");
            assert_eq!(probed(&t, k).unwrap(), *w, "{what}: the probe records, key {k:02x?}");
        }
        // Run headers' magic broken: a run opened from its header faults, so every answer below comes through
        // the records.
        for d in &all {
            let p = header_page(&mut t, d);
            t.h.get_mut(Region::Page(p), Owner(15)).unwrap()[0] ^= 0xFF;
        }
        for (k, w) in keys.iter().zip(&want) {
            assert_eq!(probed(&t, k), Ok(*w), "{what}: key {k:02x?} without a header");
        }
    }
}

#[test]
fn probe_records_hold_the_runs_shared_prefixes() {
    let mut g = XorShift(0x5EA7);
    // Keys sharing starts of 0 to 17,000 bytes (past 16 KiB too), in runs whose keys fit their key areas,
    // overflow them, or both: each run stores the whole words its first and last keys share, short of the
    // shorter key's end, and its record holds that length; the record of a run
    // that stores none holds the bytes its fences share in the comparator's sequence (zero-padded to W), at
    // most `PREFIX_SCAN` or W. Counts through the records equal the model's.
    let classes = [(0usize, 4usize, 30usize), (5, 8, 30), (8, 9, 40), (40, 60, 200), (1_500, 2_000, 2_000)];
    for (start, min, max) in classes.into_iter().chain([(5_000, 5_100, 5_100), (17_000, 17_010, 17_100)]) {
        let mut t = TTable::new(FILLS[1]);
        let mut lists = Vec::new();
        let shapes: &[Option<(u64, u64)>] = &[None, Some((8, 0)), Some((64, 8)), Some((256, 0))];
        for (i, &force) in shapes.iter().enumerate() {
            let items = long_items(&mut g, 300, start, min, max, 1_000_000 * i as u64, i % 2 == 1);
            let d = t.make_run(&VecItems(items.clone()), force, 0);
            t.list_run(d);
            lists.push(items);
        }
        let runs = t.runs();
        for (i, d) in runs.iter().enumerate() {
            let r = record(&t, i as u64);
            assert!(r.names(d), "start {start}: record {i} names its run");
            let items = &lists[i];
            let (a, b) = (&items[0].0, &items[items.len() - 1].0);
            let shared = a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count() as u64;
            let want = native_table::index::run::shared_prefix(shared, a.len().min(b.len()) as u64);
            let hdr = native_table::index::RunHeader::read(&t.image_bytes(d)[..BLOCK as usize]).unwrap();
            assert_eq!(hdr.prefix, want, "start {start}: run {i}'s header");
            assert!(start < 8 || want >= start as u64 & !7);
            if want > 0 {
                assert_eq!(r.prefix, want, "start {start}: run {i}");
            } else {
                let w = u64::from(d.w) as usize;
                let seq = |k: &[u8]| {
                    let mut x = k.to_vec();
                    x.resize(k.len().max(w), 0);
                    x
                };
                let (a, b) = (seq(a), seq(b));
                let fences = a.iter().zip(&b).take_while(|(x, y)| x == y).count() as u64;
                assert_eq!(
                    r.prefix,
                    fences.min(runprobe::PREFIX_SCAN.max(w as u64)),
                    "start {start}: run {i} of W {w}"
                );
            }
        }
        let m = model(&lists);
        let keys = long_probe_keys(&m, &mut g);
        for k in &keys {
            let w = m.get(k).copied().unwrap_or(0);
            assert_eq!(probed(&t, k).unwrap(), w, "start {start}: key {k:02x?}");
            assert_eq!(opened(&t, k).unwrap(), w, "start {start}, opened: key {k:02x?}");
        }
        // A record of a run without a shared prefix whose fences' shared bytes are 0 skips nothing: the same
        // counts.
        let b = store_block(&t, 1);
        for i in 0..runs.len() {
            if record(&t, i as u64).lo != u64::MAX {
                t.h.get_mut(Region::Block(b), Owner(0)).unwrap()[i * PROBE as usize + 44..][..4].fill(0);
            }
        }
        for k in &keys {
            let w = m.get(k).copied().unwrap_or(0);
            assert_eq!(probed(&t, k).unwrap(), w, "start {start}, records without shared bytes: key {k:02x?}");
        }
    }
}

#[test]
fn a_run_list_or_directory_of_another_table_format_is_refused() {
    use native_table::consts::FORMAT_TABLE;
    use native_table::index::directory;
    use native_table::{CorruptAt, Corruption};
    let header = |r: Result<i128, Fault>| matches!(r, Err(Fault::Corrupt(CorruptAt { kind: Corruption::Header, .. })));
    let items: Items = (0..300u64).map(|i| (k8(3 * i), i + 1, false)).collect();
    let mut t = TTable::new(FILLS[1]);
    let d = t.make_run(&VecItems(items), None, 0);
    t.list_run(d);
    assert_eq!(probed(&t, &k8(30)).unwrap(), 1);
    assert_eq!(probed(&t, &k8(31)).unwrap(), 0);
    // Format 2 held in a probe record's bytes 44..48 the bytes the run's fences share for every run, where this
    // format holds the length of the prefix a run stores: a run list of format 2 is refused, so no count reads
    // its records with this format's meaning.
    let rl = t.run_list;
    let old = (FORMAT_TABLE - 1).to_le_bytes();
    t.h.get_mut(Region::Block(rl), Owner(0)).unwrap()[4..6].copy_from_slice(&old);
    assert!(header(probed(&t, &k8(30))));
    t.h.get_mut(Region::Block(rl), Owner(0)).unwrap()[4..6].copy_from_slice(&FORMAT_TABLE.to_le_bytes());
    assert_eq!(probed(&t, &k8(30)).unwrap(), 1);
    // The index directory likewise.
    let (dir, _) = directory::locate(&t.h, t.tbl, t.ix).unwrap().unwrap();
    t.h.get_mut(Region::Block(dir), Owner(0)).unwrap()[4..6].copy_from_slice(&old);
    assert!(header(directory::locate(&t.h, t.tbl, t.ix).map(|_| 0)));
}

#[test]
fn key_runs_of_overflowed_keys_are_counted_through_the_records() {
    let mut g = XorShift(0x0F_10);
    let mut t = TTable::new(FILLS[1]);
    let mut lists = Vec::new();
    // Keys of a 100-byte text and an 8-byte counter: key runs of consecutive counters and rids, at widths that
    // overflow them (8, 16, 64) and one that holds them.
    let head = text(&[b'p'; 100]);
    let key = |j: u64| {
        let mut k = head.clone();
        k.extend_from_slice(&j.to_be_bytes());
        k
    };
    for (i, force) in [Some((8, 8)), Some((16, 16)), Some((64, 8)), Some((120, 8))].into_iter().enumerate() {
        let mut items: Items = Vec::new();
        let mut rid = 1_000_000 * (i as u64 + 1);
        let mut j = 10_000 * i as u64;
        let end = j + 4_000;
        while j < end {
            let stretch = 1 + g.below(30);
            for _ in 0..stretch {
                items.push((key(j), rid, false));
                j += 1;
                rid += 1;
            }
            j += 1 + g.below(3);
            rid += 5;
            if g.below(4) == 0 {
                items.push((key(j), rid, false));
                items.push((key(j), rid + 7, false));
                j += 2;
                rid += 20;
            }
        }
        items.sort();
        let d = t.make_run(&VecItems(items.clone()), force, 8);
        t.list_run(d);
        lists.push(items);
    }
    let runs = t.runs();
    assert!(runs.iter().all(|d| d.flags & native_table::index::runlist::D_KEY_RUNS != 0));
    let m = model(&lists);
    let mut keys: Vec<Vec<u8>> = m.keys().cloned().collect();
    keys.extend((0..2_000).map(|_| key(g.below(45_000))));
    let want: Vec<i128> = keys.iter().map(|k| m.get(k).copied().unwrap_or(0)).collect();
    for (k, w) in keys.iter().zip(&want) {
        assert_eq!(opened(&t, k).unwrap(), *w, "the opened runs, key {k:02x?}");
        assert_eq!(probed(&t, k).unwrap(), *w, "key {k:02x?}");
    }
    for d in &runs {
        let p = header_page(&mut t, d);
        t.h.get_mut(Region::Page(p), Owner(15)).unwrap()[0] ^= 0xFF;
    }
    for (k, w) in keys.iter().zip(&want) {
        assert_eq!(probed(&t, k), Ok(*w), "key {k:02x?} without a header");
    }
}

#[test]
fn counts_of_wide_long_and_key_run_keys() {
    let mut g = XorShift(0x57A7);
    // Text keys, some longer than the run's W.
    let mut t = TTable::new(FILLS[1]);
    let mut lists = Vec::new();
    for (i, force) in [None, Some((16, 8)), Some((8, 0)), Some((24, 16))].into_iter().enumerate() {
        let items = text_items(&mut g, 2_500, 1_000_000 * i as u64);
        let d = t.make_run(&VecItems(items.clone()), force, 0);
        t.list_run(d);
        lists.push(items);
    }
    let m = model(&lists);
    check_counts(&t, &m, &mut g, 1, "text keys");
    // Key runs: consecutive integer keys with consecutive rids, beside ordinary keys.
    let mut t = TTable::new(FILLS[0]);
    let mut lists = Vec::new();
    for (i, force) in [Some((8, 8)), Some((8, 16)), None].into_iter().enumerate() {
        let start = i as u64 * 3_000;
        let mut items: Items = (0..5_000u64).map(|j| (k8(start + j), 10_000 * i as u64 + j, false)).collect();
        items.extend((0..300u64).map(|j| (k8(100_000 + j * 3), 900_000 + j * 7 + i as u64 * 5_000, false)));
        items.sort();
        let d = t.make_run(&VecItems(items.clone()), force, 8);
        t.list_run(d);
        lists.push(items);
    }
    assert!(t.runs().iter().any(|d| d.flags & native_table::index::runlist::D_KEY_RUNS != 0));
    let m = model(&lists);
    check_counts(&t, &m, &mut g, 1, "key runs");
}

/// The block of the probe store at entry `e`.
fn store_block(t: &TTable, e: u64) -> u64 {
    t.list().loaded.probes.get(&t.h, e).unwrap()
}

/// The probe record of listed run j.
fn record(t: &TTable, j: u64) -> RunProbe {
    let b = store_block(t, 1 + j / LOADED_PER_BLOCK);
    RunProbe::read(t.h.get(Region::Block(b)).unwrap(), (j % LOADED_PER_BLOCK) * PROBE).unwrap()
}

#[test]
fn probe_records_are_written_as_runs_are_listed() {
    // Records of every listed run, in more than one block of records; the store's entry 0 is absent.
    let mut g = XorShift(0x1A57);
    let mut t = TTable::new(FILLS[0]);
    for i in 0..70u64 {
        let items = mixed_items(&mut g, 20 + 30 * (i % 5), 20_000, 100_000 * i, if i == 4 { 6 } else { 0 });
        let d = t.make_run(&VecItems(items), None, 0);
        t.list_run(d);
        for (j, d) in t.runs().iter().enumerate() {
            let r = record(&t, j as u64);
            assert!(r.names(d), "record {j} names its run");
            assert_eq!(r, RunProbe::of(&t.h, d), "record {j}");
        }
    }
    assert_eq!(store_block(&t, 0), 0);
}

#[test]
fn records_that_name_no_run_and_a_missing_store_fall_back() {
    let mut g = XorShift(0xFA11);
    let mut t = TTable::new(FILLS[1]);
    let mut lists = Vec::new();
    for i in 0..3u64 {
        let items = mixed_items(&mut g, 2_000, 30_000, 100_000 * i, 0);
        let d = t.make_run(&VecItems(items.clone()), None, 0);
        t.list_run(d);
        lists.push(items);
    }
    let items: Items = mixed_items(&mut g, 1_000, 30_000, 0, 0);
    let items: Items = items.into_iter().enumerate().map(|(r, (k, _, t))| (k, r as u64, t)).collect();
    t.add_loaded(&VecItems(items.clone()), 4_096 * 100, 1);
    lists.push(items);
    let m = model(&lists);
    // Record 1 left from another image, record 0 zeroed, the uploaded run's record from another sequence.
    let b = store_block(&t, 1);
    let blk = t.h.get_mut(Region::Block(b), Owner(0)).unwrap();
    blk[PROBE as usize + 8] ^= 0x10;
    blk[..PROBE as usize].fill(0);
    blk[3 * PROBE as usize] ^= 1;
    check_counts(&t, &m, &mut g, 1, "records that name no run");
    // No store at all: every run is opened.
    let rl = t.run_list;
    let s = t.h.get_mut(Region::Block(rl), Owner(0)).unwrap();
    s[LOADED_RECORD as usize + 56..LOADED_RECORD as usize + 64].fill(0);
    s[LOADED_RECORD as usize + 49] = 0;
    assert_eq!(t.list().loaded.probes.root, 0);
    check_counts(&t, &m, &mut g, 1, "no probe store");
}

/// The page of image block 0 of a run, writable.
fn header_page(t: &mut TTable, d: &RunDesc) -> u64 {
    let img = d.image(&t.h).unwrap();
    img.entry(&t.h, 0).unwrap()
}

#[test]
fn counts_read_no_run_header_and_fault_as_opened_runs() {
    let mut g = XorShift(0x4EAD);
    let mut t = TTable::new(FILLS[0]);
    let mut lists = Vec::new();
    for i in 0..4u64 {
        let items = mixed_items(&mut g, 3_000, 30_000, 100_000 * i, if i == 2 { 9 } else { 0 });
        let d = t.make_run(&VecItems(items.clone()), if i == 3 { Some((8, 0)) } else { None }, 0);
        t.list_run(d);
        lists.push(items);
    }
    let m = model(&lists);
    let keys = probe_keys(&m, &mut g);
    let want: Vec<i128> = keys.iter().map(|k| m.get(k).copied().unwrap_or(0)).collect();
    // Every run's header magic broken: a run opened from its header faults, the records still count.
    let runs = t.runs();
    for d in &runs {
        let p = header_page(&mut t, d);
        t.h.get_mut(Region::Page(p), Owner(15)).unwrap()[0] ^= 0xFF;
    }
    let (mut short, mut answered) = (0usize, 0usize);
    for (k, w) in keys.iter().zip(&want) {
        assert!(opened(&t, k).is_err());
        short += usize::from(k.len() <= 8);
        match probed(&t, k) {
            Ok(x) => {
                assert_eq!(x, *w, "key {k:02x?}");
                answered += usize::from(k.len() <= 8);
            }
            Err(f) => assert!(matches!(f, Fault::Corrupt { .. }), "{f:?}"),
        }
    }
    // Keys that fit W are answered from the records, unless their entries reach into another leaf.
    assert!(answered * 1_000 >= short * 995, "{answered} of {short} keys of at most 8 bytes answered");
    for d in &runs {
        let p = header_page(&mut t, d);
        t.h.get_mut(Region::Page(p), Owner(15)).unwrap()[0] ^= 0xFF;
    }
    // Damaged roots (the main tree's, and the count tree's when there is one): both paths give the same
    // fault, for every key the fences and the filter admit.
    for d in &runs {
        let hdr = native_table::index::RunHeader::read(&t.image_bytes(d)[..BLOCK as usize]).unwrap();
        let mut roots = vec![hdr.root];
        if hdr.has_count_tree() {
            roots.push(hdr.ct_root);
        }
        let img = d.image(&t.h).unwrap();
        for at in roots {
            let p = img.entry(&t.h, at / PAGE).unwrap();
            t.h.get_mut(Region::Page(p), Owner(15)).unwrap()[(at % PAGE) as usize + 2] = 9;
        }
    }
    let mut faults = 0usize;
    for k in &keys {
        let a = opened(&t, k);
        assert_eq!(probed(&t, k), a, "key {k:02x?}");
        faults += usize::from(a.is_err());
    }
    assert!(faults > 0);
}

#[test]
fn counts_equal_through_the_probe_export_and_the_opened_runs() {
    // The probe's count over its loaded runs equals the opened runs' count, key by key.
    let mut mem = vec![0u8; nt_probe::COUNT_MEM];
    for loaded in [1u32, 4, 70] {
        for key_sel in [5u64, 17, 123, 43, 0, 99_999] {
            let a = nt_probe::count_case(&mut mem, 1, loaded, key_sel).unwrap();
            let b = nt_probe::count_case_opened(&mut mem, loaded, key_sel).unwrap();
            assert_eq!(a, b, "loaded {loaded}, key {key_sel}");
        }
    }
    // Through the crate's fixed part of a count too, and over a sweep of keys.
    use nt_probe::{BarSpec, BAR_ENTRY};
    let spec = BarSpec { count: 1, loaded: 4, key_sel: 17, options: BAR_ENTRY, sweep: 0 };
    let a = nt_probe::count_bar(&mut mem, &spec, nt_probe::no_counter, &mut || {}).unwrap();
    assert_eq!(a, nt_probe::count_case(&mut mem, 1, 4, 17).unwrap());
    let spec = BarSpec { sweep: 20, ..spec };
    let mut per_key = [0u64; 20];
    nt_probe::count_bar_keys(&mut mem, &spec, nt_probe::no_counter, &mut || {}, &mut per_key).unwrap();
}

#[test]
fn count_bar_keys_take_the_values_oqls_text_key_counts_count() {
    // Positions 5, 17 and 123 of the even sweep, and of the odd sweep for keys held in none.
    assert_eq!([5u64, 17, 123].map(|p| nt_probe::bar_value(p, false)), [39_594, 34_622, 74_036]);
    assert_eq!(nt_probe::bar_value(0, true), 7_919 * 1_000 % 100_000 | 1);
    let mut k = [0u8; 2_000];
    assert_eq!(nt_probe::bar_text_key(1, 39_594, &mut k), Ok(18));
    assert_eq!(&k[..18], b"baaaaaaaaaaaajkkk\0");
}

#[test]
fn the_cost_workloads_run() {
    let mut mem = vec![0u8; nt_probe::COUNT_MEM];
    // The primary index's inserts: the same tree from the same inserts, whatever the result asked.
    for order in [0u64, 1] {
        let w = |what: u64| [2_000, order, 300, what, 7, 0];
        assert_eq!(nt_probe::primary_inserts(&mut mem, w(0), nt_probe::no_counter, &mut || {}), Ok(300));
        let d = nt_probe::primary_inserts(&mut mem, w(2), nt_probe::no_counter, &mut || {}).unwrap();
        assert_eq!(nt_probe::primary_inserts(&mut mem, w(2), nt_probe::no_counter, &mut || {}), Ok(d));
        assert!(nt_probe::primary_inserts(&mut mem, w(3), nt_probe::no_counter, &mut || {}).unwrap() >= 2);
    }
    // The count's fixed part on the crate's side: the encoded key's length (escaped and terminated text).
    for (class, len) in [(0u64, 8u64), (1, 19)] {
        let r = nt_probe::count_entry_case(&mut mem, [class, 5, 0, 0, 0, 0], &mut || {}).unwrap();
        assert_eq!(r & 0xFFFF, len, "class {class}");
    }
    // A table's inserts, updates (of buffered and of sealed rows) and removes with 0 to 3 secondary indexes,
    // each taking, replacing or removing its row.
    for (op, rows) in [(0u64, 9_300u64), (1, 8_700), (2, 9_000), (3, 9_000)] {
        for k in 0..4u64 {
            let w = |what: u64| [op, 9_000, k, 300, what, 7];
            assert_eq!(nt_probe::table_ops(&mut mem, w(0), nt_probe::no_counter, &mut || {}), Ok(300));
            assert_eq!(nt_probe::table_ops(&mut mem, w(2), nt_probe::no_counter, &mut || {}), Ok(rows));
        }
    }
}

/// Items whose keys hold 1 row (85%) or 5 to 7 rows or tombstones: many keys' entries straddle a leaf
/// boundary or a separator, and no run gets a count tree.
fn straddling_items(g: &mut XorShift, keys: u64, rid0: u64) -> Items {
    let mut v = Items::new();
    let mut rid = rid0;
    for k in 0..keys {
        let n = if g.below(100) < 85 { 1 } else { 5 + g.below(3) };
        for _ in 0..n {
            v.push((k8(k * 3 + 1), rid, g.below(11) == 0));
            rid += 1;
        }
    }
    v.sort();
    v
}

#[test]
fn keys_whose_entries_straddle_leaves_are_counted_without_a_header() {
    let mut g = XorShift(0x57AD);
    let mut t = TTable::new(FILLS[0]);
    let mut lists = Vec::new();
    // Small nodes (W = 64: 56 entries a leaf, 7 an internal node at V = 0), so trees are deep and separators
    // many; and W = 8 at V = 0 and V = 8.
    for (i, force) in [Some((64, 0)), Some((8, 0)), Some((8, 8)), Some((128, 0))].into_iter().enumerate() {
        let items = straddling_items(&mut g, 3_000, 1_000_000 * i as u64);
        let d = t.make_run(&VecItems(items.clone()), force, 0);
        t.list_run(d);
        lists.push(items);
    }
    assert!(t.runs().iter().all(|d| d.flags & native_table::index::runlist::D_COUNT_TREE == 0));
    let m = model(&lists);
    let keys: Vec<Vec<u8>> = probe_keys(&m, &mut g).into_iter().filter(|k| k.len() <= 8).collect();
    for k in &keys {
        assert_eq!(probed(&t, k).unwrap(), m.get(k).copied().unwrap_or(0), "key {k:02x?}");
    }
    // Headers broken: every key that fits W is still answered, whatever leaves its entries span.
    let runs = t.runs();
    for d in &runs {
        let p = header_page(&mut t, d);
        t.h.get_mut(Region::Page(p), Owner(15)).unwrap()[0] ^= 0xFF;
    }
    for k in &keys {
        assert_eq!(probed(&t, k), Ok(m.get(k).copied().unwrap_or(0)), "key {k:02x?} without a header");
    }
}

#[test]
fn key_run_items_are_counted_without_a_header() {
    let mut g = XorShift(0xCE7);
    let mut t = TTable::new(FILLS[1]);
    let mut lists = Vec::new();
    // A positional run (one key run), runs of key runs between ordinary keys, and a run whose first entry is
    // a key run, at several V.
    let mut shapes: Vec<(Items, Option<(u64, u64)>)> =
        vec![((0..50_000u64).map(|i| (k8(i), i, false)).collect(), None)];
    for (i, force) in [Some((8, 8)), Some((8, 16)), Some((16, 32))].into_iter().enumerate() {
        let base = 60_000 + 20_000 * i as u64;
        let mut items: Items = Vec::new();
        let mut rid = 1_000_000 * (i as u64 + 1);
        let mut key = base;
        while key < base + 15_000 {
            let stretch = 1 + g.below(40);
            for _ in 0..stretch {
                items.push((k8(key), rid, false));
                key += 1;
                rid += 1;
            }
            key += 1 + g.below(3);
            rid += 5;
            if g.below(4) == 0 {
                items.push((k8(key), rid, false));
                items.push((k8(key), rid + 7, false));
                key += 2;
                rid += 20;
            }
        }
        items.sort();
        shapes.push((items, force));
    }
    for (items, force) in shapes {
        let d = t.make_run(&VecItems(items.clone()), force, 8);
        t.list_run(d);
        lists.push(items);
    }
    let runs = t.runs();
    assert!(runs.iter().all(|d| d.flags & native_table::index::runlist::D_KEY_RUNS != 0));
    let m = model(&lists);
    let mut keys: Vec<Vec<u8>> = m.keys().cloned().collect();
    keys.extend((0..2_000).map(|_| k8(g.below(130_000))));
    let want: Vec<i128> = keys.iter().map(|k| m.get(k).copied().unwrap_or(0)).collect();
    for (k, w) in keys.iter().zip(&want) {
        assert_eq!(probed(&t, k).unwrap(), *w, "key {k:02x?}");
    }
    for d in &runs {
        let p = header_page(&mut t, d);
        t.h.get_mut(Region::Page(p), Owner(15)).unwrap()[0] ^= 0xFF;
    }
    for (k, w) in keys.iter().zip(&want) {
        assert_eq!(probed(&t, k), Ok(*w), "key {k:02x?} without a header");
    }
}

/// Items of `keys` keys of `len` bytes (a big-endian number, then filler), keys differing in their first 8
/// bytes, each of 1 row or tombstone (85%) or 5 to 7: at V = 0 one entry per item, no count entries, and
/// fewer items than twice the keys, so no count tree.
fn kept_items(g: &mut XorShift, keys: u64, len: usize, rid0: u64) -> Items {
    let mut v = Items::new();
    let mut rid = rid0;
    for k in 0..keys {
        let mut key = (k * 5 + 2).to_be_bytes().to_vec();
        key.resize(len, b'x');
        let n = if g.below(100) < 85 { 1 } else { 5 + g.below(3) };
        for _ in 0..n {
            v.push((key.clone(), rid, g.below(9) == 0));
            rid += 1;
        }
    }
    v.sort();
    v
}

/// A run's main tree at W = 1,016 and V = 0, read node by node from its image, as an independent check of
/// where a count's walk back goes.
struct MainTree<'a> {
    r: ImageReader<'a, nt_testhost::VecHost>,
    w: usize,
    leaf_cap: u64,
    internal: native_table::tree::NodeShape,
}

impl MainTree<'_> {
    /// Node n: whether it is a leaf, its entries' key areas, and its children.
    fn node(&self, n: u64) -> (bool, Vec<Vec<u8>>, Vec<u64>) {
        let b = self.r.bytes(n, BLOCK).unwrap();
        let c = u16::from_le_bytes([b[0], b[1]]) as usize;
        let leaf = u16::from_le_bytes([b[2], b[3]]) & native_table::consts::FLAG_LEAF != 0;
        assert!(c as u64 <= if leaf { self.leaf_cap } else { self.internal.cap });
        let areas: Vec<Vec<u8>> = (0..c)
            .map(|i| {
                let ko = 8 + i * (self.w + 8);
                assert_eq!(b[ko + self.w] & 0x40, 0, "no count entries");
                b[ko..ko + self.w].to_vec()
            })
            .collect();
        let children = if leaf {
            Vec::new()
        } else {
            let co = self.internal.coff as usize;
            (0..=c).map(|j| u64::from_le_bytes(b[co + 8 * j..co + 8 * j + 8].try_into().unwrap())).collect()
        };
        (leaf, areas, children)
    }

    /// Whether the count of `key` (its key area `area`, the key's first W bytes zero-padded, which no other
    /// key shares) walks back past the first entry of the node of the separator its descent keeps, that node
    /// not being the root: its descent passes a separator left of its path (the lowest one is kept), and the
    /// first entry of the kept node's subtree is the key's.
    fn climbs(&self, area: &[u8]) -> bool {
        let mut n = BLOCK;
        let mut kept: Option<(u64, bool)> = None;
        let mut root = true;
        loop {
            let (leaf, areas, children) = self.node(n);
            if leaf {
                break;
            }
            let j = areas.iter().filter(|a| a.as_slice() <= area).count();
            if j > 0 {
                kept = Some((n, root));
            }
            root = false;
            n = children[j];
        }
        let Some((p, false)) = kept else { return false };
        let mut n = p;
        loop {
            let (leaf, areas, children) = self.node(n);
            if leaf {
                return areas[0] == area;
            }
            n = children[0];
        }
    }
}

#[test]
fn keys_whose_entries_reach_above_the_kept_separators_node_are_counted() {
    let mut g = XorShift(0x6E9D);
    let mut t = TTable::new(FILLS[1]);
    // W = 1,016 at V = 0: at most 3 entries a node, so a subtree under a node that is not the root can hold
    // fewer than 8 entries, and a key's 5 to 7 entries can fill all of it. Keys of 8 bytes (counted with the
    // key area's words), of 80 bytes (longer than 64: compared as byte strings) and of 1,100 bytes (longer
    // than W: their rest in overflow records).
    let mut lists = Vec::new();
    let mut classes = Vec::new();
    for (i, len) in [8usize, 80, 1_100].into_iter().enumerate() {
        let items = kept_items(&mut g, 2_000, len, 1_000_000 * i as u64);
        let d = t.make_run(&VecItems(items.clone()), Some((1_016, 0)), 0);
        t.list_run(d);
        lists.push(items);
        classes.push((d, len));
    }
    assert!(t.runs().iter().all(|d| d.flags & native_table::index::runlist::D_COUNT_TREE == 0));
    let m = model(&lists);
    let shape = native_table::tree::IndexShape::new(1_016, 0).unwrap();
    let mut climbs = [0usize; 3];
    for (c, (d, len)) in classes.iter().enumerate() {
        let id = ImageId::Run { index: t.ix, start_seq: d.start_seq };
        let tree = MainTree {
            r: ImageReader::new(&t.h, id, d.image(&t.h).unwrap()),
            w: 1_016,
            leaf_cap: shape.leaf.cap,
            internal: shape.internal,
        };
        for k in m.keys().filter(|k| k.len() == *len) {
            let mut area = k.clone();
            area.resize(1_016.max(k.len()), 0);
            area.truncate(1_016);
            if tree.climbs(&area) {
                climbs[c] += 1;
            }
        }
    }
    // Every key class has keys whose walk must climb above the kept node.
    assert!(climbs.iter().all(|&x| x > 0), "keys that climb above the kept node: {climbs:?}");
    let mut keys: Vec<Vec<u8>> = m.keys().cloned().collect();
    for (_, len) in &classes {
        for k in 0..40u64 {
            let mut key = (k * 37 + 4).to_be_bytes().to_vec();
            key.resize(*len, b'x');
            keys.push(key);
        }
    }
    for k in &keys {
        assert_eq!(probed(&t, k).unwrap(), m.get(k).copied().unwrap_or(0), "key {:02x?}", &k[..8]);
    }
    // Headers broken: every key is still answered on the probe path, none from an opened run.
    let runs = t.runs();
    for d in &runs {
        let p = header_page(&mut t, d);
        t.h.get_mut(Region::Page(p), Owner(15)).unwrap()[0] ^= 0xFF;
    }
    for k in &keys {
        assert_eq!(probed(&t, k), Ok(m.get(k).copied().unwrap_or(0)), "key {:02x?} without a header", &k[..8]);
    }
}
