//! The crate's fail-fast cost checks against OQL's bars, run on the calibration canister (`nt-calib`):
//!
//! - `nt-fuel <nt_calib.wasm>`: two checks.
//!   - Counts: the count of one key over an index's uploaded segments (1, 4, 64 and 1,024 loaded runs holding
//!     10^5 rows of 10^5 values between them, the index's tree empty), and a sweep of 300 keys over one loaded
//!     run. Each count first does the crate's fixed part of a count for its key (the key encoded, the index
//!     located: the canister's `BAR_ENTRY`). The canister's count is metered
//!     under wasmtime (`meter`), from the end of the build to the end of the count, and taken to the
//!     replica's instructions with the recorded calibration at the top of its band; a sweep's counts are read
//!     with the meter's counter, which answers at that ratio. The rest of a whole count, which the canister
//!     does not run, is the Motoko host's part, taken at the high end of its estimate (`HOST`). **The bar is
//!     OQL's**: its count with the same number of uploaded segments (moc 1.16.1, the lower compiler). The run
//!     fails when a key's count is above its bar.
//!   - Bulk memory: the bytes `memory.copy` and `memory.fill` move per primary-index insert, exact under the
//!     meter; the run fails when a figure rises above its budget (`BULK`).
//! - `nt-fuel calibrate <nt_calib.wasm> <pocket-ic server binary>`: the calibration run. The tree mix (typed
//!   and index trees), a copy-heavy unit of whole-page copies and two counts over loaded runs run once metered
//!   and once on PocketIC; the ratio r of the replica's instructions to the meter's plain fuel is fitted so
//!   that the largest relative error over them is smallest, and that error is the band. The replica's
//!   readings are taken less its figure for an empty workload, the cost of the counter's reads. Then every
//!   count row runs on PocketIC too, and its measured figure is printed beside its estimate and its bar.
//! - `nt-fuel writes <nt_calib.wasm> <pocket-ic server binary>`: write costs measured on PocketIC: the primary
//!   index's insert through its lookup as a table write makes it; the secondary index's insert, delete and
//!   count; a table's insert and delete with 0 to 3 secondary indexes, beside OQL's lone-insert bars; and the
//!   build of an index a schema change adds, in maintenance units. The run fails when a workload faults or
//!   gives a wrong answer, or when one unit of a build takes more than a unit's bound (`UNIT_WORK`).
//! - `nt-fuel scans <nt_calib.wasm> <pocket-ic server binary>`: filtered scans measured on PocketIC, per row
//!   of a table of 10^5 rows, beside a scan that reads every row whole: a count under an equality, a range and
//!   a filter every row meets, and every row handed out by the filtered scan. The run fails when a workload
//!   faults or gives a wrong answer.
//! - `nt-fuel cells <nt_calib.wasm> <pocket-ic server binary>`: cell scans measured on PocketIC, per row of a
//!   table of 10^5 rows of four columns: every row's one column alone, the key with it, and every column, with
//!   no filter and under a filter every row meets, beside a scan that reads every row whole by rid and the
//!   filtered scan's whole rows. The run fails when a workload faults or gives a wrong answer.
//! - `nt-fuel groups <nt_calib.wasm> <pocket-ic server binary>`: an index's group counts measured on PocketIC,
//!   over a table of 10^5 rows, for an index of 10^3 keys and one of 3, beside a count of each key. The run
//!   fails when a workload faults or gives a wrong answer.
//! - `nt-fuel extremes <nt_calib.wasm> <pocket-ic server binary>`: an integer column's minimum and maximum
//!   over a table of 10^5 rows in 25 groups, and the table's open alone, measured on PocketIC, one call each.
//!   The run fails when a workload faults or gives a wrong answer.
//! - `nt-fuel stages <nt_probe.wasm>`: the count's plain fuel by stage, for diagnosis.

mod meter;
mod replica;

use std::path::PathBuf;
use std::process::ExitCode;

use meter::{reply_words, Figure, Meter};
use wasmtime::{Config, Engine, Instance, Module, Store};

/// The ratio of the replica's instructions to the meter's plain fuel, and its band (the largest relative
/// error of the fit over the calibration's workloads), as `calibrate` last measured them: PocketIC 14.0.0,
/// wasmtime 42.0.1, the canister built for wasm64. A workload whose estimate at R lies within the band of
/// the replica's figure x has x at most estimate / (1 − BAND): that is the top of the band.
const R: f64 = 1.1814;
const BAND: f64 = 0.0505;

/// The Motoko host's part of a count's fixed cost, ESTIMATE at its high end: OQL's preamble 2,500, its plan
/// encoding 6,000, the runtime call 250, the reply shell and its one-cell answer 2,950. The crate's part (the
/// key encoded and the index located) is measured with every count, and so is every uploaded segment.
const HOST: u64 = 2_500 + 6_000 + 250 + 2_950;

/// The key positions each row counts (`nt_probe::bar_value`); a row's figure is the worst of them.
const KEYS: [u64; 3] = [5, 17, 123];

/// The keys a sweep counts, one after another: positions 0 to 299, as OQL's sweeps.
const SWEEP: u64 = 300;

/// The count workload's option (the probe's `BAR_ENTRY`): the crate's fixed part of a count first. Every row
/// takes it.
const ENTRY: u64 = 128;

/// OQL's counts with uploaded segments alone: (loaded runs, OQL's figure, its source).
const LOADED: [(u32, u64, &str); 4] = [
    (1, 61_692, "OQL, 1 uploaded segment, measured (moc 1.16.1)"),
    (4, 114_136, "OQL, 4 uploaded segments, measured (moc 1.16.1)"),
    (64, 1_082_860, "OQL, 64 uploaded segments, measured (moc 1.16.1)"),
    (1_024, 15_836_789, "OQL, 1,021 uploaded segments, measured (moc 1.16.1)"),
];

/// The value a count bar counts at key position `pos` (as `nt_probe::bar_value`, for keys the rows may hold).
fn bar_value(pos: u64) -> u64 {
    (pos * 7_919 % 100_000) & !1
}

struct Row {
    name: String,
    loaded: u32,
    sweep: u64,
    bar: u64,
    source: &'static str,
}

impl Row {
    /// The key positions it counts.
    fn positions(&self) -> Vec<u64> {
        if self.sweep > 0 {
            (0..self.sweep).collect()
        } else {
            KEYS.to_vec()
        }
    }
}

/// "n loaded run(s)".
fn loaded_name(l: u32) -> String {
    let n = if l == 1_024 { String::from("1,024") } else { l.to_string() };
    format!("{n} loaded run{}", if l == 1 { "" } else { "s" })
}

/// The checked rows: each count of loaded runs alone, and a sweep over one loaded run.
fn rows() -> Vec<Row> {
    let mut v: Vec<Row> =
        LOADED.iter().map(|&(l, bar, source)| Row { name: loaded_name(l), loaded: l, sweep: 0, bar, source }).collect();
    let (l, bar, source) = LOADED[0];
    v.push(Row { name: format!("{SWEEP} keys: {}", loaded_name(l)), loaded: l, sweep: SWEEP, bar, source });
    v
}

/// The canister's count workload for a row and a key position (for a sweep, its first).
fn count_words(r: &Row, pos: u64) -> [u64; 6] {
    [u64::from(r.loaded), pos, ENTRY, r.sweep, 0, 0]
}

/// A fault code in place of a workload's result.
fn faulted(x: u64) -> bool {
    x >= u64::MAX - 64
}

/// The replica's estimate of a figure at the top of the recorded band.
fn top(f: &Figure) -> u64 {
    (f.at(R) / (1.0 - BAND)).ceil() as u64
}

fn delta(x: u64, bar: u64) -> String {
    format!("{:+.1}%", (x as f64 - bar as f64) / bar as f64 * 100.0)
}

fn main() -> ExitCode {
    let a: Vec<String> = std::env::args().collect();
    let r = match a.get(1).map(String::as_str) {
        Some("calibrate") => match (a.get(2), a.get(3)) {
            (Some(c), Some(s)) => calibrate(c, PathBuf::from(s)),
            _ => Err("usage: nt-fuel calibrate <nt_calib.wasm> <pocket-ic server binary>".into()),
        },
        Some("one") => match (a.get(2), a.get(3).and_then(|x| x.parse::<usize>().ok())) {
            (Some(c), Some(i)) => one(c, i),
            _ => Err("usage: nt-fuel one <nt_calib.wasm> <row number>".into()),
        },
        Some("stages") => match a.get(2) {
            Some(p) => stages(p),
            None => Err("usage: nt-fuel stages <nt_probe.wasm>".into()),
        },
        Some("scans") => match (a.get(2), a.get(3)) {
            (Some(c), Some(s)) => scans(c, PathBuf::from(s)),
            _ => Err("usage: nt-fuel scans <nt_calib.wasm> <pocket-ic server binary>".into()),
        },
        Some("cells") => match (a.get(2), a.get(3)) {
            (Some(c), Some(s)) => cells(c, PathBuf::from(s)),
            _ => Err("usage: nt-fuel cells <nt_calib.wasm> <pocket-ic server binary>".into()),
        },
        Some("groups") => match (a.get(2), a.get(3)) {
            (Some(c), Some(s)) => groups(c, PathBuf::from(s)),
            _ => Err("usage: nt-fuel groups <nt_calib.wasm> <pocket-ic server binary>".into()),
        },
        Some("extremes") => match (a.get(2), a.get(3)) {
            (Some(c), Some(s)) => extremes(c, PathBuf::from(s)),
            _ => Err("usage: nt-fuel extremes <nt_calib.wasm> <pocket-ic server binary>".into()),
        },
        Some("writes") => match (a.get(2), a.get(3)) {
            (Some(c), Some(s)) => writes(c, PathBuf::from(s)),
            _ => Err("usage: nt-fuel writes <nt_calib.wasm> <pocket-ic server binary>".into()),
        },
        Some(c) => checks(c),
        None => Err("usage: nt-fuel <nt_calib.wasm>".into()),
    };
    match r {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn read(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{path}: {e}"))
}

/// The meter, its counter answering at the top of the recorded band.
fn meter(canister: &str) -> Result<Meter, String> {
    let mut m = Meter::new(&read(canister)?)?;
    m.ratio = R / (1.0 - BAND);
    Ok(m)
}

/// One counted key of a row: its position, its value, and the count's figure (the index's uploaded segments
/// and the crate's fixed part, replica instructions).
struct Keyed {
    pos: u64,
    value: u64,
    count: u64,
}

/// A row's counts, key by key, each at the top of the band: for a sweep, as the meter's counter read them;
/// else each key's metered figure. With the run's figure (for a sweep, its first key's).
fn row_keys(m: &Meter, r: &Row) -> Result<(Vec<Keyed>, Figure), String> {
    let mut out = Vec::new();
    if r.sweep > 0 {
        let run = m.run_all(3, count_words(r, 0))?;
        if faulted(run.last.result) {
            return Err(format!("{}: a count faulted ({:#x})", r.name, run.last.result));
        }
        let per = reply_words(&run.reply);
        if per.len() as u64 != r.sweep {
            return Err(format!("{}: {} keys' figures for a sweep of {}", r.name, per.len(), r.sweep));
        }
        for (pos, count) in r.positions().into_iter().zip(per) {
            out.push(Keyed { pos, value: bar_value(pos), count });
        }
        return Ok((out, run.span));
    }
    let mut last = Figure::default();
    for pos in r.positions() {
        let f = m.run(3, count_words(r, pos))?;
        if faulted(f.result) {
            return Err(format!("{}: the count faulted ({:#x})", r.name, f.result));
        }
        out.push(Keyed { pos, value: bar_value(pos), count: top(&f) });
        last = f;
    }
    Ok((out, last))
}

/// A row's verdict over its keys: the worst key (the most its count, with the rest, is above the bar, or the
/// least below) and the keys above the bar.
struct Verdict<'a> {
    worst: &'a Keyed,
    total: u64,
    excess: i64,
    above: usize,
}

fn verdict<'a>(r: &Row, keys: &'a [Keyed]) -> Option<Verdict<'a>> {
    let excess = |k: &Keyed| (k.count + HOST) as i64 - r.bar as i64;
    let worst = keys.iter().max_by_key(|k| excess(k))?;
    Some(Verdict {
        worst,
        total: worst.count + HOST,
        excess: excess(worst),
        above: keys.iter().filter(|k| excess(k) > 0).count(),
    })
}

/// Both checks: the counts against OQL's bars, then the bulk memory against its budgets.
fn checks(canister: &str) -> Result<bool, String> {
    let m = meter(canister)?;
    let counts_ok = bars(&m)?;
    println!();
    let bulk_ok = bulk(&m)?;
    Ok(counts_ok && bulk_ok)
}

/// The count rows against their bars.
fn bars(m: &Meter) -> Result<bool, String> {
    println!(
        "Count of one key over an index's uploaded segments, 10^5 rows of 10^5 values between them, and the crate's"
    );
    println!(
        "fixed part of a count, metered under wasmtime 42.0.1 (the calibration canister, wasm64) and taken to the"
    );
    println!(
        "replica's instructions at r = {R}, at the top of its band (±{:.2}%); the rest of a count, the Motoko host's",
        BAND * 100.0
    );
    println!("part, at the high end of its estimate: {HOST}. The bar is OQL's count with as many uploaded segments.");
    println!();
    println!("| composition | keys | worst key (position, value) | its count (measured part) | + the rest (ESTIMATE) | its bar | Δ | keys above the bar | bar's source | |");
    println!("|---|---:|---|---:|---:|---:|---:|---:|---|---|");
    let mut failed = false;
    for r in rows() {
        let (keys, _) = row_keys(m, &r)?;
        let v = verdict(&r, &keys).ok_or("a row without keys")?;
        let status = if v.excess > 0 {
            failed = true;
            format!("LOSS, +{}", v.excess)
        } else {
            String::from("at or below")
        };
        println!(
            "| {} | {} | {}, {} | {} | {} | {} | {} | {} of {} | {} | {status} |",
            r.name,
            keys.len(),
            v.worst.pos,
            v.worst.value,
            v.worst.count,
            v.total,
            r.bar,
            delta(v.total, r.bar),
            v.above,
            keys.len(),
            r.source
        );
    }
    if failed {
        eprintln!("a count is above its bar");
    }
    Ok(!failed)
}

/// The bulk-memory budgets: per workload, the bytes `memory.copy` and `memory.fill` move or fill over its
/// window and their number, as last measured (exact under the meter). A figure above its budget fails the
/// run; lower ones are printed, to be taken as the new budgets.
const BULK: &[(&str, u64, u64)] = &[("primary insert: (8, 0) through its lookup, rising keys", 1_046_336, 1_021)];

/// The workloads the bulk budgets cover: a name, the canister's workload and words, the units of work its
/// window holds, and the workload whose window is the harness's own share (subtracted), if any.
fn bulk_workloads() -> Vec<(&'static str, u32, [u64; 6], u64, Option<[u64; 6]>)> {
    vec![(
        "primary insert: (8, 0) through its lookup, rising keys",
        8,
        [16_256, 1, 1_000, 0, 7, 0],
        1_000,
        Some([16_256, 1, 1_000, 4, 7, 0]),
    )]
}

/// The bulk-memory figures against their budgets.
fn bulk(m: &Meter) -> Result<bool, String> {
    println!(
        "Bulk memory (`memory.copy` and `memory.fill`), exact under the meter: bytes and instructions per window,"
    );
    println!("and per unit of work; the budget is the window's figure as last measured.");
    println!();
    println!(
        "| workload | bytes | instructions | bytes per unit | instructions per unit | budget: bytes, instructions | |"
    );
    println!("|---|---:|---:|---:|---:|---:|---|");
    let mut failed = false;
    for (name, sel, w, units, base) in bulk_workloads() {
        let f = m.run_all(sel, w)?;
        if faulted(f.span.result) {
            return Err(format!("{name}: the workload faulted ({:#x})", f.span.result));
        }
        let (mut bytes, mut ops) = (f.span.bytes, f.span.bulk);
        if let Some(b) = base {
            let g = m.run_all(sel, b)?;
            bytes = bytes.saturating_sub(g.span.bytes);
            ops = ops.saturating_sub(g.span.bulk);
        }
        let budget = BULK.iter().find(|x| x.0 == name).map(|&(_, b, o)| (b, o));
        let status = match budget {
            None => {
                failed = true;
                String::from("no budget")
            }
            Some((b, o)) if bytes > b || ops > o => {
                failed = true;
                String::from("ABOVE its budget")
            }
            Some((b, o)) if bytes < b || ops < o => String::from("below its budget: lower it"),
            Some(_) => String::from("at its budget"),
        };
        let (b, o) = budget.unwrap_or((0, 0));
        println!(
            "| {name} | {bytes} | {ops} | {:.1} | {:.2} | {b}, {o} | {status} |",
            bytes as f64 / units as f64,
            ops as f64 / units as f64
        );
    }
    if failed {
        eprintln!("bulk memory above its budgets");
    }
    Ok(!failed)
}

/// One row's count, metered, for diagnosis (with `NT_METER_LENGTHS` set, its bulk instructions' lengths).
fn one(canister: &str, i: usize) -> Result<bool, String> {
    let m = meter(canister)?;
    let rs = rows();
    let r = rs.get(i).ok_or("no such row")?;
    for pos in KEYS {
        let f = m.run(3, count_words(r, pos))?;
        println!("{} key {pos}: {f:?}, plain {}, bulk cost {}", r.name, f.plain(), f.bulk_cost());
    }
    Ok(true)
}

/// The calibration's workloads: the tree mix, the copy-heavy unit, and counts over loaded runs, whose mix of
/// calls and checked reads the replica charges more for than the tree mix's, so that the band covers the
/// figures the bars check.
fn fit_workloads() -> Vec<(&'static str, u32, [u64; 6])> {
    vec![
        ("count, 64 loaded runs", 3, [64, 5, 0, 0, 0, 0]),
        ("count, 1,024 loaded runs", 3, [1_024, 5, 0, 0, 0, 0]),
        ("typed tree, 300 keys", 0, [3, 300, 4, 50, 0, 0]),
        ("typed tree, 1,500 keys", 0, [5, 1_500, 7, 400, 0, 0]),
        ("index tree, 300 keys", 1, [3, 300, 4, 50, 0, 0]),
        ("index tree, 1,000 keys", 1, [7, 1_000, 6, 300, 0, 0]),
        ("copy unit, 1 page", 2, [1, 0, 0, 0, 0, 0]),
        ("copy unit, 16 pages", 2, [16, 0, 0, 0, 0, 0]),
    ]
}

/// The calibration run.
fn calibrate(canister: &str, server: PathBuf) -> Result<bool, String> {
    let wasm = read(canister)?;
    let m = meter(canister)?;
    let ic = replica::Replica::new(server, &wasm);
    // The replica counts the system API calls that read its counter too: the empty workload's figure is
    // taken off every reading.
    let (reads, _) = ic.run(5, [0; 6])?;
    let empty = m.run(5, [0; 6])?;
    println!("The counter's reads alone: replica {reads}, meter {} fuel.", empty.fuel);
    println!();
    let mut seen: Vec<(&str, Figure, u64)> = Vec::new();
    println!("| workload | fuel | bulk instructions | bulk bytes | replica | r alone |");
    println!("|---|---:|---:|---:|---:|---:|");
    for (name, sel, w) in fit_workloads() {
        let f = m.run(sel, w)?;
        let (x, out) = ic.run(sel, w)?;
        let x = x.saturating_sub(reads);
        if out != f.result || faulted(out) {
            return Err(format!("{name}: the replica's result {out:#x}, the meter's {:#x}", f.result));
        }
        let alone = (x as f64 - f.bulk_cost() as f64) / f.plain() as f64;
        println!("| {name} | {} | {} | {} | {x} | {alone:.4} |", f.fuel, f.bulk, f.bytes);
        seen.push((name, f, x));
    }
    // The largest relative error is convex in r: a ternary search finds its least.
    let err = |r: f64| seen.iter().map(|(_, f, x)| (f.at(r) / *x as f64 - 1.0).abs()).fold(0.0f64, f64::max);
    let (mut lo, mut hi) = (0.01f64, 100.0f64);
    for _ in 0..200 {
        let (a, b) = (lo + (hi - lo) / 3.0, hi - (hi - lo) / 3.0);
        if err(a) <= err(b) {
            hi = b;
        } else {
            lo = a;
        }
    }
    let r = (lo + hi) / 2.0;
    let band = err(r);
    println!();
    println!("r = {r:.4}, band ±{:.2}% (recorded: r = {R}, band {:.2}%)", band * 100.0, BAND * 100.0);
    for (name, f, x) in &seen {
        println!("  {name}: estimate {:.0}, replica {x}, {:+.2}%", f.at(r), (f.at(r) / *x as f64 - 1.0) * 100.0);
    }
    println!();
    println!(
        "Counts on the replica beside their estimates, key by key (the worst key against its bar; the rest of a \
         count as in the checks)."
    );
    println!();
    println!(
        "| composition | worst key | replica, measured | estimate, top of band | estimate over replica | count (replica + the rest) | its bar | Δ | keys above the bar |"
    );
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|");
    for row in rows() {
        let (est, _) = row_keys(&m, &row)?;
        let measured: Result<Vec<u64>, String> = if row.sweep > 0 {
            ic.run_reply(3, count_words(&row, 0)).and_then(|(_, out, reply)| {
                if faulted(out) {
                    Err(format!("{}: a count faulted on the replica ({out:#x})", row.name))
                } else {
                    Ok(reply_words(&reply))
                }
            })
        } else {
            row.positions()
                .into_iter()
                .map(|pos| {
                    let (x, out) = ic.run(3, count_words(&row, pos))?;
                    if faulted(out) {
                        return Err(format!("{}: the count faulted on the replica ({out:#x})", row.name));
                    }
                    Ok(x.saturating_sub(reads))
                })
                .collect()
        };
        let measured = match measured {
            Ok(v) if v.len() == est.len() => v,
            Ok(v) => return Err(format!("{}: {} figures on the replica for {} keys", row.name, v.len(), est.len())),
            Err(e) => {
                let why = if e.contains("InstructionLimitExceeded") {
                    "its build exceeds one message's instruction limit"
                } else {
                    e.lines().next().unwrap_or("")
                };
                println!("| {} | – | not run: {why} | | | | | | |", row.name);
                continue;
            }
        };
        let on_replica: Vec<Keyed> =
            est.iter().zip(&measured).map(|(k, &x)| Keyed { pos: k.pos, value: k.value, count: x }).collect();
        let v = verdict(&row, &on_replica).ok_or("a row without keys")?;
        let e = est.iter().find(|k| k.pos == v.worst.pos).map_or(0, |k| k.count);
        println!(
            "| {} | {}, {} | {} | {e} | {:+.2}% | {} | {} | {} | {} of {} |",
            row.name,
            v.worst.pos,
            v.worst.value,
            v.worst.count,
            (e as f64 / v.worst.count as f64 - 1.0) * 100.0,
            v.total,
            row.bar,
            delta(v.total, row.bar),
            v.above,
            on_replica.len()
        );
    }
    Ok(true)
}

/// The fuel one call of the probe's `nt_count_bar` consumes.
fn fuel(engine: &Engine, module: &Module, args: (u32, u32, u64)) -> Result<(u64, u64), String> {
    let mut store = Store::new(engine, ());
    let start = u64::MAX / 4;
    store.set_fuel(start).map_err(|e| e.to_string())?;
    let inst = Instance::new(&mut store, module, &[]).map_err(|e| e.to_string())?;
    let f = inst.get_typed_func::<(u32, u32, u64), u64>(&mut store, "nt_count_bar").map_err(|e| e.to_string())?;
    let before = store.get_fuel().map_err(|e| e.to_string())?;
    let out = f.call(&mut store, args).map_err(|e| e.to_string())?;
    let after = store.get_fuel().map_err(|e| e.to_string())?;
    Ok((before - after, out))
}

/// The count's plain fuel by stage: 2 the run list and each run's image and header; 3 also the fences; 4 also
/// one descent of each main tree; 1 the whole count.
fn stages(probe: &str) -> Result<bool, String> {
    let mut cfg = Config::new();
    cfg.consume_fuel(true);
    cfg.wasm_memory64(true);
    let engine = Engine::new(&cfg).map_err(|e| e.to_string())?;
    let module = Module::from_file(&engine, probe).map_err(|e| format!("{probe}: {e}"))?;
    for loaded in [1u32, 1_024] {
        for key in KEYS {
            let mut line = format!("loaded {loaded} key {key}:");
            let base = fuel(&engine, &module, (0, loaded, key))?.0;
            for stage in [2u32, 3, 4, 1] {
                let f = fuel(&engine, &module, (stage, loaded, key))?.0;
                line += &format!(" stage {stage}: {}", f.saturating_sub(base));
            }
            println!("{line}");
        }
    }
    Ok(true)
}

/// OQL's lone-insert bars at 10^5 rows, the lower compiler, with 0 to 3 indexes.
const W_LONE: [(u64, &str); 4] = [
    (17_162, "core Map add, op16 (the IndexedMap path, 31,003; page-free 11,014)"),
    (42_099, "IndexedMap, 1 index on a 1,000-value field"),
    (70_197, "IndexedMap, 2 indexes"),
    (79_800, "IndexedMap, 3 indexes"),
];

/// A maintenance unit's bound on its own instructions, the crate's `UNIT_WORK`: every unit of an index build
/// must stay within it on PocketIC as well.
const UNIT_WORK: u64 = 96_000;

/// The write costs on PocketIC; false when a unit of an index build passed `UNIT_WORK`.
fn writes(canister: &str, server: PathBuf) -> Result<bool, String> {
    let wasm = read(canister)?;
    let ic = replica::Replica::new(server, &wasm);
    let (reads, _) = ic.run(5, [0; 6])?;
    // The primary index's inserts: (name, entries before, key order); per insert = (window − the keys' own) /
    // inserts.
    println!(
        "The primary index's inserts at 10^5 rows, on PocketIC: 1,000 inserts, less the harness's own key making,"
    );
    println!(
        "each as a table write makes it: a path made for the write, the lookup, then the insert at the position it"
    );
    println!("found.");
    println!();
    println!("| tree | entries before | keys | per insert | most one insert took |");
    println!("|---|---:|---|---:|---:|");
    for (keys, order) in [("rising", 1u64), ("random, distinct", 0)] {
        let w = |what: u64| [16_256, order, 1_000, what, 7, 0];
        let (x0, r0) = ic.run(8, w(0))?;
        let (x4, _) = ic.run(8, w(4))?;
        let (_, most) = ic.run(8, w(1))?;
        if faulted(r0) || faulted(most) {
            return Err(format!("the primary index's inserts, {keys} keys: faulted ({r0:#x}, {most:#x})"));
        }
        let per = (x0.saturating_sub(x4)) as f64 / 1_000.0;
        println!("| the primary index's (8, 0), lookup and insert | 16256 | {keys} | {per:.0} | {most} |");
    }
    // The secondary index's operations: (operation, its workload code) by key class; per operation = (window −
    // the keys' own) / operations.
    println!();
    println!("The secondary index (one tree, a count entry per key) at 10^5 rows, keys of 8 bytes, on PocketIC:");
    println!("1,000 operations, less the harness's own key making. Inserts take the next rows (rising rids);");
    println!("deletes and counts take rows spread over the 10^5. A count here is the tree's: the uploaded segments'");
    println!("part is the count of the runs, as in the count rows.");
    println!();
    println!("| operation | keys | per operation | most one operation took |");
    println!("|---|---|---:|---:|");
    let sec_ops: [(&str, u64); 3] = [("insert", 0), ("delete", 1), ("count", 2)];
    let sec_keys: [(&str, u64); 3] = [("10^5 values", 100_000), ("10^3 values", 1_000), ("3 values", 3)];
    for (oname, op) in sec_ops {
        for (kname, values) in sec_keys {
            let w = |what: u64| [op, 100_000, values, 1_000, what, 7];
            let (x0, r0) = ic.run(12, w(0))?;
            let (x4, _) = ic.run(12, w(4))?;
            let (_, most) = ic.run(12, w(1))?;
            let (_, sum) = ic.run(12, w(2))?;
            // Every insert and delete takes its row; every counted key holds at least the row it was taken from.
            let right = if op == 2 { sum >= 1_000 } else { sum == 1_000 };
            if faulted(r0) || faulted(most) || !right {
                return Err(format!("the secondary index's {oname}, {kname}: faulted or wrong ({r0:#x}, {sum})"));
            }
            let per = x0.saturating_sub(x4) as f64 / 1_000.0;
            println!("| {oname} | {kname} | {per:.0} | {most} |");
        }
    }
    // A table's insert, update and delete with 0 to 3 secondary indexes: per operation = (window − the counter's
    // reads) / operations.
    println!();
    println!(
        "A table's writes with 0 to 3 secondary indexes at 10^5 rows, on PocketIC: 1,000 operations, the rows inserted"
    );
    println!("first sealed. Rows {{key, amount, kind, ts}}, Nat each; the indexes, W 16, in order: amount (10^3 values), kind");
    println!("(3 values), ts (nearly every key one row). Inserts take the next keys; updates give new amount, kind");
    println!("and ts values to the last 1,000 rows, which the open group still buffers (buffered), or to rows spread");
    println!("over the first 5 x 10^4, sealed (sealed); deletes take rows spread over the 10^5, sealed.");
    println!(
        "None includes the maintenance its write leaves. OQL's bars are its lone inserts (no update or delete bar)."
    );
    println!();
    println!(
        "| operation | indexes | per operation | Δ to one index fewer | most one operation took | OQL's bar | Δ |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|");
    // The inserts' instructions by indexes, for the index builds below.
    let mut inserts = [0.0f64; 4];
    for (oname, op) in [("insert", 0u64), ("update, buffered", 2), ("update, sealed", 3), ("delete", 1)] {
        let mut fewer = 0.0f64;
        for k in 0..4u64 {
            let w = |what: u64| [op, 100_000, k, 1_000, what, 7];
            let (x0, sum) = ic.run(13, w(0))?;
            let (_, most) = ic.run(13, w(1))?;
            let (_, rows) = ic.run(13, w(2))?;
            // Every insert takes its row, every update replaces one and every delete removes one.
            let want = match op {
                0 => 101_000,
                2 | 3 => 100_000,
                _ => 99_000,
            };
            if faulted(sum) || faulted(most) || sum != 1_000 || rows != want {
                return Err(format!("the table's {oname} with {k} indexes: faulted or wrong ({sum:#x}, {rows})"));
            }
            let per = x0.saturating_sub(reads) as f64 / 1_000.0;
            let step = if k == 0 { String::new() } else { format!("{:+.0}", per - fewer) };
            let (bar, delta) = if op == 0 {
                let b = W_LONE[k as usize].0;
                (b.to_string(), format!("{:+.1}%", (per - b as f64) / b as f64 * 100.0))
            } else {
                (String::new(), String::new())
            };
            println!("| {oname} | {k} | {per:.0} | {step} | {most} | {bar} | {delta} |");
            if op == 0 {
                inserts[k as usize] = per;
            }
            fewer = per;
        }
    }
    // An index a schema change adds, built over the rows in maintenance units: per row = (window − the
    // counter's reads) / rows, beside one insert's share for that index (an insert with it less one without).
    println!();
    println!("An index a schema change adds to a table of 10^5 sealed rows (as above), built in maintenance units");
    println!("metered by the replica's counter, on PocketIC: per row = the build's instructions / 10^5, units and");
    println!("the most one unit took; beside it, one insert's share for that index (the insert rows above: with");
    println!("the index less without it).");
    println!();
    println!("| index added | indexes before | units | most one unit took | per row | one insert's share | Δ |");
    println!("|---|---:|---:|---:|---:|---:|---:|");
    let mut within = true;
    for (k, name) in [(0u64, "amount (10^3 values)"), (1, "kind (3 values)"), (2, "ts (nearly every key one row)")] {
        let w = |what: u64| [4, 100_000, k, 0, what, 7];
        let (x0, units) = ic.run(13, w(0))?;
        let (_, most) = ic.run(13, w(1))?;
        let (_, rows) = ic.run(13, w(2))?;
        if faulted(units) || faulted(most) || rows != 100_000 {
            return Err(format!("the build of index {name}: faulted or wrong ({units:#x}, {rows:#x})"));
        }
        let per = x0.saturating_sub(reads) as f64 / 100_000.0;
        let share = inserts[k as usize + 1] - inserts[k as usize];
        let delta = format!("{:+.1}%", (per - share) / share * 100.0);
        println!("| {name} | {k} | {units} | {most} | {per:.0} | {share:.0} | {delta} |");
        if most > UNIT_WORK {
            eprintln!("a unit of the build of index {name} took {most} instructions, above {UNIT_WORK}");
            within = false;
        }
    }
    Ok(within)
}

/// Filtered scans on PocketIC, per row of a table of 10^5 rows, beside the scan that reads every row whole.
fn scans(canister: &str, server: PathBuf) -> Result<bool, String> {
    let wasm = read(canister)?;
    let ic = replica::Replica::new(server, &wasm);
    let (reads, _) = ic.run(5, [0; 6])?;
    let rows = 100_000u64;
    let run = |what: u64| -> Result<(u64, u64), String> {
        let (x, out) = ic.run(14, [rows, what, 64, 0, 0, 7])?;
        if faulted(out) {
            return Err(format!("filtered scan workload {what}: faulted ({out:#x})"));
        }
        Ok((x.saturating_sub(reads), out))
    };
    let (whole, d_whole) = run(4)?;
    let (every, d_every) = run(3)?;
    let (eq, n_eq) = run(0)?;
    let (_, n_eq_rows) = run(5)?;
    let (band, n_band) = run(1)?;
    let (all, n_all) = run(2)?;
    if d_every != d_whole || n_eq != n_eq_rows || n_band != rows / 100 * 6 || n_all != rows {
        return Err(format!(
            "wrong answers: digests {d_every:#x} and {d_whole:#x}, counts {n_eq} ({n_eq_rows}), {n_band}, {n_all}"
        ));
    }
    let per = |x: u64| x as f64 / rows as f64;
    let base = per(whole);
    println!("Filtered scans of 10^5 rows {{key, amount, cat, ts}}, Nat each (cat among 33 values, ts the key), on");
    println!("PocketIC: rows inserted in key order and every frozen group sealed, so 98,304 rows sealed in 24 groups");
    println!("and 1,696 buffered. Instructions per row of the table, the call's own fixed part included; Δ against");
    println!("the scan that reads every row whole by rid, as the runtime's scan does.");
    println!();
    println!("| answer | filter | rows matching | per row | Δ |");
    println!("|---|---|---:|---:|---:|");
    println!("| every row read whole (`get_rid` by rid) | – | {rows} | {base:.0} | – |");
    for (name, filter, n, x) in [
        ("rows (`scan_where`, 64 a call)", "ts ≥ 0", rows, every),
        ("count (`count_where`)", "cat = 7", n_eq, eq),
        ("count (`count_where`)", "ts in [47,000, 53,000)", n_band, band),
        ("count (`count_where`)", "ts ≥ 0", n_all, all),
    ] {
        let p = per(x);
        println!("| {name} | {filter} | {n} | {p:.0} | {:+.0} ({:+.1}%) |", p - base, (p - base) / base * 100.0);
    }
    Ok(true)
}

/// An index's group counts on PocketIC, over a table of 10^5 rows, beside a count of each of its keys.
fn groups(canister: &str, server: PathBuf) -> Result<bool, String> {
    let wasm = read(canister)?;
    let ic = replica::Replica::new(server, &wasm);
    let (reads, _) = ic.run(5, [0; 6])?;
    let rows = 100_000u64;
    // (index, its keys, groups a call: 0 a count of each key).
    let run = |index: u64, keys: u64, per: u64| -> Result<u64, String> {
        let what = u64::from(per == 0);
        let per = if per == 0 { keys } else { per };
        let (x, out) = ic.run(15, [rows, index, what, per, 0, 7])?;
        if faulted(out) || out != keys {
            return Err(format!("group count workload ({index}, {per}, {what}): {out:#x}, {keys} groups wanted"));
        }
        Ok(x.saturating_sub(reads))
    };
    println!("Group counts of an index over 10^5 rows {{key, amount, kind, ts}}, Nat each, on PocketIC: rows inserted");
    println!("in key order and every frozen group sealed. Instructions for every group of the index, the calls'");
    println!("own fixed parts included; Δ against a count of each key, one call a key.");
    println!();
    println!("| index | keys | answer | calls | instructions | per group | Δ |");
    println!("|---|---:|---|---:|---:|---:|---:|");
    for (name, index, keys) in [("amount", 1u64, 1_000u64), ("kind", 2, 3)] {
        let each = run(index, keys, 0)?;
        println!("| {name} | {keys} | a count of each key (`count`) | {keys} | {each} | {} | – |", each / keys);
        for per in [u64::MAX, 64] {
            let x = run(index, keys, per)?;
            let calls = keys / per + 1;
            let d = x as f64 - each as f64;
            println!(
                "| {name} | {keys} | `group_counts`, {} a call | {calls} | {x} | {} | {d:+.0} ({:+.1}%) |",
                if per == u64::MAX { "all".to_string() } else { per.to_string() },
                x / keys,
                d / each as f64 * 100.0
            );
        }
    }
    Ok(true)
}

/// Cell scans on PocketIC, per row of a table of 10^5 rows, beside the scan that reads every row whole by rid
/// and the filtered scan's whole rows.
fn cells(canister: &str, server: PathBuf) -> Result<bool, String> {
    let wasm = read(canister)?;
    let ic = replica::Replica::new(server, &wasm);
    let (reads, _) = ic.run(5, [0; 6])?;
    let rows = 100_000u64;
    let run = |what: u64| -> Result<(u64, u64), String> {
        let (x, out) = ic.run(14, [rows, what, 64, 0, 0, 7])?;
        if faulted(out) {
            return Err(format!("scan workload {what}: faulted ({out:#x})"));
        }
        Ok((x.saturating_sub(reads), out))
    };
    let (whole, d_whole) = run(4)?;
    let (rows_where, d_where) = run(3)?;
    let (one, d_one) = run(6)?;
    let (_, d_one_whole) = run(9)?;
    let (alone, d_alone) = run(10)?;
    let (_, d_alone_whole) = run(11)?;
    let (four, d_four) = run(7)?;
    let (four_where, d_four_where) = run(8)?;
    if d_where != d_whole
        || d_four != d_whole
        || d_four_where != d_whole
        || d_one != d_one_whole
        || d_alone != d_alone_whole
    {
        return Err(format!(
            "wrong answers: digests {d_where:#x}, {d_four:#x}, {d_four_where:#x} against {d_whole:#x}, \
             {d_one:#x} against {d_one_whole:#x}, {d_alone:#x} against {d_alone_whole:#x}"
        ));
    }
    let per = |x: u64| x as f64 / rows as f64;
    let (base, base_where) = (per(whole), per(rows_where));
    let delta = |p: f64, b: f64| format!("{:+.0} ({:+.1}%)", p - b, (p - b) / b * 100.0);
    println!("Cell scans of 10^5 rows {{key, amount, cat, ts}}, Nat each, on PocketIC: rows inserted in key order and");
    println!("every frozen group sealed, so 98,304 rows sealed in 24 groups and 1,696 buffered. Instructions per row,");
    println!("the calls' own fixed parts included, 64 rows a call; Δ against the scan that reads every row whole by");
    println!("rid, as the runtime's scan does, and against the filtered scan's whole rows.");
    println!();
    println!("| answer | filter | per row | Δ vs whole rows by rid | Δ vs `scan_where` rows |");
    println!("|---|---|---:|---:|---:|");
    println!("| every row read whole (`get_rid` by rid) | – | {base:.0} | – | {} |", delta(base, base_where));
    println!("| whole rows (`scan_where`) | ts ≥ 0 | {base_where:.0} | {} | – |", delta(base_where, base));
    for (name, filter, x) in [
        ("amount (`scan_cells`)", "–", alone),
        ("key and amount (`scan_cells`)", "–", one),
        ("key, amount, cat, ts (`scan_cells`)", "–", four),
        ("key, amount, cat, ts (`scan_cells`)", "ts ≥ 0", four_where),
    ] {
        let p = per(x);
        println!("| {name} | {filter} | {p:.0} | {} | {} |", delta(p, base), delta(p, base_where));
    }
    Ok(true)
}

/// An integer column's minimum and maximum, and the table's open alone, on PocketIC, one call each.
fn extremes(canister: &str, server: PathBuf) -> Result<bool, String> {
    let wasm = read(canister)?;
    let ic = replica::Replica::new(server, &wasm);
    let (reads, _) = ic.run(5, [0; 6])?;
    let rows = 100_000u64;
    let run = |what: u64, want: u64| -> Result<u64, String> {
        let (x, out) = ic.run(16, [rows, what, 0, 0, 0, 7])?;
        if out != want {
            return Err(format!("extremes workload {what}: {out:#x}, {want} wanted"));
        }
        Ok(x.saturating_sub(reads))
    };
    println!("An integer column's minimum and maximum over 10^5 rows {{key, amount, cat, ts}}, Nat each (amount among");
    println!("10^3 values, ts the key), on PocketIC: rows inserted in key order and every frozen group sealed, so 24");
    println!("groups sealed and one buffered. Instructions for one call.");
    println!();
    println!("| call | answer | instructions |");
    println!("|---|---:|---:|");
    for (name, what, want) in [
        ("minimum of amount (`extreme`)", 0u64, 0u64),
        ("maximum of amount (`extreme`)", 1, 999),
        ("minimum of ts (`extreme`)", 2, 0),
        ("maximum of ts (`extreme`)", 3, rows - 1),
        ("the table's open, its schema page in place (`Table::open`)", 4, 4),
    ] {
        println!("| {name} | {want} | {} |", run(what, want)?);
    }
    Ok(true)
}
