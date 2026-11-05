# Lone writes: im1, 100,000 rows pre-filled, moc 1.16.1

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.


Structure: `IndexedMap<Nat, R>` with `ts #ordered` (each key holds one row: every delete empties its key). Row R = {ts : Int = 1,760,000,000,000,000,000 + 1,000 k, amount : Nat = 7k mod 1000, kind : Text (k mod 10 = 0 "burn", k mod 3 = 0 "mint", else "xfer"), owner : Principal (1,000 distinct 29-byte principals), note : Text (10 + k mod 51 bytes), flag : Bool}, OQL's implicit `_toRow` derivation (a Principal becomes `#text` of its text form). The row is built before the first counter read.

Method: harness canister (`moc --release --default-persistent-actors`, OQL's code unchanged), PocketIC, one ingress message per operation. Pre-fill keys 0 .. rows-1 in batches; tick until the collector spent under 100,000 for three messages; sequences of consecutive lone messages: insert of keys rows .. rows+N-1 (Table: `append`), replace of key (7919 i + 13) mod rows moving `ts` by 10^12 (core Map and IndexedMap only), delete of key (7919 i + 13) mod rows (Table: position); after each sequence tick until idle again (the tail's collection is added to the mean). In-method = `performanceCounter(0)` around the call. Collection = the message's `rts_collector_instructions`. Structural = a B-tree node split or merge on a touched path (the height or an internal node's child count along the key's path changed, in the primary map or an index map; a borrow between siblings is not counted), or for a Table a new segment.
Page fees: PocketIC on this host charges the dirty-page fee per dirty 16 KiB host page (calibrated: one write into each of p pages 4 KiB apart costs ceil(p/4) fees of 4,000; a message that does nothing dirties 3 host pages). `pages16` is that count, derived per message from its cycles: cycles = fee + 2 x (mutator + collector + 4,000 x pages16 + 19), fee = 5,000,000 through the relay canister, or the ingress fee calibrated on the same canister (the residue 19 is constant in every message, column `resid`). On the IC's own hosts the fee is per 4 KiB page, between pages16 and 4 x pages16 per message. `charged16` = in-method + the message's collection + 4,000 x pages16; `op16` = the same less the 3 host pages of a message that does nothing.

Command: `node wseq.mjs <wasm dir built by src/build.sh> <out> im1 100000 8192 0` (sources in `src/`); ingress fee 6,496,000 cycles; the no-op message: 4,000 x pages16 + resid = 12,019.
Pre-filled: 100,000 rows, heap 124,215,896 B; warm-up 3 ticks.

| sequence | messages | in-method mean / p50 / max | in-method + collection, mean (tail included) | collection per message mean / max | tail collection (ticks) | pages16 mean / p50 / max | charged16 mean (tail included) / p50 / max | op16 mean | structural messages | max charged16, non-structural | resid |
|---|---:|---|---:|---|---:|---|---|---:|---:|---:|---|
| insert | 8,192 | 62,742 / 60,419 / 217,311 | 63,154 | 412 / 412 | 1,236 (3) | 5.44 / 5 / 7 | 84,907 / 80,839 / 241,723 | 72,907 | 482 | 90,531 | [19] |
| replace | 8,192 | 111,220 / 108,645 / 198,144 | 111,632 | 412 / 412 | 1,236 (3) | 6.71 / 7 / 13 | 138,457 / 135,450 / 242,556 | 126,457 | 1400 | 183,502 | [19] |
| delete | 8,192 | 70,812 / 68,105 / 151,752 | 71,224 | 412 / 412 | 1,236 (3) | 8.21 / 8 / 14 | 104,051 / 101,336 / 196,164 | 92,051 | 1402 | 150,576 | [19] |

Per-message records: `raw/lone-1161-im1-100000-<sequence>.jsonl` (fields: i, k, instr, mut, gc, cyc, n16, resid, structural).
