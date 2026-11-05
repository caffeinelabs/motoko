// the jcust / jord join tables (rows and query set of OQL's scale runs:
// jcust row k = (region = k mod 100,000, tier = k mod 3), jord row k =
// (customerId = k mod CUST, amount = k mod 1,000, status = floor(k / CUST) mod 2,
// day = floor(k / (ORDERS / 1,000)))), bulk-loaded through OQL's shipped producer,
// region #hash indexes uploaded on jcust.region, jord.customerId and jord.status
// (32,000,000 rows per index segment), then the twelve queries as real query calls.
//   node jord.mjs <oql dir> <out dir> <orders> <customers>
import { mkdirSync, appendFileSync, readFileSync, writeFileSync } from 'node:fs';
import { IDL } from '@dfinity/candid';
import { startPic, natValue } from './lib.mjs';

const [OQLDIR, OUTDIR, ORD_S, CUST_S] = process.argv.slice(2);
const ORDERS = Number(ORD_S), CUST = Number(CUST_S), REGIONS = 100_000, DAY_ROWS = Math.max(1, Math.floor(ORDERS / 1000));
const INDEX_SEG = 32_000_000;
const { uploadRows, putIndexSegment } = await import(`${OQLDIR}/tools/oql-ingest/lib.mjs`);
mkdirSync(OUTDIR, { recursive: true });
const tag = `jord-${ORDERS}-${CUST}`;
const log = (s) => { const l = `[${new Date().toISOString()}] ${tag} rss=${(process.memoryUsage().rss / 1e6).toFixed(0)}MB ${s}`; console.log(l); appendFileSync(`${OUTDIR}/${tag}.log`, l + '\n'); };
const out = (o) => appendFileSync(`${OUTDIR}/${tag}.queries.jsonl`, JSON.stringify(o) + '\n');
const JM = IDL.Record({ instrs: IDL.Nat, rows: IDL.Nat, value: IDL.Nat });
const Kind = IDL.Variant({ hash: IDL.Null, ordered: IDL.Null });
const idlFactory = ({ IDL }) => IDL.Service({
  putSegment: IDL.Func([IDL.Text, IDL.Nat, IDL.Vec(IDL.Nat8)], [IDL.Nat], []),
  putIndexChunk: IDL.Func([IDL.Text, IDL.Text, Kind, IDL.Nat, IDL.Nat, IDL.Nat, IDL.Vec(IDL.Nat8)], [IDL.Nat], []),
  commitIndex: IDL.Func([IDL.Text, IDL.Text], [IDL.Nat], []),
  abortIndexUpload: IDL.Func([IDL.Text], [IDL.Bool], []),
  measure: IDL.Func([IDL.Text], [JM], ['query']),
});
const custRow = (k) => [BigInt(k % REGIONS), BigInt(k % 3)];
const ordRow = (k) => [BigInt(k % CUST), BigInt(k % 1000), BigInt(Math.floor(k / CUST) % 2), BigInt(Math.floor(k / DAY_ROWS))];
const { server, pic } = await startPic(`jo${ORDERS}`);
const res = { orders: ORDERS, customers: CUST, stages: {} };
try {
  const f = await pic.setupCanister({ idlFactory, wasm: readFileSync(`${OQLDIR}/bench/scale/oqlbars.wasm`), cycles: 10n ** 18n, wasmMemoryLimit: 0n });
  const t = f.actor;
  let t0 = performance.now(); let g = 0;
  await uploadRows({ actor: t, target: 'jcust', types: ['#nat', '#nat'], firstRow: 0, inFlight: 1, next: () => (g >= CUST ? null : custRow(g++)) });
  log(`jcust loaded ${CUST} in ${((performance.now() - t0) / 1000).toFixed(0)} s`);
  t0 = performance.now(); g = 0;
  await uploadRows({ actor: t, target: 'jord', types: ['#nat', '#nat', '#nat', '#nat'], firstRow: 0, inFlight: 1, next: () => (g >= ORDERS ? null : ordRow(g++)),
    onProgress: (s) => { if (s % 10_000_000 < 50_000) log(`jord ${s}/${ORDERS}`); } });
  res.stages.jord_load_s = (performance.now() - t0) / 1000;
  log(`jord loaded ${ORDERS} in ${res.stages.jord_load_s.toFixed(0)} s`);
  const idx = async (target, col, total, keyAt) => {
    const t1 = performance.now(); let bytes = 0;
    for (let first = 0; first < total; first += INDEX_SEG) {
      const n = Math.min(INDEX_SEG, total - first);
      bytes += await putIndexSegment({ actor: t, target, col, colType: '#nat', firstRow: first, rows: n, keyAt: (r) => keyAt(first + r), inFlight: 1 });
      if (global.gc) global.gc();
    }
    res.stages[`${target}.${col}`] = { bytes, s: (performance.now() - t1) / 1000, bytes_per_row: bytes / total };
    log(`${target}.${col} index: ${bytes} B (${(bytes / total).toFixed(3)} B/row) in ${res.stages[`${target}.${col}`].s.toFixed(0)} s`);
  };
  await idx('jcust', 'region', CUST, (k) => BigInt(k % REGIONS));
  await idx('jord', 'customerId', ORDERS, (k) => BigInt(k % CUST));
  await idx('jord', 'status', ORDERS, (k) => BigInt(Math.floor(k / CUST) % 2));
  writeFileSync(`${OUTDIR}/${tag}.summary.json`, JSON.stringify(res, null, 2));
  const Q = [
    ['footer count', { start: 'jord', aggregate: [{ fn: 'count', as: 'orders' }] }],
    ['footer amount sum', { start: 'jord', aggregate: [{ fn: 'sum', field: 'amount', as: 'total_amount' }] }],
    ['indexed customer region count', { start: 'jcust', where: { eq: { field: 'region', value: 4242 } }, aggregate: [{ fn: 'count', as: 'customers' }] }],
    ['indexed customerId count', { start: 'jord', where: { eq: { field: 'customerId', value: 12345 } }, aggregate: [{ fn: 'count', as: 'orders' }] }],
    ['indexed ordering and pagination', { start: 'jord', where: { eq: { field: 'customerId', value: 12345 } }, orderBy: [{ field: 'day', dir: 'desc' }], offset: 10, limit: 5, select: ['id', 'customerId', 'amount', 'status', 'day'] }],
    ['indexed region IN grouped by tier', { start: 'jcust', where: { in: { field: 'region', value: [7, 8, 9] } }, groupBy: ['tier'], aggregate: [{ fn: 'count', as: 'customers' }], orderBy: [{ field: 'tier', dir: 'asc' }] }],
    ['region edge semijoin count', { start: 'jord', where: { eq: { field: 'customerId.region', value: 7 } }, aggregate: [{ fn: 'count', as: 'orders' }] }],
    ['region edge plus status', { start: 'jord', where: { and: [{ eq: { field: 'customerId.region', value: 7 } }, { eq: { field: 'status', value: 0 } }] }, aggregate: [{ fn: 'count', as: 'orders' }, { fn: 'sum', field: 'amount', as: 'total_amount' }] }],
    ['region edge grouped by customer', { start: 'jord', where: { eq: { field: 'customerId.region', value: 7 } }, groupBy: ['customerId'], aggregate: [{ fn: 'count', as: 'orders' }, { fn: 'sum', field: 'amount', as: 'total_amount' }], orderBy: [{ field: 'customerId', dir: 'asc' }] }],
    ['day zone-map grouped by status', { start: 'jord', where: { eq: { field: 'day', value: 777 } }, groupBy: ['status'], aggregate: [{ fn: 'count', as: 'orders' }, { fn: 'sum', field: 'amount', as: 'total_amount' }], orderBy: [{ field: 'status', dir: 'asc' }] }],
    ['region edge grouped by dotted tier', { start: 'jord', where: { eq: { field: 'customerId.region', value: 7 } }, groupBy: ['customerId.tier'], aggregate: [{ fn: 'count', as: 'orders' }], orderBy: [{ field: 'customerId.tier', dir: 'asc' }] }],
    ['full grouping by status', { start: 'jord', groupBy: ['status'], aggregate: [{ fn: 'count', as: 'orders' }], orderBy: [{ field: 'status', dir: 'asc' }] }],
  ];
  for (const [i, [label, json]] of Q.entries()) {
    try { const r = await t.measure(JSON.stringify(json)); const o = { n: i + 1, label, json, instrs: Number(r.instrs), rows: Number(r.rows), value: natValue(r.value) }; out(o); log(`QUERY ${i + 1} ${label}: ${o.instrs.toLocaleString('en-US')} instrs, rows ${o.rows}, value ${o.value}`); }
    catch (e) { const msg = String(e.message ?? e); const o = { n: i + 1, label, json, trapped: true, error: (msg.match(/exceeded the limit of \d+ instructions/) || [msg.slice(0, 200)])[0] }; out(o); log(`QUERY ${i + 1} ${label}: TRAPPED ${o.error}`); }
  }
  log('done');
} catch (e) { log(`FAILED ${e.stack ?? e}`); process.exitCode = 1; }
finally { try { await pic.tearDown(); } catch {} try { await server.stop(); } catch {} }
