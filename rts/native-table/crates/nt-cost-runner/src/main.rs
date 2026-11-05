//! Runs the `nt-cost` module under wasmtime and meters every operation three ways: the transcription of
//! today's `PBTreeMap` (`ref`), the crate's typed tree unchecked (`unc`) and checked (`chk`). Writes
//! `results.json` and `results.md` into the output directory and applies the gate: get and insert with room
//! must cost at or below what `ref` costs (the mean at most 0.05 fuel above, no single sample more than 1
//! above); every other row, at most max(4 fuel, 0.5%) more than
//! `ref` on average. Exits with status 1 when a row fails.
//!
//! Usage: `nt-cost-runner <nt_cost.wasm> <out dir> [samples]` (200 by default). The environment variable
//! `NT_COST_BUILD` names how the module was built; `NT_COST_SHAPES` (a comma-separated list of indices into
//! the size table) restricts the run, for development only; `NT_COST_DEBUG` prints, for the rows that must
//! be exact, every sample whose fuel differs, with where its key lies.

use std::fmt::Write as _;
use wasmtime::{Config, Engine, Instance, Module, Result, Store, TypedFunc, WasmParams};

/// The wasmtime this runner links, which the `build` field records.
const WASMTIME: &str = "42.0.1";
/// How the runner configures and reads fuel, recorded in every result.
const FUEL: &str = "wasmtime::Config::new() with wasm_memory64(true) and consume_fuel(true), every other \
                    setting default (Cranelift, OptLevel::Speed, default per-operator fuel costs); the fuel of \
                    a call is Store::get_fuel() before it minus after it, the store refilled to 2^60 whenever \
                    it falls below 2^50";
/// Samples per row, unless the command line says otherwise (development only).
const SAMPLES: u64 = 200;

// Categories of the module's `plan`.
const GET: u32 = 0;
const INS_ROOM: u32 = 1;
const INS_SPLIT: u32 = 2;
const REM_LEAF: u32 = 3;
const REM_MERGE_LEAF: u32 = 4;
const REM_ROTL_LEAF: u32 = 5;
const REM_ROTR_LEAF: u32 = 6;
const REM_MERGE_INNER: u32 = 7;
const REM_ROTL_INNER: u32 = 8;
const REM_ROTR_INNER: u32 = 9;
const ENTRY: u32 = 10;
const PRESENT: u32 = 11;
const ABSENT: u32 = 12;
const NONE: u64 = u64::MAX;

/// Iterator steps metered after each seek in the `next` rows.
const STEPS: u64 = 256;
/// Entries a bounded range covers on average.
const SPAN: u64 = 1_000;

/// The sizes measured: (export prefix, shape, shape name, entries, whether the iteration rows run).
const SIZES: [(&str, u32, &str, u64, bool); 6] = [
    ("a", 0, "<u64, u64>", 1_000, false),
    ("a", 0, "<u64, u64>", 100_000, true),
    ("a", 0, "<u64, u64>", 1_000_000, true),
    ("b", 1, "<u64, [u8; 1024]>", 10_000, false),
    ("c", 2, "<PString, u64>", 100_000, false),
    ("d", 3, "<[u8; 1013], u64>", 100_000, false),
];

struct Way {
    get: TypedFunc<u64, u64>,
    ins: TypedFunc<(u64, u64), u64>,
    rem: TypedFunc<u64, u64>,
    ent: TypedFunc<(u64, u64), u64>,
    seek: TypedFunc<(u32, u64, u32, u64), u64>,
    open: TypedFunc<(u32, u64, u32, u64), u64>,
    next: TypedFunc<(), u64>,
    scan: TypedFunc<(), u64>,
    span: TypedFunc<(u64, u64), u64>,
}

struct Ctl {
    setup: TypedFunc<(u32, u64, u64), u64>,
    plan: TypedFunc<u32, u64>,
    prep: TypedFunc<(u64, u64, u32), ()>,
    check: TypedFunc<(), u64>,
    info: TypedFunc<u32, u64>,
    removal_kind: TypedFunc<u64, u64>,
    insert_splits: TypedFunc<u64, u64>,
    position: TypedFunc<u64, u64>,
    nop: TypedFunc<u64, u64>,
}

#[derive(Clone, Copy, Default)]
struct Stat {
    samples: u64,
    elements: u64,
    total: u64,
    min: u64,
    max: u64,
}

impl Stat {
    fn add(&mut self, fuel: u64, elements: u64) {
        if self.samples == 0 || fuel < self.min {
            self.min = fuel;
        }
        self.max = self.max.max(fuel);
        self.samples += 1;
        self.elements += elements;
        self.total += fuel;
    }
    /// Mean fuel per call, or per element for the iteration rows.
    fn mean(&self) -> f64 {
        self.total as f64 / self.elements.max(1) as f64
    }
    fn json(&self) -> String {
        format!(
            "{{\"samples\": {}, \"elements\": {}, \"total\": {}, \"mean\": {:.3}, \"min\": {}, \"max\": {}}}",
            self.samples,
            self.elements,
            self.total,
            self.mean(),
            self.min,
            self.max
        )
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Gate {
    /// Equal to `ref` on every sample.
    Exact,
    /// At most max(4 fuel, 0.5%) above `ref` on average.
    Bound,
}

struct Row {
    shape: &'static str,
    n: u64,
    depth: u64,
    op: &'static str,
    gate: Gate,
    per_element: bool,
    ways: [Stat; 3],
    /// Samples whose fuel equals `ref`'s exactly: unchecked, checked.
    equal: [u64; 2],
    /// The largest amount by which one unchecked sample costs more than `ref`'s (0 when none does).
    max_over: u64,
    /// Why the row has no samples.
    absent: Option<&'static str>,
}

impl Row {
    fn limit(&self) -> f64 {
        match self.gate {
            Gate::Exact => 0.0,
            Gate::Bound => f64::max(4.0, 0.005 * self.ways[0].mean()),
        }
    }
    fn delta(&self, w: usize) -> f64 {
        self.ways[w].mean() - self.ways[0].mean()
    }
    /// None when the row has no samples.
    fn pass(&self) -> Option<bool> {
        if self.absent.is_some() {
            return None;
        }
        Some(match self.gate {
            // At or below `ref`: the mean at most 0.05 fuel above, and no single sample more than 1 above.
            Gate::Exact => self.delta(1) <= 0.05 + 1e-9 && self.max_over <= 1,
            Gate::Bound => self.delta(1) <= self.limit() + 1e-9,
        })
    }
}

struct Runner {
    store: Store<()>,
    ctl: Ctl,
    ways: [Way; 3],
    /// The last planned key and where it lies, for `NT_COST_DEBUG`.
    last: String,
}

impl Runner {
    fn fuel<P: WasmParams, R: wasmtime::WasmResults>(&mut self, f: &TypedFunc<P, R>, p: P) -> Result<(u64, R)> {
        if self.store.get_fuel()? < 1 << 50 {
            self.store.set_fuel(1 << 60)?;
        }
        let a = self.store.get_fuel()?;
        let r = f.call(&mut self.store, p)?;
        let b = self.store.get_fuel()?;
        Ok((a - b, r))
    }

    fn call<P: WasmParams, R: wasmtime::WasmResults>(&mut self, f: &TypedFunc<P, R>, p: P) -> Result<R> {
        Ok(self.fuel(f, p)?.1)
    }

    fn plan(&mut self, cat: u32) -> Result<u64> {
        let f = self.ctl.plan.clone();
        let x = self.call(&f, cat)?;
        if std::env::var_os("NT_COST_DEBUG").is_some() && x != NONE {
            let f = self.ctl.position.clone();
            let p = self.call(&f, x)?;
            self.last =
                format!("key {x:#x}: depth {}, index {}, count {}", p >> 48, (p >> 16) & 0xFFFF_FFFF, p & 0xFFFF);
        }
        Ok(x)
    }

    fn prep(&mut self, x: u64, e: u64, owned: bool) -> Result<()> {
        let f = self.ctl.prep.clone();
        self.call(&f, (x, e, u32::from(owned)))
    }

    fn checked(&mut self, what: &str) -> Result<()> {
        let f = self.ctl.check.clone();
        if self.call(&f, ())? != 1 {
            wasmtime::bail!("{what}: the three trees differ");
        }
        Ok(())
    }

    /// Meters one call per way of the function `pick` selects; the answers must agree. `elements` turns an
    /// answer into the number of entries the call visited (1 for single operations).
    fn three<P: WasmParams + Copy>(
        &mut self,
        pick: impl Fn(&Way) -> TypedFunc<P, u64>,
        p: P,
        elements: impl Fn(u64) -> u64,
        row: &mut Row,
        what: &str,
    ) -> Result<()> {
        let mut fuel = [0u64; 3];
        let mut out = [0u64; 3];
        for w in 0..3 {
            let f = pick(&self.ways[w]);
            let (u, r) = self.fuel(&f, p)?;
            fuel[w] = u;
            out[w] = r;
            row.ways[w].add(u, elements(r));
        }
        if out[0] != out[1] || out[0] != out[2] {
            wasmtime::bail!("{what}: answers differ: {out:?}");
        }
        for w in 0..2 {
            if fuel[w + 1] == fuel[0] {
                row.equal[w] += 1;
            }
        }
        row.max_over = row.max_over.max(fuel[1].saturating_sub(fuel[0]));
        if fuel[1] != fuel[0] && row.gate == Gate::Exact && std::env::var_os("NT_COST_DEBUG").is_some() {
            eprintln!(
                "{what}: sample {} differs: ref {} unc {} ({})",
                row.ways[0].samples, fuel[0], fuel[1], self.last
            );
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        wasmtime::bail!("usage: nt-cost-runner <nt_cost.wasm> <out dir> [samples]");
    }
    let samples: u64 = args.get(3).map(|s| s.parse()).transpose()?.unwrap_or(SAMPLES);
    let build = std::env::var("NT_COST_BUILD").unwrap_or_default();

    let mut config = Config::new();
    config.wasm_memory64(true);
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let module = Module::from_file(&engine, &args[1])?;
    let mut store = Store::new(&engine, ());
    store.set_fuel(1 << 60)?;
    let inst = Instance::new(&mut store, &module, &[])?;

    let mk_ctl = |store: &mut Store<()>| -> Result<Ctl> {
        Ok(Ctl {
            setup: inst.get_typed_func(&mut *store, "setup")?,
            plan: inst.get_typed_func(&mut *store, "plan")?,
            prep: inst.get_typed_func(&mut *store, "prep")?,
            check: inst.get_typed_func(&mut *store, "check")?,
            info: inst.get_typed_func(&mut *store, "info")?,
            removal_kind: inst.get_typed_func(&mut *store, "removal_kind")?,
            insert_splits: inst.get_typed_func(&mut *store, "insert_splits")?,
            position: inst.get_typed_func(&mut *store, "position")?,
            nop: inst.get_typed_func(&mut *store, "nop")?,
        })
    };
    let mk_way = |store: &mut Store<()>, prefix: &str, w: &str| -> Result<Way> {
        let f = |name: &str| format!("{prefix}_{name}_{w}");
        Ok(Way {
            get: inst.get_typed_func(&mut *store, &f("get"))?,
            ins: inst.get_typed_func(&mut *store, &f("ins"))?,
            rem: inst.get_typed_func(&mut *store, &f("rem"))?,
            ent: inst.get_typed_func(&mut *store, &f("ent"))?,
            seek: inst.get_typed_func(&mut *store, &f("seek"))?,
            open: inst.get_typed_func(&mut *store, &f("open"))?,
            next: inst.get_typed_func(&mut *store, &f("next"))?,
            scan: inst.get_typed_func(&mut *store, &f("scan"))?,
            span: inst.get_typed_func(&mut *store, &f("span"))?,
        })
    };

    let pick: Vec<usize> = match std::env::var("NT_COST_SHAPES") {
        Ok(v) => v.split(',').map(|x| x.trim().parse()).collect::<std::result::Result<_, _>>()?,
        Err(_) => (0..SIZES.len()).collect(),
    };
    let mut rows: Vec<Row> = Vec::new();
    let mut nop_fuel = 0;
    for (prefix, shape, shape_name, n, iterate) in pick.iter().map(|&i| SIZES[i]) {
        let ways = [
            mk_way(&mut store, prefix, "ref")?,
            mk_way(&mut store, prefix, "unc")?,
            mk_way(&mut store, prefix, "chk")?,
        ];
        let ctl = mk_ctl(&mut store)?;
        let mut r = Runner { store, ctl, ways, last: String::new() };
        eprintln!("== {shape_name} at {n}");
        let setup = r.ctl.setup.clone();
        if r.call(&setup, (shape, n, 0x5EED_0000 + n))? != 1 {
            wasmtime::bail!("setup: the three trees differ");
        }
        let info = r.ctl.info.clone();
        let depth = r.call(&info, 1)?;
        let nop = r.ctl.nop.clone();
        nop_fuel = r.fuel(&nop, 7)?.0;
        let new_row = |op: &'static str, gate: Gate, per_element: bool| Row {
            shape: shape_name,
            n,
            depth,
            op,
            gate,
            per_element,
            ways: [Stat::default(); 3],
            equal: [0; 2],
            max_over: 0,
            absent: None,
        };
        let one = |_: u64| 1u64;

        // get
        let mut row = new_row("get (hit)", Gate::Exact, false);
        for _ in 0..samples {
            let x = r.plan(GET)?;
            r.prep(x, 0, false)?;
            r.three(|w| w.get.clone(), x, one, &mut row, "get")?;
        }
        rows.push(row);

        // inserts, each checked against the reference tree before it runs
        let inserts: [(&'static str, u32, u64, Gate); 3] = [
            ("insert, room", INS_ROOM, 0, Gate::Exact),
            ("insert, split", INS_SPLIT, 1, Gate::Bound),
            ("entry, vacant (one search, insert along the saved path)", ENTRY, 0, Gate::Bound),
        ];
        for (op, cat, splits, gate) in inserts {
            let mut row = new_row(op, gate, false);
            for _ in 0..samples {
                let x = r.plan(cat)?;
                let f = r.ctl.insert_splits.clone();
                if r.call(&f, x)? != splits {
                    wasmtime::bail!("{op}: the planned insert of {x} is of another kind");
                }
                if cat == ENTRY {
                    r.prep(x, 0, false)?;
                    r.three(|w| w.ent.clone(), (x, x), one, &mut row, op)?;
                } else {
                    r.prep(x, 0, true)?;
                    r.three(|w| w.ins.clone(), (x, x), one, &mut row, op)?;
                }
            }
            r.checked(op)?;
            rows.push(row);
        }

        // removals, each kind checked against the reference tree before it runs
        let removals: [(&'static str, u32, u64); 7] = [
            ("remove, leaf", REM_LEAF, 0),
            ("remove, merge at the leaf level", REM_MERGE_LEAF, 0o1),
            ("remove, rotation from the left at the leaf level", REM_ROTL_LEAF, 0o2),
            ("remove, rotation from the right at the leaf level", REM_ROTR_LEAF, 0o3),
            ("remove, leaf merge, then merge at an internal level", REM_MERGE_INNER, 0o11),
            ("remove, leaf merge, then rotation from the left at an internal level", REM_ROTL_INNER, 0o21),
            ("remove, leaf merge, then rotation from the right at an internal level", REM_ROTR_INNER, 0o31),
        ];
        for (op, cat, want) in removals {
            let mut row = new_row(op, Gate::Bound, false);
            for _ in 0..samples {
                let x = r.plan(cat)?;
                if x == NONE {
                    row.absent = Some("no internal level below the root");
                    break;
                }
                let f = r.ctl.removal_kind.clone();
                let kind = r.call(&f, x)?;
                if kind != want {
                    wasmtime::bail!("{op}: the planned removal of {x} is of kind {kind:o}");
                }
                r.prep(x, 0, false)?;
                r.three(|w| w.rem.clone(), x, one, &mut row, op)?;
            }
            r.checked(op)?;
            rows.push(row);
        }

        // seek with its first step, each start bound; the end unbounded
        for (op, mode) in [
            ("seek and first step, start unbounded", 0u32),
            ("seek and first step, start included", 1),
            ("seek and first step, start excluded", 2),
        ] {
            let mut row = new_row(op, Gate::Bound, false);
            for i in 0..samples {
                let x = r.plan(if i % 2 == 0 { PRESENT } else { ABSENT })?;
                r.prep(x, 0, false)?;
                r.three(|w| w.seek.clone(), (mode, x, 0u32, 0u64), one, &mut row, op)?;
            }
            rows.push(row);
        }

        // next, each end bound, from an included start; the end lies beyond every step
        for (op, end) in [("next, end unbounded", 0u32), ("next, end included", 1), ("next, end excluded", 2)] {
            let mut row = new_row(op, Gate::Bound, false);
            for _ in 0..samples.div_ceil(10) {
                let x = r.plan(PRESENT)?;
                r.prep(x, u64::MAX, false)?;
                for w in 0..3 {
                    let f = r.ways[w].open.clone();
                    r.call(&f, (1, x, end, u64::MAX))?;
                }
                for _ in 0..STEPS {
                    r.three(|w| w.next.clone(), (), one, &mut row, op)?;
                }
            }
            rows.push(row);
        }

        // iteration per element: a full Iter loop, and a range loop over about SPAN entries
        if iterate {
            let mut row = new_row("iteration, full scan (per element)", Gate::Bound, true);
            r.prep(0, 0, false)?;
            for _ in 0..samples {
                r.three(|w| w.scan.clone(), (), |a| a >> 32, &mut row, "scan")?;
            }
            rows.push(row);
            let mut row = new_row("iteration, bounded range of about 1,000 (per element)", Gate::Bound, true);
            let width = ((SPAN as u128) << 63) / n as u128;
            for _ in 0..samples {
                let x = r.plan(PRESENT)?;
                let e = x + width as u64;
                r.prep(x, e, false)?;
                r.three(|w| w.span.clone(), (x, e), |a| a >> 32, &mut row, "span")?;
            }
            rows.push(row);
        }
        r.checked("end")?;
        store = r.store;
    }

    // Output.
    let failed: Vec<&Row> = rows.iter().filter(|r| r.pass() == Some(false)).collect();
    let out = std::path::Path::new(&args[2]);
    std::fs::create_dir_all(out)?;
    let esc = |s: &str| s.replace('"', "'");
    let mut json = String::new();
    writeln!(json, "{{")?;
    writeln!(
        json,
        "  \"build\": {{\"wasmtime\": \"{WASMTIME}\", \"fuel\": \"{}\", \"samples_per_row\": {samples}, \"module\": \"{}\"}},",
        esc(FUEL),
        esc(&build)
    )?;
    writeln!(json, "  \"call_baseline_fuel\": {nop_fuel},")?;
    writeln!(json, "  \"gate\": \"get and insert with room: unc at or below ref (mean at most +0.05, no sample above +1); every other row: unc mean at most max(4, 0.5% of ref's mean) above ref's\",")?;
    writeln!(json, "  \"passed\": {},", failed.is_empty())?;
    writeln!(json, "  \"rows\": [")?;
    for (i, row) in rows.iter().enumerate() {
        let comma = if i + 1 < rows.len() { "," } else { "" };
        let pass = match row.pass() {
            Some(p) => p.to_string(),
            None => "null".into(),
        };
        writeln!(
            json,
            "    {{\"shape\": \"{}\", \"n\": {}, \"depth\": {}, \"op\": \"{}\", \"per_element\": {}, \"gate\": \"{}\", \"limit\": {:.3}, \"pass\": {pass}, \"absent\": {}, \"ref\": {}, \"unc\": {}, \"chk\": {}, \"delta_unc\": {:.3}, \"delta_chk\": {:.3}, \"unc_equal_samples\": {}, \"chk_equal_samples\": {}}}{comma}",
            row.shape,
            row.n,
            row.depth,
            row.op,
            row.per_element,
            if row.gate == Gate::Exact { "exact" } else { "bound" },
            row.limit(),
            row.absent.map(|a| format!("\"{a}\"")).unwrap_or_else(|| "null".into()),
            row.ways[0].json(),
            row.ways[1].json(),
            row.ways[2].json(),
            row.delta(1),
            row.delta(2),
            row.equal[0],
            row.equal[1]
        )?;
    }
    writeln!(json, "  ]\n}}")?;
    std::fs::write(out.join("results.json"), &json)?;

    let mut md = String::new();
    writeln!(md, "# cost_vs_ref: wasmtime fuel per operation\n")?;
    writeln!(md, "Harness: wasmtime {WASMTIME}; {FUEL}; {samples} samples per row.\n")?;
    writeln!(md, "Module: {build}.\n")?;
    writeln!(
        md,
        "Mean fuel per call over the samples, the export's own instructions included (an empty export costs \
         {nop_fuel}); the iteration rows give fuel per element. `ref` is the transcription of today's \
         PBTreeMap, `unc` the crate's typed tree with CHECKED false, `chk` with CHECKED true. Δ is against \
         `ref`; a negative Δ means cheaper. \"Equal\" counts the samples whose fuel equals `ref`'s exactly.\n"
    )?;
    writeln!(
        md,
        "Gate on `unc`: get and insert with room at or below `ref` (mean at most +0.05 fuel, no single sample more than 1 above); every other row at most \
         max(4 fuel, 0.5% of `ref`) above `ref` on average. Result: **{}**.\n",
        if failed.is_empty() { "pass".to_string() } else { format!("{} rows fail", failed.len()) }
    )?;
    writeln!(
        md,
        "| shape | n | depth | operation | samples | ref | unc | Δ unc | equal | limit | gate | chk | Δ chk |"
    )?;
    writeln!(md, "|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---|---:|---:|")?;
    for row in &rows {
        if let Some(why) = row.absent {
            writeln!(
                md,
                "| {} | {} | {} | {} | – | – | – | – | – | – | n/a: {why} | – | – |",
                row.shape, row.n, row.depth, row.op
            )?;
            continue;
        }
        let [rf, u, c] = row.ways;
        let d = |w: usize| {
            let delta = row.delta(w);
            format!("{:+.2} ({:+.2}%)", delta, 100.0 * delta / rf.mean())
        };
        let limit = match row.gate {
            Gate::Exact => "= on every sample".to_string(),
            Gate::Bound => format!("≤ +{:.2}", row.limit()),
        };
        writeln!(
            md,
            "| {} | {} | {} | {} | {} | {:.2} | {:.2} | {} | {}/{} | {} | {} | {:.2} | {} |",
            row.shape,
            row.n,
            row.depth,
            row.op,
            rf.samples,
            rf.mean(),
            u.mean(),
            d(1),
            row.equal[0],
            rf.samples,
            limit,
            if row.pass() == Some(true) { "pass" } else { "**FAIL**" },
            c.mean(),
            d(2)
        )?;
    }
    std::fs::write(out.join("results.md"), &md)?;
    print!("{md}");
    if !failed.is_empty() {
        eprintln!("cost_vs_ref: {} rows above the gate", failed.len());
        std::process::exit(1);
    }
    Ok(())
}
