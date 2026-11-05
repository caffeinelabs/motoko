// Text-key counts through OQL's heap #hash index written on chain (oqltext.mo), and the
// mo:core Map<Text, Nat> get over the same keys.
//   node textidx.mjs <oqltext.wasm> <out dir> <tag> <shape 0|1|2> <rows> [batch]
// Builds the rows on chain (gen, `batch` rows per message), flushes the Table, ticks
// until the collector is idle (settle), then sends real query calls, one per measured
// operation, for each counted key:
//   tJson  measure(qJson) on the Table entity (the `measure` path of `oqlbars.mo`: JSON parsed
//          outside the counter, the counter around Executor.runWith)
//   tFlat  measureFlat: the same query built in Motoko around the key as Candid decodes it
//   mJson, mFlat  the same two on the IndexedMap entity
//   gFlat  Map.get with the key as Candid decodes it;  gJson  Map.get with the key parsed from qJson
import { writeFileSync, mkdirSync, appendFileSync, readFileSync } from 'node:fs';
import { IDL } from '@dfinity/candid';
import { startPic, settle, stats } from './lib.mjs';

const [WASM, OUTDIR, TAG, SHAPE_S, ROWS_S, BATCH_S] = process.argv.slice(2);
const SHAPE = Number(SHAPE_S), ROWS = Number(ROWS_S), BATCH = Number(BATCH_S || 25_000);
mkdirSync(OUTDIR, { recursive: true });
const tag = `${TAG}-s${SHAPE}-${ROWS}`;
const LOG = `${OUTDIR}/${tag}.log`;
const log = (s) => { const line = `[${new Date().toISOString()}] ${tag} ${s}`; console.log(line); appendFileSync(LOG, line + '\n'); };
const out = (name, obj) => appendFileSync(`${OUTDIR}/${tag}.${name}.jsonl`, JSON.stringify(obj) + '\n');
const JM = IDL.Record({ instrs: IDL.Nat, rows: IDL.Nat, value: IDL.Nat });
const GM = IDL.Record({ instrs: IDL.Nat, value: IDL.Nat });
const idlFactory = ({ IDL }) => IDL.Service({
  setup: IDL.Func([IDL.Nat, IDL.Nat], [], []),
  gen: IDL.Func([IDL.Nat, IDL.Nat], [IDL.Nat], []),
  flush: IDL.Func([], [], []),
  keyOf: IDL.Func([IDL.Nat64], [IDL.Text], ['query']),
  info: IDL.Func([], [IDL.Record({ shape: IDL.Nat, rows: IDL.Nat, appended: IDL.Nat, tableSize: IDL.Nat, mapSize: IDL.Nat, distinct: IDL.Nat, segments: IDL.Nat, heap: IDL.Nat, reclaimed: IDL.Nat, totalAllocation: IDL.Nat })], ['query']),
  tick: IDL.Func([], [], []),
  last: IDL.Func([], [IDL.Record({ mut: IDL.Nat64, gc: IDL.Nat64 })], ['query']),
  measure: IDL.Func([IDL.Text], [JM], ['query']),
  measureFlat: IDL.Func([IDL.Text, IDL.Text], [JM], ['query']),
  mapGet: IDL.Func([IDL.Text], [GM], ['query']),
  mapGetJson: IDL.Func([IDL.Text], [GM], ['query']),
});
const pick = (s) => (s * 7919) % 100000;
const PLANTED = [
  { set: 'held in 1 row', u: 39595, expect: 1 },
  { set: 'held in 6 rows', u: 34623, expect: 6 },
  { set: 'held in 1,000 rows', u: 74037, expect: 1000 },
  { set: 'absent', u: 75701, expect: 0 },
];
const SWEEP = Array.from({ length: 300 }, (_, s) => ({ set: 'sweep even', u: pick(s) & ~1 }));
const ABSENT = Array.from({ length: 300 }, (_, s) => ({ set: 'sweep odd (absent)', u: pick(1000 + s) | 1, expect: 0 }));
const PATHS = ['tJson', 'tFlat', 'mJson', 'mFlat', 'gFlat', 'gJson'];

const { server, pic } = await startPic(`tx-${tag}`);
const results = { tag, shape: SHAPE, rows: ROWS, wasm: WASM, stages: {} };
const save = () => writeFileSync(`${OUTDIR}/${tag}.summary.json`, JSON.stringify(results, null, 2));
let bad = 0;
try {
  const f = await pic.setupCanister({ idlFactory, wasm: readFileSync(WASM), cycles: 10n ** 18n, wasmMemoryLimit: 0n });
  const t = f.actor;
  await t.setup(BigInt(SHAPE), BigInt(ROWS));
  const t0 = performance.now();
  for (let off = 0; off < ROWS; off += BATCH) {
    await t.gen(BigInt(off), BigInt(Math.min(BATCH, ROWS - off)));
    const l = await t.last();
    if ((off / BATCH) % 4 === 0) { const st = await t.info(); log(`appended ${off + Math.min(BATCH, ROWS - off)}/${ROWS}: message mutator ${l.mut} collector ${l.gc}; heap ${st.heap} segments ${st.segments}`); }
  }
  await t.flush();
  const st = await t.info();
  const build = { wall_s: (performance.now() - t0) / 1000 };
  for (const [k, v] of Object.entries(st)) build[k] = Number(v);
  results.stages.build = build; save();
  log(`built: ${st.tableSize} Table rows (${st.segments} segments), ${st.mapSize} IndexedMap rows, ${st.distinct} distinct keys, heap ${st.heap} B, ${build.wall_s.toFixed(0)} s`);
  const s0 = await settle(t);
  results.stages.settle = { ticks: s0.ticks, collection: Number(s0.spent) }; save();
  log(`settled before the queries: ${s0.ticks} ticks, collection ${s0.spent}`);
  const recs = [];
  for (const k of [...PLANTED, ...SWEEP, ...ABSENT]) {
    const key = await t.keyOf(BigInt(k.u));
    const qT = JSON.stringify({ start: 'ttext', where: { eq: { field: 'k', value: key } }, aggregate: [{ fn: 'count' }] });
    const qM = JSON.stringify({ start: 'mtext', where: { eq: { field: 'k', value: key } }, aggregate: [{ fn: 'count' }] });
    const r = {
      tJson: await t.measure(qT), tFlat: await t.measureFlat('ttext', key),
      mJson: await t.measure(qM), mFlat: await t.measureFlat('mtext', key),
      gFlat: await t.mapGet(key), gJson: await t.mapGetJson(qT),
    };
    const rec = { set: k.set, u: k.u, len: key.length };
    const vals = new Set();
    for (const p of PATHS) { rec[p] = Number(r[p].instrs); vals.add(Number(r[p].value)); }
    rec.value = Number(r.gFlat.value);
    if (vals.size !== 1 || (k.expect !== undefined && rec.value !== k.expect)) { bad++; rec.mismatch = [...vals]; log(`MISMATCH ${JSON.stringify(rec)}`); }
    out('queries', rec); recs.push(rec);
    if (k.set !== 'sweep even' && k.set !== 'sweep odd (absent)') log(`Q ${k.set} (u ${k.u}, ${key.length} chars): value ${rec.value}; ${PATHS.map((p) => `${p} ${rec[p].toLocaleString('en-US')}`).join(', ')}`);
  }
  const groups = { 'sweep even, held': recs.filter((r) => r.set === 'sweep even' && r.value > 0), 'sweep even, absent': recs.filter((r) => r.set === 'sweep even' && r.value === 0), 'sweep odd (absent)': recs.filter((r) => r.set === 'sweep odd (absent)') };
  results.sweeps = {};
  for (const [g, rs] of Object.entries(groups)) {
    if (rs.length === 0) continue;
    results.sweeps[g] = { n: rs.length };
    for (const p of PATHS) results.sweeps[g][p] = stats(rs.map((r) => r[p]));
    log(`S ${g}: n=${rs.length}; ${PATHS.map((p) => `${p} min/p50/mean/max ${results.sweeps[g][p].min}/${results.sweeps[g][p].p50}/${results.sweeps[g][p].mean.toFixed(0)}/${results.sweeps[g][p].max}`).join('; ')}`);
  }
  results.planted = recs.filter((r) => !r.set.startsWith('sweep'));
  const lA = await t.last();
  results.stages.after = { mut: Number(lA.mut), gc: Number(lA.gc) };
  results.mismatches = bad; save();
  log(`done; mismatches ${bad}`);
} catch (e) { log(`FAILED ${e.stack ?? e}`); process.exitCode = 1; }
finally { try { await pic.tearDown(); } catch {} try { await server.stop(); } catch {} }
if (bad) process.exitCode = 2;
