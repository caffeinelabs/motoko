# OQL scale script: bench-default-2M

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.

What: OQL's scale bench, default mode: 2,000,000 fixture rows (`gen-rows.mjs`, uniform over 100,000 accounts, seed 1) appended into the ledger Table with heap `account_from #hash` and `day #hash` indexes over update calls of 20,000 rows; then count and rows of `account_from = 1967` and `day = 0` as real query calls.

Scripts: OQL's own `bench/scale/*.mjs` at `bb5a23f`, unchanged, run in the scratch copy's `bench/scale` (`$OQL/bench/scale`; README.md, "Setting up the OQL copies") with `bench.wasm` built by OQL's own `bench/scale/build.sh` in the scratch copy (not this folder's `src/build.sh`; moc 1.14.0 `--force-gc --default-persistent-actors -fshared-code --max-stable-pages 1638400`); one scratch fix: `bench/scale/main.mo` lacks `import List "mo:core/List"` at `bb5a23f` and does not compile without it under moc 1.14.0. The copy's `src/ImportData.mo` is the instrumented one (`src/ImportData.mo` here), so `bench.wasm` also carries its counter reads around `putSegment`, `putIndexChunk` and `commitIndex`. Node packages in `bench/scale/node_modules`: `@dfinity/pic` 0.22.0 and `@dfinity/candid` 3.4.3, with the dependency versions `src/package-lock.json` pins (OQL's `bench/scale/package.json` asks for `^0.22.0` and `^3.4.3` and has no lockfile); `node_modules/@dfinity/pic/pocket-ic` was a link to `src/pic-ttl`, which starts PocketIC with `--ttl 86400`. @dfinity/pic 0.22.0 always starts that file and ignores the `POCKET_IC_BIN` and `binPath` the scripts set, so without the link PocketIC stops 60 s after its last finished operation, which a long Node-side step (such as building one large index segment) outlasts. Queries are real query calls (`measure`: the counter around `Executor.runWith`).

Command (in `bench/scale`): `node bench.mjs`

Output:

```
rows.csv.gz not found — generating it (see gen-rows.mjs for row count / distribution / seed)...
  generated 2,000,000 rows over 100,000 accounts (dist=uniform, seed=1)
reading + decompressing CSV...
rows to ingest: 2,000,000 (2,000,000 CSV rows x1, batch 20,000)
ingest done in 195.1s

=== columnar store (generated ledger-shaped rows) ===
rows loaded             : 2,000,000
retained heap           : 135,746,576 bytes  (~135.7 MB; account_from + day indexes + segment meta)
  reserved heap          : 135,746,576 bytes  (~135.7 MB; incl. GC headroom)
  peak during ingest     : 210,077,960 bytes  (~210.1 MB; max_live_size — includes ingest churn, NOT retained)
Region bytes (row data) : 81,264,640 bytes  (~81.3 MB; off-heap, GC-free)

-- queries run as real query calls (5B instruction limit enforced) --
count account_from==1967    : 63,354 instrs, 1 rows
eq    account_from==1967    : 1,071,587 instrs, 24 rows
count day==0                : 57,502 instrs, 1 rows
eq    day==0                : 122,996,658 instrs, 3,000 rows
```
