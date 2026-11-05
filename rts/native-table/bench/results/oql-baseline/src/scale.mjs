// Scale driver: the generated ledger (bench/scale `bigRow`) bulk-loaded
// through OQL's shipped producer (tools/oql-ingest uploadRows / putIndexSegment)
// into `ledger_unindexed`, its account_from (or, with COL=day, day) #hash region index uploaded in
// segments of ceil(rows / K) rows, then queries (real query calls), on-chain delta appends (the
// mixed composition), the emptying delete and Table deletes (lone, update context).
//   [COL=day] [DK_KILLS=<d,d,...>] node scale.mjs <oql dir> <out dir> <rows> <K> <deltas,comma> <stages,comma> [N]
// K: the region index is uploaded in segments of ceil(rows / K) rows, which is K
// segments, or fewer when the last of K such segments would hold no row (at 100,000
// rows with K = 1,024: 1,021 segments, 1,020 of 98 rows and a last one of 40).
// deltas: the cumulative row counts appended on chain for stage delta ('' for none).
// stages (default load,q0,index,q1,delta,emptydel,tabledel), run in this order:
//   load            the rows bulk-loaded into ledger_unindexed (putSegment)
//   q0              queries with no index
//   index           the region #hash index uploaded in segments of ceil(rows / K) rows
//   q1              queries with the region index
//   delta           for each D in deltas, rows appended on chain until D rows in
//                   all were added after the load, then three counts (delta<D>)
//   emptydel        N rows with fresh keys appended, each deleted alone
//   tabledel        N lone Table deletes of loaded rows, then 10 x 1,000 per message
//   dk              group count by day with whole days deleted (DK_KILLS)
//   dropbase        dropIndexBase on the account_from index
//   tabledelnobase  tabledel again, on other rows, after dropbase
// N: messages per lone sequence (emptydel, tabledel, tabledelnobase), default 8192.
// COL: the column the region index is uploaded on: account_from (default) or day
// (key floor(row / 3,000)); a value other than account_from adds -<COL> to the
// output file names (COL=day: ledger-<rows>-k<K>-day.*).
// DK_KILLS: for stage dk, the number of days dead at each group count after the
// first, ascending (111,222: days 0..110 dead, then days 0..221 dead).
import { writeFileSync, mkdirSync, appendFileSync, readFileSync } from 'node:fs';
import { IDL } from '@dfinity/candid';
import { startPic, settle, stats, relayIdl, natValue } from './lib.mjs';

const [OQLDIR, OUTDIR, ROWS_S, K_S, DELTAS_S, STAGES_S, NSEQ_S] = process.argv.slice(2);
const ROWS = Number(ROWS_S), K = Number(K_S);
const DELTAS = (DELTAS_S || '').split(',').filter(Boolean).map(Number);
const STAGES = new Set((STAGES_S || 'load,q0,index,q1,delta,emptydel,tabledel').split(','));
const NSEQ = Number(NSEQ_S || 8192);
const { uploadRows, putIndexSegment } = await import(`${OQLDIR}/tools/oql-ingest/lib.mjs`);
const { bigRow } = await import(`${OQLDIR}/bench/scale/image.mjs`);
mkdirSync(OUTDIR, { recursive: true });
const COL = process.env.COL || 'account_from';
const tag = `ledger-${ROWS}-k${K}${COL === 'account_from' ? '' : '-' + COL}`;
const LOG = `${OUTDIR}/${tag}.log`;
const log = (s) => { const line = `[${new Date().toISOString()}] ${tag} rss=${(process.memoryUsage().rss / 1e6).toFixed(0)}MB ${s}`; console.log(line); appendFileSync(LOG, line + '\n'); };
const out = (name, obj) => appendFileSync(`${OUTDIR}/${tag}.${name}.jsonl`, JSON.stringify(obj) + '\n');
const LEDGER_TYPES = ['#nat', '#nat', '#nat', '#nat', '#nat'];
const TARGET = 'ledger_unindexed';

const JM = IDL.Record({ instrs: IDL.Nat, rows: IDL.Nat, value: IDL.Nat });
const MO = IDL.Record({ instr: IDL.Nat64, pmut: IDL.Nat64, pgc: IDL.Nat64, r: IDL.Nat64 });
const Kind = IDL.Variant({ hash: IDL.Null, ordered: IDL.Null });
const idlFactory = ({ IDL }) => IDL.Service({
  putSegment: IDL.Func([IDL.Text, IDL.Nat, IDL.Vec(IDL.Nat8)], [IDL.Nat], []),
  putIndexChunk: IDL.Func([IDL.Text, IDL.Text, Kind, IDL.Nat, IDL.Nat, IDL.Nat, IDL.Vec(IDL.Nat8)], [IDL.Nat], []),
  commitIndex: IDL.Func([IDL.Text, IDL.Text], [IDL.Nat], []),
  abortIndexUpload: IDL.Func([IDL.Text], [IDL.Bool], []),
  dropIndexBase: IDL.Func([IDL.Text, IDL.Text, IDL.Nat], [IDL.Bool], []),
  indexCoverage: IDL.Func([IDL.Text, IDL.Text], [IDL.Opt(IDL.Nat)], ['query']),
  rows: IDL.Func([IDL.Text], [IDL.Opt(IDL.Nat)], ['query']),
  oqlbarsLastImportInstr: IDL.Func([], [IDL.Nat64], ['query']),
  appendBig: IDL.Func([IDL.Nat, IDL.Nat], [IDL.Nat], []),
  op: IDL.Func([IDL.Nat64, IDL.Nat64, IDL.Nat64], [MO], []),
  tick: IDL.Func([], [], []),
  last: IDL.Func([], [IDL.Record({ mut: IDL.Nat64, gc: IDL.Nat64 })], ['query']),
  measure: IDL.Func([IDL.Text], [JM], ['query']),
  bigStats: IDL.Func([], [IDL.Record({ loaded: IDL.Nat, maxLive: IDL.Nat, reclaimed: IDL.Nat, totalAllocation: IDL.Nat, regionBytes: IDL.Nat, segments: IDL.Nat })], ['query']),
});

const CPI = 2, PAGE = 4000;
const { server, pic } = await startPic(`sc${ROWS}k${K}`);
const results = { rows: ROWS, k: K, oql: OQLDIR, stages: {} };
const save = () => writeFileSync(`${OUTDIR}/${tag}.summary.json`, JSON.stringify(results, null, 2));
try {
  const f = await pic.setupCanister({ idlFactory, wasm: readFileSync(`${OQLDIR}/bench/scale/oqlbars.wasm`), cycles: 10n ** 18n, wasmMemoryLimit: 0n });
  const t = f.actor, cid = f.canisterId;
  const relayF = await pic.setupCanister({ idlFactory: relayIdl, wasm: readFileSync(new URL('./out-1140/relay.wasm', import.meta.url).pathname), cycles: 10n ** 16n });
  const relayRaw = (method, argBytes) => pic.updateCall({ canisterId: relayF.canisterId, method: 'raw', arg: new Uint8Array(IDL.encode([IDL.Principal, IDL.Text, IDL.Vec(IDL.Nat8)], [cid, method, argBytes])) });

  // Cycles model: a message's cost on this canister = FEE + 2 x (mutator + collector + 4000 x pages16 + resid).
  // Through the relay FEE = 5,000,000 exactly, which fixes the noop's pages; the
  // ingress FEE of each method/argument size then follows from the same noop sent as ingress.
  const opArg = (k, a, b) => new Uint8Array(IDL.encode([IDL.Nat64, IDL.Nat64, IDL.Nat64], [BigInt(k), BigInt(a), BigInt(b)]));
  async function calibrateOp() {
    await settle(t);
    const r = [];
    for (let i = 0; i < 5; i++) { const c0 = await pic.getCyclesBalance(cid); await relayRaw('op', opArg(99, 0, 0)); const c1 = await pic.getCyclesBalance(cid); const l = await t.last(); r.push((Number(c0 - c1) - 5_000_000) / 2 - Number(l.mut) - Number(l.gc)); }
    const x0 = r.sort()[2];
    const fees = [];
    for (let i = 0; i < 9; i++) { const c0 = await pic.getCyclesBalance(cid); await t.op(99n, 0n, 0n); const c1 = await pic.getCyclesBalance(cid); const l = await t.last(); fees.push(Number(c0 - c1) - 2 * (Number(l.mut) + Number(l.gc)) - 2 * x0); }
    const fee = fees.sort()[4];
    await settle(t);
    return { x0, fee, fees, r };
  }
  // A lone sequence of op messages: per message in-method, collector, pages16.
  async function loneSeq(name, msgs, FEE) {
    await settle(t);
    const recs = [];
    let c0 = await pic.getCyclesBalance(cid);
    let prev = null;
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
      in_method_plus_gc_mean: (sIn + sGc + Number(tail.spent)) / n, n16: stats(recs.map((x) => x.n16)),
      charged16: stats(recs.map((x) => x.instr + x.gc + PAGE * x.n16)), charged16_mean_with_tail: (sIn + sGc + Number(tail.spent) + PAGE * sN) / n,
      residues: [...new Set(recs.map((x) => x.resid))].slice(0, 8) };
    results.stages[name] = s; save();
    log(`${name}: n=${n} in-method mean ${s.in_method.mean.toFixed(0)} p50 ${s.in_method.p50} max ${s.in_method.max}; +gc mean ${s.in_method_plus_gc_mean.toFixed(0)}; charged16 mean ${s.charged16_mean_with_tail.toFixed(0)} p50 ${s.charged16.p50} max ${s.charged16.max}; residues ${JSON.stringify(s.residues)}`);
    return { recs, s };
  }
  async function q(label, json, stage) {
    try {
      const r = await t.measure(JSON.stringify(json));
      const rec = { stage, label, json, instrs: Number(r.instrs), rows: Number(r.rows), value: natValue(r.value) };
      out('queries', rec); log(`Q ${stage} ${label}: ${Number(r.instrs).toLocaleString('en-US')} instrs, rows ${r.rows}, value ${r.value}`);
      return rec;
    } catch (e) {
      const msg = String(e.message ?? e).split('\n').find((x) => /instructions|trap|rror/.test(x)) ?? String(e).slice(0, 200);
      const rec = { stage, label, json, trapped: true, error: msg.slice(0, 300) };
      out('queries', rec); log(`Q ${stage} ${label}: TRAPPED ${rec.error}`);
      return rec;
    }
  }
  const L = TARGET;
  const cnt = (field, v) => ({ start: L, where: { eq: { field, value: v } }, aggregate: [{ fn: 'count' }] });
  const eq = (field, v, limit) => ({ start: L, where: { eq: { field, value: v } }, ...(limit ? { limit } : {}) });
  const lastDay = Math.floor(ROWS / 3000) - 1;

  // ── load ──
  if (STAGES.has('load')) {
    let gen = 0, bytes = 0, msgs = 0;
    const putSegmentRecs = [];
    const wrapped = {
      putSegment: async (target, at, image) => {
        const c0 = await pic.getCyclesBalance(cid);
        const r = await t.putSegment(target, at, image);
        const c1 = await pic.getCyclesBalance(cid);
        const ins = Number(await t.oqlbarsLastImportInstr());
        const l = await t.last();
        bytes += image.length; msgs++;
        putSegmentRecs.push({ at: Number(at), bytes: image.length, rows: Number(r), instr: ins, mut: Number(l.mut), gc: Number(l.gc), cyc: Number(c0 - c1) });
        return r;
      },
    };
    const t0 = performance.now();
    await uploadRows({ actor: wrapped, target: TARGET, types: LEDGER_TYPES, firstRow: 0, inFlight: 1, next: () => (gen >= ROWS ? null : bigRow(gen++)),
      onProgress: (sent) => { if (sent % 10_000_000 < 40_000) log(`load ${sent}/${ROWS}`); } });
    const wall = (performance.now() - t0) / 1000;
    for (const r of putSegmentRecs) out('putSegment', r);
    const st = await t.bigStats();
    const sIn = putSegmentRecs.reduce((q, x) => q + x.instr, 0), sMut = putSegmentRecs.reduce((q, x) => q + x.mut, 0), sGc = putSegmentRecs.reduce((q, x) => q + x.gc, 0), sCyc = putSegmentRecs.reduce((q, x) => q + x.cyc, 0);
    results.stages.load = { msgs, bytes, wall_s: wall, rows_per_s: ROWS / wall, in_method_total: sIn, in_method_per_byte: sIn / bytes, in_method_per_row: sIn / ROWS,
      whole_msg_per_byte: (sMut + sGc) / bytes, cycles_total: sCyc, cycles_per_byte: sCyc / bytes,
      in_method_per_msg: stats(putSegmentRecs.map((x) => x.instr)), bytes_per_msg: stats(putSegmentRecs.map((x) => x.bytes)),
      region_bytes: Number(st.regionBytes), segments: Number(st.segments), region_bytes_per_row: Number(st.regionBytes) / ROWS };
    save();
    log(`load: ${msgs} putSegment, ${bytes} B, ${wall.toFixed(1)} s; in-method ${(sIn / bytes).toFixed(4)}/B; whole msg ${((sMut + sGc) / bytes).toFixed(4)}/B; cycles ${(sCyc / bytes).toFixed(3)}/B; region ${st.regionBytes} B, ${st.segments} segments`);
  }
  // ── queries with no index ──
  if (STAGES.has('q0')) {
    { const st = await settle(t); log(`settled before q0: ${st.ticks} ticks, collection ${st.spent}`); }
    await q('count(*)', { start: L, aggregate: [{ fn: 'count' }] }, 'q0');
    await q('sum(amount_e8s)', { start: L, aggregate: [{ fn: 'sum', field: 'amount_e8s' }] }, 'q0');
    await q('min(amount_e8s)', { start: L, aggregate: [{ fn: 'min', field: 'amount_e8s' }] }, 'q0');
    await q('max(amount_e8s)', { start: L, aggregate: [{ fn: 'max', field: 'amount_e8s' }] }, 'q0');
    await q(`range day==${lastDay} count (zone maps)`, { start: L, where: { and: [{ ge: { field: 'day', value: lastDay } }, { le: { field: 'day', value: lastDay } }] }, aggregate: [{ fn: 'count' }] }, 'q0');
    await q('count day==0 (zone maps)', cnt('day', 0), 'q0');
    await q('count account_from==123 (no index)', cnt('account_from', 123), 'q0');
    await q('eq account_from==123 limit 5 (no index)', eq('account_from', 123, 5), 'q0');
    await q('eq account_from==123 (no index)', eq('account_from', 123), 'q0');
  }
  // ── index upload ──
  if (STAGES.has('index')) {
    const segRows = Math.ceil(ROWS / K);
    const recs = [];
    const wrapped = {
      putIndexChunk: async (...a) => {
        const c0 = await pic.getCyclesBalance(cid); const r = await t.putIndexChunk(...a); const c1 = await pic.getCyclesBalance(cid);
        const ins = Number(await t.oqlbarsLastImportInstr()); const l = await t.last();
        recs.push({ m: 'putIndexChunk', firstRow: Number(a[3]), segLen: Number(a[4]), off: Number(a[5]), bytes: a[6].length, instr: ins, mut: Number(l.mut), gc: Number(l.gc), cyc: Number(c0 - c1) });
        return r;
      },
      commitIndex: async (...a) => {
        const c0 = await pic.getCyclesBalance(cid); const r = await t.commitIndex(...a); const c1 = await pic.getCyclesBalance(cid);
        const ins = Number(await t.oqlbarsLastImportInstr()); const l = await t.last();
        recs.push({ m: 'commitIndex', rows: Number(r), instr: ins, mut: Number(l.mut), gc: Number(l.gc), cyc: Number(c0 - c1) });
        return r;
      },
      abortIndexUpload: (...a) => t.abortIndexUpload(...a),
    };
    let peak = 0, segBytes = 0, nseg = 0;
    const t0 = performance.now();
    for (let first = 0; first < ROWS; first += segRows) {
      const n = Math.min(segRows, ROWS - first);
      segBytes += await putIndexSegment({ actor: wrapped, target: TARGET, col: COL, colType: '#nat', firstRow: first, rows: n, keyAt: COL === 'day' ? (r) => BigInt(Math.floor((first + r) / 3000)) : (r) => bigRow(first + r)[0], inFlight: 1 });
      peak = Math.max(peak, process.memoryUsage().rss); nseg++;
      if (global.gc) global.gc();
      if (nseg % Math.max(1, Math.floor(K / 8)) === 0 || nseg === K) log(`index segment ${nseg}/${K} committed (producer peak rss ${(peak / 1e6).toFixed(0)} MB)`);
    }
    const wall = (performance.now() - t0) / 1000;
    for (const r of recs) out('index', r);
    const ch = recs.filter((x) => x.m === 'putIndexChunk'), cm = recs.filter((x) => x.m === 'commitIndex');
    const sum = (xs, f) => xs.reduce((q, x) => q + f(x), 0);
    results.stages.index = { k: nseg, segRows, index_bytes: segBytes, wall_s: wall, producer_peak_rss_mb: peak / 1e6,
      chunks: ch.length, chunk_in_method_per_byte: sum(ch, (x) => x.instr) / sum(ch, (x) => x.bytes), chunk_whole_per_byte: sum(ch, (x) => x.mut + x.gc) / sum(ch, (x) => x.bytes), chunk_cycles_per_byte: sum(ch, (x) => x.cyc) / sum(ch, (x) => x.bytes),
      commit_in_method: stats(cm.map((x) => x.instr)), commit_cycles: stats(cm.map((x) => x.cyc)),
      in_method_per_index_byte_all: (sum(ch, (x) => x.instr) + sum(cm, (x) => x.instr)) / segBytes,
      index_bytes_per_row: segBytes / ROWS };
    save();
    log(`index: ${nseg} segments, ${segBytes} B (${(segBytes / ROWS).toFixed(3)} B/row), ${wall.toFixed(1)} s, peak rss ${(peak / 1e6).toFixed(0)} MB; chunk in-method ${results.stages.index.chunk_in_method_per_byte.toFixed(4)}/B; all ${results.stages.index.in_method_per_index_byte_all.toFixed(4)}/B`);
  }
  // ── queries with the region index ──
  if (STAGES.has('q1')) {
    { const st = await settle(t); log(`settled before q1: ${st.ticks} ticks, collection ${st.spent}`); }
    await q('count account_from==123', cnt('account_from', 123), 'q1');
    await q('count account_from==456', cnt('account_from', 456), 'q1');
    await q('count account_from==100001 (absent)', cnt('account_from', 100001), 'q1');
    await q('eq account_from==123', eq('account_from', 123), 'q1');
    await q('eq account_from==123 limit 5', eq('account_from', 123, 5), 'q1');
    await q('eq account_from==100001 (absent)', eq('account_from', 100001), 'q1');
    await q('count(*)', { start: L, aggregate: [{ fn: 'count' }] }, 'q1');
    await q('count day==0 (zone maps)', cnt('day', 0), 'q1');
  }
  // ── mixed compositions: k segments + on-chain delta postings ──
  let appended = ROWS;
  if (STAGES.has('delta')) {
    for (const D of DELTAS) {
      const BATCH = 20_000;
      while (appended < ROWS + D) { const n = Math.min(BATCH, ROWS + D - appended); await t.appendBig(BigInt(appended), BigInt(n)); appended += n; }
      { const st = await settle(t); log(`delta: ${appended - ROWS} rows appended on chain; settled ${st.ticks} ticks, collection ${st.spent}`); }
      await q(`count account_from==123 (k=${K} + delta ${D})`, cnt('account_from', 123), `delta${D}`);
      await q(`count account_from==456 (k=${K} + delta ${D})`, cnt('account_from', 456), `delta${D}`);
      await q(`count account_from==100001 absent (k=${K} + delta ${D})`, cnt('account_from', 100001), `delta${D}`);
    }
  }
  let cal = null;
  if (STAGES.has('emptydel') || STAGES.has('tabledel') || STAGES.has('tabledelnobase')) {
    cal = await calibrateOp();
    results.calibration = cal; save();
    log(`op calibration: noop relay 4000*p0+resid ${cal.x0}; ingress fee ${cal.fee} (${JSON.stringify(cal.fees)})`);
  }
  // ── the emptying delete: NSEQ rows with fresh unique account_from keys appended
  //    on chain (one message), then each deleted, one per message ──
  if (STAGES.has('emptydel')) {
    const FRESH = 10_000_000;
    const m = await t.op(7n, BigInt(FRESH), BigInt(NSEQ));
    const lastPos = Number(m.r), firstPos = lastPos - NSEQ + 1;
    const msgs = Array.from({ length: NSEQ }, (_, i) => [1, firstPos + i, 0]);
    await loneSeq('emptying_delete', msgs, cal.fee);
    await q(`count account_from==${FRESH} after its delete`, cnt('account_from', FRESH), 'emptydel');
  }
  // ── Table deletes: Table.delete of loaded (base) rows, lone, then 1,000 per message ──
  if (STAGES.has('tabledel')) {
    const msgs = Array.from({ length: NSEQ }, (_, i) => [1, (7919 * i + 13) % ROWS, 0]);
    await loneSeq('table_delete_lone_base', msgs, cal.fee);
    const bm = Array.from({ length: 10 }, (_, i) => [2, 1_000_003 * (i + 1) + 7, 1000]);
    const b = await loneSeq('table_delete_batch1000_base', bm, cal.fee);
    results.stages.table_delete_batch1000_base.per_delete = { in_method_plus_gc_mean: b.s.in_method_plus_gc_mean / 1000, charged16_mean_with_tail: b.s.charged16_mean_with_tail / 1000 };
    save();
  }
  // ── dk: group count by `day` (region-indexed when COL=day) with dead keys. A day
  //    is killed by deleting all its 3,000 rows (op 9, one message per day); the group
  //    count is measured with every day live, then once for each count in DK_KILLS.
  if (STAGES.has('dk')) {
    await settle(t);
    const days = Math.floor(ROWS / 3000);
    const gq = { start: L, groupBy: ['day'], aggregate: [{ fn: 'count' }] };
    await q(`group count by day, ${days} live`, gq, 'dk');
    const kills = (process.env.DK_KILLS || '').split(',').filter(Boolean).map(Number);
    let killed = 0;
    for (const target of kills) {
      while (killed < target) { await t.op(9n, BigInt(killed * 3000), 3000n); killed++; }
      await settle(t);
      await q(`group count by day, ${days - killed} live, ${killed} dead`, gq, 'dk');
    }
  }
  if (STAGES.has('dropbase')) {
    const cov = await t.indexCoverage(TARGET, 'account_from');
    const ok = await t.dropIndexBase(TARGET, 'account_from', cov[0]);
    log(`dropIndexBase: ${ok} (coverage ${cov[0]})`);
  }
  if (STAGES.has('tabledelnobase')) {
    const msgs = Array.from({ length: NSEQ }, (_, i) => [1, (7919 * i + 101) % ROWS, 0]);
    await loneSeq('table_delete_lone_nobase', msgs, cal.fee);
    const bm = Array.from({ length: 10 }, (_, i) => [2, 1_000_003 * (i + 1) + 101, 1000]);
    const b = await loneSeq('table_delete_batch1000_nobase', bm, cal.fee);
    results.stages.table_delete_batch1000_nobase.per_delete = { in_method_plus_gc_mean: b.s.in_method_plus_gc_mean / 1000, charged16_mean_with_tail: b.s.charged16_mean_with_tail / 1000 };
    save();
  }
  log('done');
} catch (e) {
  log(`FAILED ${e.stack ?? e}`);
  process.exitCode = 1;
} finally {
  try { await pic.tearDown(); } catch {}
  try { await server.stop(); } catch {}
}
