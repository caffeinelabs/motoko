// Shared driver pieces for the update-context harness.
import { PocketIc, PocketIcServer } from '@dfinity/pic';
import { IDL } from '@dfinity/candid';
import { readFileSync, mkdirSync, realpathSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = fileURLToPath(new URL('.', import.meta.url));
// PocketIC's state and temporary files, one directory per driver run tag.
const PIC_TMP = join(tmpdir(), 'oql-baseline-pocketic');
// @dfinity/pic 0.22.0's PocketIcServer.start() always runs the file
// node_modules/@dfinity/pic/pocket-ic: it reads neither POCKET_IC_BIN nor a
// binary path option. The setup (README.md, "Setting up the drivers") makes
// that file a link to pic-ttl, which starts PocketIC with --ttl 86400; started
// any other way, the server stops 60 s after its last finished operation.
const PIC_BIN = join(HERE, 'node_modules', '@dfinity', 'pic', 'pocket-ic');
const PIC_TTL = join(HERE, 'pic-ttl');

export const M = IDL.Record({ instr: IDL.Nat64, pmut: IDL.Nat64, pgc: IDL.Nat64, hb: IDL.Nat, ha: IDL.Nat });
export const whIdl = ({ IDL }) => IDL.Service({
  setup: IDL.Func([IDL.Text], [], []),
  prefill: IDL.Func([IDL.Nat64, IDL.Nat64], [IDL.Nat], []),
  op: IDL.Func([IDL.Nat64, IDL.Nat64], [M], []),
  calib: IDL.Func([IDL.Nat64, IDL.Nat64], [M], []),
  tick: IDL.Func([], [], []),
  last: IDL.Func([], [IDL.Record({ mut: IDL.Nat64, gc: IDL.Nat64 })], ['query']),
  stats: IDL.Func([], [IDL.Record({ size: IDL.Nat, heap: IDL.Nat, alloc: IDL.Nat, reclaimed: IDL.Nat, segs: IDL.Nat })], ['query']),
  sig: IDL.Func([IDL.Nat64], [IDL.Vec(IDL.Nat)], ['query']),
  sh: IDL.Func([IDL.Nat64], [IDL.Nat], ['query']),
  wBuild: IDL.Func([IDL.Nat64, IDL.Nat64], [M], []),
  wReindex: IDL.Func([IDL.Nat64], [M], []),
  wDelete: IDL.Func([IDL.Nat64], [M], []),
  wBuildT: IDL.Func([IDL.Nat64, IDL.Nat64], [M], []),
  wBackfill: IDL.Func([IDL.Nat64], [M], []),
});
export const relayIdl = ({ IDL }) => IDL.Service({
  op: IDL.Func([IDL.Principal, IDL.Nat64, IDL.Nat64], [M], []),
  calib: IDL.Func([IDL.Principal, IDL.Nat64, IDL.Nat64], [M], []),
  batch: IDL.Func([IDL.Principal, IDL.Text, IDL.Nat64, IDL.Nat64], [M], []),
});

export const SEP = 999_999_999;

export async function startPic(tag) {
  let linked = false;
  try { linked = realpathSync(PIC_BIN) === realpathSync(PIC_TTL); } catch {}
  if (!linked) throw new Error(`${PIC_BIN} is not a link to ${PIC_TTL}; see README.md, "Setting up the drivers"`);
  const tmp = join(PIC_TMP, tag);
  mkdirSync(tmp, { recursive: true });
  process.env.TMPDIR = tmp;
  const server = await PocketIcServer.start();
  const pic = await PocketIc.create(server.getUrl(), { processingTimeoutMs: 3_600_000 });
  return { server, pic };
}

export async function install(pic, wasmPath, idlFactory) {
  const f = await pic.setupCanister({ idlFactory, wasm: readFileSync(wasmPath), cycles: 10n ** 16n, wasmMemoryLimit: 0n });
  return f;
}

// Send `tick` (an empty update message, so its collector instructions are the
// collection left over from earlier messages) until the collector spent under
// 100,000 instructions in each of three messages in a row. Returns the ticks and
// their collector instructions summed, nothing subtracted; a caller adds the sum
// as a tail after a measured message, or drops it as warm-up before one.
export async function settle(target, max = 3000) {
  let ticks = 0, spent = 0n, idle = 0;
  while (idle < 3) {
    if (ticks >= max) throw new Error('collector still busy');
    await target.tick();
    const { gc } = await target.last();
    spent += gc; ticks++;
    idle = gc < 100_000n ? idle + 1 : 0;
  }
  return { ticks, spent };
}

// Structural: per map segment [height, internal counts on the path..., SEP],
// a node split/merge changes the height or an internal node's count. Segment
// 0 of a Table config is the segment count (a flush). Compares two lists from
// the harness's `sig`; no driver calls it (wseq.mjs compares the `sh` hashes).
export function structural(a, b) {
  if (a.length === 0 || b.length === 0) return false;
  const seg = (v) => { const out = []; let cur = []; for (const x of v) { const n = Number(x); if (n === SEP) { out.push(cur); cur = []; } else cur.push(n); } return out; };
  const A = seg(a), B = seg(b);
  if (A.length !== B.length) return true;
  for (let i = 0; i < A.length; i++) {
    const x = A[i], y = B[i];
    if (x[0] !== y[0]) return true;           // height (or segment count)
    const n = Math.min(x.length, y.length);
    for (let j = 1; j < n; j++) if (x[j] !== y[j]) return true;
  }
  return false;
}

// A Candid Nat (a BigInt) for a JSON record: a JSON number up to 2^53 - 1,
// above that its exact decimal string (a JSON number there would be rounded to
// the nearest double).
export function natValue(n) {
  return n <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(n) : n.toString();
}

export function stats(xs) {
  const s = [...xs].sort((a, b) => a - b);
  const n = s.length;
  const sum = s.reduce((a, b) => a + b, 0);
  return { n, mean: sum / n, p50: s[Math.floor((n - 1) / 2)], p90: s[Math.floor(0.9 * (n - 1))], p99: s[Math.floor(0.99 * (n - 1))], max: s[n - 1], min: s[0] };
}

// Ingress fee calibration: the noop op(99, 0) costs, through the relay,
// 5,000,000 + 2 x (mut + gc + 4000 x p0 + resid); sent as ingress it costs F more
// on top of the same execution. Returns the ingress fixed fee and the noop's p0.
export async function calibrateIngress(pic, t, r) {
  const one = async (viaRelay) => {
    const c0 = await pic.getCyclesBalance(t.canisterId);
    if (viaRelay) await r.actor.op(t.canisterId, 99n, 0n); else await t.actor.op(99n, 0n);
    const c1 = await pic.getCyclesBalance(t.canisterId);
    const l = await t.actor.last();
    return { cyc: Number(c0 - c1), w: Number(l.mut) + Number(l.gc) };
  };
  const rel = [], ing = [];
  for (let i = 0; i < 5; i++) rel.push(await one(true));
  for (let i = 0; i < 20; i++) ing.push(await one(false));
  const mode = (xs) => { const c = new Map(); for (const x of xs) c.set(x, (c.get(x) || 0) + 1); return [...c.entries()].sort((a, b) => b[1] - a[1])[0][0]; };
  const relX = rel.map((x) => (x.cyc - 5_000_000) / 2 - x.w);   // = 4000 p0 + resid
  const x0 = mode(relX);
  const feeIng = ing.map((x) => x.cyc - 2 * x.w - 2 * x0);
  return { relX, x0, fee: mode(feeIng), feeAgree: feeIng.filter((f) => f === mode(feeIng)).length, feeIng, ing, rel };
}
