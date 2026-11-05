// Canister snapshots of the target's wasm memory, to count the 4 KiB pages a
// message changed (PocketIC on this host tracks dirty pages per 16 KiB host page).
import { IDL } from '@dfinity/candid';
import { Principal } from '@dfinity/principal';

const MGMT = Principal.fromText('aaaaa-aa');
const TakeArg = IDL.Record({ canister_id: IDL.Principal, replace_snapshot: IDL.Opt(IDL.Vec(IDL.Nat8)) });
const Snap = IDL.Record({ id: IDL.Vec(IDL.Nat8), taken_at_timestamp: IDL.Nat64, total_size: IDL.Nat64 });
const Range = IDL.Record({ offset: IDL.Nat64, size: IDL.Nat64 });
const Kind = IDL.Variant({ wasm_module: Range, wasm_memory: Range, stable_memory: Range, wasm_chunk: IDL.Record({ hash: IDL.Vec(IDL.Nat8) }) });
const ReadArg = IDL.Record({ canister_id: IDL.Principal, snapshot_id: IDL.Vec(IDL.Nat8), kind: Kind });
const ReadRes = IDL.Record({ chunk: IDL.Vec(IDL.Nat8) });
const MetaArg = IDL.Record({ canister_id: IDL.Principal, snapshot_id: IDL.Vec(IDL.Nat8) });
const Meta = IDL.Record({ wasm_memory_size: IDL.Nat64, stable_memory_size: IDL.Nat64 });

async function mgmt(pic, method, argT, arg, resT) {
  const raw = await pic.updateCall({ canisterId: MGMT, method, arg: new Uint8Array(IDL.encode([argT], [arg])), sender: Principal.anonymous() });
  return IDL.decode([resT], raw)[0];
}

const DelArg = IDL.Record({ canister_id: IDL.Principal, snapshot_id: IDL.Vec(IDL.Nat8) });

export class Snapper {
  constructor(pic, canisterId) { this.pic = pic; this.cid = canisterId; this.id = null; }
  async take() {
    const s = await mgmt(this.pic, 'take_canister_snapshot', TakeArg, { canister_id: this.cid, replace_snapshot: this.id ? [this.id] : [] }, Snap);
    this.id = s.id;
    // metadata: decode only the field we need (the record has more fields; candid subtyping drops them)
    const meta = await mgmt(this.pic, 'read_canister_snapshot_metadata', MetaArg, { canister_id: this.cid, snapshot_id: s.id }, Meta);
    const size = Number(meta.wasm_memory_size);
    const buf = Buffer.allocUnsafe(size);
    const CH = 2_000_000;
    for (let off = 0; off < size; off += CH) {
      const n = Math.min(CH, size - off);
      const r = await mgmt(this.pic, 'read_canister_snapshot_data', ReadArg, { canister_id: this.cid, snapshot_id: s.id, kind: { wasm_memory: { offset: BigInt(off), size: BigInt(n) } } }, ReadRes);
      Buffer.from(r.chunk).copy(buf, off);
    }
    return buf;
  }
  async drop() {
    if (!this.id) return;
    const raw = await this.pic.updateCall({ canisterId: MGMT, method: 'delete_canister_snapshot', arg: new Uint8Array(IDL.encode([DelArg], [{ canister_id: this.cid, snapshot_id: this.id }])), sender: Principal.anonymous() });
    this.id = null;
  }
}

// Changed 4 KiB pages between two memory images, and the distinct 16 KiB host
// pages holding them. Growth counts as changed for every new non-zero page.
export function diffPages(a, b) {
  const P = 4096;
  const n = Math.max(a.length, b.length);
  let p4 = 0; const h16 = new Set();
  for (let off = 0; off < n; off += P) {
    const ea = Math.min(off + P, a.length), eb = Math.min(off + P, b.length);
    let changed;
    if (off >= a.length) { changed = false; for (let i = off; i < eb; i++) if (b[i] !== 0) { changed = true; break; } }
    else if (off >= b.length) changed = false;
    else changed = a.compare(b, off, eb, off, ea) !== 0;
    if (changed) { p4++; h16.add(Math.floor(off / 16384)); }
  }
  return { p4, h16: h16.size };
}
