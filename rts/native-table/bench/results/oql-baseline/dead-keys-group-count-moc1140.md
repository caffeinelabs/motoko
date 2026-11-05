# Group count with fully deleted keys over uploaded region-hash segments, moc 1.14.0

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.


What: a table `dkt` (two `#nat` columns: key = row mod K, v = row), 100,000 rows bulk-loaded through OQL's shipped producer (`uploadRows`); the `key` region `#hash` index uploaded in S segments of 100,000 / S rows each (`putIndexSegment`), so every key lies in every segment. Query `{start: dkt, groupBy: [key], aggregate: [count]}` (a real query call, after ticking the collector idle) with all K keys live; then every row of keys 3 .. K-1 deleted (`Table.delete` of each position, one message per key: the region base marks the rows dead), leaving 3 live keys and D = K - 3 fully deleted ones, and the same query again; then a count of key 0. Canister `src/oqlbars-deadkeys.mo`, built as `bench/scale/oqlbars-deadkeys.wasm` (README.md, "Setting up the OQL copies"): `src/oqlbars.mo`, the canister of the ledger, heap-index and jord runs, plus the table `dkt` and `dktDeleteKey`, added for this run (the `dk2.mjs` as run read this canister from `bench/scale/oqlbars.wasm`, rebuilt from it; README.md, "Other sources"); driver `src/dk2.mjs`, `node dk2.mjs <oql copy> <out> 100000 <S> 8,17,48`.

| segments S | keys K | live | dead | instructions | groups returned |
|---:|---:|---:|---:|---:|---:|
| 3 | 11 | 11 | 0 | 1,653,684 | 11 |
| 3 | 11 | 3 | 8 | 1,357,314 | 3 |
| 3 | 20 | 20 | 0 | 2,087,744 | 20 |
| 3 | 20 | 3 | 17 | 1,424,391 | 3 |
| 3 | 51 | 51 | 0 | 3,992,389 | 51 |
| 3 | 51 | 3 | 48 | 1,687,469 | 3 |
| 6 | 11 | 11 | 0 | 1,968,477 | 11 |
| 6 | 11 | 3 | 8 | 1,508,106 | 3 |
| 6 | 20 | 20 | 0 | 2,697,275 | 20 |
| 6 | 20 | 3 | 17 | 1,649,037 | 3 |
| 6 | 51 | 51 | 0 | 5,899,705 | 51 |
| 6 | 51 | 3 | 48 | 2,141,895 | 3 |

Count of key 0 (each key holds 100,000 / K rows):

| segments S | keys K | instructions | value |
|---:|---:|---:|---:|
| 3 | 11 | 160,966 | 9,091 |
| 3 | 20 | 115,900 | 5,000 |
| 3 | 51 | 81,683 | 1,961 |
| 6 | 11 | 178,090 | 9,091 |
| 6 | 20 | 135,676 | 5,000 |
| 6 | 51 | 101,938 | 1,961 |

Raw records: none kept; the tables above list every query of the run.
