//! The test host's cost meter: wasmtime fuel made aware of bulk memory instructions.
//!
//! Fuel charges a `memory.copy` or a `memory.fill` one unit whatever its length, while the replica charges
//! it 100 plus its length. So the module is rewritten first: before every `memory.copy` and `memory.fill`
//! it calls an import of the meter's with the length operand (`local.set`, `local.get`, `call`,
//! `local.get`: four instructions of fuel each). A workload's figure on the replica is then estimated as
//! r × (fuel − 5 × bulk instructions) + Σ over the bulk instructions of (100 + length), with r fitted by the
//! calibration on the replica.
//!
//! The rewritten module is the calibration canister's: its system API imports are served by the meter, which
//! notes the fuel and the bulk counters at each read of the instruction counter, so a workload's figure is
//! read between the same two points as on the replica. The read itself answers with the replica's estimate
//! of everything spent so far, at the meter's ratio (`Meter::ratio`), so a workload that meters its own units
//! with the counter sees replica instructions.

use wasm_encoder::reencode::{utils, Error as ReError, Reencode};
use wasm_encoder::{CodeSection, Function, ImportSection, Instruction, TypeSection, ValType};
use wasmparser::{FunctionBody, Operator, Parser, Payload, TypeRef};
use wasmtime::{Caller, Engine, Linker, Module, Store};

/// The fuel a run starts with.
const START: u64 = u64::MAX / 4;

/// What the meter saw between the last two reads of the instruction counter.
#[derive(Clone, Copy, Debug, Default)]
pub struct Figure {
    /// Fuel, the meter's own instructions included.
    pub fuel: u64,
    /// Bulk memory instructions and their lengths.
    pub bulk: u64,
    pub bytes: u64,
    /// The workload's result.
    pub result: u64,
}

impl Figure {
    /// The fuel of the workload's instructions other than bulk ones.
    pub fn plain(&self) -> u64 {
        self.fuel.saturating_sub(self.bulk.saturating_mul(5))
    }
    /// The bulk instructions' cost on the replica: 100 plus the length each.
    pub fn bulk_cost(&self) -> u64 {
        self.bulk.saturating_mul(100).saturating_add(self.bytes)
    }
    /// The replica's figure at ratio r.
    pub fn at(&self, r: f64) -> f64 {
        r * self.plain() as f64 + self.bulk_cost() as f64
    }
}

/// The rewrite: one import appended after the module's function imports, every later function index moved
/// up by one, and the length operand of every bulk instruction passed to it first.
struct Bulk {
    /// Function imports of the original module: the meter's import takes this index.
    imported: u32,
    /// The index of the meter's function type, appended after the module's types.
    ty: u32,
    /// Each defined function's parameter count, by defined index.
    params: Vec<u32>,
    /// Defined functions rewritten so far.
    next: usize,
}

impl Reencode for Bulk {
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
        imports.import("nt", "bulk", wasm_encoder::EntityType::Function(self.ty));
        Ok(())
    }

    fn parse_function_body(&mut self, code: &mut CodeSection, func: FunctionBody<'_>) -> Result<(), ReError<String>> {
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
        let meter = self.imported;
        let mut reader = func.get_operators_reader()?;
        while !reader.eof() {
            let op = reader.read()?;
            if matches!(op, Operator::MemoryCopy { .. } | Operator::MemoryFill { .. }) {
                f.instruction(&Instruction::LocalSet(scratch));
                f.instruction(&Instruction::LocalGet(scratch));
                f.instruction(&Instruction::Call(meter));
                f.instruction(&Instruction::LocalGet(scratch));
            }
            f.instruction(&self.instruction(op)?);
        }
        code.function(&f);
        Ok(())
    }
}

/// The module with every bulk memory instruction metered. Only a module with function imports and one 64-bit
/// memory is accepted, as the calibration canister is.
pub fn instrument(wasm: &[u8]) -> Result<Vec<u8>, String> {
    let mut imported = 0u32;
    let mut types: Vec<u32> = Vec::new();
    let mut funcs: Vec<u32> = Vec::new();
    let mut mem64 = false;
    for p in Parser::new(0).parse_all(wasm) {
        match p.map_err(|e| e.to_string())? {
            Payload::TypeSection(r) => {
                for t in r.into_iter_err_on_gc_types() {
                    let t = t.map_err(|e| e.to_string())?;
                    types.push(t.params().len() as u32);
                }
            }
            Payload::ImportSection(r) => {
                for imp in r.into_imports() {
                    if let TypeRef::Func(_) = imp.map_err(|e| e.to_string())?.ty {
                        imported += 1;
                    }
                }
            }
            Payload::FunctionSection(r) => {
                for f in r {
                    funcs.push(f.map_err(|e| e.to_string())?);
                }
            }
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
    let mut b = Bulk { imported, ty: types.len() as u32, params, next: 0 };
    let mut module = wasm_encoder::Module::new();
    b.parse_core_module(&mut module, Parser::new(0), wasm).map_err(|e| format!("{e:?}"))?;
    Ok(module.finish())
}

/// The meter's state during one call.
#[derive(Default)]
struct State {
    args: Vec<u8>,
    reply: Vec<u8>,
    bulk: u64,
    bytes: u64,
    /// At each read of the instruction counter: fuel spent, bulk instructions, their bytes.
    reads: Vec<(u64, u64, u64)>,
    /// With `NT_METER_LENGTHS` set: every bulk instruction's length, in order.
    lengths: Option<Vec<u64>>,
    /// The ratio the counter's answers are estimated at.
    ratio: f64,
}

/// The calibration canister, metered under wasmtime.
pub struct Meter {
    engine: Engine,
    module: Module,
    /// The ratio of replica instructions to plain fuel the counter's answers are estimated at.
    pub ratio: f64,
}

fn memory(c: &mut Caller<'_, State>) -> Result<wasmtime::Memory, wasmtime::Error> {
    c.get_export("memory").and_then(|e| e.into_memory()).ok_or_else(|| wasmtime::Error::msg("no memory export"))
}

impl Meter {
    pub fn new(canister: &[u8]) -> Result<Meter, String> {
        let mut cfg = wasmtime::Config::new();
        cfg.consume_fuel(true);
        cfg.wasm_memory64(true);
        let engine = Engine::new(&cfg).map_err(|e| e.to_string())?;
        let module = Module::new(&engine, instrument(canister)?).map_err(|e| e.to_string())?;
        Ok(Meter { engine, module, ratio: 1.0 })
    }

    /// Runs workload `sel` with its six words, as the canister's `run` method does on the replica.
    pub fn run(&self, sel: u32, words: [u64; 6]) -> Result<Figure, String> {
        self.run_all(sel, words).map(|r| r.last)
    }

    /// Runs workload `sel` with its six words: the figure between the canister's last two reads of the
    /// counter, the figure of its whole window (from its second read, the workload's mark, to its last; the
    /// same for a workload that reads the counter only twice), and the reply's bytes.
    pub fn run_all(&self, sel: u32, words: [u64; 6]) -> Result<Run, String> {
        let mut st = State::default();
        st.args = args(sel, words);
        st.ratio = self.ratio;
        if std::env::var_os("NT_METER_LENGTHS").is_some() {
            st.lengths = Some(Vec::new());
        }
        let mut store = Store::new(&self.engine, st);
        store.set_fuel(START).map_err(|e| e.to_string())?;
        let mut l: Linker<State> = Linker::new(&self.engine);
        let e = |x: wasmtime::Error| x.to_string();
        l.func_wrap("ic0", "msg_arg_data_size", |c: Caller<'_, State>| -> i64 { c.data().args.len() as i64 })
            .map_err(e)?;
        l.func_wrap("ic0", "msg_arg_data_copy", |mut c: Caller<'_, State>, dst: i64, off: i64, n: i64| {
            let m = memory(&mut c)?;
            let src = c.data().args.get(off as usize..(off + n) as usize).unwrap_or(&[]).to_vec();
            m.write(&mut c, dst as usize, &src).map_err(wasmtime::Error::msg)
        })
        .map_err(e)?;
        l.func_wrap("ic0", "msg_reply_data_append", |mut c: Caller<'_, State>, src: i64, n: i64| {
            let m = memory(&mut c)?;
            let mut buf = vec![0u8; n as usize];
            m.read(&c, src as usize, &mut buf).map_err(wasmtime::Error::msg)?;
            c.data_mut().reply.extend_from_slice(&buf);
            Ok(())
        })
        .map_err(e)?;
        l.func_wrap("ic0", "msg_reply", |_: Caller<'_, State>| {}).map_err(e)?;
        l.func_wrap(
            "ic0",
            "performance_counter",
            |mut c: Caller<'_, State>, _t: i32| -> Result<i64, wasmtime::Error> {
                let spent = START - c.get_fuel()?;
                let (b, y) = (c.data().bulk, c.data().bytes);
                c.data_mut().reads.push((spent, b, y));
                let f = Figure { fuel: spent, bulk: b, bytes: y, result: 0 };
                Ok(f.at(c.data().ratio).ceil() as i64)
            },
        )
        .map_err(e)?;
        l.func_wrap("nt", "bulk", |mut c: Caller<'_, State>, len: i64| {
            let d = c.data_mut();
            d.bulk += 1;
            d.bytes += len as u64;
            if let Some(l) = d.lengths.as_mut() {
                l.push(len as u64);
            }
        })
        .map_err(e)?;
        let inst = l.instantiate(&mut store, &self.module).map_err(e)?;
        let f = inst.get_typed_func::<(), ()>(&mut store, "canister_update run").map_err(e)?;
        f.call(&mut store, ()).map_err(e)?;
        let st = store.data();
        if let (Some(l), Some(&(_, b0, _))) = (&st.lengths, st.reads.get(st.reads.len().wrapping_sub(2))) {
            // The lengths between the last two reads, by length.
            let mut by: std::collections::BTreeMap<u64, u64> = std::collections::BTreeMap::new();
            for &x in l.get(b0 as usize..).unwrap_or(&[]) {
                *by.entry(x).or_default() += 1;
            }
            eprintln!("bulk lengths (length × times): {by:?}");
        }
        let n = st.reads.len();
        let (Some(a), Some(b)) = (st.reads.get(n.wrapping_sub(2)), st.reads.get(n.wrapping_sub(1))) else {
            return Err("the canister read its counter fewer than twice".into());
        };
        let s = st.reads.get(if n >= 3 { 1 } else { 0 }).copied().unwrap_or_default();
        let result = st.reply.get(8..16).and_then(|s| s.try_into().ok()).map(u64::from_le_bytes);
        let result = result.ok_or("no reply")?;
        Ok(Run {
            last: Figure { fuel: b.0 - a.0, bulk: b.1 - a.1, bytes: b.2 - a.2, result },
            span: Figure { fuel: b.0 - s.0, bulk: b.1 - s.1, bytes: b.2 - s.2, result },
            reply: st.reply.clone(),
        })
    }
}

/// One run of a workload under the meter.
pub struct Run {
    /// Between the canister's last two reads of the counter.
    pub last: Figure,
    /// From the workload's mark (the canister's second read) to its last read.
    pub span: Figure,
    /// The reply's bytes.
    pub reply: Vec<u8>,
}

/// The little-endian words of a reply from byte 16 on: a sweep's per-key figures.
pub fn reply_words(reply: &[u8]) -> Vec<u64> {
    reply.get(16..).unwrap_or(&[]).chunks_exact(8).filter_map(|c| c.try_into().ok()).map(u64::from_le_bytes).collect()
}

/// The argument bytes of the canister's `run`.
pub fn args(sel: u32, words: [u64; 6]) -> Vec<u8> {
    let mut a = sel.to_le_bytes().to_vec();
    for w in words {
        a.extend_from_slice(&w.to_le_bytes());
    }
    a
}
