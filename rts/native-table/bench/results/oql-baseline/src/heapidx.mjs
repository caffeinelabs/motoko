// the generated ledger appended ON CHAIN into an OQL.Table with one heap
// #hash index (account_from), at `rows`; then count by eq (real query calls),
// the emptying delete (rows with fresh unique keys, each deleted alone) and
// Table deletes of loaded rows, lone and 1,000 per message (update context).
//   [QONLY=1] node heapidx.mjs <oql dir> <out dir> <rows> [N] [batch]
// N: messages per lone delete sequence (default 8192); batch: rows appended per
// genIndexed1 message during the build (default 50,000). QONLY=1 (any non-empty
// value): stop after the queries, before the update-context operations; the
// output files are then named heapidx-<rows>-q.* instead of heapidx-<rows>.*.
import { writeFileSync, mkdirSync, appendFileSync, readFileSync } from 'node:fs';
import { IDL } from '@dfinity/candid';
import { startPic, settle, stats, relayIdl, natValue } from './lib.mjs';

const [OQLDIR, OUTDIR, ROWS_S, N_S, BATCH_S] = process.argv.slice(2);
const ROWS = Number(ROWS_S), NSEQ = Number(N_S || 8192), BATCH = Number(BATCH_S || 50_000);
mkdirSync(OUTDIR, { recursive: true });
const tag = `heapidx-${ROWS}${process.env.QONLY ? '-q' : ''}`;
const LOG = `${OUTDIR}/${tag}.log`;
const log = (s) => { const line = `[${new Date().toISOString()}] ${tag} ${s}`; console.log(line); appendFileSync(LOG, line + '\n'); };
const out = (name, obj) => appendFileSync(`${OUTDIR}/${tag}.${name}.jsonl`, JSON.stringify(obj) + '\n');
const JM = IDL.Record({ instrs: IDL.Nat, rows: IDL.Nat, value: IDL.Nat });
const MO = IDL.Record({ instr: IDL.Nat64, pmut: IDL.Nat64, pgc: IDL.Nat64, r: IDL.Nat64 });
const idlFactory = ({ IDL }) => IDL.Service({
  genIndexed1: IDL.Func([IDL.Nat, IDL.Nat, IDL.Nat], [IDL.Nat], []),
  m1Flush: IDL.Func([], [], []),
  op: IDL.Func([IDL.Nat64, IDL.Nat64, IDL.Nat64], [MO], []),
  tick: IDL.Func([], [], []),
  last: IDL.Func([], [IDL.Record({ mut: IDL.Nat64, gc: IDL.Nat64 })], ['query']),
  measure: IDL.Func([IDL.Text], [JM], ['query']),
  m1Stats: IDL.Func([], [IDL.Record({ size: IDL.Nat, segments: IDL.Nat, reclaimed: IDL.Nat, totalAllocation: IDL.Nat, heap: IDL.Nat })], ['query']),
});
const CPI = 2, PAGE = 4000;
const { server, pic } = await startPic(`hx${ROWS}`);
const results = { rows: ROWS, oql: OQLDIR, stages: {} };
const save = () => writeFileSync(`${OUTDIR}/${tag}.summary.json`, JSON.stringify(results, null, 2));
// `break run` (QONLY) leaves the try block, so the finally block still stops PocketIC.
run: try {
  const f = await pic.setupCanister({ idlFactory, wasm: readFileSync(`${OQLDIR}/bench/scale/oqlbars.wasm`), cycles: 10n ** 18n, wasmMemoryLimit: 0n });
  const t = f.actor, cid = f.canisterId;
  const relayF = await pic.setupCanister({ idlFactory: relayIdl, wasm: readFileSync(new URL('./out-1140/relay.wasm', import.meta.url).pathname), cycles: 10n ** 16n });
  const relayRaw = (method, argBytes) => pic.updateCall({ canisterId: relayF.canisterId, method: 'raw', arg: new Uint8Array(IDL.encode([IDL.Principal, IDL.Text, IDL.Vec(IDL.Nat8)], [cid, method, argBytes])) });
  const opArg = (k, a, b) => new Uint8Array(IDL.encode([IDL.Nat64, IDL.Nat64, IDL.Nat64], [BigInt(k), BigInt(a), BigInt(b)]));
  const t0 = performance.now();
  for (let off = 0; off < ROWS; off += BATCH) {
    await t.genIndexed1(BigInt(off), BigInt(Math.min(BATCH, ROWS - off)), 0n);
    if ((off / BATCH) % 40 === 0) { const st = await t.m1Stats(); log(`appended ${off + Math.min(BATCH, ROWS - off)}/${ROWS} heap ${st.heap} segments ${st.segments}`); }
  }
  await t.m1Flush();
  const st = await t.m1Stats();
  results.stages.build = { wall_s: (performance.now() - t0) / 1000, size: Number(st.size), segments: Number(st.segments), heap: Number(st.heap) }; save();
  log(`built: ${st.size} rows, ${st.segments} segments, heap ${st.heap} B, ${results.stages.build.wall_s.toFixed(0)} s`);
  const q = async (label, json) => {
    try { const r = await t.measure(JSON.stringify(json)); const rec = { label, json, instrs: Number(r.instrs), rows: Number(r.rows), value: natValue(r.value) }; out('queries', rec); log(`Q ${label}: ${rec.instrs.toLocaleString('en-US')} instrs, rows ${rec.rows}, value ${rec.value}`); }
    catch (e) { const rec = { label, json, trapped: true, error: String(e.message ?? e).slice(0, 300) }; out('queries', rec); log(`Q ${label}: TRAPPED`); }
  };
  const cnt = (v) => ({ start: 'ledger1', where: { eq: { field: 'account_from', value: v } }, aggregate: [{ fn: 'count' }] });
  for (const v of [123, 456, 1967]) await q(`count account_from==${v} (collector as the build left it)`, cnt(v));
  const st0 = await settle(t);   // queries read objects through the collector's barriers while a collection is in progress
  log(`settled before the queries: ${st0.ticks} ticks, collection ${st0.spent}`);
  for (const v of [123, 456, 1967]) await q(`count account_from==${v}`, cnt(v));
  await q('count account_from==100001 (absent)', cnt(100001));
  await q('eq account_from==123', { start: 'ledger1', where: { eq: { field: 'account_from', value: 123 } } });
  await q('count(*)', { start: 'ledger1', aggregate: [{ fn: 'count' }] });
  if (process.env.QONLY) { log('done (queries only)'); break run; }
  // cycles model calibration (as scale.mjs)
  await settle(t);
  const rr = [];
  for (let i = 0; i < 5; i++) { const c0 = await pic.getCyclesBalance(cid); await relayRaw('op', opArg(99, 0, 0)); const c1 = await pic.getCyclesBalance(cid); const l = await t.last(); rr.push((Number(c0 - c1) - 5_000_000) / 2 - Number(l.mut) - Number(l.gc)); }
  const x0 = rr.sort()[2];
  const fees = [];
  for (let i = 0; i < 9; i++) { const c0 = await pic.getCyclesBalance(cid); await t.op(99n, 0n, 0n); const c1 = await pic.getCyclesBalance(cid); const l = await t.last(); fees.push(Number(c0 - c1) - 2 * (Number(l.mut) + Number(l.gc)) - 2 * x0); }
  const FEE = fees.sort()[4];
  results.calibration = { x0, fee: FEE, fees }; save();
  log(`calibration: noop 4000*p0+resid ${x0}; ingress fee ${FEE}`);
  async function loneSeq(name, msgs) {
    await settle(t);
    const recs = []; let c0 = await pic.getCyclesBalance(cid); let prev = null;
    const fin = (rec, mut, gc) => { rec.mut = mut; rec.gc = gc; const pf = (rec.cyc - FEE) / CPI / PAGE - (mut + gc) / PAGE; rec.n16 = Math.round(pf); rec.resid = Math.round((pf - rec.n16) * PAGE); };
    for (let i = 0; i < msgs.length; i++) {
      const [k, a, b] = msgs[i];
      const m = await t.op(BigInt(k), BigInt(a), BigInt(b));
      const c1 = await pic.getCyclesBalance(cid);
      if (prev) fin(prev, Number(m.pmut), Number(m.pgc));
      const rec = { i, a, b, instr: Number(m.instr), cyc: Number(c0 - c1), r: Number(m.r) };
      recs.push(rec); prev = rec; c0 = c1;
    }
    const l = await t.last(); fin(prev, Number(l.mut), Number(l.gc));
    const tail = await settle(t);
    for (const r of recs) out(name, r);
    const n = recs.length, sIn = recs.reduce((q, x) => q + x.instr, 0), sGc = recs.reduce((q, x) => q + x.gc, 0), sN = recs.reduce((q, x) => q + x.n16, 0);
    const s = { n, tail: { ticks: tail.ticks, gc: Number(tail.spent) }, in_method: stats(recs.map((x) => x.instr)), gc: stats(recs.map((x) => x.gc)),
      in_method_plus_gc_mean: (sIn + sGc + Number(tail.spent)) / n, in_method_plus_gc: stats(recs.map((x) => x.instr + x.gc)), n16: stats(recs.map((x) => x.n16)),
      charged16: stats(recs.map((x) => x.instr + x.gc + PAGE * x.n16)), charged16_mean_with_tail: (sIn + sGc + Number(tail.spent) + PAGE * sN) / n,
      residues: [...new Set(recs.map((x) => x.resid))].slice(0, 8) };
    results.stages[name] = s; save();
    log(`${name}: n=${n} in-method mean ${s.in_method.mean.toFixed(0)} p50 ${s.in_method.p50} max ${s.in_method.max}; +gc mean ${s.in_method_plus_gc_mean.toFixed(0)}; n16 mean ${s.n16.mean.toFixed(2)}; charged16 mean ${s.charged16_mean_with_tail.toFixed(0)} p50 ${s.charged16.p50} max ${s.charged16.max}; residues ${JSON.stringify(s.residues)}`);
    return s;
  }
  // emptying delete: NSEQ rows with fresh unique keys (one message), then each deleted alone
  const FRESH = 10_000_000;
  const m = await t.op(8n, BigInt(FRESH), BigInt(NSEQ));
  const lastPos = Number(m.r), firstPos = lastPos - NSEQ + 1;
  await loneSeq('emptying_delete', Array.from({ length: NSEQ }, (_, i) => [3, firstPos + i, 0]));
  await q(`count account_from==${FRESH} after its delete`, cnt(FRESH));
  // deletes of loaded rows (their key keeps other rows: a posting removal, no Map delete)
  await loneSeq('delete_loaded', Array.from({ length: NSEQ }, (_, i) => [3, (7919 * i + 13) % ROWS, 0]));
  const b = await loneSeq('delete_batch1000', Array.from({ length: 10 }, (_, i) => [4, 1_000_003 * (i + 1) + 7, 1000]));
  results.stages.delete_batch1000.per_delete = { in_method_plus_gc_mean: b.in_method_plus_gc_mean / 1000, charged16_mean_with_tail: b.charged16_mean_with_tail / 1000 }; save();
  for (const v of [123, 456]) await q(`count account_from==${v} after the deletes`, cnt(v));
  log('done');
} catch (e) { log(`FAILED ${e.stack ?? e}`); process.exitCode = 1; }
finally { try { await pic.tearDown(); } catch {} try { await server.stop(); } catch {} }
