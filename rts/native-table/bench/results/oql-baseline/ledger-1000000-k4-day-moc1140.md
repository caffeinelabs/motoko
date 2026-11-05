# Ledger, 1,000,000 rows, region index on `day` in 4 segments, group count by day with dead days, moc 1.14.0

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.

Rows: OQL's scale-bench ledger generator `bigRow(k)` (bench/scale/main.mo and image.mjs): account_from = splitmix64(k) mod 100,000; account_to; account_spender (null on odd k); amount_e8s; day = floor(k / 3,000); five `#nat` columns. Canister: `bench/scale/oqlbars.mo`, copied from `src/oqlbars.mo` and built as `bench/scale/oqlbars.wasm` (README.md, "Setting up the OQL copies"; not `src/oqlbars-deadkeys.mo`, whose extra table `dkt` was added after this run had installed its canister and changes the counts), OQL's `bench/scale/main.mo` plus an `op` endpoint (one measured operation per message), a table `ledger1` with one heap index, and `appendBig`; built with moc `--default-persistent-actors -fshared-code --max-stable-pages 1638400` (OQL's scale build without `--force-gc`). `ImportData.mo` of the scratch copy records the in-method instructions of `putSegment`, `putIndexChunk` and `commitIndex` (two counter reads added; source in `src/ImportData.mo`). Loads and index uploads go through OQL's shipped producer (`tools/oql-ingest/lib.mjs`: `uploadRows`, `putIndexSegment`, in flight 1), so each message's cycles are read alone. Queries are real query calls of `measure(qJson)` (the counter around `Executor.runWith`, JSON parsing outside).

Index: this run uploads the region `#hash` index on `day` (`COL=day`), not on `account_from`: 4 segments of 250,000 rows, each row's key floor(row / 3,000). The driver names the run's files `ledger-1000000-k4-day.*`: K = 4 segments, `-day` from `COL=day`.

Page fees: PocketIC on this host charges the dirty-page fee per dirty 16 KiB host page (calibrated: one write into each of p pages 4 KiB apart costs ceil(p/4) fees of 4,000; a message that does nothing dirties 3 host pages). `pages16` is that count, derived per message from its cycles: cycles = fee + 2 x (mutator + collector + 4,000 x pages16 + 19), fee = 5,000,000 through the relay canister, or the ingress fee calibrated on the same canister (the residue 19 is constant in every message, column `resid`). On the IC's own hosts the fee is per 4 KiB page, between pages16 and 4 x pages16 per message. `charged16` = in-method + the message's collection + 4,000 x pages16; `op16` = the same less the 3 host pages of a message that does nothing.

## Load

28 `putSegment` messages, 40,635,096 image bytes, Region 40,697,856 B in 28 segments (40.70 B per row). In the method: 45,037,883 instructions, 1.1083 per image byte, 45.04 per row; per message mean 1,608,496, max 1,664,348. Whole messages (Candid decoding of the image included, collection included): 3.4576 per byte. Cycles: 2012.443 per image byte (ingress fees included). Wall 4.0 s (248,832 rows/s, this host; the machine slept during some loads, see the log). Per message: `raw/scale-1140-ledger-1000000-k4-day.putSegment.jsonl`.

## Index upload

4 segments of 250,000 rows, 8,026,176 B (8.026 B per row), 8 chunks. In the method: chunks 1.0757 per index byte; chunks and commits 1.7430 per index byte; commit per segment mean 1,338,865, max 1,345,761. Whole messages: 3.0904 per chunk byte; cycles 2013.771 per chunk byte. Producer (Node, `node --expose-gc`) peak RSS 272 MB. Wall 1.0 s. Per message: `raw/scale-1140-ledger-1000000-k4-day.index.jsonl`.

## Queries (real query calls)

Stage `dk` (`src/scale.mjs`; README.md, "Ledger runs"): the group count `{start: ledger_unindexed, groupBy: [day], aggregate: [count]}` as a real query call, first with every day live; then days 0 .. 110 killed, each by deleting all its 3,000 rows in one message (`op` 9: `Table.delete` of positions 3,000 d .. 3,000 d + 2,999), the collector ticked idle and the query repeated; then days 111 .. 221 killed the same way and the query repeated (`DK_KILLS=111,222`: the number of dead days at each repeat). The labels count the 333 full days (floor(1,000,000 / 3,000)); `rows` is the groups returned, which also include day 333 (rows 999,000 .. 999,999) and leave out the dead days; `value` is the first cell of the first returned row (`measure` in `src/oqlbars.mo`), here the lowest live day.

| stage | query | instructions | rows | value |
|---|---|---:|---:|---:|
| dk | group count by day, 333 live | 22,704,086 | 334 | 0 |
| dk | group count by day, 222 live, 111 dead | 19,035,118 | 223 | 111 |
| dk | group count by day, 111 live, 222 dead | 15,450,368 | 112 | 222 |


Log of the run: `raw/scale-1140-ledger-1000000-k4-day.log`. Queries: `raw/scale-1140-ledger-1000000-k4-day.queries.jsonl` (fields: stage, label, json, instrs, rows, value; trapped and error for a query that trapped). Command: `COL=day DK_KILLS=111,222 node --expose-gc --max-old-space-size=8192 scale.mjs <oql copy> <out> 1000000 4 '' load,index,dk 8192` (source `src/scale.mjs`; stages load, index, dk; no deltas).
