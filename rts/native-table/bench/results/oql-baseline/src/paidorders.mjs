// The paid-orders query (count and sum(amount) where paid and amount >= 500 over
// an IndexedMap of orders) in update context, collection counted. Per row count
// and index choice, a fresh harness canister: the orders loaded in 50,000-row
// messages (not measured); `tick` until the collector is idle (lib.mjs `settle`),
// that warm-up's collection not counted; five `ordQuery` messages back to back
// through the relay, each one's in-method count (its reply) and collector
// instructions (`last`, a query right after it); `tick` until idle again, the
// ticks' collection added as the tail. Nothing is subtracted.
//   node paidorders.mjs <wasm dir built by build2.sh> <out dir> <n,n> <indexed:0|1,...>
import { writeFileSync, mkdirSync, appendFileSync } from 'node:fs';
import { IDL } from '@dfinity/candid';
import { startPic, install, relayIdl, settle, whIdl } from './lib.mjs';
const [WASM, OUTDIR, NS, IXS] = process.argv.slice(2);
mkdirSync(OUTDIR, { recursive: true });
const log = (s) => { const l = `[${new Date().toISOString()}] ${s}`; console.log(l); appendFileSync(`${OUTDIR}/paidorders.log`, l + '\n'); };
const M = IDL.Record({ instr: IDL.Nat64, pmut: IDL.Nat64, pgc: IDL.Nat64, hb: IDL.Nat, ha: IDL.Nat });
const idl = ({ IDL }) => IDL.Service({ ordLoad: IDL.Func([IDL.Bool, IDL.Nat64, IDL.Nat64], [IDL.Nat], []), tick: IDL.Func([], [], []),
  last: IDL.Func([], [IDL.Record({ mut: IDL.Nat64, gc: IDL.Nat64 })], ['query']) });
const { server, pic } = await startPic('paidorders');
try {
  const r = await install(pic, `${WASM}/relay.wasm`, relayIdl);
  for (const n of NS.split(',').map(Number)) for (const ix of IXS.split(',').map((x) => x === '1')) {
    const t = await install(pic, `${WASM}/wh.wasm`, idl);
    for (let off = 0; off < n; off += 50_000) await t.actor.ordLoad(ix, BigInt(off), BigInt(Math.min(50_000, n - off)));
    const warm = await settle(t.actor);
    const runs = [];
    for (let i = 0; i < 5; i++) { const m = await r.actor.batch(t.canisterId, 'ordQuery', 0n, 0n); const l = await t.actor.last(); runs.push({ instr: Number(m.instr), gc: Number(l.gc), rows: Number(m.hb) }); }
    const tail = await settle(t.actor);
    const sIn = runs.reduce((a, x) => a + x.instr, 0), sGc = runs.reduce((a, x) => a + x.gc, 0) + Number(tail.spent);
    const rec = { n, indexed: ix, warmTicks: warm.ticks, runs, tailGc: Number(tail.spent), tailTicks: tail.ticks, mean_in_method: sIn / 5, mean_with_gc: (sIn + sGc) / 5, per_row_in_method: sIn / 5 / n, per_row_with_gc: (sIn + sGc) / 5 / n };
    appendFileSync(`${OUTDIR}/paidorders.jsonl`, JSON.stringify(rec) + '\n');
    log(`n=${n} indexed=${ix}: in-method ${JSON.stringify(runs.map((x) => x.instr))} gc ${JSON.stringify(runs.map((x) => x.gc))} tail ${tail.spent}; per row ${rec.per_row_in_method.toFixed(0)} in the method, ${rec.per_row_with_gc.toFixed(0)} with collection`);
  }
} finally { try { await pic.tearDown(); } catch {} try { await server.stop(); } catch {} }
