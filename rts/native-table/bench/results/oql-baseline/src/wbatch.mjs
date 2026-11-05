// Batched IndexedMap writes (OQL's index_write and backfill bench shapes) in
// update context with Motoko's collection counted, the collector settled before and after.
//   node wbatch.mjs <wasm dir> <out dir> <N,N,...> [cells]
// <wasm dir> holds wh.wasm and relay.wasm (src/build.sh or src/build2.sh); N: the
// row counts, each run through every cell. cells (optional): a prefix such as
// buildR runs only the cells whose names start with it; a prefix after a minus
// sign, such as -buildR, runs every cell except those. The buildR_* cells call
// wBuildR, which only the wh.wasm of src/build2.sh (wh2.mo) has.
// Each measured message runs on a fresh canister: setup, (the pre-state, not
// measured), tick until the collector spent < 100,000 for three messages, the
// measured message through the relay (so its cycles carry no ingress fee), its
// collector instructions, then ticks until idle again (tail collection).
// Per op = (in-method + the message's collection + tail collection) / N.
import { writeFileSync, mkdirSync, appendFileSync } from 'node:fs';
import { IDL } from '@dfinity/candid';
import { startPic, install, whIdl, relayIdl, settle } from './lib.mjs';

const [WASM, OUTDIR, NS, ONLY] = process.argv.slice(2);
const SIZES = NS.split(',').map(Number);
mkdirSync(OUTDIR, { recursive: true });
const LOG = `${OUTDIR}/wbatch.log`;
const log = (s) => { const line = `[${new Date().toISOString()}] ${s}`; console.log(line); appendFileSync(LOG, line + '\n'); };
const CPI = 2, PAGE = 4000, FEE = 5_000_000;

// [name, pre-state messages (ingress), measured message (relay batch method, a, b)]
const cells = (n) => [
  ['build_0idx', [], ['wBuild', 0, n]],
  ['build_1idx', [], ['wBuild', 1, n]],
  ['build_2idx', [], ['wBuild', 2, n]],
  ['build_3idx', [], ['wBuild', 3, n]],
  ['reindex_2', [['wBuild', 2, n]], ['wReindex', 0, n]],
  ['delete_2', [['wBuild', 2, n]], ['wDelete', 0, n]],
  ['buildT_0idx', [], ['wBuildT', 0, n]],
  ['buildT_price', [], ['wBuildT', 1, n]],
  ['buildT_comp', [], ['wBuildT', 2, n]],
  ['backfill_price', [['wBuildT', 0, n]], ['wBackfill', 1, n]],
  ['backfill_comp', [['wBuildT', 0, n]], ['wBackfill', 2, n]],
  ['buildR_0idx', [], ['wBuildR', 0, n]],
  ['buildR_1idx', [], ['wBuildR', 1, n]],
  ['buildR_2idx', [], ['wBuildR', 2, n]],
  ['buildR_3idx', [], ['wBuildR', 3, n]],
].filter(([name]) => !ONLY || (ONLY.startsWith('-') ? !name.startsWith(ONLY.slice(1)) : name.startsWith(ONLY)));

const { server, pic } = await startPic('wbatch');
const results = {};
try {
  const r = await install(pic, `${WASM}/relay.wasm`, relayIdl);
  for (const n of SIZES) {
    for (const [name, pre, [method, a, b]] of cells(n)) {
      const t = await install(pic, `${WASM}/wh.wasm`, whIdl);
      await t.actor.setup('x');
      for (const [pm, pa, pb] of pre) {
        if (pm === 'wBuild') await t.actor.wBuild(BigInt(pa), BigInt(pb)); else await t.actor.wBuildT(BigInt(pa), BigInt(pb));
      }
      const warm = await settle(t.actor);
      const c0 = await pic.getCyclesBalance(t.canisterId);
      const m = await r.actor.batch(t.canisterId, method, BigInt(a), BigInt(b));
      const c1 = await pic.getCyclesBalance(t.canisterId);
      const l = await t.actor.last();
      const tail = await settle(t.actor);
      const instr = Number(m.instr), mut = Number(l.mut), gc = Number(l.gc), cyc = Number(c0 - c1), tailGc = Number(tail.spent);
      const pf = (cyc - FEE) / CPI / PAGE - (mut + gc) / PAGE;
      const n16 = Math.round(pf);
      const rec = { name, n, instr, gc, tailGc, tailTicks: tail.ticks, warmTicks: warm.ticks, mut, cyc, n16, resid: Math.round((pf - n16) * PAGE),
        per_op_in_method: instr / n, per_op_with_gc: (instr + gc + tailGc) / n, per_op_charged16: (instr + gc + tailGc + PAGE * n16) / n };
      results[`${name}:${n}`] = rec;
      appendFileSync(`${OUTDIR}/wbatch.jsonl`, JSON.stringify(rec) + '\n');
      log(`${name} n=${n}: in-method ${instr} (${rec.per_op_in_method.toFixed(0)}/op), gc ${gc} + tail ${tailGc} (${tail.ticks} ticks) -> ${rec.per_op_with_gc.toFixed(0)}/op with collection; pages16 ${n16} (resid ${rec.resid}) -> ${rec.per_op_charged16.toFixed(0)}/op charged16`);
      writeFileSync(`${OUTDIR}/wbatch.summary.json`, JSON.stringify(results, null, 2));
      try { await pic.stopCanister({ canisterId: t.canisterId }); await pic.deleteCanister({ canisterId: t.canisterId }); } catch {}
    }
  }
  log('done');
} finally {
  try { await pic.tearDown(); } catch {}
  try { await server.stop(); } catch {}
}
