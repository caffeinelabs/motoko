//! Meters the table's maintenance units by the replica's own cost model (memory64 on): the module is
//! rewritten so that it counts, in a global of its own, what the replica charges for the code it runs (its
//! per-basic-block cost of each instruction, as the IC's instrumentation assigns it for a 64-bit memory),
//! every `memory.copy` and `memory.fill` passing its length to the runner (`nt.bulk`), which adds it, as the
//! replica charges a bulk instruction's bytes. A reading of the host meter costs `READ_FEE` more, as the
//! replica's system call does.
//!
//! Each scenario of the `nt-cost` module's `units_*` exports is set up unmetered, then every unit runs as a
//! call of its own and is metered, with the scenario's writes between units (`units_between`) unmetered.
//! Twice: with the units' own instruction estimates only, and with the replica's count as their host meter (the
//! `env.nt_fuel_used` import, as `ic0.performance_counter` on the IC). Each unit's changed 4 KiB pages are
//! counted from the writes the module's host lends and its allocator's own blocks (the `env.nt_dirty`
//! import). A unit is charged its instructions plus `PAGE_FEE` per changed page. The bulk memory
//! instructions of the host's own work (a fresh page's fill, its vector's growth), which the module marks
//! with the `env.nt_host` import, are left out.
//!
//! Prints, per scenario, job and rewrite step, the units, the most and the mean instructions of one unit,
//! the most the units' own estimates gave, the most changed pages, the most charged and, with the meter, the
//! longest stretch between two of a unit's meter readings; then, per scenario, its first group's seal and the
//! drain after it per row of the group; writes the same tables to the output file when given. Fails when any
//! unit passes `UNIT_WORK` = 96,000 instructions, `UNIT_BLOCKS_MAX` = 17 changed pages or `CHARGED_MAX` =
//! 164,000 charged, or when a unit of the seal's pass 1 over values sharing 64 KiB prefixes goes more than
//! `GAP_MAX` = 6,000 instructions between two meter readings. The module is built with `fuel-meter`.
//!
//! Given the same module built with the crate's `meter-notes` too, the runner runs the metered pass once more
//! with it: there each of the crate's maintenance decisions tells the runner the look-ahead it took for the
//! work it lets begin and, when it checks the unit's pages, the pages it took that work to change
//! (`env.nt_note`); the runner fails when that work, up to the next decision, costs more than the look-ahead
//! (`NOTE_SLACK` more for the note's own call), when the work from a decision that lets work begin to the next
//! such decision, or to the unit's end, costs more than its look-ahead and the unit's end (`TAIL`), or when the
//! work a decision that checks pages lets begin, up to the next such decision or the unit's end, changes more
//! pages than it took. So a piece that could carry a unit past the bound fails the run wherever its unit's
//! pieces happened to lie. The notes cost a few percent, so the tables come from the first module.
//!
//! Usage: `units <nt_cost.wasm> [out file] [nt_cost.wasm with meter-notes]`. The test
//! `every_unit_within_the_bound` runs the same with the modules named by `NT_COST_WASM` and
//! `NT_COST_NOTES_WASM` (scripts/check.sh sets them). For development: `NT_UNITS_ONLY` runs a part (below),
//! and `NT_UNITS_GAPS` = n prints, for every stretch between two meter readings longer than n, the functions
//! on the stack at the reading that began it and at the one that ended it.

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use wasmtime::{Caller, Config, Engine, Linker, Module, Result, Store, TypedFunc};

/// The bound on one unit's own instructions, on its changed 4 KiB pages, and on what it is charged.
const UNIT_WORK: u64 = 96_000;
const UNIT_BLOCKS_MAX: u64 = 17;
const PAGE_FEE: u64 = 4_000;
const CHARGED_MAX: u64 = UNIT_WORK + UNIT_BLOCKS_MAX * PAGE_FEE;
/// The longest stretch between two meter readings of one unit of the seal's pass 1, in the scenario of 64 KiB
/// values sharing a 64 KiB prefix.
const GAP_MAX: u64 = 6_000;
const GAP_SCENARIO: usize = 9;
const GAP_JOB: &str = "seal pass1";
/// What one reading of the host meter costs besides the code that reads it: the replica's system call.
const READ_FEE: u64 = 200;
/// The look-ahead of a unit's first piece (the first job's runs with no decision before it: a rewrite's piece
/// estimate at most, scaled), what a unit's end after its last piece may cost (the crate's `TAIL_METERED`),
/// and what a decision's note adds to the work after it.
const FIRST_AHEAD: u64 = 60_000;
const TAIL: u64 = 14_000;
const NOTE_SLACK: u64 = 1_000;
const BLOCK: u64 = 4_096;
const PAGE: u64 = 65_536;

const SCENARIOS: [&str; 40] = [
    "mixed rows: seal, compaction of deletes and patches, a drop",
    "64 KiB-class values: seal, compaction of patches",
    "621 leaves: seal, compaction of 600 patch units",
    "values sharing long prefixes: seal, compaction",
    "1,024 leaves: seal, compaction of 700 patch units and deletes, a drop",
    "rows of three schema versions: seal, column release, patches",
    "writes between units of a seal, then a compaction and a second seal",
    "170 Text columns: seal, compaction of patches and deletes, column release",
    "120 Bool and u8 leaves: seal, a group dropped",
    "64 KiB values sharing a 64 KiB prefix: seal, compaction",
    "the reference row: seal",
    "every flat leaf kind: seal, compaction, column release, a drop",
    "every flat leaf kind, columns added and rows written in the seal's write step",
    "mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop",
    "mixed rows, the table opened with a newer schema in its seal",
    "Nat and Int sums past 65,535 bytes: seal, compaction",
    "120 Bool and u8 leaves: compaction, column release",
    "170 Text, Bool and u8 leaves: compaction, column release",
    "90 Text, Bool and u8 leaves: compaction, column release",
    "60 Text, Bool and u8 leaves: compaction, column release",
    "Nat and Int values of 60,000 bytes updated in a seal and a compaction",
    "narrow rows, a Nat key and two Nats: seal",
    "100,000-byte Nat and Int sums that carry across their length, updated after the snapshots",
    "150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots",
    "50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots",
    "an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in",
    "an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in",
    "an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in",
    "120 Int leaves, each with one 6,000-byte negative value: two groups sealed",
    "Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release",
    "300 optional Text columns, half dropped and as many Nat added before the first row",
    "Nat and Text values sharing long prefixes",
    "Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes",
    "an index on a Nat of 1,000 values added to sealed rows: its build",
    "an index on a Text of 1,000 to 4,000 letters added to sealed rows, a tree ten nodes high: its build",
    "an index on a Text of 1,000 to 4,000 letters, every 64th of 64 KiB to 96 KiB, added to sealed rows: its build",
    "scenario 35's index built, then removed: its tree and key log freed by the drain",
    "an index on a Text, every 16th row one of four keys of 64 KiB to 96 KiB sharing 60 KiB, rows removed during its build",
    "an index on a Nat and an Int of 1,000 to 4,000 LEB128 bytes, every 32nd of 100,000 to 140,000 bytes: its build",
    "an index on a Text of 1,000 to 4,000 bytes, every 64th of 64 KiB to 96 KiB, seven in eight bytes 00: its build",
];

fn job(code: u64) -> String {
    let step = match code % 16 {
        0 => "begin",
        1 => "snapshot",
        2 => "pass1",
        3 => "layout",
        4 => "write",
        8 => "flip",
        9 => "last",
        _ => "other",
    };
    match code / 16 {
        1 => format!("seal {step}"),
        2 => format!("compaction {step}"),
        3 => "drop".into(),
        4 => format!("release {step}"),
        5 => "drain".into(),
        6 => "build".into(),
        _ => "nothing".into(),
    }
}

#[derive(Default)]
struct Stat {
    units: u64,
    total: u64,
    max: u64,
    est: u64,
    blocks: u64,
    charged: u64,
    gap: u64,
}

/// The store's data: the 4 KiB blocks written since the last unit began, the bulk instructions' bytes and the
/// readings' fees so far, the unit's last meter reading and longest stretch between two, and the module's
/// counting global.
#[derive(Default)]
struct Data {
    dirty: HashSet<u64>,
    dynamic: u64,
    last_read: u64,
    gap: u64,
    host: bool,
    counter: Option<wasmtime::Global>,
    /// With `NT_UNITS_GAPS`: the stack at the last reading.
    last_stack: String,
    /// The last decision (its count, look-ahead and site) and the last that let work begin; the work past
    /// its look-ahead found so far.
    dec: (u64, u64, String),
    cont: (u64, u64, String),
    /// The last decision that checked the unit's pages: its changed pages then, the pages it took and its site.
    page_dec: (u64, u64, String),
    past: Vec<String>,
    decisions: u64,
    files: std::collections::HashMap<u64, String>,
}

/// A decision of look-ahead `a` at site `to` (or the unit's end) at count `now`: the work since the last
/// decision within that one's look-ahead, and the work since the last decision that let work begin within
/// its look-ahead and the unit's end.
fn judge(d: &mut Data, now: u64, to: &str, a: u64, pages: u64, end: bool) {
    if pages != u64::MAX || end {
        let dirty = d.dirty.len() as u64;
        let (then, took, site) = d.page_dec.clone();
        if took > 0 && dirty.saturating_sub(then) > took {
            d.past.push(format!(
                "the work after the decision at {site} changed {} pages to {to}, past the {took} it took",
                dirty.saturating_sub(then)
            ));
        }
        d.page_dec = (dirty, if end { 0 } else { pages }, to.to_string());
    }
    let (at, la, site) = d.dec.clone();
    if la > 0 {
        let seg = now.saturating_sub(at);
        if seg > la + if end { TAIL } else { NOTE_SLACK } {
            d.past.push(format!("the work after the decision at {site} took {seg} instructions to {to}, past its look-ahead {la}"));
        }
    }
    if a > 0 || end {
        let (at, la, site) = d.cont.clone();
        let seg = now.saturating_sub(at);
        if seg > la + TAIL {
            d.past.push(format!(
                "the work after the decision at {site} took {seg} instructions to {to}, past its look-ahead {la} and the unit's end"
            ));
        }
        d.cont = (now, a, to.to_string());
    }
    d.dec = (now, a, to.to_string());
}

/// The replica's instructions so far: the module's count of its code and the bulk bytes and fees added.
fn now_c(c: &mut Caller<'_, Data>) -> u64 {
    let g = c.data().counter.expect("the counter global");
    let v = g.get(&mut *c).i64().unwrap_or(0) as u64;
    v.saturating_add(c.data().dynamic)
}

fn now(store: &mut Store<Data>) -> u64 {
    let g = store.data().counter.expect("the counter global");
    let v = g.get(&mut *store).i64().unwrap_or(0) as u64;
    v.saturating_add(store.data().dynamic)
}

/// The identifiers of a mangled Rust name (v0 or legacy), joined: enough to tell functions apart.
fn readable(name: &str) -> String {
    let b = name.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() && (i == 0 || !b[i - 1].is_ascii_digit()) {
            let mut j = i;
            let mut n = 0usize;
            while j < b.len() && b[j].is_ascii_digit() {
                n = n.saturating_mul(10).saturating_add(usize::from(b[j] - b'0'));
                j += 1;
            }
            if j < b.len() && b[j] == b'_' {
                j += 1;
            }
            if n > 0 && n < 80 && j + n <= b.len() {
                let id = &name[j..j + n];
                if id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') && !id.starts_with("Cs") {
                    out.push(id.to_string());
                    i = j + n;
                    continue;
                }
            }
        }
        i += 1;
    }
    out.retain(|x| !matches!(x.as_str(), "native_table" | "table" | "nt_cost" | "units" | "VecPages" | "jobs" | "Tracked"));
    out.join("::")
}

/// The module counting what the replica charges: a global (exported as `__ic`) that each basic block adds its
/// instructions' costs to as it begins, and an import `nt.bulk`, appended after the module's function imports
/// (every later function index moved up by one), that every `memory.copy` and `memory.fill` passes its length
/// to first (`local.set`, `local.get`, `call`, `local.get`).
mod exact {
    use wasm_encoder::reencode::{utils, Error as ReError, Reencode};
    use wasm_encoder::{
        CodeSection, ConstExpr, ExportKind, ExportSection, Function, GlobalSection, GlobalType, ImportSection,
        Instruction, TypeSection, ValType,
    };
    use wasmparser::{FunctionBody, Operator, Parser, Payload, TypeRef};

    /// The replica's cost of one instruction in a module with a 64-bit memory (its bulk instructions' bytes
    /// are counted apart).
    fn cost(op: &Operator) -> u64 {
        use Operator::*;
        match op {
            Block { .. } | Else | End | Loop { .. } => 0,
            I32Const { .. } | I64Const { .. } | F32Const { .. } | F64Const { .. } => 1,
            I32DivS | I32DivU | I32RemS | I32RemU | I64DivS | I64DivU | I64RemS | I64RemU => 10,
            I32Load { .. } | I64Load { .. } | F32Load { .. } | F64Load { .. } | I32Load8S { .. }
            | I32Load8U { .. } | I32Load16S { .. } | I32Load16U { .. } | I64Load8S { .. } | I64Load8U { .. }
            | I64Load16S { .. } | I64Load16U { .. } | I64Load32S { .. } | I64Load32U { .. } | I32Store { .. }
            | I64Store { .. } | F32Store { .. } | F64Store { .. } | I32Store8 { .. } | I32Store16 { .. }
            | I64Store8 { .. } | I64Store16 { .. } | I64Store32 { .. } => 2,
            GlobalGet { .. } | GlobalSet { .. } => 2,
            LocalGet { .. } | LocalSet { .. } | LocalTee { .. } | Select | TypedSelect { .. } => 1,
            MemoryGrow { .. } => 300,
            MemorySize { .. } => 20,
            MemoryFill { .. } | MemoryCopy { .. } => 100,
            Call { .. } => 5,
            CallIndirect { .. } => 10,
            Return | Drop | Unreachable | Nop => 1,
            If { .. } | Br { .. } | BrIf { .. } | BrTable { .. } => 2,
            F32Add | F32Sub | F32Mul | F64Add | F64Sub | F64Mul => 2,
            F32Div => 3,
            F64Div => 5,
            F64ConvertI64U | F64ConvertI32U | F32ConvertI64U | F32ConvertI32U => 16,
            F64ConvertI64S | F64ConvertI32S | F32ConvertI64S | F32ConvertI32S => 3,
            F32Eq | F32Ne | F32Lt | F32Gt | F32Le | F32Ge | F64Eq | F64Ne | F64Lt | F64Gt | F64Le | F64Ge => 3,
            I64TruncSatF64U | I64TruncSatF64S | I32TruncSatF64U | I32TruncSatF64S => 20,
            _ => 1,
        }
    }

    /// The basic blocks of a function body: each block's first instruction and its instructions' cost (the
    /// function's entry one more).
    fn blocks(ops: &[Operator]) -> Vec<(usize, u64)> {
        use Operator::*;
        let mut res = Vec::new();
        let mut cur = (0usize, 1u64);
        for (p, op) in ops.iter().enumerate() {
            cur.1 += cost(op);
            if matches!(
                op,
                Loop { .. }
                    | If { .. }
                    | Else
                    | Br { .. }
                    | BrIf { .. }
                    | BrTable { .. }
                    | End
                    | Return
                    | Unreachable
                    | ReturnCall { .. }
                    | ReturnCallIndirect { .. }
            ) {
                res.push(cur);
                cur = (p + 1, 0);
            }
        }
        if cur.1 > 0 {
            res.push(cur);
        }
        res
    }

    struct Counted {
        imported: u32,
        params: Vec<u32>,
        next: usize,
        global: u32,
        bulk_ty: u32,
    }

    impl Reencode for Counted {
        type Error = String;

        fn function_index(&mut self, func: u32) -> Result<u32, ReError<String>> {
            Ok(if func >= self.imported { func + 1 } else { func })
        }

        fn parse_type_section(
            &mut self,
            types: &mut TypeSection,
            section: wasmparser::TypeSectionReader<'_>,
        ) -> Result<(), ReError<String>> {
            utils::parse_type_section(self, types, section)?;
            types.ty().function([ValType::I64], []);
            Ok(())
        }

        fn parse_import_section(
            &mut self,
            imports: &mut ImportSection,
            section: wasmparser::ImportSectionReader<'_>,
        ) -> Result<(), ReError<String>> {
            utils::parse_import_section(self, imports, section)?;
            imports.import("nt", "bulk", wasm_encoder::EntityType::Function(self.bulk_ty));
            Ok(())
        }

        fn parse_global_section(
            &mut self,
            globals: &mut GlobalSection,
            section: wasmparser::GlobalSectionReader<'_>,
        ) -> Result<(), ReError<String>> {
            utils::parse_global_section(self, globals, section)?;
            globals.global(GlobalType { val_type: ValType::I64, mutable: true, shared: false }, &ConstExpr::i64_const(0));
            Ok(())
        }

        fn parse_export_section(
            &mut self,
            exports: &mut ExportSection,
            section: wasmparser::ExportSectionReader<'_>,
        ) -> Result<(), ReError<String>> {
            utils::parse_export_section(self, exports, section)?;
            exports.export("__ic", ExportKind::Global, self.global);
            Ok(())
        }

        fn parse_custom_section(
            &mut self,
            module: &mut wasm_encoder::Module,
            section: wasmparser::CustomSectionReader<'_>,
        ) -> Result<(), ReError<String>> {
            // The function names, moved with their functions (backtraces name them); every other custom
            // section is left out.
            if let wasmparser::KnownCustom::Name(names) = section.as_known() {
                let mut ns = wasm_encoder::NameSection::new();
                for n in names {
                    if let Ok(wasmparser::Name::Function(m)) = n {
                        let mut v = Vec::new();
                        for x in m {
                            let x = x.map_err(|e| ReError::UserError(e.to_string()))?;
                            v.push((if x.index >= self.imported { x.index + 1 } else { x.index }, x.name.to_string()));
                        }
                        v.sort_by_key(|x| x.0);
                        let mut map = wasm_encoder::NameMap::new();
                        for (i, n) in &v {
                            map.append(*i, n);
                        }
                        ns.functions(&map);
                    }
                }
                module.section(&ns);
            }
            Ok(())
        }

        fn parse_function_body(
            &mut self,
            code: &mut CodeSection,
            func: FunctionBody<'_>,
        ) -> Result<(), ReError<String>> {
            let mut locals = Vec::new();
            let mut declared = 0u32;
            for pair in func.get_locals_reader()? {
                let (n, ty) = pair?;
                declared += n;
                locals.push((n, self.val_type(ty)?));
            }
            let params =
                *self.params.get(self.next).ok_or_else(|| ReError::UserError("a body without a function".into()))?;
            self.next += 1;
            let scratch = params + declared;
            locals.push((1, ValType::I64));
            let mut f = Function::new(locals);
            let mut ops = Vec::new();
            let mut reader = func.get_operators_reader()?;
            while !reader.eof() {
                ops.push(reader.read()?);
            }
            let starts = blocks(&ops);
            let mut bi = 0usize;
            for (p, op) in ops.iter().enumerate() {
                while let Some(&(at, c)) = starts.get(bi) {
                    if at != p {
                        break;
                    }
                    if c > 0 {
                        f.instruction(&Instruction::GlobalGet(self.global));
                        f.instruction(&Instruction::I64Const(c as i64));
                        f.instruction(&Instruction::I64Add);
                        f.instruction(&Instruction::GlobalSet(self.global));
                    }
                    bi += 1;
                }
                match op {
                    Operator::MemoryCopy { .. } | Operator::MemoryFill { .. } => {
                        f.instruction(&Instruction::LocalSet(scratch));
                        f.instruction(&Instruction::LocalGet(scratch));
                        f.instruction(&Instruction::Call(self.imported));
                        f.instruction(&Instruction::LocalGet(scratch));
                    }
                    Operator::ReturnCall { .. } | Operator::ReturnCallIndirect { .. } => {
                        return Err(ReError::UserError("tail calls are not counted".into()));
                    }
                    _ => {}
                }
                f.instruction(&self.instruction(op.clone())?);
            }
            code.function(&f);
            Ok(())
        }
    }

    /// The rewritten module; only a module with function imports and one 64-bit memory is accepted.
    pub fn instrument(wasm: &[u8]) -> Result<Vec<u8>, String> {
        let mut imported = 0u32;
        let mut imported_globals = 0u32;
        let mut defined_globals = 0u32;
        let mut types: Vec<u32> = Vec::new();
        let mut funcs: Vec<u32> = Vec::new();
        let mut mem64 = false;
        for p in Parser::new(0).parse_all(wasm) {
            match p.map_err(|e| e.to_string())? {
                Payload::TypeSection(r) => {
                    for t in r.into_iter_err_on_gc_types() {
                        types.push(t.map_err(|e| e.to_string())?.params().len() as u32);
                    }
                }
                Payload::ImportSection(r) => {
                    for imp in r.into_imports() {
                        match imp.map_err(|e| e.to_string())?.ty {
                            TypeRef::Func(_) => imported += 1,
                            TypeRef::Global(_) => imported_globals += 1,
                            _ => {}
                        }
                    }
                }
                Payload::FunctionSection(r) => {
                    for f in r {
                        funcs.push(f.map_err(|e| e.to_string())?);
                    }
                }
                Payload::GlobalSection(r) => defined_globals = r.count(),
                Payload::MemorySection(r) => {
                    for m in r {
                        mem64 |= m.map_err(|e| e.to_string())?.memory64;
                    }
                }
                _ => {}
            }
        }
        if imported == 0 || !mem64 {
            return Err("the meter takes a module with function imports and a 64-bit memory".into());
        }
        let params = funcs.iter().map(|&t| types.get(t as usize).copied().unwrap_or(0)).collect();
        let mut c = Counted {
            imported,
            params,
            next: 0,
            global: imported_globals + defined_globals,
            bulk_ty: types.len() as u32,
        };
        let mut module = wasm_encoder::Module::new();
        c.parse_core_module(&mut module, Parser::new(0), wasm).map_err(|e| format!("{e:?}"))?;
        Ok(module.finish())
    }
}

/// What a run measured: the tables, and every unit past the bound.
pub struct Measured {
    pub text: String,
    pub breaches: Vec<String>,
}

/// Runs every scenario in both modes with the module at `module_path`, and, when `notes_path` names the same
/// module built with the crate's `meter-notes`, with the meter once more to check every decision (its notes
/// cost a little, so the tables come from the first module alone).
pub fn measure(module_path: &str, notes_path: Option<&str>) -> Result<Measured> {
    let mut config = Config::new();
    config.wasm_memory64(true);
    let engine = Engine::new(&config)?;
    let load = |p: &str| -> Result<Module> {
        let wasm = std::fs::read(p)?;
        Module::new(&engine, exact::instrument(&wasm).map_err(wasmtime::Error::msg)?)
    };
    let module = load(module_path)?;
    let notes_module = notes_path.map(load).transpose()?;
    let mut linker: Linker<Data> = Linker::new(&engine);
    // With `NT_UNITS_GAPS` set to n: the stacks at the two readings around every stretch longer than n, each
    // pair with its longest stretch and how often it came, printed at the end (diagnosis).
    let show: Option<u64> = std::env::var("NT_UNITS_GAPS").ok().and_then(|x| x.parse().ok());
    let stacks = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::<String, (u64, u64)>::new()));
    let seen = stacks.clone();
    linker.func_wrap("env", "nt_fuel_used", move |mut caller: Caller<'_, Data>| -> u64 {
        caller.data_mut().dynamic += READ_FEE;
        let c = now_c(&mut caller);
        let gap = c.saturating_sub(caller.data().last_read);
        if let Some(n) = show {
            let bt = wasmtime::WasmBacktrace::force_capture(&caller);
            let frames: Vec<String> = bt
                .frames()
                .iter()
                .skip(1)
                .take(5)
                .map(|f| f.func_name().map(readable).unwrap_or_default())
                .collect();
            let here = frames.join(" < ");
            if gap > n {
                let key = format!("from [{}] to [{}]", caller.data().last_stack, here);
                if let Ok(mut m) = seen.lock() {
                    let e = m.entry(key).or_insert((0, 0));
                    e.0 = e.0.max(gap);
                    e.1 += 1;
                }
            }
            caller.data_mut().last_stack = here;
        }
        let d = caller.data_mut();
        d.gap = d.gap.max(gap);
        d.last_read = c;
        c
    })?;
    linker.func_wrap("env", "nt_note", |mut caller: Caller<'_, Data>, a: u64, pages: u64, file: u64, len: u64, line: u32| {
        let now = now_c(&mut caller);
        if !caller.data().files.contains_key(&file) {
            let name = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                Some(m) => {
                    let mem = m.data(&caller);
                    let from = usize::try_from(file).unwrap_or(usize::MAX);
                    let to = from.saturating_add(usize::try_from(len).unwrap_or(0));
                    String::from_utf8_lossy(mem.get(from..to).unwrap_or(b"?")).into_owned()
                }
                None => "?".into(),
            };
            let name = name.rsplit_once("/src/").map_or(name.clone(), |(_, f)| f.to_string());
            caller.data_mut().files.insert(file, name);
        }
        let site = format!("{}:{line}", caller.data().files.get(&file).map_or("?", |s| s.as_str()));
        caller.data_mut().decisions += 1;
        judge(caller.data_mut(), now, &site, a, pages, false);
    })?;
    linker.func_wrap("nt", "bulk", |mut caller: Caller<'_, Data>, len: i64| {
        let d = caller.data_mut();
        if !d.host {
            d.dynamic += len as u64;
        }
    })?;
    linker.func_wrap("env", "nt_host", |mut caller: Caller<'_, Data>, on: u32| {
        caller.data_mut().host = on != 0;
    })?;
    linker.func_wrap("env", "nt_dirty", |mut caller: Caller<'_, Data>, at: u64| {
        let d = &mut caller.data_mut().dirty;
        if at & 1 == 0 {
            d.insert(at);
        } else {
            let p = at & !1;
            for i in 0..PAGE / BLOCK {
                d.insert(p + i * BLOCK);
            }
        }
    })?;
    let mut out = String::new();
    let mut breaches = Vec::new();
    // Per mode: each scenario's first phase (its first group's seal and the drain after it), as totals.
    let mut firsts: Vec<(bool, String)> = Vec::new();
    // `NT_UNITS_ONLY` = "modes:scenarios" (as "1:3,5": the metered mode, scenarios 3 and 5) runs a part, for
    // development only; a scenario past the named ones runs too when the module has it.
    let only = std::env::var("NT_UNITS_ONLY").ok();
    let wanted = |metered: bool, s: usize| -> bool {
        let Some(o) = only.as_deref() else { return true };
        let (m, sc) = o.split_once(':').unwrap_or((o, ""));
        m.contains(if metered { '1' } else { '0' }) && (sc.is_empty() || sc.split(',').any(|x| x == s.to_string()))
    };
    // The scenarios: the named ones, and any further one `NT_UNITS_ONLY` names.
    let mut list: Vec<(usize, String)> = SCENARIOS.iter().enumerate().map(|(i, n)| (i, n.to_string())).collect();
    if let Some((_, sc)) = only.as_deref().and_then(|o| o.split_once(':')) {
        for x in sc.split(',').filter_map(|x| x.parse::<usize>().ok()).filter(|&x| x >= SCENARIOS.len()) {
            list.push((x, format!("scenario {x}")));
        }
    }
    // The passes: estimates only, the meter, and the meter with the decisions' notes; only the first two make
    // the tables.
    let mut passes: Vec<(bool, &Module, bool)> = vec![(false, &module, true), (true, &module, true)];
    if let Some(m) = notes_module.as_ref() {
        passes.push((true, m, false));
    }
    let mut decisions = 0u64;
    let mut checked = String::new();
    for (metered, module, tables) in passes {
        if !list.iter().any(|(s, _)| wanted(metered, *s)) {
            continue;
        }
        let out = if tables { &mut out } else { &mut checked };
        // A fresh instance per mode: every scenario starts from the same state.
        let mut store = Store::new(&engine, Data::default());
        let instance = linker.instantiate(&mut store, module)?;
        store.data_mut().counter = Some(instance.get_global(&mut store, "__ic").ok_or_else(|| wasmtime::format_err!("no counter"))?);
        let setup: TypedFunc<u32, u64> = instance.get_typed_func(&mut store, "units_setup")?;
        let step: TypedFunc<(), u64> = instance.get_typed_func(&mut store, "units_step")?;
        let between: TypedFunc<(), ()> = instance.get_typed_func(&mut store, "units_between")?;
        let phase: TypedFunc<(), u64> = instance.get_typed_func(&mut store, "units_phase")?;
        let info: TypedFunc<u32, u64> = instance.get_typed_func(&mut store, "units_info")?;
        let meter: TypedFunc<u32, ()> = instance.get_typed_func(&mut store, "units_meter")?;
        meter.call(&mut store, u32::from(metered))?;
        let mode = match (metered, tables) {
            (false, _) => "their own estimates only",
            (true, true) => "the replica's count as their host meter",
            (true, false) => "the replica's count as their host meter, the decisions' notes on",
        };
        writeln!(out, "\nUnits cut by {mode} (instructions on the replica, by its cost model):\n")?;
        writeln!(
            out,
            "| scenario | job and step | units | most instructions | mean instructions | most estimated | most changed pages | most charged | longest between meter readings |"
        )?;
        writeln!(out, "|---|---|---:|---:|---:|---:|---:|---:|---:|")?;
        let (mut worst, mut worst_blocks, mut worst_charged, mut worst_gap) = (0u64, 0u64, 0u64, 0u64);
        for (s, name) in list.iter().map(|(s, n)| (*s, n.as_str())) {
            if !wanted(metered, s) {
                continue;
            }
            let rows = setup.call(&mut store, s as u32)?;
            let mut stats: BTreeMap<String, Stat> = BTreeMap::new();
            let mut units = 0u64;
            // The first phase's seal and drain: units, instructions and charged.
            let mut first = true;
            let mut seal0 = (0u64, 0u64, 0u64);
            let mut drain0 = (0u64, 0u64, 0u64);
            loop {
                units += 1;
                if units > 3_000_000 {
                    wasmtime::bail!("{name}: more than 3,000,000 units");
                }
                store.data_mut().dirty.clear();
                let a = now(&mut store);
                store.data_mut().last_read = a;
                store.data_mut().gap = 0;
                store.data_mut().last_stack = "unit begins".into();
                store.data_mut().dec = (a, FIRST_AHEAD, "the unit's start".into());
                store.data_mut().cont = (a, FIRST_AHEAD, "the unit's start".into());
                store.data_mut().page_dec = (0, 0, "the unit's start".into());
                store.data_mut().past.clear();
                let more = step.call(&mut store, ())?;
                let end = now(&mut store);
                let ins = end - a;
                if metered && store.data().decisions > 0 {
                    // A module with the decisions' notes: the unit's last work is judged too.
                    judge(store.data_mut(), end, "the unit's end", 0, 0, true);
                    // The stretch from the last reading to the unit's end counts too.
                    let tail = end.saturating_sub(store.data().last_read);
                    let d = store.data_mut();
                    d.gap = d.gap.max(tail);
                    if show.is_some_and(|n| tail > n) {
                        let key = format!("from [{}] to [the unit's end]", d.last_stack);
                        if let Ok(mut m) = stacks.lock() {
                            let e = m.entry(key).or_insert((0, 0));
                            e.0 = e.0.max(tail);
                            e.1 += 1;
                        }
                    }
                }
                let blocks = store.data().dirty.len() as u64;
                let charged = ins + blocks * PAGE_FEE;
                let gap = if metered { store.data().gap } else { 0 };
                if more == 0 {
                    if phase.call(&mut store, ())? == 0 {
                        break;
                    }
                    first = false;
                    continue;
                }
                let code = info.call(&mut store, 0)?;
                let est = info.call(&mut store, 1)?;
                if first {
                    let t = match code / 16 {
                        1 => Some(&mut seal0),
                        5 => Some(&mut drain0),
                        _ => None,
                    };
                    if let Some(t) = t {
                        t.0 += 1;
                        t.1 += ins;
                        t.2 += charged;
                    }
                }
                let e = stats.entry(job(code)).or_default();
                e.units += 1;
                e.total += ins;
                e.max = e.max.max(ins);
                e.est = e.est.max(est);
                e.blocks = e.blocks.max(blocks);
                e.charged = e.charged.max(charged);
                e.gap = e.gap.max(gap);
                worst = worst.max(ins);
                worst_blocks = worst_blocks.max(blocks);
                worst_charged = worst_charged.max(charged);
                worst_gap = worst_gap.max(gap);
                if ins > UNIT_WORK || blocks > UNIT_BLOCKS_MAX || charged > CHARGED_MAX {
                    breaches.push(format!(
                        "{name}, units cut by {mode}: unit {units} ({}) took {ins} instructions, changed {blocks} pages, charged {charged}",
                        job(code)
                    ));
                }
                for p in std::mem::take(&mut store.data_mut().past) {
                    breaches.push(format!("{name}, units cut by {mode}: unit {units} ({}): {p}", job(code)));
                }
                if s == GAP_SCENARIO && gap > GAP_MAX && job(code) == GAP_JOB {
                    breaches.push(format!(
                        "{name}, units cut by {mode}: unit {units} ({}) went {gap} instructions between meter readings",
                        job(code)
                    ));
                }
                between.call(&mut store, ())?;
            }
            // The rows the first group holds: the scenario's rows set up but the last, which opened the next.
            let n = rows.saturating_sub(1).max(1);
            if !tables {
                continue;
            }
            firsts.push((
                metered,
                format!(
                    "| {name} | {n} | {} | {} | {} | {} | {} | {} | {} | {} |",
                    seal0.0,
                    seal0.1 / n,
                    seal0.2 / n,
                    drain0.0,
                    drain0.1 / n,
                    drain0.2 / n,
                    (seal0.1 + drain0.1) / n,
                    (seal0.2 + drain0.2) / n
                ),
            ));
            for (k, v) in &stats {
                writeln!(
                    out,
                    "| {name} | {k} | {} | {} | {} | {} | {} | {} | {} |",
                    v.units,
                    v.max,
                    v.total / v.units.max(1),
                    v.est,
                    v.blocks,
                    v.charged,
                    if metered { v.gap.to_string() } else { "-".into() }
                )?;
            }
        }
        writeln!(
            out,
            "\nMost instructions of any unit: {worst}. Most changed pages: {worst_blocks}. Most charged: {worst_charged}."
        )?;
        if metered {
            writeln!(out, "Longest stretch between two meter readings of a unit: {worst_gap}.")?;
        }
        decisions = decisions.saturating_add(store.data().decisions);
    }
    if notes_module.is_some() {
        writeln!(
            out,
            "\nThe meter's decisions, checked in a module built with the crate's `meter-notes`: {decisions} decisions, the work each let begin within the look-ahead and the pages it took."
        )?;
    }
    for metered in [false, true] {
        if !firsts.iter().any(|(m, _)| *m == metered) {
            continue;
        }
        let mode = if metered { "the replica's count as their host meter" } else { "their own estimates only" };
        writeln!(
            out,
            "\nEach scenario's first group: its seal and the drain after it, per row of the group (units cut by {mode}; charged: instructions and {PAGE_FEE} per changed page):\n"
        )?;
        writeln!(
            out,
            "| scenario | rows | seal units | seal instructions per row | seal charged per row | drain units | drain instructions per row | drain charged per row | seal and drain instructions per row | seal and drain charged per row |"
        )?;
        writeln!(out, "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")?;
        for (_, line) in firsts.iter().filter(|(m, _)| *m == metered) {
            writeln!(out, "{line}")?;
        }
    }
    if let Ok(m) = stacks.lock() {
        let mut v: Vec<_> = m.iter().collect();
        v.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
        for (k, (g, n)) in v {
            eprintln!("gap {g} x{n}: {k}");
        }
    }
    Ok(Measured { text: out, breaches })
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let usage = "usage: units <nt_cost.wasm> [out file] [nt_cost.wasm with meter-notes]";
    let module_path = args.get(1).ok_or_else(|| wasmtime::format_err!("{usage}"))?;
    let m = measure(module_path, args.get(3).map(|s| s.as_str()))?;
    print!("{}", m.text);
    if let Some(p) = args.get(2) {
        std::fs::write(p, &m.text)?;
    }
    for b in m.breaches.iter().take(50) {
        eprintln!("past the bound: {b}");
    }
    if !m.breaches.is_empty() {
        wasmtime::bail!("{} units passed the bound", m.breaches.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    /// Every unit of every scenario, cut by its own estimates and by the replica's count as its meter, takes at
    /// most 96,000 instructions on the replica, changes at most 17 pages of 4 KiB and is charged at most
    /// 164,000; the units of values sharing 64 KiB prefixes read the meter at least every 6,000; and, with the
    /// module `NT_COST_NOTES_WASM` names (built with `meter-notes`), the work every decision lets begin stays
    /// within the look-ahead and the pages the decision took.
    #[test]
    fn every_unit_within_the_bound() {
        let Ok(module) = std::env::var("NT_COST_WASM") else {
            eprintln!("NT_COST_WASM not set: the units are measured by scripts/check.sh");
            return;
        };
        let notes = std::env::var("NT_COST_NOTES_WASM").ok();
        let m = super::measure(&module, notes.as_deref()).expect("the scenarios run");
        assert!(m.breaches.is_empty(), "{} units passed the bound:\n{}", m.breaches.len(), m.breaches.join("\n"));
    }
}
