//! Areas, radix arrays and image references with direct tables.
//!
//! Every test runs once per fill pattern (`FILLS`) through `per_fill`, which requires identical results and
//! identical host-call logs from the two runs: a byte that differs between them was read without being
//! written. Changed-block counts come from `VecHost::end_message`.

use std::collections::BTreeSet;

use native_table::area::{alloc_block, alloc_zeroed, free_block, image_page_alloc, new_root, AreaRecord, IMAGE_AREA};
use native_table::consts::{AREAS, BLOCK, PAGE};
use native_table::fault::corrupt;
use native_table::image::{Blocks, Image, ImageReader, BELIEVED, DIRECT};
use native_table::mem::{put_u64, rd_u64, rd_u8};
use native_table::radix::{Radix, HEIGHT_MAX};
use native_table::table::drain::free_image;
use native_table::{CorruptAt, Corruption, Fault, ImageId, PageHost, Region};
use nt_testhost::{Call, VecHost, XorShift, FILLS};

/// Runs `run` once per fill pattern on a fresh host; the results and the host-call logs must be equal.
fn per_fill<T: PartialEq + std::fmt::Debug>(run: impl Fn(&mut VecHost) -> T) -> T {
    let mut runs: Vec<(T, Vec<Call>)> = FILLS
        .iter()
        .map(|&f| {
            let mut h = VecHost::new(f);
            let r = run(&mut h);
            (r, h.take_log())
        })
        .collect();
    let second = runs.pop().unwrap();
    let first = runs.pop().unwrap();
    assert_eq!(first.0, second.0, "results differ between fill patterns");
    assert_eq!(first.1.len(), second.1.len(), "host-call logs differ in length between fill patterns");
    assert!(first.1 == second.1, "host-call logs differ between fill patterns");
    first.0
}

fn word(h: &VecHost, b: u64, off: u64) -> u64 {
    rd_u64(h.get(Region::Block(b)).unwrap(), off).unwrap()
}

fn rec(h: &VecHost, t: u64, a: u32) -> AreaRecord {
    AreaRecord::read(h, t, a).unwrap()
}

/// Every present entry of a radix array, in index order, read block by block.
fn entries(h: &VecHost, r: Radix) -> Vec<(u64, u64)> {
    fn go(h: &VecHost, b: u64, level: u8, base: u64, out: &mut Vec<(u64, u64)>) {
        for i in 0..512u64 {
            let v = word(h, b, 8 * i);
            if v == 0 {
                continue;
            }
            let at = base + i * 512u64.pow(u32::from(level));
            if level == 0 {
                out.push((at, v));
            } else {
                go(h, v, level - 1, at, out);
            }
        }
    }
    let mut out = Vec::new();
    match (r.height, r.root) {
        (_, 0) => {}
        (0, v) => out.push((0, v)),
        (height, root) => go(h, root, height - 1, 0, &mut out),
    }
    out
}

/// Every block of a radix array with its 512 entries, depth first.
fn blocks(h: &VecHost, r: Radix) -> Vec<(u64, Vec<u64>)> {
    fn go(h: &VecHost, b: u64, level: u8, out: &mut Vec<(u64, Vec<u64>)>) {
        let e: Vec<u64> = (0..512).map(|i| word(h, b, 8 * i)).collect();
        out.push((b, e.clone()));
        if level > 0 {
            for c in e.into_iter().filter(|&c| c != 0) {
                go(h, c, level - 1, out);
            }
        }
    }
    let mut out = Vec::new();
    if r.height > 0 && r.root != 0 {
        go(h, r.root, r.height - 1, &mut out);
    }
    out
}

/// The free-block stack of area a, top first.
fn free_stack(h: &VecHost, t: u64, a: u32) -> Vec<u64> {
    let mut out = Vec::new();
    let mut b = rec(h, t, a).free_head;
    while b != 0 {
        out.push(b);
        assert!(out.len() < 100_000, "free stack cycle");
        b = word(h, b, 0);
    }
    out
}

/// Deterministic image bytes.
fn data(len: usize, seed: u64) -> Vec<u8> {
    let mut r = XorShift(seed | 1);
    (0..len).map(|_| r.next_u64() as u8).collect()
}

/// Appends `bytes` in pieces of varying sizes, so pieces start and end inside and across image blocks.
fn write_image(h: &mut VecHost, t: u64, bytes: &[u8]) -> Image {
    let mut img = Image::EMPTY;
    let cuts = [1_000usize, 5, 70_000, 8, 65_536, 3, 40_000];
    let mut at = 0;
    let mut k = 0;
    while at < bytes.len() {
        let n = cuts[k % cuts.len()].min(bytes.len() - at);
        img.append(h, t, &bytes[at..at + n]).unwrap();
        at += n;
        k += 1;
    }
    img
}

/// The image's bytes, read back block by block through the reader.
fn read_back(h: &VecHost, id: ImageId, img: Image) -> Vec<u8> {
    let r = ImageReader::new(h, id, &img);
    let mut out = Vec::new();
    let mut off = 0;
    while off < r.length() {
        let n = (PAGE - off % PAGE).min(r.length() - off);
        out.extend_from_slice(r.bytes(off, n).unwrap());
        off += n;
    }
    out
}

/// Frees an image whole; returns the blocks the call changed.
fn release_image(h: &mut VecHost, t: u64, img: &Image) -> u64 {
    h.end_message();
    free_image(h, t, img).unwrap();
    h.end_message()
}

fn is_corrupt(r: Result<impl std::fmt::Debug, Fault>, image: ImageId, kind: Corruption) -> bool {
    matches!(r, Err(Fault::Corrupt(CorruptAt { image: i, kind: k, .. })) if i == image && k == kind)
}

// ---- Areas ----

#[test]
fn block_allocation_and_free_stack_reuse() {
    per_fill(|h| {
        let t = new_root(h).unwrap();
        // The root is block 0 of area 0's first page, which the test host names 65,536.
        assert_eq!(t, PAGE);
        let r0 = rec(h, t, 0);
        assert_eq!(
            r0,
            AreaRecord {
                page_list: Radix { root: PAGE, height: 0 },
                newest: PAGE,
                pages: 1,
                bump: 1,
                ..AreaRecord::default()
            }
        );
        for a in 1..AREAS as u32 {
            assert_eq!(rec(h, t, a), AreaRecord::default(), "area {a}");
        }
        // Blocks 1..16 of the first page by bump.
        for i in 1..16 {
            assert_eq!(alloc_block(h, t, 0).unwrap(), PAGE + i * BLOCK);
        }
        // A new page: its block 0, and block 1 for the page list's first radix block (height 0 → 1).
        let p2 = 2 * PAGE;
        assert_eq!(alloc_block(h, t, 0).unwrap(), p2);
        let r0 = rec(h, t, 0);
        assert_eq!((r0.newest, r0.bump, r0.pages), (p2, 2, 2));
        assert_eq!(r0.page_list, Radix { root: p2 + BLOCK, height: 1 });
        assert_eq!(entries(h, r0.page_list), vec![(0, PAGE), (1, p2)]);
        assert_eq!(alloc_block(h, t, 0).unwrap(), p2 + 2 * BLOCK);

        // The free stack: last freed, first taken.
        let (x, y) = (PAGE + BLOCK, PAGE + 2 * BLOCK);
        free_block(h, t, 0, x).unwrap();
        free_block(h, t, 0, y).unwrap();
        assert_eq!(free_stack(h, t, 0), vec![y, x]);
        assert_eq!(alloc_block(h, t, 0).unwrap(), y);
        // Test mode: the stale link of a taken block is overwritten with P.
        assert_eq!(&h.get(Region::Block(y)).unwrap()[..8], &[h.fill_pattern(); 8]);
        assert_eq!(free_stack(h, t, 0), vec![x]);
        assert_eq!(alloc_block(h, t, 0).unwrap(), x);
        assert_eq!(free_stack(h, t, 0), Vec::<u64>::new());
        assert_eq!(alloc_block(h, t, 0).unwrap(), p2 + 3 * BLOCK);

        // Area 1: its own pages, its page list's radix blocks from area 0.
        let p3 = 3 * PAGE;
        assert_eq!(alloc_block(h, t, 1).unwrap(), p3);
        assert_eq!(rec(h, t, 1).page_list, Radix { root: p3, height: 0 });
        for i in 1..16 {
            assert_eq!(alloc_block(h, t, 1).unwrap(), p3 + i * BLOCK);
        }
        let p4 = 4 * PAGE;
        assert_eq!(alloc_block(h, t, 1).unwrap(), p4);
        assert_eq!(rec(h, t, 1).page_list, Radix { root: p2 + 4 * BLOCK, height: 1 });
        assert_eq!(entries(h, rec(h, t, 1).page_list), vec![(0, p3), (1, p4)]);
        let b1 = alloc_block(h, t, 1).unwrap();
        free_block(h, t, 1, b1).unwrap();
        assert_eq!(free_stack(h, t, 1), vec![b1]);
        assert_eq!(free_stack(h, t, 0), Vec::<u64>::new());
        assert_eq!(alloc_block(h, t, 1).unwrap(), b1);

        // Reserved areas and the image area allocate no blocks; their records stay zero.
        for a in 2..=16u32 {
            assert!(is_corrupt(alloc_block(h, t, a), ImageId::Runtime, Corruption::Range), "area {a}");
            assert!(is_corrupt(free_block(h, t, a, b1), ImageId::Runtime, Corruption::Range), "area {a}");
        }
        for a in 2..IMAGE_AREA {
            assert_eq!(rec(h, t, a), AreaRecord::default());
        }
        assert!(is_corrupt(AreaRecord::read(h, t, 16), ImageId::Runtime, Corruption::Range));

        // A zero-filled block reads zero, a reused one included.
        let z = alloc_zeroed(h, t, 0).unwrap();
        free_block(h, t, 0, z).unwrap();
        assert_eq!(alloc_zeroed(h, t, 0).unwrap(), z);
        assert!(h.get(Region::Block(z)).unwrap().iter().all(|&b| b == 0));
        (0..AREAS as u32).map(|a| rec(h, t, a)).collect::<Vec<_>>()
    });
}

#[test]
fn area_page_lists() {
    per_fill(|h| {
        let t = new_root(h).unwrap();
        // Area 1 takes 600 pages; each page-taking allocation changes the root's record and at most one new
        // radix block per level of the page list: 2 blocks at the first growth, 3 at index 512.
        let mut pages = Vec::new();
        let mut appends = Vec::new();
        h.end_message();
        while pages.len() < 600 {
            let b = alloc_block(h, t, 1).unwrap();
            let changed = h.end_message();
            if b % PAGE == 0 {
                pages.push(b);
                appends.push(changed);
            } else {
                assert_eq!(changed, 1, "an allocation from the newest page changes the area record only");
            }
        }
        assert_eq!(appends[0], 1, "the first page is the list's root at height 0");
        assert_eq!(appends[1], 2, "one new radix block: the list grows to height 1");
        assert!(appends[2..512].iter().all(|&c| c == 2));
        assert_eq!(appends[512], 3, "two new radix blocks: a new root and a new leaf at height 2");
        assert!(appends[513..].iter().all(|&c| c == 2));
        let r1 = rec(h, t, 1);
        assert_eq!((r1.pages, r1.page_list.height, r1.bump), (600, 2, 1));
        let listed: Vec<u64> = entries(h, r1.page_list).into_iter().map(|(_, p)| p).collect();
        assert_eq!(listed, pages);
        let list_blocks: Vec<u64> = blocks(h, r1.page_list).into_iter().map(|(b, _)| b).collect();
        assert_eq!(list_blocks.len(), 3);

        // Area 0 to 20 pages: its own list lives in its own blocks.
        while rec(h, t, 0).pages < 20 {
            alloc_block(h, t, 0).unwrap();
        }
        let r0 = rec(h, t, 0);
        assert_eq!(r0.page_list.height, 1);
        assert_eq!(entries(h, r0.page_list).len(), 20);
        (r1, r0)
    });
}

// ---- Radix arrays ----

#[test]
fn radix_arrays_at_every_height() {
    per_fill(|h| {
        let t = new_root(h).unwrap();
        let mut r = Radix::EMPTY;
        // Height 0: the root is entry 0.
        r.set(h, t, 0, 0xA0).unwrap();
        assert_eq!(r, Radix { root: 0xA0, height: 0 });
        assert_eq!(r.get(h, 0).unwrap(), 0xA0);
        assert_eq!(r.get(h, 1).unwrap(), 0, "past the capacity reads absent");
        // Growth: each added level makes the old root its entry 0.
        let mut model: Vec<(u64, u64)> = vec![(0, 0xA0)];
        let mut heights = Vec::new();
        for (d, i) in
            [1u64, 512 + 3, (1 << 18) + 7, (1 << 27) + 1, (1 << 36) + 9, (1 << 45) + 2, 1 << 62].into_iter().enumerate()
        {
            let v = (i << 1) | 1 | (1 << 63);
            r.set(h, t, i, v).unwrap();
            model.push((i, v));
            assert_eq!(r.height as usize, d + 1);
            heights.push(r);
            for &(j, w) in &model {
                assert_eq!(r.get(h, j).unwrap(), w, "height {} entry {j}", r.height);
            }
            // Absent entries: inside a present leaf block, under an absent inner entry, and past the end.
            assert_eq!(r.get(h, 2).unwrap(), 0);
            assert_eq!(r.get(h, i ^ 0x100).unwrap(), 0);
            assert_eq!(r.get(h, i + 512).unwrap(), 0);
            assert_eq!(entries(h, r), model);
            // Every radix block reads zero but at the entries set and the links to its children: each was
            // zero-filled when allocated.
            let bl = blocks(h, r);
            let nonzero: usize = bl.iter().map(|(_, e)| e.iter().filter(|&&x| x != 0).count()).sum();
            assert_eq!(nonzero, model.len() + bl.len() - 1);
        }
        assert_eq!(r.height, HEIGHT_MAX);
        let mut full = r;
        assert!(is_corrupt(full.set(h, t, 1 << 63, 1), ImageId::Runtime, Corruption::Range));
        assert!(is_corrupt(Radix { root: 1, height: HEIGHT_MAX + 1 }.get(h, 0), ImageId::Runtime, Corruption::Depth));

        // A set into a present leaf changes that leaf only; a new leaf at height 2 changes the new leaf, the
        // root block (the allocation) and its parent. A fresh table, so no allocation below takes a page.
        let t = new_root(h).unwrap();
        let mut s = Radix::EMPTY;
        s.set(h, t, 600, 1).unwrap();
        assert_eq!(s.height, 2);
        h.end_message();
        s.set(h, t, 601, 2).unwrap();
        assert_eq!(h.end_message(), 1);
        s.set(h, t, 5, 3).unwrap();
        assert_eq!(h.end_message(), 3);
        // Clears: no block emptied, one changed block; a leaf emptied, its parent's entry cleared and the
        // leaf freed: leaf, root block (free head) and parent.
        s.clear(h, t, 601).unwrap();
        assert_eq!(h.end_message(), 1);
        let leaf = word(h, s.root, 0);
        s.clear(h, t, 5).unwrap();
        assert_eq!(h.end_message(), 3);
        assert_eq!(free_stack(h, t, 0)[0], leaf);
        assert_eq!(entries(h, s), vec![(600, 1)]);
        // The last entry: every block up to the root is freed, the root becomes 0 and the height is kept.
        let before = free_stack(h, t, 0).len();
        s.clear(h, t, 600).unwrap();
        assert_eq!(s, Radix { root: 0, height: 2 });
        assert_eq!(free_stack(h, t, 0).len(), before + 2);
        assert_eq!(entries(h, s), vec![]);
        s.set(h, t, 9, 4).unwrap();
        assert_eq!(entries(h, s), vec![(9, 4)]);
        assert_eq!(s.get(h, 600).unwrap(), 0);
        // Clearing an absent entry or one past the end changes nothing.
        h.end_message();
        s.clear(h, t, 1 << 40).unwrap();
        s.clear(h, t, 100_000).unwrap();
        // The leaf of entry 9 is unchanged by the clear of 100,000, whose leaf is absent.
        assert_eq!(h.end_message(), 0);
        // Clearing in a dense height-3 array down to nothing frees every block.
        let mut d = Radix::EMPTY;
        let idx: Vec<u64> = (0..40).map(|k| k * 7_919 + (1 << 18)).collect();
        for &i in &idx {
            d.set(h, t, i, i + 1).unwrap();
        }
        assert_eq!(d.height, 3);
        let nblocks = blocks(h, d).len();
        let before = free_stack(h, t, 0).len();
        for &i in &idx {
            d.clear(h, t, i).unwrap();
        }
        assert_eq!(d.root, 0);
        assert_eq!(free_stack(h, t, 0).len(), before + nblocks);
        heights
    });
}

// ---- Images ----

/// Lengths from 8 bytes to 3 image blocks, at and around every block boundary.
const LENGTHS: [u64; 16] = [
    8,
    16,
    24,
    4_088,
    4_096,
    4_104,
    65_528,
    65_536,
    65_544,
    100_000,
    131_064,
    131_072,
    131_080,
    196_600,
    196_600 - 8,
    196_608,
];

#[test]
fn image_round_trips_from_8_bytes_to_3_blocks() {
    for len in LENGTHS {
        per_fill(|h| {
            let t = new_root(h).unwrap();
            let host_pages = h.live_pages();
            let bytes = data(len as usize, len);
            let img = write_image(h, t, &bytes);
            let n = len.div_ceil(PAGE);
            assert_eq!(img.length, len);
            assert_eq!(img.block_count(), n);
            assert!(matches!(img.blocks, Blocks::Direct { count, .. } if u64::from(count) == n));
            assert_eq!(rec(h, t, IMAGE_AREA).pages, n);
            assert_eq!(h.live_pages(), host_pages + n as usize);

            // The record round trips through a block; reads go through the reader.
            let holder = alloc_zeroed(h, t, 0).unwrap();
            img.put_record(h.get_mut(Region::Block(holder)).unwrap(), 16).unwrap();
            let stored = h.get(Region::Block(holder)).unwrap();
            assert_eq!(rd_u8(stored, 16 + 8).unwrap(), DIRECT);
            assert_eq!(rd_u8(stored, 16 + 9).unwrap() as u64, n);
            let back = Image::read_record(stored, 16).unwrap();
            assert_eq!(back, img);
            let id = ImageId::Group(len);
            assert_eq!(read_back(h, id, back), bytes);
            let r = ImageReader::new(&*h, id, &back);
            assert_eq!(r.u64_at(len - 8).unwrap(), u64::from_le_bytes(bytes[len as usize - 8..].try_into().unwrap()));
            assert_eq!(r.bytes(len, 0).unwrap(), &[] as &[u8]);
            assert!(is_corrupt(r.bytes(len - 8, 16), id, Corruption::Bounds));
            assert!(is_corrupt(r.bytes(len, 1), id, Corruption::Bounds));
            assert!(is_corrupt(r.bytes(u64::MAX, 2), id, Corruption::Bounds));
            if len > PAGE + 8 {
                // A range across an image block boundary is refused.
                assert_eq!(r.bytes(PAGE - 8, 16), Err(corrupt(id, Corruption::Bounds, PAGE - 8)));
            }

            // The reference form: 1 block at height 0, 2 or 3 blocks in one new radix block at height 1, in one
            // block allocation and one 4 KiB write.
            h.end_message();
            let rf = img.ref_form(h, t).unwrap();
            let changed = h.end_message();
            if n == 1 {
                assert_eq!(rf, img);
                assert_eq!(changed, 0);
            } else {
                assert!(matches!(rf.blocks, Blocks::Radix(Radix { height: 1, .. })));
                assert_eq!(changed, 2);
                assert!(img.put_ref(&mut [0u8; 24], 0).is_err(), "a direct table of {n} entries has no reference form");
            }
            rf.put_ref(h.get_mut(Region::Block(holder)).unwrap(), 200).unwrap();
            let stored = h.get(Region::Block(holder)).unwrap();
            assert_eq!(rd_u8(stored, 200 + 16).unwrap() as u64, if n == 1 { 0 } else { 1 });
            assert_eq!(rd_u8(stored, 200 + 17).unwrap(), 0);
            let back_ref = Image::read_ref(stored, 200).unwrap();
            assert_eq!(back_ref, rf);
            assert_eq!(read_back(h, id, back_ref), bytes);

            // The believed flag (record byte 10, reference byte 17) is kept wherever the image moves.
            let believed = Image { believed: true, ..img };
            believed.put_record(h.get_mut(Region::Block(holder)).unwrap(), 300).unwrap();
            let moved = Image::read_record(h.get(Region::Block(holder)).unwrap(), 300).unwrap();
            assert!(moved.believed);
            let moved_ref = Image { believed: true, ..rf };
            moved_ref.put_ref(h.get_mut(Region::Block(holder)).unwrap(), 400).unwrap();
            let stored = h.get(Region::Block(holder)).unwrap();
            assert_eq!(rd_u8(stored, 400 + 17).unwrap(), BELIEVED);
            assert!(Image::read_ref(stored, 400).unwrap().believed);

            // Freed whole; every page back to the host and off area 15's count.
            let units = release_image(h, t, &rf);
            assert_eq!(rec(h, t, IMAGE_AREA).pages, 0);
            assert_eq!(h.live_pages(), host_pages);
            (rf, units)
        });
    }
}

#[test]
fn image_records_with_radix_block_lists() {
    // 9 blocks: the ninth moves the direct table to a radix of height 1; 513 blocks: height 2.
    for blocks_n in [9u64, 513] {
        per_fill(|h| {
            let t = new_root(h).unwrap();
            let host_pages = h.live_pages();
            let len = blocks_n * PAGE - 24;
            let bytes = data(len as usize, blocks_n);
            let img = write_image(h, t, &bytes);
            let want = if blocks_n == 9 { 1 } else { 2 };
            assert!(matches!(img.blocks, Blocks::Radix(Radix { height, .. }) if height == want));
            let holder = alloc_zeroed(h, t, 0).unwrap();
            img.put_record(h.get_mut(Region::Block(holder)).unwrap(), 0).unwrap();
            let stored = h.get(Region::Block(holder)).unwrap();
            assert_eq!(rd_u8(stored, 8).unwrap(), want);
            assert_eq!(rd_u8(stored, 9).unwrap(), 0, "a radix record keeps no direct entries");
            let back = Image::read_record(stored, 0).unwrap();
            assert_eq!(back, img);
            // A radix image is already in reference form.
            assert_eq!(img.ref_form(h, t).unwrap(), img);
            img.put_ref(h.get_mut(Region::Block(holder)).unwrap(), 100).unwrap();
            assert_eq!(Image::read_ref(h.get(Region::Block(holder)).unwrap(), 100).unwrap(), img);
            assert_eq!(read_back(h, ImageId::Group(2), back), bytes);
            // Freed whole, each radix block after its pages.
            let units = release_image(h, t, &img);
            assert_eq!(h.live_pages(), host_pages);
            assert_eq!(rec(h, t, IMAGE_AREA).pages, 0);
            let radix_blocks = if blocks_n == 9 { 1 } else { 3 };
            assert_eq!(free_stack(h, t, 0).len(), radix_blocks);
            units
        });
    }
}

#[test]
fn stored_forms_refuse_bad_bytes() {
    per_fill(|h| {
        let t = new_root(h).unwrap();
        let img = write_image(h, t, &data(2 * PAGE as usize, 5));
        let mut b = [0u8; 128];
        img.put_record(&mut b, 0).unwrap();
        let rt = ImageId::Runtime;
        let mut bad = b;
        bad[10] = 2;
        assert!(is_corrupt(Image::read_record(&bad, 0), rt, Corruption::Flags));
        let mut bad = b;
        bad[12] = 1;
        assert!(is_corrupt(Image::read_record(&bad, 0), rt, Corruption::Header));
        let mut bad = b;
        bad[8] = 0;
        assert!(is_corrupt(Image::read_record(&bad, 0), rt, Corruption::Header), "height 0 is no record form");
        let mut bad = b;
        bad[8] = HEIGHT_MAX + 1;
        assert!(is_corrupt(Image::read_record(&bad, 0), rt, Corruption::Header));
        let mut bad = b;
        bad[9] = 1;
        assert!(is_corrupt(Image::read_record(&bad, 0), rt, Corruption::Count), "2 blocks need 2 entries");
        let mut bad = b;
        bad[9] = 9;
        assert!(is_corrupt(Image::read_record(&bad, 0), rt, Corruption::Count));
        let mut bad = b;
        bad[8] = 1;
        assert!(
            is_corrupt(Image::read_record(&bad, 0), rt, Corruption::Header),
            "a radix record has no direct entries"
        );
        assert!(is_corrupt(Image::read_record(&b, 60), rt, Corruption::Bounds));
        // References.
        let rf = img.ref_form(h, t).unwrap();
        let mut r = [0u8; 24];
        rf.put_ref(&mut r, 0).unwrap();
        let mut bad = r;
        bad[8] = 4;
        assert!(is_corrupt(Image::read_ref(&bad, 0), rt, Corruption::Header), "a length not a multiple of 8");
        let mut bad = r;
        bad[17] = 0x80;
        assert!(is_corrupt(Image::read_ref(&bad, 0), rt, Corruption::Flags));
        let mut bad = r;
        bad[23] = 1;
        assert!(is_corrupt(Image::read_ref(&bad, 0), rt, Corruption::Header));
        let mut bad = r;
        bad[16] = 0;
        assert!(is_corrupt(Image::read_ref(&bad, 0), rt, Corruption::Count), "height 0 holds one block");
        let mut bad = r;
        bad[16] = HEIGHT_MAX + 1;
        assert!(is_corrupt(Image::read_ref(&bad, 0), rt, Corruption::Depth));
        let odd = Image {
            length: 12,
            blocks: Blocks::Direct { count: 1, entries: [PAGE, 0, 0, 0, 0, 0, 0, 0] },
            believed: false,
        };
        assert!(is_corrupt(odd.put_ref(&mut r, 0), rt, Corruption::Layout));
        assert!(odd.put_record(&mut b, 0).is_ok(), "a record may hold a length that is not a multiple of 8");
        let short = Image { length: 12, ..Image::EMPTY };
        assert!(is_corrupt(short.put_ref(&mut r, 0), rt, Corruption::Count), "no block entry for 12 bytes");
        // An area record whose bump passes 16.
        let mut root = h.get(Region::Block(t)).unwrap().to_vec();
        root[160 + 32] = 17;
        assert!(is_corrupt(AreaRecord::take(&root, 0), rt, Corruption::Header));
        // Random bytes never panic.
        let mut x = XorShift(99);
        for _ in 0..20_000 {
            let mut buf = [0u8; 96];
            for c in buf.iter_mut() {
                *c = if x.below(3) == 0 { x.next_u64() as u8 } else { 0 };
            }
            buf[8] = [DIRECT, 0, 1, 2, 7, 8, x.next_u64() as u8][x.below(7) as usize];
            let _ = Image::read_record(&buf, x.below(20));
            let _ = Image::read_ref(&buf, x.below(80));
            let _ = AreaRecord::take(&buf, x.below(20) as u32);
        }
        rf
    });
}

// ---- poisoned pages ----

/// Everything the scenario's runtime structures read.
#[derive(Debug, PartialEq)]
struct Trace {
    records: Vec<AreaRecord>,
    lists: Vec<Vec<(u64, u64)>>,
    directory: Vec<(u64, Vec<u64>)>,
    stacks: Vec<Vec<u64>>,
    images: Vec<(Image, Vec<u8>)>,
    zeroed: Vec<bool>,
    reused: Vec<u64>,
}

/// Areas, radix arrays and images written, freed and reused, pages handed back by the host and taken again.
fn poisoned_scenario(h: &mut VecHost) -> Trace {
    let t = new_root(h).unwrap();
    let mut zeroed = Vec::new();
    for _ in 0..20 {
        zeroed.push(alloc_zeroed(h, t, 0).unwrap());
    }
    // Area-1 blocks written whole, as every index node is.
    for k in 0..40u64 {
        let b = alloc_block(h, t, 1).unwrap();
        h.get_mut(Region::Block(b)).unwrap().fill(k as u8);
    }
    // Images of 8 bytes, 65,544 bytes and 3 blocks; the second and third in reference form.
    let mut images = Vec::new();
    for (i, len) in [8usize, 65_544, 3 * PAGE as usize].into_iter().enumerate() {
        let bytes = data(len, i as u64 + 1);
        let img = write_image(h, t, &bytes);
        let img = if i == 0 { img } else { img.ref_form(h, t).unwrap() };
        images.push((img, bytes));
    }
    // A sparse directory-like radix array in area 0.
    let mut dir = Radix::EMPTY;
    for (k, i) in [3u64, 900, 70_000, 5, 262_143].into_iter().enumerate() {
        dir.set(h, t, i, (1 << 63) | (k as u64 + 1)).unwrap();
    }
    // The first and third images are freed: four image pages go back to the host, and the third's radix block
    // to area 0's free stack.
    let mut freed = BTreeSet::new();
    for k in [0usize, 2] {
        let img = images[k].0;
        for j in 0..img.block_count() {
            freed.insert(img.entry(h, j).unwrap());
        }
        release_image(h, t, &img);
    }
    assert_eq!(freed.len(), 4);
    // Area 1 and area 0 take new pages: the host hands back the freed image pages, poisoned again.
    let mut reused = Vec::new();
    for _ in 0..(3 * 16) {
        let b = alloc_block(h, t, 1).unwrap();
        h.get_mut(Region::Block(b)).unwrap().fill(0x11);
        if b % PAGE == 0 && freed.contains(&b) {
            reused.push(b);
        }
    }
    for _ in 0..40 {
        let b = alloc_zeroed(h, t, 0).unwrap();
        if b % PAGE == 0 && freed.contains(&b) {
            reused.push(b);
        }
        zeroed.push(b);
    }
    // More of the directory, its new blocks from reused pages and from the free stack.
    for i in [1u64 << 20, (1 << 20) + 513, 40] {
        dir.set(h, t, i, i + 1).unwrap();
    }
    dir.clear(h, t, 70_000).unwrap();
    // Blocks freed and taken again: zero-filled ones read zero, whatever they held.
    for &b in zeroed.iter().take(5) {
        put_u64(h.get_mut(Region::Block(b)).unwrap(), 4_000, 0xDEAD).unwrap();
        free_block(h, t, 0, b).unwrap();
    }
    for _ in 0..5 {
        zeroed.push(alloc_zeroed(h, t, 0).unwrap());
    }
    // Area 1 takes more blocks.
    for _ in 0..20 {
        let b = alloc_block(h, t, 1).unwrap();
        h.get_mut(Region::Block(b)).unwrap().fill(0x22);
    }
    let live_zeroed: Vec<u64> = zeroed.iter().copied().skip(5).collect();
    Trace {
        records: (0..AREAS as u32).map(|a| rec(h, t, a)).collect(),
        lists: (0..2).map(|a| entries(h, rec(h, t, a).page_list)).collect(),
        directory: blocks(h, dir),
        stacks: (0..2).map(|a| free_stack(h, t, a)).collect(),
        images: vec![(images[1].0, read_back(h, ImageId::Group(1), images[1].0))],
        zeroed: live_zeroed.iter().map(|&b| h.get(Region::Block(b)).unwrap().iter().all(|&x| x == 0)).collect(),
        reused,
    }
}

#[test]
fn poisoned_pages() {
    let trace = per_fill(poisoned_scenario);
    // Reused pages were taken: freed image pages came back as area blocks.
    assert!(!trace.reused.is_empty());
    // The blocks zero-filled at allocation read zero, reused ones included.
    assert!(trace.zeroed.iter().all(|&z| z));
    // Every radix block of the directory reads zero except at the entries set.
    let set: usize = trace.directory.iter().map(|(_, e)| e.iter().filter(|&&x| x != 0).count()).sum();
    assert_eq!(set, 7 + trace.directory.len() - 1);
    assert_eq!(trace.images[0].1, data(65_544, 2));

    // The two runs do detect a byte nobody wrote: a block taken from a page has the pattern there, and a
    // block taken from the free stack has it in its link (test mode).
    let unwritten: Vec<(u64, [u8; 8])> = FILLS
        .iter()
        .map(|&f| {
            let mut h = VecHost::new(f);
            let t = new_root(&mut h).unwrap();
            let b = alloc_block(&mut h, t, 0).unwrap();
            let c = alloc_block(&mut h, t, 0).unwrap();
            free_block(&mut h, t, 0, c).unwrap();
            assert_eq!(alloc_block(&mut h, t, 0).unwrap(), c);
            let link: [u8; 8] = h.get(Region::Block(c)).unwrap()[..8].try_into().unwrap();
            (word(&h, b, 8), link)
        })
        .collect();
    assert_ne!(unwritten[0], unwritten[1]);
    assert_eq!(unwritten[0].1, [FILLS[0]; 8]);
    assert_eq!(unwritten[1].1, [FILLS[1]; 8]);
}

// ---- Changed blocks per operation ----

#[test]
fn changed_blocks_per_operation() {
    per_fill(|h| {
        let mut rows = Vec::new();
        let mut measure = |h: &mut VecHost, name: &'static str, want: u64, f: &mut dyn FnMut(&mut VecHost)| {
            h.end_message();
            f(h);
            let got = h.end_message();
            assert_eq!(got, want, "{name}");
            rows.push((name, got));
        };
        let mut t = 0;
        measure(h, "new_root: the root block", 1, &mut |h| t = new_root(h).unwrap());
        let mut b = 0;
        // the area record in the root, nothing else.
        measure(h, "alloc_block from the newest page", 1, &mut |h| b = alloc_block(h, t, 0).unwrap());
        // the link written into b, the head in the root.
        measure(h, "free_block", 2, &mut |h| free_block(h, t, 0, b).unwrap());
        // the head in the root, and in test mode the taken block's link.
        measure(h, "alloc_block from the free stack (test mode)", 2, &mut |h| {
            assert_eq!(alloc_block(h, t, 0).unwrap(), b)
        });
        // the record and the 4,096 zero bytes.
        measure(h, "alloc_zeroed", 2, &mut |h| {
            alloc_zeroed(h, t, 0).unwrap();
        });
        // a new page as the page list's root at height 0.
        measure(h, "alloc_block(1), first page", 1, &mut |h| {
            alloc_block(h, t, 1).unwrap();
        });
        for _ in 0..15 {
            alloc_block(h, t, 1).unwrap();
        }
        // "a radix append, at most one new radix block per level".
        measure(h, "alloc_block(1), second page: one new radix block", 2, &mut |h| {
            alloc_block(h, t, 1).unwrap();
        });
        let mut img = Image::EMPTY;
        // area 15's count in the root, and the page's first block.
        measure(h, "image append of 8 bytes, a new page", 2, &mut |h| img.append(h, t, &[7; 8]).unwrap());
        measure(h, "image append of 8 bytes in the same block", 1, &mut |h| img.append(h, t, &[8; 8]).unwrap());
        // 10.3.5: one block entry at height 0, nothing written.
        measure(h, "ref_form of 1 block", 0, &mut |h| assert_eq!(img.ref_form(h, t).unwrap(), img));
        measure(h, "append to 2 blocks: 32 image blocks of 4 KiB and the area-15 count", 16 + 16 + 1, &mut |h| {
            img.append(h, t, &vec![9u8; 2 * PAGE as usize - 16]).unwrap()
        });
        // 10.3.5, 7.5.8: one block allocation and one 4 KiB write.
        let mut rf = img;
        measure(h, "ref_form of 2 to 8 blocks", 2, &mut |h| rf = img.ref_form(h, t).unwrap());
        let mut r = Radix::EMPTY;
        r.set(h, t, 700, 1).unwrap();
        measure(h, "radix set in a present leaf", 1, &mut |h| r.set(h, t, 701, 2).unwrap());
        measure(h, "radix set: a new leaf at height 2", 3, &mut |h| r.set(h, t, 3, 3).unwrap());
        measure(h, "radix clear, nothing emptied", 1, &mut |h| r.clear(h, t, 701).unwrap());
        measure(h, "radix clear, a leaf emptied and freed", 3, &mut |h| r.clear(h, t, 3).unwrap());
        // The release of a 2-block image: its pages (the area-15 count in the root) and its radix block (its
        // link; the root again).
        measure(h, "image release", 2, &mut |h| free_image(h, t, &rf).unwrap());
        rows
    });
}

#[test]
fn image_area_counts_pages() {
    per_fill(|h| {
        let t = new_root(h).unwrap();
        let p = image_page_alloc(h, t).unwrap();
        assert_eq!(rec(h, t, IMAGE_AREA).pages, 1);
        assert_eq!(rec(h, t, IMAGE_AREA).page_list, Radix::EMPTY, "area 15 has no page list");
        native_table::area::image_page_free(h, t, p).unwrap();
        assert_eq!(rec(h, t, IMAGE_AREA).pages, 0);
        assert!(is_corrupt(native_table::area::image_page_free(h, t, p), ImageId::Runtime, Corruption::Count));
        p
    });
}
