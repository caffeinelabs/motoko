// Lone-write sequences on OQL: sequences of consecutive lone messages.
//   node wseq.mjs <wasm dir> <out dir> <cfg> <rows> [N] [samples]
// wasm dir: holds wh.wasm and relay.wasm (src/build.sh). N: messages per
// sequence (default 8192). samples: messages per sequence in pass B (default 32;
// 0 skips pass B).
// cfg: cm | cmt | im0 | im1 | im2 | im3 | im1a | tb | tba | tbt
//
// One canister, pre-filled with `rows` rows (keys 0..rows-1), then the
// sequences of N consecutive lone messages: insert (new keys rows..rows+N-1;
// Table: append, N >= 50,000 for the index-free Table so one flush falls inside),
// replace (core Map / IndexedMap only; key p(i) = (7919 i + 13) mod rows, the
// replace moves ts), delete (keys p(i)). `tick` until the collector is idle
// (< 100,000 for three messages) before the first sequence and after each.
//
// Pass A (every message): in-method instructions (performanceCounter(0) around
// the call), the message's mutator and collector instructions (Motoko runtime,
// read back in the next message), the cycles the message cost on the target
// (each `op` is sent to it directly as an ingress message; the ingress fee,
// calibrated by lib.mjs `calibrateIngress`, is subtracted), the 16 KiB host pages
// derived from them (PocketIC on this Apple-silicon host tracks dirty memory per
// 16 KiB host page: 4 writes 4 KiB apart cost one page fee), and a structural
// flag: the target's `sh(k)`, a hash of every B-tree path the key touches,
// differs before and after the message (a B-tree split or merge on a touched
// path, or a Table flush). Pass B (if samples > 0): the same history replayed on a fresh
// canister, with canister snapshots before and after every sampled message, to
// count the 4 KiB pages it changed (lower bound: changed pages + the dirty host
// pages that hold no changed byte).
import { writeFileSync, mkdirSync, appendFileSync } from 'node:fs';
import { startPic, install, whIdl, relayIdl, settle, stats, calibrateIngress } from './lib.mjs';
import { Snapper, diffPages } from './snap.mjs';

const [WASM, OUTDIR, CFG, ROWS_S, N_S, SAMPLES_S] = process.argv.slice(2);
const ROWS = Number(ROWS_S);
const N = Number(N_S || 8192);
const SAMPLES = Number(SAMPLES_S ?? 32);
const CPI = 2, PAGE = 4000;
mkdirSync(OUTDIR, { recursive: true });
const tag = `${CFG}-${ROWS}`;
const log = (s) => { const line = `[${new Date().toISOString()}] ${tag} ${s}`; console.log(line); appendFileSync(`${OUTDIR}/${tag}.log`, line + '\n'); };

const isTable = CFG.startsWith('tb');
const prefillBatch = { cm: 50_000, cmt: 20_000, im0: 20_000, im1: 20_000, im2: 10_000, im3: 10_000, im1a: 20_000, tb: 10_000, tba: 10_000, tbt: 10_000 }[CFG];
const perm = (i) => (i * 7919 + 13) % ROWS;
const seqs = isTable
  ? [['insert', 0, (i) => ROWS + i, N], ['delete', 2, (i) => perm(i), N]]
  : [['insert', 0, (i) => ROWS + i, N], ['replace', 1, (i) => perm(i), N], ['delete', 2, (i) => perm(i), N]];

async function pass(pic, sampling, onSeqDone) {
  const t = await install(pic, `${WASM}/wh.wasm`, whIdl);
  const r = await install(pic, `${WASM}/relay.wasm`, relayIdl);
  const snap = new Snapper(pic, t.canisterId);
  await t.actor.setup(CFG);
  for (let off = 0; off < ROWS; off += prefillBatch) {
    await t.actor.prefill(BigInt(off), BigInt(Math.min(prefillBatch, ROWS - off)));
    if (!sampling && (off / prefillBatch) % 20 === 0) log(`prefill ${off + Math.min(prefillBatch, ROWS - off)}/${ROWS}`);
  }
  const st0 = await t.actor.stats();
  await settle(t.actor);
  const cal = await calibrateIngress(pic, t, r);
  if (cal.feeAgree < 15) throw new Error('fee calibration not constant');
  const FEE = cal.fee;  // ingress fixed fee: 5,000,000 execution + induction (1,200,000 + 2,000 per byte)
  const warm = await settle(t.actor);
  if (!sampling) log(`prefilled: size=${st0.size} heap=${st0.heap} segs=${st0.segs}; noop through relay 4000*p0+resid = ${cal.x0}; ingress fixed fee ${FEE} (${cal.feeAgree}/20 agree); warm-up ${warm.ticks} ticks, gc ${warm.spent}`);
  const head = { fee: FEE, noopRelayX: cal.x0, warmup: { ticks: warm.ticks, gc: Number(warm.spent) }, prefilled: { size: Number(st0.size), heap: Number(st0.heap) } };

  for (const [name, kind, keyOf, nFull] of seqs) {
    const n = sampling ? SAMPLES : nFull;
    const recs = [];
    const every = 1;
    let c0 = await pic.getCyclesBalance(t.canisterId);
    let prev = null;   // the previous record, completed by the next reply's pmut/pgc
    const finish = (rec, mut, gc) => {
      rec.mut = mut; rec.gc = gc;
      const pagesF = (rec.cyc - FEE) / CPI / PAGE - (mut + gc) / PAGE;
      rec.n16 = Math.round(pagesF);
      rec.resid = Math.round((pagesF - rec.n16) * PAGE);
      if (rec.p4diff !== undefined) rec.p4lo = rec.p4diff + Math.max(0, rec.n16 - rec.h16);
    };
    for (let i = 0; i < n; i++) {
      const k = keyOf(i);
      const sampled = sampling && i % every === 0;
      let memB = null;
      if (sampled) {
        if (prev) { const l = await t.actor.last(); finish(prev, Number(l.mut), Number(l.gc)); prev = null; }
        memB = await snap.take(); c0 = await pic.getCyclesBalance(t.canisterId);
      }
      const hb = sampling ? 0n : await t.actor.sh(BigInt(k));
      const m = await t.actor.op(BigInt(kind), BigInt(k));
      const c1 = await pic.getCyclesBalance(t.canisterId);
      const ha = sampling ? 0n : await t.actor.sh(BigInt(k));
      if (prev) finish(prev, Number(m.pmut), Number(m.pgc));
      const rec = { i, k, instr: Number(m.instr), cyc: Number(c0 - c1), structural: hb !== ha };
      if (sampled) {
        const l = await t.actor.last(); finish(rec, Number(l.mut), Number(l.gc));
        const memA = await snap.take();
        const d = diffPages(memB, memA);
        rec.p4diff = d.p4; rec.h16 = d.h16; rec.p4lo = d.p4 + Math.max(0, rec.n16 - d.h16);
        await snap.drop();
        c0 = await pic.getCyclesBalance(t.canisterId);
        prev = null;
      } else { c0 = c1; prev = rec; }
      recs.push(rec);
      if (!sampling && i % 4096 === 0) log(`${name} ${i}/${n} instr=${rec.instr}`);
    }
    if (prev) { const l = await t.actor.last(); finish(prev, Number(l.mut), Number(l.gc)); prev = null; }
    const tail = await settle(t.actor);
    await onSeqDone(name, n, recs, tail, head);
  }
  const st1 = await t.actor.stats();
  return { final: { size: Number(st1.size), heap: Number(st1.heap), segs: Number(st1.segs) } };
}

const { server, pic } = await startPic(`${CFG}${ROWS}`);
const summary = { cfg: CFG, rows: ROWS, wasm: WASM, n: N, samples: SAMPLES, seqs: {} };
try {
  // Pass A: every message.
  const A = {};
  const resA = await pass(pic, false, async (name, n, recs, tail, head) => {
    summary.head = head;
    A[name] = recs;
    writeFileSync(`${OUTDIR}/${tag}-${name}.jsonl`, recs.map((x) => JSON.stringify(x)).join('\n') + '\n');
    const sumIn = recs.reduce((a, x) => a + x.instr, 0), sumGc = recs.reduce((a, x) => a + x.gc, 0), sumN16 = recs.reduce((a, x) => a + x.n16, 0);
    const ch16 = recs.map((x) => x.instr + x.gc + PAGE * x.n16);
    const s = {
      n, tail: { ticks: tail.ticks, gc: Number(tail.spent) },
      in_method: stats(recs.map((x) => x.instr)),
      gc_per_msg: stats(recs.map((x) => x.gc)),
      in_method_plus_gc_mean: (sumIn + sumGc + Number(tail.spent)) / n,
      in_method_plus_gc: stats(recs.map((x) => x.instr + x.gc)),
      n16: stats(recs.map((x) => x.n16)),
      charged16: stats(ch16),
      charged16_mean_with_tail: (sumIn + sumGc + Number(tail.spent) + PAGE * sumN16) / n,
      whole_msg_mean: (recs.reduce((a, x) => a + x.mut, 0) + sumGc + Number(tail.spent)) / n,
      structural_msgs: recs.filter((x) => x.structural).length,
      max_charged16_nonstructural: Math.max(...recs.filter((x) => !x.structural).map((x) => x.instr + x.gc + PAGE * x.n16)),
      max_charged16_structural: Math.max(0, ...recs.filter((x) => x.structural).map((x) => x.instr + x.gc + PAGE * x.n16)),
      residues: [...new Set(recs.map((x) => x.resid))].slice(0, 10),
    };
    summary.seqs[name] = s;
    log(`${name}: in-method mean ${s.in_method.mean.toFixed(0)} p50 ${s.in_method.p50} max ${s.in_method.max}; +gc mean ${s.in_method_plus_gc_mean.toFixed(0)}; charged16 mean ${s.charged16_mean_with_tail.toFixed(0)} p50 ${s.charged16.p50} max ${s.charged16.max}; structural ${s.structural_msgs}; residues ${JSON.stringify(s.residues)}`);
    writeFileSync(`${OUTDIR}/${tag}.summary.json`, JSON.stringify(summary, null, 2));
  });
  summary.final = resA.final;
  writeFileSync(`${OUTDIR}/${tag}.summary.json`, JSON.stringify(summary, null, 2));
  // Pass B: the same history with snapshots on sampled messages.
  if (SAMPLES > 0) {
    log(`pass B (fresh canister, the same history up to the sequences, then ${SAMPLES} messages per sequence, each between two snapshots)`);
    await pass(pic, true, async (name, n, recs) => {
      const smp = recs.filter((x) => x.p4diff !== undefined);
      const a = A[name];
      const mismatch = smp.filter((x) => a[x.i].n16 !== x.n16 || a[x.i].instr !== x.instr).length;  // same key, same state for the first message only
      writeFileSync(`${OUTDIR}/${tag}-${name}.sample.jsonl`, smp.map((x) => JSON.stringify(x)).join('\n') + '\n');
      const ratio = smp.reduce((s, x) => s + x.p4lo, 0) / smp.reduce((s, x) => s + x.n16, 0);
      const s = summary.seqs[name];
      const sumIn = a.reduce((q, x) => q + x.instr, 0), sumGc = a.reduce((q, x) => q + x.gc, 0), sumN16 = a.reduce((q, x) => q + x.n16, 0);
      s.sample = {
        m: smp.length, mismatch_vs_passA: mismatch,
        p4diff: stats(smp.map((x) => x.p4diff)), h16: stats(smp.map((x) => x.h16)), p4lo: stats(smp.map((x) => x.p4lo)), n16: stats(smp.map((x) => x.n16)),
        charged4lo: stats(smp.map((x) => x.instr + x.gc + PAGE * x.p4lo)),
        ratio_p4lo_over_n16: ratio,
      };
      s.charged4est_mean_with_tail = (sumIn + sumGc + Number(s.tail.gc) + PAGE * sumN16 * ratio) / a.length;
      log(`${name} sample: m=${smp.length} mismatch=${mismatch} p4lo mean ${s.sample.p4lo.mean.toFixed(2)} n16 mean ${s.sample.n16.mean.toFixed(2)} ratio ${ratio.toFixed(3)} charged4est mean ${s.charged4est_mean_with_tail.toFixed(0)}`);
      writeFileSync(`${OUTDIR}/${tag}.summary.json`, JSON.stringify(summary, null, 2));
    });
  }
  log('done');
} finally {
  try { await pic.tearDown(); } catch {}
  try { await server.stop(); } catch {}
}
