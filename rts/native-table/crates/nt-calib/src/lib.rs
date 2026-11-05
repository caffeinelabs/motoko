//! The cost meter's calibration canister. Its one update method, `run`, takes a workload selector (u32) and
//! six u64 arguments, little-endian, runs one of the probe's workloads between two reads of the replica's
//! instruction counter, and replies with 16 bytes: the instructions spent and the workload's result; a sweep
//! of keys (workload 3 with a sweep) appends each key's instructions, 8 bytes each.
//!
//! Run under wasmtime with the meter's own functions in place of the system API's, the same module gives the
//! meter's figure for the same workload, read at the same two points: the calibration compares the two.
//!
//! Workloads: 0 typed-tree operations (`nt_tree`); 1 index-tree operations (`nt_index`); 2 a copy-heavy unit
//! of whole-page copies (`nt_copy_unit`); 3 the count of one key over an index's uploaded segments, counted
//! from the end of their build (`count_bar`: words loaded runs, key position, the `BAR_` options, and a
//! sweep's key count, whose result is the most instructions one count took, read with the counter); 5
//! nothing, so that its figure is the cost of the counter's reads alone; 8 the primary index's inserts
//! through its lookup (`primary_inserts`); 9 the fixed part of an indexed count on the crate's side
//! (`count_entry_case`); 10 a run's distinct keys stepped through in order, counted from the run's opening
//! (`key_steps`: words rows, the `BAR_` key options, and 1 to step backward); 12 the secondary index's
//! inserts, deletes or counts, counted from the end of the rows inserted first (`secondary_ops`); 13 a
//! table's inserts, updates or removes with 0 to 3 secondary indexes, counted from the end of the rows inserted
//! first and their seal (`table_ops`); 14 filtered scans, cell scans and a scan reading every row whole,
//! counted from the end of the rows inserted first and their seal (`filter_scans`); 15 an index's group
//! counts, or a count of each of its keys, counted from the end of the rows inserted first and their seal
//! (`group_counts`); 16 an integer leaf's minimum or maximum, or the table's open alone, counted from the end
//! of the rows inserted first and their seal (`extremes_open`).
#![no_std]

#[link(wasm_import_module = "ic0")]
extern "C" {
    fn msg_arg_data_size() -> usize;
    fn msg_arg_data_copy(dst: usize, offset: usize, size: usize);
    fn msg_reply_data_append(src: usize, size: usize);
    fn msg_reply();
    fn performance_counter(counter_type: u32) -> u64;
}

/// The argument bytes: the selector and six words.
const ARGS: usize = 4 + 6 * 8;

/// The workloads' table memory, which no stack holds.
static mut MEM: [u8; nt_probe::CALIB_MEM] = [0; nt_probe::CALIB_MEM];

/// A sweep's per-key instructions, replied after the first 16 bytes.
static mut KEYS: [u8; 8 * nt_probe::SWEEP_MAX] = [0; 8 * nt_probe::SWEEP_MAX];

fn counter() -> u64 {
    // SAFETY: a system API call without memory arguments.
    unsafe { performance_counter(0) }
}

fn word(a: &[u8; ARGS], i: usize) -> u64 {
    let at = 4 + 8 * i;
    match a.get(at..at + 8).and_then(|s| s.first_chunk::<8>()) {
        Some(w) => u64::from_le_bytes(*w),
        None => 0,
    }
}

fn code<E>(r: Result<u64, E>) -> u64 {
    r.unwrap_or(u64::MAX)
}

#[export_name = "canister_update run"]
pub extern "C" fn run() {
    let mut a = [0u8; ARGS];
    // SAFETY: the system API copies at most ARGS bytes into `a`.
    unsafe {
        let n = msg_arg_data_size().min(ARGS);
        msg_arg_data_copy(a.as_mut_ptr() as usize, 0, n);
    }
    let sel = match a.first_chunk::<4>() {
        Some(s) => u32::from_le_bytes(*s),
        None => u32::MAX,
    };
    let w = |i| word(&a, i);
    // SAFETY: update calls run one at a time, so this is the only reference to MEM.
    let mem: &mut [u8] = unsafe { &mut *core::ptr::addr_of_mut!(MEM) };
    let mut per_key = [0u64; nt_probe::SWEEP_MAX];
    let mut swept = 0usize;
    let mut start = counter();
    let out = match sel {
        0 => nt_probe::nt_tree(w(0), w(1) as u32, w(2) as u32, w(3)),
        1 => nt_probe::nt_index(w(0), w(1) as u32, w(2) as u32, w(3)),
        2 => nt_probe::nt_copy_unit(w(0)),
        3 => {
            let spec = nt_probe::BarSpec { count: 1, loaded: w(0) as u32, key_sel: w(1), options: w(2), sweep: w(3) };
            swept = (spec.sweep as usize).min(nt_probe::SWEEP_MAX);
            code(nt_probe::count_bar_keys(
                mem,
                &spec,
                counter,
                &mut || {
                    start = counter();
                },
                &mut per_key,
            ))
        }
        5 => 0,
        8 => code(nt_probe::primary_inserts(mem, [w(0), w(1), w(2), w(3), w(4), w(5)], counter, &mut || {
            start = counter();
        })),
        9 => code(nt_probe::count_entry_case(mem, [w(0), w(1), w(2), w(3), w(4), w(5)], &mut || {
            start = counter();
        })),
        10 => code(nt_probe::key_steps(mem, w(0), w(1), w(2) != 0, &mut || {
            start = counter();
        })),
        12 => code(nt_probe::secondary_ops(mem, [w(0), w(1), w(2), w(3), w(4), w(5)], counter, &mut || {
            start = counter();
        })),
        13 => code(nt_probe::table_ops(mem, [w(0), w(1), w(2), w(3), w(4), w(5)], counter, &mut || {
            start = counter();
        })),
        14 => code(nt_probe::filter_scans(mem, [w(0), w(1), w(2), w(3), w(4), w(5)], &mut || {
            start = counter();
        })),
        15 => code(nt_probe::group_counts(mem, [w(0), w(1), w(2), w(3), w(4), w(5)], &mut || {
            start = counter();
        })),
        16 => code(nt_probe::extremes_open(mem, [w(0), w(1), w(2), w(3), w(4), w(5)], &mut || {
            start = counter();
        })),
        _ => u64::MAX,
    };
    let spent = counter().wrapping_sub(start);
    let mut r = [0u8; 16];
    if let Some((x, y)) = r.split_first_chunk_mut::<8>() {
        *x = spent.to_le_bytes();
        if let Some(z) = y.first_chunk_mut::<8>() {
            *z = out.to_le_bytes();
        }
    }
    // SAFETY: update calls run one at a time, so this is the only reference to KEYS.
    let keys: &mut [u8] = unsafe { &mut *core::ptr::addr_of_mut!(KEYS) };
    for (k, chunk) in per_key.iter().take(swept).zip(keys.chunks_exact_mut(8)) {
        if let Some(c) = chunk.first_chunk_mut::<8>() {
            *c = k.to_le_bytes();
        }
    }
    // SAFETY: the system API reads the 16 bytes of `r` and the first 8 · `swept` bytes of KEYS.
    unsafe {
        msg_reply_data_append(r.as_ptr() as usize, r.len());
        if swept > 0 {
            msg_reply_data_append(core::ptr::addr_of!(KEYS) as usize, 8 * swept);
        }
        msg_reply();
    }
}
