# OQL scale script: verify-region-index-2M

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.

What: OQL's `verify-region-index.mjs`, region mode only, 2,000,000 rows, one index segment of 2,000,000 rows: producer peak RSS (sampled by the script after each segment), queries.

Scripts: OQL's own `bench/scale/*.mjs` at `bb5a23f`, unchanged, run in the scratch copy's `bench/scale` (`$OQL/bench/scale`; README.md, "Setting up the OQL copies") with `bench.wasm` built by OQL's own `bench/scale/build.sh` in the scratch copy (not this folder's `src/build.sh`; moc 1.14.0 `--force-gc --default-persistent-actors -fshared-code --max-stable-pages 1638400`); one scratch fix: `bench/scale/main.mo` lacks `import List "mo:core/List"` at `bb5a23f` and does not compile without it under moc 1.14.0. The copy's `src/ImportData.mo` is the instrumented one (`src/ImportData.mo` here), so `bench.wasm` also carries its counter reads around `putSegment`, `putIndexChunk` and `commitIndex`. Node packages in `bench/scale/node_modules`: `@dfinity/pic` 0.22.0 and `@dfinity/candid` 3.4.3, with the dependency versions `src/package-lock.json` pins (OQL's `bench/scale/package.json` asks for `^0.22.0` and `^3.4.3` and has no lockfile); `node_modules/@dfinity/pic/pocket-ic` was a link to `src/pic-ttl`, which starts PocketIC with `--ttl 86400`. @dfinity/pic 0.22.0 always starts that file and ignores the `POCKET_IC_BIN` and `binPath` the scripts set, so without the link PocketIC stops 60 s after its last finished operation, which a long Node-side step (such as building one large index segment) outlasts. Queries are real query calls (`measure`: the counter around `Executor.runWith`).

Command (in `bench/scale`): `REGION_ONLY=1 ROWS=2000000 INDEX_SEG=2000000 node --expose-gc --max-old-space-size=6144 verify-region-index.mjs`

Output:

```
ROWS = 2,000,000, account_from = 123 matches 28 rows

[11:43:41 rss=131MB] data load: 0/2,000,000
[11:43:41 rss=183MB] data load: 36,915/2,000,000
[11:43:52 rss=278MB] data load: done
[11:43:52 rss=278MB] index seg 1/1: building + uploading 2,000,000 rows
[11:44:03 rss=325MB] index seg 1/1: committed (peak rss so far 325MB)
region: loaded + committed 1 segment(s) (index 22.7 MB, INDEX_SEG=2,000,000) in 22.7 s

  metric                                 region
  ------------------------------------------------
  index segments (K)                          1
  producer peak RSS (MB)                  324.8
  build+commit wall (s)                    11.1
  point instrs                        1,129,867
  count(k) instrs                        63,466
  point rows (want 28)                       28
  count value (want 28)                      28
  settled canister heap (MB)               35.9
  index bytes in region (MB)               22.7
  raw data in region (MB)                  81.3
  raw data bytes/row                       40.7

  base probe results: count=28 rows=28 oracle=28 → OK

  index-served query         rows     oracle       instrs
  --------------------------------------------------------
  OK  point hot key            28         28    1,129,871
  OK  count hot key            28         28       63,470
  OK  point absent key          0          0       94,277
  OK  in [3 keys]              68         68    2,731,204

  correctness: base probe count=28 rows=28 oracle=28; index queries OK → OK
```
