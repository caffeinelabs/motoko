// OQL's group count over uploaded region-hash segments with dead keys.
// Table `dkt` (key = row mod K, v = row), N rows bulk-loaded through OQL's
// producer, the `key` region #hash index uploaded in S segments (every key in
// every segment). Group count by key (count) with all K keys live; then every row
// of keys 3 .. K-1 deleted (D = K - 3 dead keys, 3 live), group count again.
//   node dk2.mjs <oql dir> <out dir> <N> <S> <D,D,...>
// Canister: <oql dir>/bench/scale/oqlbars-deadkeys.wasm, built from oqlbars-deadkeys.mo
// (oqlbars.mo plus the table `dkt`); the other drivers use oqlbars.wasm, without it.
import { mkdirSync, appendFileSync, readFileSync } from 'node:fs';
import { IDL } from '@dfinity/candid';
import { startPic, settle, natValue } from './lib.mjs';

const [OQLDIR, OUTDIR, N_S, SEG_S, DS] = process.argv.slice(2);
const N = Number(N_S), SEG = Number(SEG_S);
const { uploadRows, putIndexSegment } = await import(`${OQLDIR}/tools/oql-ingest/lib.mjs`);
mkdirSync(OUTDIR, { recursive: true });
const tag = `dk2-${N}-s${SEG}`;
const log = (s) => { const l = `[${new Date().toISOString()}] ${tag} ${s}`; console.log(l); appendFileSync(`${OUTDIR}/${tag}.log`, l + '\n'); };
const out = (o) => appendFileSync(`${OUTDIR}/${tag}.queries.jsonl`, JSON.stringify(o) + '\n');
const JM = IDL.Record({ instrs: IDL.Nat, rows: IDL.Nat, value: IDL.Nat });
const Kind = IDL.Variant({ hash: IDL.Null, ordered: IDL.Null });
const idlFactory = ({ IDL }) => IDL.Service({
  putSegment: IDL.Func([IDL.Text, IDL.Nat, IDL.Vec(IDL.Nat8)], [IDL.Nat], []),
  putIndexChunk: IDL.Func([IDL.Text, IDL.Text, Kind, IDL.Nat, IDL.Nat, IDL.Nat, IDL.Vec(IDL.Nat8)], [IDL.Nat], []),
  commitIndex: IDL.Func([IDL.Text, IDL.Text], [IDL.Nat], []),
  abortIndexUpload: IDL.Func([IDL.Text], [IDL.Bool], []),
  dktDeleteKey: IDL.Func([IDL.Nat, IDL.Nat, IDL.Nat], [IDL.Nat], []),
  tick: IDL.Func([], [], []),
  last: IDL.Func([], [IDL.Record({ mut: IDL.Nat64, gc: IDL.Nat64 })], ['query']),
  measure: IDL.Func([IDL.Text], [JM], ['query']),
});
const { server, pic } = await startPic(`dk${N}s${SEG}`);
try {
  for (const D of DS.split(',').map(Number)) {
    const K = 3 + D;
    const f = await pic.setupCanister({ idlFactory, wasm: readFileSync(`${OQLDIR}/bench/scale/oqlbars-deadkeys.wasm`), cycles: 10n ** 18n, wasmMemoryLimit: 0n });
    const t = f.actor;
    let g = 0;
    await uploadRows({ actor: t, target: 'dkt', types: ['#nat', '#nat'], firstRow: 0, inFlight: 1, next: () => (g >= N ? null : [BigInt(g % K), BigInt(g++)]) });
    const segRows = Math.ceil(N / SEG);
    for (let first = 0; first < N; first += segRows) {
      await putIndexSegment({ actor: t, target: 'dkt', col: 'key', colType: '#nat', firstRow: first, rows: Math.min(segRows, N - first), keyAt: (r) => BigInt((first + r) % K), inFlight: 1 });
    }
    await settle(t);
    const gq = JSON.stringify({ start: 'dkt', groupBy: ['key'], aggregate: [{ fn: 'count' }] });
    const m0 = await t.measure(gq);
    const r0 = { K, D, live: K, dead: 0, segments: SEG, rows: N, instrs: Number(m0.instrs), groups: Number(m0.rows) };
    out(r0); log(`K=${K}: group count, ${K} live: ${r0.instrs.toLocaleString('en-US')} instrs, ${r0.groups} groups`);
    for (let key = 3; key < K; key++) await t.dktDeleteKey(BigInt(key), BigInt(K), BigInt(N));
    await settle(t);
    const m1 = await t.measure(gq);
    const r1 = { K, D, live: 3, dead: D, segments: SEG, rows: N, instrs: Number(m1.instrs), groups: Number(m1.rows) };
    out(r1); log(`K=${K}: group count, 3 live + ${D} dead: ${r1.instrs.toLocaleString('en-US')} instrs, ${r1.groups} groups`);
    const cq = JSON.stringify({ start: 'dkt', where: { eq: { field: 'key', value: 0 } }, aggregate: [{ fn: 'count' }] });
    const m2 = await t.measure(cq);
    out({ K, D, query: 'count key = 0', segments: SEG, instrs: Number(m2.instrs), value: natValue(m2.value) });
    log(`K=${K}: count key = 0: ${Number(m2.instrs).toLocaleString('en-US')} instrs, value ${m2.value}`);
    try { await pic.stopCanister({ canisterId: f.canisterId }); await pic.deleteCanister({ canisterId: f.canisterId }); } catch {}
  }
  log('done');
} catch (e) { log(`FAILED ${e.stack ?? e}`); process.exitCode = 1; }
finally { try { await pic.tearDown(); } catch {} try { await server.stop(); } catch {} }
