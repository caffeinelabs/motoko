//! The panic-scan probe: every public entry point of `native-table`, instantiated over a
//! panic-free host and exported, so that a scan of the optimized wasm64 module can prove that no
//! panicking function is reachable from any of them. `nt_selftest` also runs the entry points under
//! wasmtime and returns 0 when every check passes.
//!
//! The probe's own code follows the crate's panic rules, so a panic path the scan finds is the crate's.
//! It is not `forbid(unsafe_code)`: exporting by name needs `#[no_mangle]`. It is `no_std` on wasm only,
//! where the scan runs.
#![cfg_attr(target_family = "wasm", no_std)]
// Probe scaffolding: written for the panic scan, not as library style.
#![allow(clippy::option_map_unit_fn, clippy::should_implement_trait, clippy::unnecessary_cast)]

use native_table::area::{self, AreaRecord};
use native_table::bytes::{put_uleb, rd_uleb};
use native_table::consts::{form_code, seal_scratch, BLOCK, PAGE, RADIX_STEP};
use native_table::hash::{nt_hash64, Hasher64};
use native_table::host::{wake_from_state, Progress};
use native_table::image::{Image, ImageReader};
use native_table::key::{self, KeyKind, KeyPart, KeySpec, LenSink, Sink, SliceSink};
use native_table::mem::{copy, put_u64, rd_u16, rd_u32, rd_u64, shift, sub};
use native_table::radix::{Radix, Units};
use native_table::table::cell::{enc_key, Val};
use native_table::table::layout::{LeafInfo, Ty};
use native_table::value::{self, Utf8Stream};
use native_table::{CorruptAt, Corruption, Fault, ImageId, PageHost, Region, Wake};

#[cfg(all(target_family = "wasm", not(test)))]
#[panic_handler]
fn on_panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}

/// A panic-free host over a fixed array of M bytes: page k (1..=M/65,536) has reference k · 65,536.
pub struct ArrayHostN<const M: usize> {
    mem: [u8; M],
    next: u64,
}

/// The probe host of four pages.
pub type ArrayHost = ArrayHostN<{ 4 * 65_536 }>;

impl<const M: usize> ArrayHostN<M> {
    pub const fn new() -> Self {
        ArrayHostN { mem: [0; M], next: 1 }
    }

    fn span(r: Region) -> Result<(usize, usize), Fault> {
        let (base, len) = match r {
            Region::Page(p) if p >= PAGE && p % PAGE == 0 => (p - PAGE, PAGE),
            Region::Block(b) if b >= PAGE && b % BLOCK == 0 => (b - PAGE, BLOCK),
            _ => return Err(Fault::BadRef),
        };
        let end = base.checked_add(len).ok_or(Fault::BadRef)?;
        if end > M as u64 {
            return Err(Fault::BadRef);
        }
        let s = usize::try_from(base).map_err(|_| Fault::BadRef)?;
        let e = usize::try_from(end).map_err(|_| Fault::BadRef)?;
        Ok((s, e))
    }
}

impl<const M: usize> Default for ArrayHostN<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const M: usize> PageHost for ArrayHostN<M> {
    fn block_of(&self, page: u64, i: u32) -> u64 {
        page.wrapping_add(u64::from(i).wrapping_mul(BLOCK))
    }
    fn get(&self, r: Region) -> Result<&[u8], Fault> {
        let (s, e) = Self::span(r)?;
        self.mem.get(s..e).ok_or(Fault::BadRef)
    }
    fn get_mut(&mut self, r: Region) -> Result<&mut [u8], Fault> {
        let (s, e) = Self::span(r)?;
        self.mem.get_mut(s..e).ok_or(Fault::BadRef)
    }
    fn pair_mut(&mut self, a: Region, b: Region) -> Result<(&mut [u8], &mut [u8]), Fault> {
        let (sa, ea) = Self::span(a)?;
        let (sb, eb) = Self::span(b)?;
        if sa < eb && sb < ea {
            return Err(Fault::BadRef);
        }
        if sa < sb {
            let (lo, hi) = self.mem.split_at_mut_checked(sb).ok_or(Fault::BadRef)?;
            let x = lo.get_mut(sa..ea).ok_or(Fault::BadRef)?;
            let y = hi.get_mut(..eb.wrapping_sub(sb)).ok_or(Fault::BadRef)?;
            Ok((x, y))
        } else {
            let (lo, hi) = self.mem.split_at_mut_checked(sa).ok_or(Fault::BadRef)?;
            let y = lo.get_mut(sb..eb).ok_or(Fault::BadRef)?;
            let x = hi.get_mut(..ea.wrapping_sub(sa)).ok_or(Fault::BadRef)?;
            Ok((x, y))
        }
    }
    fn read_write(&mut self, r: Region, wr: Region) -> Result<(&[u8], &mut [u8]), Fault> {
        let (a, b) = self.pair_mut(r, wr)?;
        Ok((&*a, b))
    }
    fn page_alloc(&mut self) -> Result<u64, Fault> {
        if self.next > (M as u64) / PAGE {
            return Err(Fault::OutOfMemory);
        }
        let p = self.next.wrapping_mul(PAGE);
        self.next = self.next.wrapping_add(1);
        Ok(p)
    }
    fn page_free(&mut self, p: u64) -> Result<(), Fault> {
        Self::span(Region::Page(p)).map(|_| ())
    }
    fn alloc_changes(&self) -> u64 {
        1
    }
}

mod segments;
pub use segments::*;

mod costs;
pub use costs::{
    count_entry_case, extremes_open, filter_scans, group_counts, primary_inserts, secondary_ops, table_ops,
};

mod sorted;
pub use sorted::{nt_sorted, nt_table_sorted};

fn code(r: Result<u64, Fault>) -> u64 {
    match r {
        Ok(v) => v,
        Err(f) => u64::MAX - u64::from(f.text().len() as u8),
    }
}

// ---- Exported entry points: inputs come from parameters so that nothing folds away. ----

#[no_mangle]
pub extern "C" fn nt_mem(off: u64, len: u64, from: u64, to: u64, v: u64) -> u64 {
    let mut h = ArrayHost::new();
    let r = (|| -> Result<u64, Fault> {
        let p = h.page_alloc()?;
        let q = h.page_alloc()?;
        {
            let b = h.get_mut(Region::Page(p))?;
            put_u64(b, off, v)?;
            shift(b, from, to, len)?;
        }
        let qb = h.block_of(q, 1);
        copy(&mut h, Region::Page(p), from, Region::Block(qb), to, len)?;
        let b = h.get(Region::Block(qb))?;
        let x = rd_u64(b, to)? ^ u64::from(rd_u32(b, off)?) ^ u64::from(rd_u16(b, from)?);
        Ok(x ^ u64::try_from(sub(b, off, len)?.len()).unwrap_or(0))
    })();
    code(r)
}

#[no_mangle]
pub extern "C" fn nt_shift(from: u64, to: u64, len: u64) -> u64 {
    let mut b = [0u8; 4096];
    code(shift(&mut b, from, to, len).map(|_| u64::from(b.get(7).copied().unwrap_or(0))))
}

#[no_mangle]
pub extern "C" fn nt_put(off: u64, v: u64) -> u64 {
    let mut b = [0u8; 4096];
    code(put_u64(&mut b, off, v).and_then(|_| rd_u64(&b, off)))
}

#[no_mangle]
pub extern "C" fn nt_pair(a: u64, b: u64) -> u64 {
    let mut h = ArrayHost::new();
    code(h.pair_mut(Region::Block(a), Region::Block(b)).map(|(x, y)| (x.len() ^ y.len()) as u64))
}

#[no_mangle]
pub extern "C" fn nt_copy(from: u64, to: u64, len: u64) -> u64 {
    let mut h = ArrayHost::new();
    code(copy(&mut h, Region::Page(PAGE), from, Region::Page(2 * PAGE), to, len).map(|_| 0))
}

#[no_mangle]
pub extern "C" fn nt_bytes(off: u64, v: u64) -> u64 {
    let mut buf = [0u8; 64];
    let r = (|| -> Result<u64, Fault> {
        let n = put_uleb(&mut buf, off.wrapping_add(8), v)?;
        let (x, m) = rd_uleb(&buf, off.wrapping_add(8), ImageId::Group(off))?;
        Ok(x ^ v ^ n ^ m)
    })();
    code(r)
}

#[no_mangle]
pub extern "C" fn nt_fault_text(sel: u32, x: u64) -> u64 {
    let kinds = [
        Corruption::Header,
        Corruption::Count,
        Corruption::Flags,
        Corruption::Layout,
        Corruption::Bounds,
        Corruption::Order,
        Corruption::Encoding,
        Corruption::Leaf,
        Corruption::Range,
        Corruption::Link,
        Corruption::Depth,
    ];
    let kind = kinds.get(usize::try_from(x % 11).unwrap_or(0)).copied().unwrap_or(Corruption::Header);
    let image = match sel % 4 {
        0 => ImageId::Runtime,
        1 => ImageId::Group(x),
        2 => ImageId::Run { index: (x & 0x3F) as u8, start_seq: x },
        _ => ImageId::Loading { slot: (x & 3) as u8, image: x },
    };
    let at = CorruptAt { image, kind, offset: x };
    Fault::Corrupt(at).text().len() as u64
}

#[no_mangle]
pub extern "C" fn nt_consts(w: u64, v: u64, l: u64, state: u32) -> u64 {
    let f = u64::from(form_code(w, v).unwrap_or(0));
    let s = seal_scratch(l, v, w).unwrap_or(0);
    let k = match wake_from_state((state & 0xFF) as u8) {
        Wake::Idle => 0,
        Wake::Later => 1,
        Wake::Now => 2,
    };
    f ^ s ^ k
}

/// Runs the entry points with known answers; 0 when all pass, else the number of the first failure.
#[no_mangle]
pub extern "C" fn nt_selftest() -> u32 {
    // put at 0, shift 8 bytes from 0 to 16, copy 8 bytes from page offset 0 to block offset 16.
    // Block bytes 0..16 stay zero, so the u32 and u16 reads add nothing.
    if nt_mem(0, 8, 0, 16, 0x0102_0304_0506_0708) != (0x0102_0304_0506_0708 ^ 8) {
        return 1;
    }
    // Out of bounds fails with a fault code, never a trap.
    if nt_mem(PAGE - 4, 8, 0, 0, 1) < u64::MAX - 64 {
        return 2;
    }
    // The LEB round trip gives 300 back, and both LEB forms take 2 bytes: they cancel.
    if nt_bytes(3, 300) != 0 {
        return 3;
    }
    if nt_bytes(60, 1) < u64::MAX - 64 {
        return 4;
    }
    if nt_consts(8, 0, 6, 2) != (0x8100_0010 ^ seal_scratch(6, 0, 8).unwrap_or(0) ^ 2) {
        return 5;
    }
    let d = nt_fault_text(2, 42);
    if d == 0 {
        return 6;
    }
    // The key digest is a pure function of its inputs.
    if key_digest(3, 50) != key_digest(3, 50) || key_digest(3, 50) == key_digest(5, 50) {
        return 7;
    }
    // Areas, radix arrays and images run without a fault on valid inputs and fault, never trap, on
    // refused ones.
    if is_fault(nt_area(1, 20, 0)) || !is_fault(nt_area(2, 1, 0)) || is_fault(nt_area(0, 47, 1)) {
        return 8;
    }
    if is_fault(nt_radix(5, 9, 600, 0))
        || is_fault(nt_radix(1 << 62, 3, 7, 1 << 40))
        || !is_fault(nt_radix(1 << 63, 1, 0, 0))
    {
        return 9;
    }
    // A 2-block image in reference form, read inside the image; then a read across its block boundary.
    if is_fault(nt_image(2 * PAGE / 8, 8, 64, 1)) || !is_fault(nt_image(2 * PAGE / 8, PAGE - 8, 16, 1)) {
        return 10;
    }
    // A direct table of 2 entries has no reference form; one block has.
    if !is_fault(nt_image(2 * PAGE / 8, 0, 8, 0)) || is_fault(nt_image(100, 0, 8, 0)) {
        return 11;
    }
    if nt_image_parse(0, 8, 0xFF01, 0) != nt_image_parse(0, 8, 0xFF01, 0) {
        return 12;
    }
    // The table runs its writes, reads and maintenance without a fault, a seal included, and a forged group
    // image faults, never traps.
    if is_fault(nt_table(1, 150)) || is_fault(nt_table(2, 120)) || nt_table(3, 90) != nt_table(3, 90) {
        return 40;
    }
    if is_fault(nt_table_seal(4_200)) {
        return 41;
    }
    // Updates, a schema change and maintenance in units.
    if is_fault(nt_table_evolve(5, 200, 1)) || is_fault(nt_table_evolve(6, 150, 5)) {
        return 33;
    }
    let forged = nt_group_read(0x1234_5678, 0x9ABC_DEF0, 3, 1);
    if forged != nt_group_read(0x1234_5678, 0x9ABC_DEF0, 3, 1) {
        return 42;
    }
    // The secondary index's counts and group walk agree with the rows it keeps, at two key widths.
    if is_fault(nt_secondary(5, 250, 0, 0)) || is_fault(nt_secondary(9, 299, 2, 0)) {
        return 43;
    }
    match runs_selftest() {
        0 => sorted::sorted_selftest(),
        e => e,
    }
}

/// Whether an export's result is a fault code.
fn is_fault(x: u64) -> bool {
    x >= u64::MAX - 64
}

// ---- memory ----

/// Areas: a new root; `n % 48` allocations from area `a & 3` (2 and 3 are refused), every third freed; a
/// zero-filled block; an image page taken and returned.
#[no_mangle]
pub extern "C" fn nt_area(a: u32, n: u64, x: u64) -> u64 {
    let mut h = ArrayHost::new();
    let r = (|| -> Result<u64, Fault> {
        let t = area::new_root(&mut h)?;
        let a = a & 3;
        let mut acc = 0u64;
        for i in 0..n % 48 {
            let b = area::alloc_block(&mut h, t, a)?;
            if i % 3 == x % 3 {
                area::free_block(&mut h, t, a, b)?;
            }
            acc ^= b;
        }
        acc ^= area::alloc_zeroed(&mut h, t, a & 1)?;
        let p = area::image_page_alloc(&mut h, t)?;
        area::image_page_free(&mut h, t, p)?;
        let rec = AreaRecord::read(&h, t, 0)?;
        Ok(acc ^ rec.free_head)
    })();
    code(r)
}

/// Radix arrays: entries i and j set, two reads, a walk from `from`, a clear and one release unit.
#[no_mangle]
pub extern "C" fn nt_radix(i: u64, v: u64, j: u64, from: u64) -> u64 {
    let mut h = ArrayHost::new();
    let r = (|| -> Result<u64, Fault> {
        let t = area::new_root(&mut h)?;
        let mut r = Radix::EMPTY;
        r.set(&mut h, t, i, v)?;
        r.set(&mut h, t, j, v ^ 1)?;
        let g = r.get(&h, i)? ^ r.get(&h, from)?;
        let mut sum = 0u64;
        let w = r.walk_max(&h, from, RADIX_STEP, |k, x| {
            sum = sum.wrapping_add(k ^ x);
            Ok(())
        })?;
        r.clear(&mut h, t, j)?;
        let mut cursor = from % 1_024;
        let mut u = Units::DRAIN;
        let done = r.release_step(&mut h, t, &mut cursor, &mut u, |_, _| Ok(()))?;
        Ok(g ^ sum
            ^ w.examined
            ^ w.next.unwrap_or(7)
            ^ cursor
            ^ u64::from(done == Progress::Done)
            ^ u64::from(r.height))
    })();
    code(r)
}

/// Images: `8 · (words % 16,385)` bytes appended in 1 KiB pieces (at most 2 image blocks), the record round
/// trip, the reference form when `sel` is odd, the reference round trip, a read of [off, off + len) through
/// the reader.
#[no_mangle]
pub extern "C" fn nt_image(words: u64, off: u64, len: u64, sel: u32) -> u64 {
    let mut h = ArrayHost::new();
    let r = (|| -> Result<u64, Fault> {
        let t = area::new_root(&mut h)?;
        let mut img = Image::EMPTY;
        let piece = [0x5Au8; 1_024];
        let mut left = (words % (2 * PAGE / 8 + 1)).wrapping_mul(8);
        while left > 0 {
            let n = left.min(1_024);
            img.append(&mut h, t, piece.get(..usize::try_from(n).unwrap_or(0)).unwrap_or(&[]))?;
            left = left.wrapping_sub(n);
        }
        let mut rec = [0u8; 80];
        img.put_record(&mut rec, 0)?;
        let back = Image::read_record(&rec, 0)?;
        let rf = if sel & 1 == 1 { back.ref_form(&mut h, t)? } else { back };
        let mut rb = [0u8; 24];
        rf.put_ref(&mut rb, 0)?;
        let back_ref = Image::read_ref(&rb, 0)?;
        let x = {
            let rd = ImageReader::new(&h, ImageId::Group(off), back_ref);
            let b = rd.bytes(off, len)?;
            (b.len() as u64) ^ u64::from(b.first().copied().unwrap_or(0)) ^ rd.length()
        };
        Ok(x)
    })();
    code(r)
}

/// Stored forms from arbitrary bytes: an image record, an image reference and an area record.
#[no_mangle]
pub extern "C" fn nt_image_parse(a: u64, b: u64, c: u64, off: u64) -> u64 {
    let mut buf = [0u8; 96];
    for (k, w) in [a, b, c, a ^ c].iter().enumerate() {
        if let Some(d) = buf.get_mut(k * 8..k * 8 + 8) {
            d.copy_from_slice(&w.to_le_bytes());
        }
    }
    if let Some(d) = buf.get_mut(16..24) {
        d.copy_from_slice(&c.to_le_bytes());
    }
    let r1 = Image::read_record(&buf, off % 32).map(|i| i.length ^ i.block_count());
    let r2 = Image::read_ref(&buf, off % 80).map(|i| i.length);
    let r3 = AreaRecord::take(&buf, (off % 3) as u32).map(|r| r.pages ^ u64::from(r.bump));
    code(r1) ^ code(r2) ^ code(r3)
}

// ---- keys and values ----

/// A deterministic generator shared by the native and wasm runs.
pub struct Gen(pub u64);

impl Gen {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

/// Feeds every byte it gets into a running hash, in fixed 8-byte words.
struct HashSink {
    h: u64,
    word: u64,
    fill: u32,
}

impl HashSink {
    fn new() -> Self {
        HashSink { h: 0x9E37_79B9_7F4A_7C15, word: 0, fill: 0 }
    }
    fn finish(mut self) -> u64 {
        self.h = native_table::hash::mix(self.h ^ self.word ^ u64::from(self.fill));
        self.h
    }
}

impl Sink for HashSink {
    fn put(&mut self, bytes: &[u8]) -> Result<(), Fault> {
        for &b in bytes {
            self.word = (self.word << 8) | u64::from(b);
            self.fill = self.fill.wrapping_add(1);
            if self.fill % 8 == 0 {
                self.h = native_table::hash::mix(self.h ^ self.word);
                self.word = 0;
            }
        }
        Ok(())
    }
}

/// The digest of `n` random keys of every kind, their decodings and their lengths: equal on every target,
/// with NaN canonicalization on or off.
pub fn key_digest(seed: u64, n: u32) -> u64 {
    let mut g = Gen(seed | 1);
    let mut s = HashSink::new();
    let mut buf = [0u8; 256];
    for _ in 0..n {
        let x = g.next();
        let y = g.next();
        let w = [1u8, 2, 4, 8][(y % 4) as usize];
        let bits = u32::from(w) * 8;
        let _ = key::enc_u64(&mut s, if bits < 64 { x & ((1 << bits) - 1) } else { x }, w);
        let _ = key::enc_i64(&mut s, ((x << (64 - bits)) as i64) >> (64 - bits), w);
        let _ = key::enc_f64(&mut s, x);
        let _ = key::enc_f32(&mut s, y as u32);
        let _ = key::enc_u128(&mut s, (u128::from(x) << 64) | u128::from(y));
        let _ = key::enc_i128(&mut s, ((u128::from(y) << 64) | u128::from(x)) as i128);
        let _ = key::enc_nat_u64(&mut s, x >> (y % 64));
        let _ = key::enc_int_i64(&mut s, (x as i64) >> (y % 64));
        let len = (y % 64) as usize;
        let mut b = [0u8; 64];
        for (i, slot) in b.iter_mut().enumerate().take(len) {
            *slot = (x >> ((i % 8) * 8)) as u8 & 0x0F;
        }
        let bytes = b.get(..len).unwrap_or(&[]);
        let _ = key::enc_bytes(&mut s, bytes);
        let _ = key::enc_principal(&mut s, b.get(..len % 30).unwrap_or(&[]));
        let _ = key::enc_int_be(&mut s, y & 1 == 1, bytes);
        let mut out = SliceSink::new(&mut buf);
        let _ = key::enc_f64(&mut out, x);
        let _ = s.put(&(out.len as u64).to_le_bytes());
        let mut ls = LenSink(0);
        let _ = key::enc_bytes(&mut ls, bytes);
        let _ = s.put(&ls.0.to_le_bytes());
    }
    let parts = [
        KeyPart { leaf: 1, kind: KeyKind::Text, nullable: false },
        KeyPart { leaf: 2, kind: KeyKind::Nat, nullable: true },
    ];
    let spec = KeySpec { parts: &parts };
    let _ = s.put(&spec.hash(1).to_le_bytes());
    let _ = enc_key(&mut s, &leaf(1, Ty::Text, false), &Val::Bytes(b"a\0b"))
        .and_then(|_| enc_key(&mut s, &leaf(2, Ty::Nat, true), &Val::U(seed)));
    s.finish() ^ nt_hash64(&seed.to_le_bytes())
}

/// A key leaf of id `id`, type `ty`.
fn leaf(id: u32, ty: Ty, nullable: bool) -> LeafInfo {
    LeafInfo { ordinal: 0, id, ty, nullable, key: true, path: 0, added: 0, dropped: 0 }
}

#[no_mangle]
pub extern "C" fn nt_key_digest(seed: u64, n: u32) -> u64 {
    key_digest(seed, n)
}

#[no_mangle]
pub extern "C" fn nt_key_decode(sel: u32, a: u64, b: u64, pos: u64) -> u64 {
    let mut k = [0u8; 48];
    k.get_mut(..8).map(|d| d.copy_from_slice(&a.to_le_bytes()));
    k.get_mut(8..16).map(|d| d.copy_from_slice(&b.to_le_bytes()));
    k.get_mut(16..24).map(|d| d.copy_from_slice(&(a ^ b).to_be_bytes()));
    let img = ImageId::Group(a);
    let mut out = [0u8; 64];
    let mut sink = SliceSink::new(&mut out);
    let kinds = [
        KeyKind::Bool,
        KeyKind::U16,
        KeyKind::I64,
        KeyKind::U128,
        KeyKind::Char,
        KeyKind::Nat,
        KeyKind::F64,
        KeyKind::F32,
        KeyKind::Text,
        KeyKind::Principal,
        KeyKind::Fixed(5),
    ];
    let kind = kinds.get((sel % 11) as usize).copied().unwrap_or(KeyKind::Unit);
    let r = match sel / 11 % 4 {
        0 => kind.skip(&k, pos, img),
        1 => key::int_header(&k, pos, img).map(|(neg, n, p)| p ^ n ^ u64::from(neg)),
        2 => key::dec_bytes(&k, pos, &mut sink, img),
        _ => key::dec_principal(&k, pos, img).map(|(_, n, p)| p ^ u64::from(n)),
    };
    code(r)
}

#[no_mangle]
pub extern "C" fn nt_key_encode(sel: u32, a: u64, b: u64) -> u64 {
    let mut out = [0u8; 128];
    let mut s = SliceSink::new(&mut out);
    let src = a.to_be_bytes();
    let len = (b % 9) as usize;
    let bytes = src.get(..len).unwrap_or(&[]);
    let r = match sel % 6 {
        0 => key::enc_int_leb(&mut s, bytes, b & 1 == 1, ImageId::Runtime),
        1 => key::enc_int_be(&mut s, b & 1 == 1, bytes),
        2 => key::enc_i64(&mut s, a as i64, (b % 9) as u8),
        3 => key::enc_u64(&mut s, a, (b % 9) as u8),
        4 => {
            let parts = [KeyPart { leaf: 3, kind: KeyKind::Int, nullable: true }];
            let spec = KeySpec { parts: &parts };
            enc_key(&mut s, &leaf(3, Ty::Int, true), &Val::Big(bytes))
                .and_then(|_| spec.end(s.written(), ImageId::Runtime).map(|_| ()))
        }
        _ => key::enc_principal(&mut s, bytes),
    };
    code(r.map(|_| s.len as u64))
}

#[no_mangle]
pub extern "C" fn nt_value(sel: u32, a: u64, b: u64, c: u64) -> u64 {
    // A small pool: Text; Record{u8, Opt(Text)}; Tuple(Char, [u8; 3]).
    let pool: &[u8] = &[
        0x0F, // 0: Text
        0x21, 2, 0, 0, 0, // 1: Record, 2 main fields
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0x02, // field 1: u8
        2, 0, 0, 0, 0, 0, 0, 0, 0, 0x20, 0x0F, // field 2: Opt(Text)
        0x22, 2, 0, 0x0E, 0x14, 3, 0, // 27: Tuple(Char, [u8; 3])
    ];
    let mut ve = [0u8; 32];
    ve.get_mut(..8).map(|d| d.copy_from_slice(&a.to_le_bytes()));
    ve.get_mut(8..16).map(|d| d.copy_from_slice(&b.to_le_bytes()));
    let n = (c % 33) as usize;
    let v = ve.get(..n).unwrap_or(&[]);
    let mut scratch = [0u8; 64 * value::FRAME];
    let t = [0u32, 1, 27][(sel % 3) as usize];
    let r = match sel / 4 % 2 {
        0 => value::skip_ty(pool, t, &mut scratch).map(u64::from),
        _ => {
            let mut u = Utf8Stream::new();
            let ok = u.feed(v.get(..n / 2).unwrap_or(&[])) && u.feed(v.get(n / 2..).unwrap_or(&[])) && u.finish();
            Ok(u64::from(ok))
        }
    };
    let mut h = Hasher64::new(8);
    h.update(&code(r).to_le_bytes());
    h.finish()
}

// ---- the one B-tree, typed form, over a panic-free node arena ----

use native_table::host::{NodeMem, NodeRead, Pending};
use native_table::tree::{self, Cursor, Fwd, Path, Probe, Rev, Search, Typed, TypedShape};

const NODES: usize = 24;
const ARENA: usize = NODES * 4096;

/// Typed nodes of <u64, u64> in a fixed arena; reference k · 4,096 for slot k − 1.
pub struct ArrayNodes {
    mem: [u8; ARENA],
    next: u64,
    free: [u64; NODES],
    nfree: usize,
}

impl ArrayNodes {
    fn new() -> Self {
        ArrayNodes { mem: [0; ARENA], next: 1, free: [0; NODES], nfree: 0 }
    }
    fn at(&self, n: u64, off: u64, len: u64) -> Result<(usize, usize), Fault> {
        if n == 0 || n % 4096 != 0 {
            return Err(Fault::BadRef);
        }
        let end = off.checked_add(len).ok_or(Fault::BadRef)?;
        if end > 4080 {
            return Err(Fault::BadRef);
        }
        let s = (n - 4096).checked_add(off).ok_or(Fault::BadRef)?;
        let e = s.checked_add(len).ok_or(Fault::BadRef)?;
        if e > ARENA as u64 {
            return Err(Fault::BadRef);
        }
        Ok((s as usize, e as usize))
    }
}

pub struct ProbePending {
    key: [u8; 8],
    val: [u8; 8],
    child: u64,
}

impl Pending for ProbePending {
    fn empty() -> Self {
        ProbePending { key: [0; 8], val: [0; 8], child: 0 }
    }
    fn child(&self) -> u64 {
        self.child
    }
    fn set_child(&mut self, c: u64) {
        self.child = c;
    }
}

impl NodeRead for ArrayNodes {
    type Bytes<'a> = &'a [u8];
    fn count(&self, n: u64) -> Result<u16, Fault> {
        let (s, e) = self.at(n, 0, 2)?;
        let b: [u8; 2] = self.mem.get(s..e).and_then(|x| x.try_into().ok()).ok_or(Fault::BadRef)?;
        Ok(u16::from_le_bytes(b))
    }
    fn flags(&self, n: u64) -> Result<u16, Fault> {
        let (s, e) = self.at(n, 2, 2)?;
        let b: [u8; 2] = self.mem.get(s..e).and_then(|x| x.try_into().ok()).ok_or(Fault::BadRef)?;
        Ok(u16::from_le_bytes(b))
    }
    fn word(&self, n: u64, off: u64) -> Result<u64, Fault> {
        let (s, e) = self.at(n, off, 8)?;
        let b: [u8; 8] = self.mem.get(s..e).and_then(|x| x.try_into().ok()).ok_or(Fault::BadRef)?;
        Ok(u64::from_le_bytes(b))
    }
    fn bytes(&self, _n: u64, _off: u64, _len: u64) -> Result<&[u8], Fault> {
        Err(Fault::BadRef)
    }
}

impl NodeMem for ArrayNodes {
    type Entry<'k> = ProbePending;
    fn set_count(&mut self, n: u64, c: u16) -> Result<(), Fault> {
        let (s, e) = self.at(n, 0, 2)?;
        self.mem.get_mut(s..e).ok_or(Fault::BadRef)?.copy_from_slice(&c.to_le_bytes());
        Ok(())
    }
    fn set_word(&mut self, n: u64, off: u64, w: u64) -> Result<(), Fault> {
        let (s, e) = self.at(n, off, 8)?;
        self.mem.get_mut(s..e).ok_or(Fault::BadRef)?.copy_from_slice(&w.to_le_bytes());
        Ok(())
    }
    fn copy(&mut self, src: u64, so: u64, dst: u64, doff: u64, len: u64) -> Result<(), Fault> {
        let (a, _) = self.at(src, so, len)?;
        let (b, _) = self.at(dst, doff, len)?;
        native_table::mem::shift(&mut self.mem, a as u64, b as u64, len)
    }
    fn put_entry(&mut self, n: u64, koff: u64, voff: u64, p: &ProbePending) -> Result<(), Fault> {
        let (s, e) = self.at(n, koff, 8)?;
        self.mem.get_mut(s..e).ok_or(Fault::BadRef)?.copy_from_slice(&p.key);
        let (s, e) = self.at(n, voff, 8)?;
        self.mem.get_mut(s..e).ok_or(Fault::BadRef)?.copy_from_slice(&p.val);
        Ok(())
    }
    fn take_entry(&mut self, n: u64, koff: u64, voff: u64, p: &mut ProbePending) -> Result<(), Fault> {
        let (s, e) = self.at(n, koff, 8)?;
        p.key.copy_from_slice(self.mem.get(s..e).ok_or(Fault::BadRef)?);
        let (s, e) = self.at(n, voff, 8)?;
        p.val.copy_from_slice(self.mem.get(s..e).ok_or(Fault::BadRef)?);
        Ok(())
    }
    fn alloc(&mut self, leaf: bool) -> Result<u64, Fault> {
        let n = if self.nfree > 0 {
            self.nfree -= 1;
            self.free.get(self.nfree).copied().ok_or(Fault::OutOfMemory)?
        } else {
            if self.next > NODES as u64 {
                return Err(Fault::OutOfMemory);
            }
            let n = self.next * 4096;
            self.next += 1;
            n
        };
        self.set_count(n, 0)?;
        let (s, e) = self.at(n, 2, 2)?;
        self.mem.get_mut(s..e).ok_or(Fault::BadRef)?.copy_from_slice(&u16::from(leaf).to_le_bytes());
        Ok(n)
    }
    fn hand_back(&mut self, n: u64) -> Result<(), Fault> {
        let slot = self.free.get_mut(self.nfree).ok_or(Fault::BadRef)?;
        *slot = n;
        self.nfree += 1;
        Ok(())
    }
    fn release(&mut self, _root: u64) -> Result<(), Fault> {
        Ok(())
    }
}

pub struct U64Shape;
impl TypedShape for U64Shape {
    const KS: u64 = 8;
    const VA: u64 = 8;
    const VS: u64 = 8;
}

pub struct U64Probe(pub u64);
impl Probe<ArrayNodes> for U64Probe {
    fn cmp_at(&self, m: &ArrayNodes, n: u64, koff: u64) -> Result<core::cmp::Ordering, Fault> {
        Ok(m.word(n, koff)?.cmp(&self.0))
    }
}

/// Inserts `count` keys from `seed`, removes every `del`-th, walks forward and backward from `from`;
/// returns a digest of what it saw, or a fault code.
#[no_mangle]
pub extern "C" fn nt_tree(seed: u64, count: u32, del: u32, from: u64) -> u64 {
    let s: Typed<U64Shape> = Typed::NEW;
    let mut m = ArrayNodes::new();
    let mut root = 0u64;
    let mut g = Gen(seed | 1);
    let r = (|| -> Result<u64, Fault> {
        let mut keys = [0u64; 64];
        for k in keys.iter_mut().take(count as usize % 64) {
            *k = g.next() % 1000;
            let mut p = ProbePending { key: k.to_le_bytes(), val: (!*k).to_le_bytes(), child: 0 };
            if root == 0 {
                root = tree::insert_first(s, &mut m, &p)?;
                continue;
            }
            let mut path = Path::new();
            if let Search::Vacant(n, j) = tree::descend(s, &m, root, &U64Probe(*k), &mut path)? {
                tree::insert_at(s, &mut m, &mut root, n, j, &mut p, &mut path)?;
            }
        }
        for (i, k) in keys.iter().enumerate() {
            if del > 0 && i as u32 % del == 0 && root != 0 {
                let mut path = Path::new();
                if let Search::Found(n, j) = tree::descend(s, &m, root, &U64Probe(*k), &mut path)? {
                    tree::remove_found(s, &mut m, &mut root, n, j, &mut path)?;
                }
            }
        }
        let mut h = Hasher64::new(0);
        let start = U64Probe(from);
        let mut c: Cursor<Fwd> = Cursor::seek(s, &m, root, core::ops::Bound::Included(&start))?;
        while let Some((n, i)) = c.next(s, &m, core::ops::Bound::<&U64Probe>::Unbounded)? {
            h.update(&n.wrapping_add(i).to_le_bytes());
        }
        let mut c: Cursor<Rev> = Cursor::seek(s, &m, root, core::ops::Bound::Excluded(&start))?;
        while let Some((n, i)) = c.next(s, &m, core::ops::Bound::Included(&U64Probe(0)))? {
            h.update(&n.wrapping_sub(i).to_le_bytes());
        }
        // The same walks through peek and advance, the halves of next a typed front end calls.
        let mut c: Cursor<Fwd> = Cursor::empty();
        c.seek_here(s, &m, root, core::ops::Bound::Included(&start))?;
        while let Some((n, i)) = c.peek(s, &m)? {
            h.update(&n.wrapping_add(i).to_le_bytes());
            c.advance(s, &m, n, i)?;
        }
        let mut c: Cursor<Rev> = Cursor::seek(s, &m, root, core::ops::Bound::<&U64Probe>::Unbounded)?;
        if let Some((n, i)) = c.peek(s, &m)? {
            h.update(&n.wrapping_sub(i).to_le_bytes());
            c.advance(s, &m, n, i)?;
        }
        c.clear();
        let f = tree::find(s, &m, root, &U64Probe(from))?;
        Ok(h.finish() ^ f.map(|x| x.0).unwrap_or(0))
    })();
    code(r)
}

// ---- the index form, over a panic-free arena of index nodes ----

use native_table::key::probe::{self as kp, IndexPending, KeyProbe, Kind, ResolveKey};
use native_table::tree::IndexShape;

/// Index nodes in a fixed arena; no key log (keys never overflow here: W = 32).
pub struct ArrayIndexNodes {
    inner: ArrayNodes,
    shape: IndexShape,
}

impl NodeRead for ArrayIndexNodes {
    type Bytes<'a> = &'a [u8];
    fn count(&self, n: u64) -> Result<u16, Fault> {
        self.inner.count(n)
    }
    fn flags(&self, n: u64) -> Result<u16, Fault> {
        self.inner.flags(n)
    }
    fn word(&self, n: u64, off: u64) -> Result<u64, Fault> {
        self.inner.word(n, off)
    }
    fn bytes(&self, n: u64, off: u64, len: u64) -> Result<&[u8], Fault> {
        let (s, e) = self.inner.at(n, off, len)?;
        self.inner.mem.get(s..e).ok_or(Fault::BadRef)
    }
}

impl NodeMem for ArrayIndexNodes {
    type Entry<'k> = IndexPending<'k>;
    fn set_count(&mut self, n: u64, c: u16) -> Result<(), Fault> {
        self.inner.set_count(n, c)
    }
    fn set_word(&mut self, n: u64, off: u64, w: u64) -> Result<(), Fault> {
        self.inner.set_word(n, off, w)
    }
    fn copy(&mut self, s: u64, so: u64, d: u64, doff: u64, len: u64) -> Result<(), Fault> {
        self.inner.copy(s, so, d, doff, len)
    }
    fn put_entry(&mut self, n: u64, koff: u64, voff: u64, p: &IndexPending<'_>) -> Result<(), Fault> {
        let w = self.shape.w;
        let ks = w + 8;
        let mut k = [0u8; 40];
        let mut v = [0u8; 8];
        let kb = k.get_mut(..ks as usize).ok_or(Fault::BadRef)?;
        let vb = v.get_mut(..self.shape.v as usize).ok_or(Fault::BadRef)?;
        p.write(w, kb, vb)?;
        let (s, e) = self.inner.at(n, koff, ks)?;
        native_table::mem::copy_bytes(self.inner.mem.get_mut(s..e).ok_or(Fault::BadRef)?, kb);
        let (s, e) = self.inner.at(n, voff, self.shape.v)?;
        native_table::mem::copy_bytes(self.inner.mem.get_mut(s..e).ok_or(Fault::BadRef)?, vb);
        Ok(())
    }
    fn take_entry(&mut self, n: u64, koff: u64, voff: u64, p: &mut IndexPending<'_>) -> Result<(), Fault> {
        let (s, e) = self.inner.at(n, koff, self.shape.w + 8)?;
        let (vs, ve) = self.inner.at(n, voff, self.shape.v)?;
        let mut k = [0u8; 40];
        let mut v = [0u8; 8];
        native_table::mem::copy_bytes(&mut k, self.inner.mem.get(s..e).ok_or(Fault::BadRef)?);
        native_table::mem::copy_bytes(&mut v, self.inner.mem.get(vs..ve).ok_or(Fault::BadRef)?);
        p.take(k.get(..e - s).unwrap_or(&[]), v.get(..ve - vs).unwrap_or(&[]))
    }
    fn alloc(&mut self, leaf: bool) -> Result<u64, Fault> {
        let n = self.inner.alloc(leaf)?;
        let (s, e) = self.inner.at(n, 2, 6)?;
        let mut hdr = [0u8; 6];
        hdr.get_mut(..2).map(|d| d.copy_from_slice(&(if leaf { 3u16 } else { 2u16 }).to_le_bytes()));
        hdr.get_mut(2..).map(|d| d.copy_from_slice(&self.shape.layout().to_le_bytes()));
        native_table::mem::copy_bytes(self.inner.mem.get_mut(s..e).ok_or(Fault::BadRef)?, &hdr);
        Ok(n)
    }
    fn hand_back(&mut self, n: u64) -> Result<(), Fault> {
        self.inner.hand_back(n)
    }
    fn release(&mut self, r: u64) -> Result<(), Fault> {
        self.inner.release(r)
    }
}

impl ResolveKey for ArrayIndexNodes {
    fn width(&self) -> u64 {
        self.shape.w
    }
    fn record(&self, _field: u64) -> Result<(u64, u64), Fault> {
        Err(Fault::BadRef)
    }
    fn key_piece(&self, _field: u64, _at: u64, _out: &mut [u8]) -> Result<usize, Fault> {
        Err(Fault::BadRef)
    }
    fn rid_base(&self) -> u64 {
        0
    }
}

/// succ(p) into `out`: p with its trailing `FF` bytes dropped and 1 added to the last remaining byte; its
/// length, or `None` when p is all `FF` or `out` is shorter.
fn succ(p: &[u8], out: &mut [u8]) -> Option<usize> {
    let n = p.iter().rposition(|&b| b != 0xFF)?.wrapping_add(1);
    let d = out.get_mut(..n)?;
    for (o, &b) in d.iter_mut().zip(p) {
        *o = b;
    }
    let last = d.last_mut()?;
    *last = last.wrapping_add(1);
    Some(n)
}

#[no_mangle]
pub extern "C" fn nt_index(seed: u64, count: u32, del: u32, lo: u64) -> u64 {
    let Some(shape) = IndexShape::new(32, 8) else { return 0 };
    let mut m = ArrayIndexNodes { inner: ArrayNodes::new(), shape };
    let mut root = 0u64;
    let mut g = Gen(seed | 1);
    let r = (|| -> Result<u64, Fault> {
        let mut keys = [[0u8; 24]; 48];
        for (i, k) in keys.iter_mut().enumerate().take(count as usize % 48) {
            let x = g.next();
            k.get_mut(..8).map(|d| d.copy_from_slice(&x.to_be_bytes()));
            let field = if i % 5 == 0 { kp::tomb_field(x % 4096) } else { kp::row_field(x % 4096) };
            let val = x.to_le_bytes();
            let probe = KeyProbe { key: k, kind: Kind::Row(x % 4096) };
            let mut p = IndexPending::Borrowed { key: k, field, value: &val, child: 0 };
            if root == 0 {
                root = tree::insert_first(shape, &mut m, &p)?;
                continue;
            }
            let mut path = Path::new();
            if let Search::Vacant(n, j) = tree::descend(shape, &m, root, &probe, &mut path)? {
                tree::insert_at(shape, &mut m, &mut root, n, j, &mut p, &mut path)?;
            }
        }
        for (i, k) in keys.iter().enumerate() {
            if del > 0 && i as u32 % del == 0 && root != 0 {
                let probe = KeyProbe { key: k, kind: Kind::Low };
                let mut path = Path::new();
                if let Search::Found(n, j) = tree::descend(shape, &m, root, &probe, &mut path)? {
                    tree::remove_found(shape, &mut m, &mut root, n, j, &mut path)?;
                }
            }
        }
        let lb = lo.to_be_bytes();
        let mut sb = [0u8; 8];
        let hi = succ(&lb, &mut sb);
        let lop = KeyProbe { key: &lb, kind: Kind::Low };
        let hip = KeyProbe { key: sb.get(..hi.unwrap_or(0)).unwrap_or(&[]), kind: Kind::Count };
        let mut h = Hasher64::new(0);
        let mut c: Cursor<Rev> = Cursor::seek(shape, &m, root, core::ops::Bound::Excluded(&hip))?;
        while let Some((n, i)) = c.next(shape, &m, core::ops::Bound::Included(&lop))? {
            h.update(&n.wrapping_add(i).to_le_bytes());
        }
        let f = tree::find(shape, &m, root, &lop)?;
        Ok(h.finish() ^ f.map(|x| x.1).unwrap_or(7))
    })();
    code(r)
}

// ---- exact sums of the totals ----

use native_table::stats::I192;

#[no_mangle]
pub extern "C" fn nt_stats(seed: u64, n: u32, sub_every: u32) -> u64 {
    let mut g = Gen(seed | 1);
    let mut i = I192::default();
    for k in 0..n % 4096 {
        let x = g.next();
        let sub = sub_every > 0 && k % sub_every == 0;
        i.apply_i128((u128::from(x) << 64 | u128::from(g.next())) as i128, sub);
    }
    i.0[0]
}

// ---- index trees in table memory ----

use native_table::index::area_nodes::{AreaNodes, KeyLog};

#[no_mangle]
pub extern "C" fn nt_area_tree(seed: u64, count: u32, w: u64) -> u64 {
    let Some(shape) = IndexShape::new(if w % 2 == 0 { 8 } else { 16 }, 0) else { return 0 };
    let mut h = ArrayHost::new();
    let mut g = Gen(seed | 1);
    let r = (|| -> Result<u64, Fault> {
        let tbl = native_table::area::new_root(&mut h)?;
        let mut m = AreaNodes::new(&mut h, tbl, 1, shape, KeyLog::default());
        let mut root = 0u64;
        let mut key = [0u8; 40];
        let mut path = Path::new();
        for i in 0..count % 200 {
            let x = g.next();
            key.get_mut(..8).map(|d| d.copy_from_slice(&x.to_be_bytes()));
            let len = (x % 40) as usize;
            let k = key.get(..len).unwrap_or(&[]);
            let field = if len as u64 > shape.w {
                kp::ovf_field(kp::row_field(u64::from(i)), m.new_record(u64::from(i), k)?)
            } else {
                kp::row_field(u64::from(i))
            };
            let probe = KeyProbe { key: k, kind: Kind::Row(u64::from(i)) };
            path.clear();
            if root == 0 {
                root =
                    tree::insert_first(shape, &mut m, &IndexPending::Borrowed { key: k, field, value: &[], child: 0 })?;
            } else if let Search::Vacant(n, j) = tree::descend(shape, &m, root, &probe, &mut path)? {
                native_table::index::area_insert::insert_vacant(&mut m, &mut root, n, j, k, field, &[], &mut path)?;
            }
        }
        let f = tree::find(shape, &m, root, &KeyProbe { key: &key, kind: Kind::Count })?;
        Ok(root ^ f.map(|x| x.0).unwrap_or(1) ^ m.log.blocks)
    })();
    code(r)
}

// ---- the secondary index ----

use native_table::index::secondary::Secondary;

/// Keys of the secondary-index probe.
const SEC_KEYS: usize = 16;

/// The secondary index: `count % 300` rows under 16 text keys (key j: 2j letters, so some are longer than the
/// key area of 8 to 32 bytes by `w`), every third row deleted again; each key's count against the rows it
/// keeps, and the group walk's groups and total against the keys and rows kept, with `run_list` as the index's
/// run list block (0: none). The tree's root and node count, a fault code, or `u64::MAX − 3` for a wrong
/// answer.
#[no_mangle]
pub extern "C" fn nt_secondary(seed: u64, count: u32, w: u64, run_list: u64) -> u64 {
    const WRONG: u64 = u64::MAX - 3;
    let mut h = ArrayHost::new();
    let mut g = Gen(seed | 1);
    let r = (|| -> Result<u64, Fault> {
        let tbl = native_table::area::new_root(&mut h)?;
        let mut ix = Secondary::create(1, 8u64.wrapping_mul(1 + w % 4)).ok_or(Fault::BadRef)?;
        ix.run_list = run_list;
        let mut path = Path::new();
        let mut keys = [[0u8; 40]; SEC_KEYS];
        let mut lens = [0usize; SEC_KEYS];
        let letters = [b'a'; 32];
        for (j, (k, l)) in keys.iter_mut().zip(lens.iter_mut()).enumerate() {
            let mut sink = SliceSink::new(k);
            key::enc_bytes(&mut sink, letters.get(..2 * j).unwrap_or(&[]))?;
            *l = sink.len;
        }
        let key_of = |j: usize| keys.get(j).and_then(|k| k.get(..lens.get(j).copied().unwrap_or(0))).unwrap_or(&[]);
        let mut kept = [0i128; SEC_KEYS];
        for i in 0..count % 300 {
            let j = (g.next() % SEC_KEYS as u64) as usize;
            let (k, rid) = (key_of(j), u64::from(i) + 1);
            let slot = kept.get_mut(j).ok_or(Fault::BadRef)?;
            if !ix.insert(&mut h, tbl, k, rid, &mut path)? {
                return Ok(WRONG);
            }
            *slot += 1;
            if i % 3 == 0 {
                if !ix.delete(&mut h, tbl, k, rid, &mut path)? {
                    return Ok(WRONG);
                }
                *slot -= 1;
            }
        }
        for (j, &c) in kept.iter().enumerate() {
            if ix.count(&h, key_of(j))? != c {
                return Ok(WRONG);
            }
        }
        let parts = [KeyPart { leaf: 0, kind: KeyKind::Text, nullable: false }];
        let spec = KeySpec { parts: &parts };
        let (mut out, mut scratch, mut prev) = ([0u8; 64], [0u8; 64], [0u8; 64]);
        let (mut after, mut total, mut groups) = (None, 0i128, 0usize);
        while let Some((len, c)) =
            ix.group_next(&h, &spec, after.and_then(|n| prev.get(..n)), &mut out, &mut scratch)?
        {
            native_table::mem::copy_bytes(prev.get_mut(..len).ok_or(Fault::BadRef)?, out.get(..len).unwrap_or(&[]));
            after = Some(len);
            total += c;
            groups += 1;
            if groups > SEC_KEYS {
                return Ok(WRONG);
            }
        }
        if total != kept.iter().sum::<i128>() || groups != kept.iter().filter(|&&c| c > 0).count() {
            return Ok(WRONG);
        }
        Ok(ix.root ^ ix.nodes.rotate_left(32))
    })();
    code(r)
}

// ---- runs ----

use native_table::index::chunk::{self as ck, Chunker};
use native_table::index::run::{check_header, filter_lines, RunExpect};
use native_table::index::{Finger, MemImage, Run, StructureCheck};
use native_table::tree::Dir;
use nt_builder::plan::{has_count_tree, Measure};
use nt_builder::{build_run, plan_run, BulkState, Item, MemRunSink, RunGeometry, RunInput, RunParams};

/// Bytes of the probe's run images.
const RUN_BYTES: usize = 4 * PAGE as usize;
/// Items of the probe's runs.
const RUN_ITEMS: usize = 96;

/// Items in fixed arrays: 12-byte keys (8 bytes of key number, then 4 bytes of length class) and rids.
pub struct ArrayItems {
    keys: [[u8; 24]; RUN_ITEMS],
    lens: [u8; RUN_ITEMS],
    rids: [u64; RUN_ITEMS],
    tombs: [bool; RUN_ITEMS],
    n: usize,
}

impl ArrayItems {
    /// `n` items of about n / 3 keys, in order, keys of 8 to 20 bytes, every fifth item a tombstone.
    fn new(seed: u64, n: u32) -> Self {
        let mut g = Gen(seed | 1);
        let mut a = ArrayItems {
            keys: [[0; 24]; RUN_ITEMS],
            lens: [0; RUN_ITEMS],
            rids: [0; RUN_ITEMS],
            tombs: [false; RUN_ITEMS],
            n: 0,
        };
        let n = (n as usize % RUN_ITEMS).max(1);
        let mut key = 0u64;
        let mut rid = 0u64;
        for i in 0..n {
            let x = g.next();
            if i == 0 || x % 3 == 0 {
                key = key.wrapping_add(1 + x % 4);
                rid = 0;
            }
            rid = rid.wrapping_add(1 + (x >> 8) % 1_000);
            if let (Some(k), Some(l), Some(r), Some(t)) =
                (a.keys.get_mut(i), a.lens.get_mut(i), a.rids.get_mut(i), a.tombs.get_mut(i))
            {
                if let Some(d) = k.get_mut(..8) {
                    native_table::mem::copy_bytes(d, &key.to_be_bytes());
                }
                // The length is a function of the key, so items of one key share it.
                *l = 8 + (key % 13) as u8;
                if let Some(d) = k.get_mut(8..usize::from(*l)) {
                    d.fill(0xEE);
                }
                *r = rid & 0xFFFF_FFFF;
                *t = x % 5 == 0;
            }
        }
        a.n = n;
        a
    }
}

impl RunInput for ArrayItems {
    fn items(&self) -> u64 {
        self.n as u64
    }
    fn item(&self, i: u64) -> Result<Item<'_>, Fault> {
        let i = usize::try_from(i).map_err(|_| Fault::BadRef)?;
        if i >= self.n {
            return Err(Fault::BadRef);
        }
        let k = self.keys.get(i).ok_or(Fault::BadRef)?;
        let l = usize::from(*self.lens.get(i).ok_or(Fault::BadRef)?);
        Ok(Item {
            key: k.get(..l).ok_or(Fault::BadRef)?,
            rid: *self.rids.get(i).ok_or(Fault::BadRef)?,
            tomb: *self.tombs.get(i).ok_or(Fault::BadRef)?,
        })
    }
}

fn run_params(w: u64, v: u64) -> RunParams {
    RunParams {
        index: 1,
        relative: true,
        w0: 0,
        key_run_l: 0,
        table_uid: 7,
        schema_hash: 8,
        key_spec_hash: 9,
        force: if w == 0 { None } else { Some((w, v)) },
    }
}

/// The bulk build: `n` entries into a W = 8, V = 16 tree; returns a digest of the image.
#[no_mangle]
pub extern "C" fn nt_bulk(seed: u64, n: u32) -> u64 {
    let r = (|| -> Result<u64, Fault> {
        let shape = IndexShape::new(8, 16).ok_or(Fault::BadRef)?;
        let n = u64::from(n % 600).max(1);
        let geo = RunGeometry { first_slot: 1, first_overflow: 0, slots: 15, v: 0 };
        let mut st = BulkState::new(shape, n, geo)?;
        let mut img = [0u8; 16 * BLOCK as usize];
        let mut g = Gen(seed | 1);
        for i in 0..n {
            let key = i.wrapping_mul(3).to_be_bytes();
            let value = g.next().to_le_bytes();
            let c = if g.next() % 4 == 0 { 2 } else { 1 };
            st.push(&mut MemRunSink, &mut img[..], &key, kp::row_field(i), &value, c)?;
        }
        let root = st.finish(&mut MemRunSink, &mut img[..])?;
        Ok(nt_hash64(&img) ^ root)
    })();
    code(r)
}

/// The chunk codec over a value part made from the arguments.
#[no_mangle]
pub extern "C" fn nt_chunk(sel: u32, a: u64, b: u64, c: u64) -> u64 {
    let r = (|| -> Result<u64, Fault> {
        let mut value = [0u8; 256];
        let v = [8usize, 16, 32, 64, 128, 256][(sel as usize) % 6];
        for (i, x) in value.iter_mut().enumerate() {
            *x = (a.rotate_left(i as u32) ^ b) as u8;
        }
        let part = value.get_mut(..v).ok_or(Fault::BadRef)?;
        match sel / 6 % 4 {
            0 => {
                let id = ImageId::Run { index: 1, start_seq: 1 };
                let h = ck::header(part, id, 0)?;
                let mut t = h.items();
                if let ck::Chunk::Posting { m, b } = h {
                    for k in 1..=m {
                        t ^= ck::code(part, m, b, k, id, 0)?;
                    }
                    t ^= ck::first(part, m, b, c, id, 0)?;
                }
                Ok(t ^ ck::abs_rid(c, a, id, 0)?)
            }
            1 => {
                let mut rids = [0u64; 256];
                let cnt = (c as usize % 256).max(1);
                let mut r = a % 1_000;
                let mut ch = Chunker::new(v as u64);
                let mut took = 0usize;
                for slot in rids.iter_mut().take(cnt) {
                    r = r.wrapping_add(b % 300);
                    *slot = r;
                    if !ch.offer(r, false) && took > 0 {
                        break;
                    }
                    took = took.wrapping_add(1);
                }
                ck::encode_posting(rids.get(..took.max(1)).unwrap_or(&[]), part)?;
                Ok(nt_hash64(part))
            }
            2 => {
                ck::encode_key_run((a % 9) as u8, b, part)?;
                let (f, r) = ck::key_run_item(c, c, (b as u32).wrapping_sub(1), (a % 7) as u32, ImageId::Runtime, 0)?;
                Ok(f ^ r ^ nt_hash64(part))
            }
            _ => {
                let k = a.to_be_bytes();
                let k2 = a.wrapping_add(1).to_be_bytes();
                let l = (b % 10) as u8;
                Ok(ck::key_field(&k, l).unwrap_or(1) ^ u64::from(ck::key_run_step(&k, &k2, l)))
            }
        }
    })();
    code(r)
}

/// Measuring and choosing (W, V) over `n` items of a seeded input.
#[no_mangle]
pub extern "C" fn nt_plan(seed: u64, n: u32, w0: u64) -> u64 {
    let r = (|| -> Result<u64, Fault> {
        let items = ArrayItems::new(seed, n);
        let mut m = Measure::new();
        let mut i = 0u64;
        while i < items.items() {
            let x = items.item(i)?;
            m.begin_key();
            m.item(x.key.len() as u64, x.rid, x.tomb)?;
            m.end_key()?;
            i = i.wrapping_add(1);
        }
        m.key_run(n as u64 % 5 + 1, 8)?;
        let c = m.choose(w0 % 2_000)?;
        Ok(c.bytes ^ c.w ^ c.v ^ filter_lines(seed % 1_000_000)? ^ u64::from(has_count_tree(seed % 50, n as u64)))
    })();
    code(r)
}

/// A run built from a seeded input with (W, V) forced (or chosen when w = 0), then opened and read: counts,
/// prefixes, ranks, key steps, extremes, a bounded scan and the structure and header checks.
#[no_mangle]
pub extern "C" fn nt_run(seed: u64, n: u32, w: u64, v: u64) -> u64 {
    let r = (|| -> Result<u64, Fault> {
        let items = ArrayItems::new(seed, n);
        let p = run_params(w, v);
        let plan = plan_run(&items, &p)?;
        let mut img = [0u8; RUN_BYTES];
        let mut filter = [0u8; BLOCK as usize];
        let len = usize::try_from(plan.layout.len).map_err(|_| Fault::BadRef)?;
        let image = img.get_mut(..len).ok_or(Fault::BadRef)?;
        build_run(&items, &p, &plan, &mut MemRunSink, image, &mut filter)?;
        let id = ImageId::Run { index: 1, start_seq: 1 };
        let run = Run::open(MemImage { bytes: image, id }, 0, 0, u64::MAX)?;
        let mut h = Hasher64::new(0);
        let probe = items.item(u64::from(n).checked_rem(items.items()).unwrap_or(0))?.key;
        h.update(&run.count_key(probe)?.to_le_bytes());
        h.update(&run.prefix(probe)?.to_le_bytes());
        let mut finger = Finger::default();
        if let Some(pos) = run.select_rank((seed % 50) as i64, &mut finger)? {
            h.update(&pos.before.to_le_bytes());
        }
        let mut buf = [0u8; 64];
        if let Some((l, t)) = run.key_step(Some(probe), if seed % 2 == 0 { Dir::Fwd } else { Dir::Rev }, 0, &mut buf)? {
            h.update(buf.get(..l).unwrap_or(&[]));
            h.update(&t.to_le_bytes());
        }
        let mut scratch = [0u8; 128];
        if let Some((l, _)) = run.extreme_key(seed % 3 == 0, 0, &mut scratch)? {
            h.update(scratch.get(..l).unwrap_or(&[]));
        }
        let t = run.main()?;
        let lo = KeyProbe { key: probe, kind: Kind::Low };
        let mut c: Cursor<Fwd> = t.seek(core::ops::Bound::Included(&lo))?;
        for _ in 0..8 {
            match t.next(&mut c, core::ops::Bound::Unbounded)? {
                Some((node, i)) => h.update(&node.wrapping_add(i).to_le_bytes()),
                None => break,
            }
        }
        if let Some((node, i)) = t.find(&KeyProbe { key: probe, kind: Kind::Count })? {
            h.update(&t.total(&t.entry(node, i)?)?.to_le_bytes());
        }
        let mut sc = StructureCheck::new();
        let done = sc.step(&MemImage { bytes: image, id }, &run.hdr, 1, 1_000).map_err(|_| Fault::BadRef)?;
        let x = RunExpect { table_uid: 7, schema_hash: 8, index: 1, key_spec_hash: 9, w: run.hdr.w, key_run_l: 0 };
        let head = image.get(..BLOCK as usize).ok_or(Fault::BadRef)?;
        let hc = check_header(head, 1, &x).map(|hh| hh.n).unwrap_or(u64::MAX);
        Ok(h.finish() ^ u64::from(done == native_table::Progress::Done) ^ hc ^ nt_hash64(image).rotate_left(1))
    })();
    code(r)
}

/// The run reader over an image of seeded bytes: every call ends in a value or a fault, never a trap.
#[no_mangle]
pub extern "C" fn nt_run_believed(seed: u64, flips: u32, at: u64) -> u64 {
    let r = (|| -> Result<u64, Fault> {
        let items = ArrayItems::new(seed, 90);
        let p = run_params(8, 16);
        let plan = plan_run(&items, &p)?;
        let mut img = [0u8; RUN_BYTES];
        let mut filter = [0u8; BLOCK as usize];
        let len = usize::try_from(plan.layout.len).map_err(|_| Fault::BadRef)?;
        let image = img.get_mut(..len).ok_or(Fault::BadRef)?;
        build_run(&items, &p, &plan, &mut MemRunSink, image, &mut filter)?;
        let mut g = Gen(seed ^ at | 1);
        for _ in 0..flips % 16 {
            let x = g.next();
            let off = (x % (2 * BLOCK)) as usize;
            if let Some(b) = image.get_mut(off) {
                *b ^= (x >> 32) as u8;
            }
        }
        let id = ImageId::Run { index: 1, start_seq: 1 };
        let run = Run::open(MemImage { bytes: image, id }, 0, 0, u64::MAX)?;
        let probe = items.item(at % 90)?.key;
        let c = run.count_key(probe)?;
        let mut buf = [0u8; 64];
        let s = run.key_step(None, Dir::Rev, 8, &mut buf)?.map(|x| x.0 as u64).unwrap_or(0);
        Ok(c as u64 ^ s)
    })();
    code(r)
}

// ---- the table core ----

use native_table::host::{Budget, Scratch};
use native_table::schema::{flatten, write_schema, Column, ColumnType, IndexDecl, Leaf, LeafSink, SchemaParts};
use native_table::table::group::GroupView;
use native_table::table::{CellIn, CellOut, RowSink, RowSource, Table};

/// Leaves into a fixed array.
struct FixedLeaves {
    l: [Leaf; 8],
    n: usize,
}

impl LeafSink for FixedLeaves {
    fn leaf(&mut self, x: &Leaf) -> Result<(), Fault> {
        let s = self.l.get_mut(self.n).ok_or(Fault::BadRef)?;
        *s = *x;
        self.n = self.n.saturating_add(1);
        Ok(())
    }
}

/// A schema image of single-leaf columns whose type expressions start at `at` in `pool`, column 0 the key,
/// with these index declarations, into `out`; returns its length.
fn probe_schema(pool: &[u8], at: &[u32], indexes: &[IndexDecl], out: &mut [u8]) -> Result<usize, Fault> {
    probe_schema_at(pool, at, 1, indexes, out)
}

/// `probe_schema` at schema version `version`: the columns past the first five are added by it.
fn probe_schema_at(
    pool: &[u8],
    at: &[u32],
    version: u32,
    indexes: &[IndexDecl],
    out: &mut [u8],
) -> Result<usize, Fault> {
    let mut leaves = FixedLeaves { l: [Leaf::default(); 8], n: 0 };
    let mut cols = [Column::default(); 8];
    let mut frames = [0u8; 64 * value::FRAME];
    for (i, &t) in at.iter().enumerate() {
        let first = u16::try_from(leaves.n).map_err(|_| Fault::BadRef)?;
        let v = if i < 5 { 1 } else { version };
        let ct = ColumnType { pool, column: i as u32, ty: t, version: v, first_ordinal: first, key: i == 0 };
        flatten(ct, &mut leaves, &mut frames)?;
        let c = cols.get_mut(i).ok_or(Fault::BadRef)?;
        *c = Column { id: i as u32, flags: 0, first_leaf: first, ty: t, default_off: 0, default_len: 0, name: 0 };
    }
    let parts = SchemaParts {
        version,
        columns: cols.get(..at.len()).unwrap_or(&[]),
        leaves: leaves.l.get(..leaves.n).unwrap_or(&[]),
        indexes,
        type_pool: pool,
        default_pool: &[],
        name_pool: &[],
    };
    let (n, _) = write_schema(&parts, 1, out)?;
    usize::try_from(n).map_err(|_| Fault::BadRef)
}

/// A row of the probe schema {key Nat, Text, i64, ?Bool, a Nat8 of three values}.
struct ProbeRow {
    key: u64,
    text: [u8; 24],
    len: usize,
    val: i64,
    flag: Option<bool>,
    small: u64,
}

impl RowSource for ProbeRow {
    fn cell(&mut self, leaf: u32) -> Result<CellIn<'_>, Fault> {
        Ok(match leaf >> 16 {
            0 => CellIn::U64(self.key),
            1 => CellIn::Bytes(self.text.get(..self.len).unwrap_or(&[])),
            2 => CellIn::I64(self.val),
            3 | 5 => self.flag.map_or(CellIn::Null, CellIn::Bool),
            _ => CellIn::U64(self.small),
        })
    }
    fn fill(&mut self, _leaf: u32, _at: u64, _dst: &mut [u8]) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
}

/// A key alone.
struct ProbeKey(u64);

impl RowSource for ProbeKey {
    fn cell(&mut self, _leaf: u32) -> Result<CellIn<'_>, Fault> {
        Ok(CellIn::U64(self.0))
    }
    fn fill(&mut self, _leaf: u32, _at: u64, _dst: &mut [u8]) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
}

/// Folds every cell it is handed into a digest.
struct ProbeSink(u64);

impl RowSink for ProbeSink {
    fn cell(&mut self, leaf: u32, v: CellOut<'_>) -> Result<(), Fault> {
        let x = match v {
            CellOut::U64(x) => x,
            CellOut::I64(x) => x as u64,
            CellOut::Bool(b) => u64::from(b),
            CellOut::Bytes(b) => nt_hash64(b),
            CellOut::Arg { len } => len,
            CellOut::Null => 7,
            _ => 9,
        };
        self.0 = native_table::hash::mix(self.0 ^ x ^ u64::from(leaf));
        Ok(())
    }
    fn row_end(&mut self, rid: u64) -> Result<(), Fault> {
        self.0 ^= rid;
        Ok(())
    }
}

/// Scratch for maintenance units over a fixed array.
struct ProbeScratch {
    buf: [u8; 81_920],
}

impl Scratch for ProbeScratch {
    fn get(&mut self, need: usize) -> Result<&mut [u8], Fault> {
        self.buf.get_mut(..need).ok_or(Fault::Scratch { need: need as u64 })
    }
}

/// A budget that never runs out.
struct ProbeBudget;

impl Budget for ProbeBudget {
    fn used(&self) -> u64 {
        0
    }
    fn limit(&self) -> u64 {
        u64::MAX
    }
}

/// The probe schema's type pool and column offsets: Nat; Text; i64; ?Bool; Nat8.
const PROBE_POOL: [u8; 6] = [
    0x0A, 0x0F, 0x09, 0x20, 0x01, // Nat, Text, i64, ?Bool
    0x02, // Nat8
];
const PROBE_COLS: [u32; 5] = [0, 1, 2, 3, 5];

/// A secondary index declaration: id `id`, W 16, on these (leaf, nullable) parts.
fn index_decl(id: u8, parts: &[(u32, bool)]) -> IndexDecl {
    let mut part = [(0u32, false); 7];
    for (p, &x) in part.iter_mut().zip(parts) {
        *p = x;
    }
    // At most 7 parts.
    #[allow(clippy::cast_possible_truncation)]
    IndexDecl { id, flags: 0, w: 16, parts: parts.len() as u8, part }
}

/// The table's writes and reads, with two secondary indexes (the Nat8; the optional Bool and the text): `n %
/// 300` upserts of keys from `seed`, updates, gets, removes, counts through each index, and maintenance; a
/// digest of what it read, or a fault code.
#[no_mangle]
pub extern "C" fn nt_table(seed: u64, n: u32) -> u64 {
    let mut h = ArrayHostN::<{ 6 * 65_536 }>::new();
    let mut g = Gen(seed | 1);
    let r = (|| -> Result<u64, Fault> {
        let mut img = [0u8; 1_024];
        let indexes = [index_decl(1, &[(4 << 16, false)]), index_decl(2, &[(3 << 16, true), (1 << 16, false)])];
        let len = probe_schema(&PROBE_POOL, &PROBE_COLS, &indexes, &mut img)?;
        let (mut t, _) = Table::create(&mut h, seed, img.get(..len).unwrap_or(&[]))?;
        let mut sc = ProbeScratch { buf: [0; 81_920] };
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
            if i % 7 == 3 {
                t.remove(&mut sc, &mut ProbeKey(g.next() % 500))?;
            }
            let mut s = ProbeSink(digest);
            if t.get(&mut sc, &mut ProbeKey(g.next() % 500), &mut s)? {
                digest = s.0;
            }
            // A row's cells are its own key source under each index.
            let ix = 1 + (i % 2) as u8;
            digest = native_table::hash::mix(digest ^ (t.count(&mut sc, ix, &mut row)? as u64));
        }
        t.write_step(&ProbeBudget)?;
        let p = t.maintain(&ProbeBudget)?;
        Ok(digest ^ t.len()? ^ u64::from(p == Progress::Done))
    })();
    code(r)
}

/// Updates, maintenance and schema change: `n % 300` upserts, a schema change adding an ?Bool column when `sel`
/// is odd, and maintenance one unit at a time; a digest of what it read, or a fault code.
#[no_mangle]
pub extern "C" fn nt_table_evolve(seed: u64, n: u32, sel: u32) -> u64 {
    let mut h = ArrayHostN::<{ 6 * 65_536 }>::new();
    let mut g = Gen(seed | 1);
    let r = (|| -> Result<u64, Fault> {
        let mut img = [0u8; 1_024];
        let len = probe_schema(&PROBE_POOL, &PROBE_COLS, &[], &mut img)?;
        let (mut t, _) = Table::create(&mut h, seed, img.get(..len).unwrap_or(&[]))?;
        let mut sc = ProbeScratch { buf: [0; 81_920] };
        let mut digest = 0u64;
        for _ in 0..n % 300 {
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
        }
        if sel & 1 == 1 {
            let mut img2 = [0u8; 1_024];
            let len2 = probe_schema_at(&PROBE_POOL, &[0, 1, 2, 3, 5, 3], 2, &[], &mut img2)?;
            t.change_schema(img2.get(..len2).unwrap_or(&[]))?;
        }
        let mut units = 0u64;
        while t.unit(&ProbeBudget)?.0 != native_table::table::jobs::Ran::Nothing {
            units = units.wrapping_add(1);
        }
        t.write_step(&ProbeBudget)?;
        t.maintain(&ProbeBudget)?;
        let mut s = ProbeSink(digest);
        if t.get(&mut sc, &mut ProbeKey(g.next() % 500), &mut s)? {
            digest = s.0;
        }
        Ok(digest ^ t.len()? ^ units)
    })();
    code(r)
}

/// The runs' known answers for the self-test: 0 when they hold.
fn runs_selftest() -> u32 {
    // A build is a function of its input.
    if nt_bulk(3, 400) != nt_bulk(3, 400) || nt_bulk(3, 400) >= u64::MAX - 64 {
        return 20;
    }
    // A posting chunk of b = 63 is refused with a fault, and a chunk of rids encodes.
    let mut v = [0u8; 8];
    v[2] = 63;
    if ck::header(&v, ImageId::Runtime, 0).is_ok() || nt_chunk(6, 5, 3, 40) >= u64::MAX - 64 {
        return 21;
    }
    // Runs build, open and read the same on every target.
    let a = nt_run(11, 80, 8, 16);
    if a >= u64::MAX - 64 || a != nt_run(11, 80, 8, 16) || nt_run(11, 80, 0, 0) >= u64::MAX - 64 {
        return 22;
    }
    if nt_plan(5, 90, 0) >= u64::MAX - 64 {
        return 23;
    }
    0
}

/// A seal: a key-only table filled until its first group freezes (most rows removed again, so the primary
/// index stays small and the image is compacted), sealed, then a sealed row read and removed, and the drain
/// run.
#[no_mangle]
pub extern "C" fn nt_table_seal(n: u32) -> u64 {
    let mut h = ArrayHostN::<{ 8 * 65_536 }>::new();
    let r = (|| -> Result<u64, Fault> {
        let mut img = [0u8; 512];
        let len = probe_schema(&PROBE_POOL, &[0], &[], &mut img)?;
        let (mut t, _) = Table::create(&mut h, 3, img.get(..len).unwrap_or(&[]))?;
        let mut sc = ProbeScratch { buf: [0; 81_920] };
        for k in 0..u64::from(n % 8_192) {
            t.upsert(&mut sc, &mut ProbeKey(k * 3))?;
            if k % 10 != 0 {
                t.remove(&mut sc, &mut ProbeKey(k * 3))?;
            }
        }
        t.maintain(&ProbeBudget)?;
        let mut s = ProbeSink(0);
        let found = t.get(&mut sc, &mut ProbeKey(30), &mut s)?;
        let mut buf = [0u8; 8];
        let big = t.read_large(10, 0, 0, &mut buf).unwrap_or(0) as u64;
        let (gone, _) = t.remove(&mut sc, &mut ProbeKey(30))?;
        t.maintain(&ProbeBudget)?;
        Ok(s.0 ^ u64::from(found) ^ gone.unwrap_or(1) ^ big ^ t.len()?)
    })();
    code(r)
}

/// A forged group: a group page naming a one-block image whose bytes come from `a` and `b`, read through the
/// group reader at leaf `leaf` and position `pos`.
#[no_mangle]
pub extern "C" fn nt_group_read(a: u64, b: u64, leaf: u64, pos: u64) -> u64 {
    let mut h = ArrayHost::new();
    let r = (|| -> Result<u64, Fault> {
        let pg_page = h.page_alloc()?;
        let im = h.page_alloc()?;
        {
            let page = h.get_mut(Region::Page(im))?;
            let mut g = Gen(a | 1);
            for (i, x) in page.iter_mut().enumerate() {
                *x = (g.next() ^ b.rotate_left(i as u32 % 64)) as u8;
            }
            native_table::schema::put_common_header(page, 1, 0, PAGE, a, b)?;
            put_u64(page, 112, b % PAGE)?;
        }
        {
            let pg = h.get_mut(Region::Block(pg_page))?;
            pg.fill(0);
            native_table::mem::put_u32(pg, 0, native_table::consts::magic::NTGP)?;
            let img = Image {
                length: PAGE,
                blocks: native_table::image::Blocks::Direct { count: 1, entries: [im, 0, 0, 0, 0, 0, 0, 0] },
                believed: true,
            };
            img.put_record(pg, 16)?;
        }
        let gv = GroupView::open(&h, pg_page)?;
        let ld = gv.leaf(leaf)?;
        let slot = gv.slot(pos)?.unwrap_or(pos);
        let c = gv.cell(&ld, slot, pos)?;
        Ok(match c {
            native_table::table::group::GCell::Bytes { off, len } => off ^ len,
            native_table::table::group::GCell::U(x) => x,
            _ => 1,
        })
    })();
    code(r)
}
