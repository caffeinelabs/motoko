// After a pure image load, OQL queries over the imported rows must answer
// exactly. Every check compares the canister against the same predicate evaluated
// in JS over bigRow — an independent oracle — so producer min/max that prune a
// segment holding real matches, or cells landed in the wrong column, show up as a
// count mismatch rather than going unnoticed.
//
//   node verify-import.mjs                # 100k rows: every check fits the budget
//   ROWS=1000000 node verify-import.mjs   # pruned/aggregate shapes at 1M
//
// Unindexed full scans and order-by exceed the 5B query instruction budget past a
// few hundred thousand rows — a property of the scan, not of how the rows were
// loaded — so those checks report the limit error at larger ROWS while the
// zone-map-pruned and footer-served shapes keep passing.
import { PocketIc, PocketIcServer } from '@dfinity/pic';
import { IDL } from '@dfinity/candid';
import { bigRow } from './image.mjs';
import { uploadRows } from '../../tools/oql-ingest/lib.mjs';
const fmt = (n) => Number(n).toLocaleString('en-US');
const HERE = new URL('.', import.meta.url).pathname;
process.env.POCKET_IC_BIN = HERE + 'node_modules/@dfinity/pic/pocket-ic';
const ROWS = Number(process.env.ROWS || 100_000);
const IN_FLIGHT = Number(process.env.IN_FLIGHT || 1);

const JM = IDL.Record({ instrs: IDL.Nat, rows: IDL.Nat, value: IDL.Nat });
const IndexStatus = IDL.Record({ columns: IDL.Text, ready: IDL.Bool, walked: IDL.Nat, total: IDL.Nat });
const idlFactory = ({ IDL }) => IDL.Service({
  putSegment: IDL.Func([IDL.Text, IDL.Nat, IDL.Vec(IDL.Nat8)], [IDL.Nat], []),
  buildIndex: IDL.Func([IDL.Text, IDL.Vec(IDL.Text), IDL.Variant({ hash: IDL.Null, ordered: IDL.Null })], [], []),
  indexStatus: IDL.Func([IDL.Text], [IDL.Opt(IDL.Vec(IndexStatus))], ['query']),
  measure: IDL.Func([IDL.Text], [JM], ['query']),
  bigStats: IDL.Func([], [IDL.Record({ loaded: IDL.Nat, maxLive: IDL.Nat, reclaimed: IDL.Nat, totalAllocation: IDL.Nat, regionBytes: IDL.Nat, segments: IDL.Nat })], ['query']),
  __motoko_gc_trigger: IDL.Func([], [], []),
});

// Retained heap = totalAllocation - reclaimed once the collector has nothing
// left to free (same technique as bench.mjs) — the index build is the first
// consumer of real heap in this canister, so this is the number that decides
// how large an indexed table can get.
async function retainedHeap(actor) {
  // A fixed number of increments, not a stability heuristic: the incremental
  // collector's `reclaimed` PAUSES while a cycle is still marking, so "three
  // stable reads" exits mid-cycle and over-reads by whatever the sweep had
  // left — hundreds of MB at tens of millions of indexed rows. 300 increments
  // runs any cycle at these heap sizes to completion.
  for (let i = 0; i < 300; i++) await actor.__motoko_gc_trigger();
  const st = await actor.bigStats();
  return Number(st.totalAllocation - st.reclaimed);
}

const server = await PocketIcServer.start();
// Late index-build rounds run a forced full GC over a growing heap (the wasm is
// compiled --force-gc), so a single round can take minutes at tens of millions
// of indexed rows; the client must outwait that rather than declare the round
// stuck.
const pic = await PocketIc.create(server.getUrl(), { processingTimeoutMs: 3_600_000 });
try {
  const { actor } = await pic.setupCanister({ idlFactory, wasm: HERE + 'bench.wasm' });
  // Through the shipped producer, so this verifies the encoder and the putSegment
  // protocol the CLI uses rather than a second copy of them.
  let generated = 0;
  const loaded = await uploadRows({
    actor, target: 'ledger_unindexed', types: ['#nat', '#nat', '#nat', '#nat', '#nat'], firstRow: 0, inFlight: IN_FLIGHT,
    next: () => (generated >= ROWS ? null : bigRow(generated++)),
  });
  if (loaded !== ROWS) throw new Error(`loaded ${loaded} rows, expected ${ROWS}`);

  // JS oracle over the same generated rows.
  let sum = 0n, day7 = 0, day10to12 = 0, from123 = 0, day7small = 0, spenderNull = 0;
  for (let k = 0; k < ROWS; k++) {
    const [from, , sp, amt, day] = bigRow(k);
    sum += amt;
    if (day === 7n) day7++;
    if (day >= 10n && day <= 12n) day10to12++;
    if (from === 123n) from123++;
    if (day === 7n && amt < 500_000_000_000n) day7small++;
    if (sp === null) spenderNull++;
  }

  const q = (o) => actor.measure(JSON.stringify(o));
  const S = 'ledger_unindexed';
  // count-with-where: the predicate runs per row over the imported cells, but no
  // row is materialized, so a broad match stays inside the query budget.
  const cnt = (where) => q({ start: S, where, aggregate: [{ fn: 'count' }] }).then(r => r.value);
  const checks = [
    ['count(*)',                  () => q({ start: S, aggregate: [{ fn: 'count' }] }).then(r => r.value), ROWS],
    ['sum(amount_e8s)',           () => q({ start: S, aggregate: [{ fn: 'sum', field: 'amount_e8s' }] }).then(r => r.value), sum],
    ['day = 7 (zone-map pruned)', () => cnt({ eq: { field: 'day', value: 7 } }), day7],
    ['10 <= day <= 12 (pruned)',  () => cnt({ and: [{ ge: { field: 'day', value: 10 } }, { le: { field: 'day', value: 12 } }] }), day10to12],
    ['account_from = 123 (scan)', () => cnt({ eq: { field: 'account_from', value: 123 } }), from123],
    ['day = 7 & amount < 5e11',   () => cnt({ and: [{ eq: { field: 'day', value: 7 } }, { lt: { field: 'amount_e8s', value: 500000000000 } }] }), day7small],
    ['spender is null (scan)',    () => cnt({ eq: { field: 'account_spender', value: null } }), spenderNull],
    ['materialize: rows of day = 7', () => q({ start: S, where: { eq: { field: 'day', value: 7 } } }).then(r => r.rows), day7],
    ['top-5 by day desc',         () => q({ start: S, orderBy: [{ field: 'day', dir: 'desc' }], limit: 5 }).then(r => r.rows), 5],
  ];
  let bad = 0;
  const run1 = async ([name, run, want]) => {
    try {
      const got = await run();
      const ok = BigInt(got) === BigInt(want);
      if (!ok) bad++;
      console.log(`${ok ? 'OK  ' : 'FAIL'} ${name}: got ${got}, expected ${want}`);
    } catch (e) {
      bad++;
      console.log(`FAIL ${name}: ${String(e.message ?? e).split('\n')[0]}`);
    }
  };
  for (const c of checks) await run1(c);

  // Phase 2 — build an index over the loaded rows, then re-ask the scan-bound
  // question. The build runs on a self-rescheduling timer; ticks drive it. The
  // instruction drop is the point: the same count answered by postings instead
  // of a scan of every row.
  const scanInstr = await q({ start: S, where: { eq: { field: 'account_from', value: 123 } }, aggregate: [{ fn: 'count' }] })
    .then(r => r.instrs).catch(() => null);   // null: scan exceeded the query budget
  await actor.buildIndex(S, ['account_from'], { hash: null });
  const t0 = performance.now();
  let ticks = 0;
  for (; ticks < 5000; ticks++) {
    await pic.tick();
    const [st] = await actor.indexStatus(S);
    if (st?.length && st.every(e => e.ready)) break;
    if (ticks % 25 === 24) {
      const [p] = await actor.indexStatus(S);
      const h = await actor.bigStats();
      console.log(`  building: ${fmt(p[0].walked)}/${fmt(p[0].total)} (round ${ticks + 1}, ${((performance.now() - t0) / 1000).toFixed(0)} s, `
        + `heap ${((Number(h.totalAllocation - h.reclaimed)) / 1e6).toFixed(0)} MB unsettled, maxLive ${(Number(h.maxLive) / 1e6).toFixed(0)} MB)`);
    }
  }
  console.log(`index build took ${((performance.now() - t0) / 1000).toFixed(1)} s over ${ticks + 1} rounds`);
  const [st] = await actor.indexStatus(S);
  console.log(`index build: ${st.map(e => `${e.columns} ready=${e.ready} ${e.walked}/${e.total}`).join('; ')}`);
  const after = await q({ start: S, where: { eq: { field: 'account_from', value: 123 } }, aggregate: [{ fn: 'count' }] });
  await run1(['account_from = 123 (indexed)', async () => after.value, from123]);
  console.log(`  scan: ${scanInstr === null ? 'exceeded the query budget' : fmt(scanInstr) + ' instrs'} -> indexed: ${fmt(after.instrs)} instrs`);
  console.log(`  heap retained after build: ${(await retainedHeap(actor) / 1e6).toFixed(1)} MB`);
  process.exitCode = bad ? 1 : 0;
} finally { await pic.tearDown(); await server.stop(); }
