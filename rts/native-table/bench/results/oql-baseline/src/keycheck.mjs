// Checks keyOf against the crate's key generator (text_key / LongMix 18-byte keys), terminator dropped.
import { readFileSync } from 'node:fs';
import { startPic } from './lib.mjs';
const M = (1n << 64n) - 1n;
const mix = (z) => { z ^= z >> 30n; z = (z * 0xBF58476D1CE4E5B9n) & M; z ^= z >> 27n; z = (z * 0x94D049BB133111EBn) & M; z ^= z >> 31n; return z; };
function crateKey(shape, u) {
  const U = BigInt(u); const out = [];
  if (shape === 0) { out.push(98); for (let i = 1; i < 17; i++) out.push(97 + Number((U >> BigInt(4 * (15 - (i - 1)))) & 15n)); return String.fromCharCode(...out); }
  const [prefix, len] = shape === 1 ? [40, 60 + Number(mix(U) % 141n)] : [1500, 2000];
  for (let i = 0; i + 1 < len; i++) out.push(i < prefix ? 97 + (i % 23) : i < prefix + 16 ? 97 + Number((U >> BigInt(4 * (15 - (i - prefix)))) & 15n) : 113 + (i % 7));
  return String.fromCharCode(...out);
}
const idlFactory = ({ IDL }) => IDL.Service({ setup: IDL.Func([IDL.Nat, IDL.Nat], [], []), keyOf: IDL.Func([IDL.Nat64], [IDL.Text], ['query']) });
const { server, pic } = await startPic('tx-keycheck');
let bad = 0, n = 0;
try {
  for (const shape of [0, 1, 2]) {
    const f = await pic.setupCanister({ idlFactory, wasm: readFileSync(process.argv[2]), cycles: 10n ** 15n });
    await f.actor.setup(BigInt(shape), 100000n);
    for (const u of [0, 1, 39594, 39595, 34623, 74037, 75701, 99998, 12345, 65535]) { n++; if ((await f.actor.keyOf(BigInt(u))) !== crateKey(shape, u)) { bad++; console.log('DIFF', shape, u); } }
  }
} finally { await pic.tearDown(); await server.stop(); }
console.log(`keys checked ${n}, differing ${bad}`);
