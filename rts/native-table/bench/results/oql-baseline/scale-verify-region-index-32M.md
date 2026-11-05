# OQL scale script: verify-region-index-32M

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.

What: OQL's `verify-region-index.mjs`, as in `scale-verify-region-index-2M.md` but with 32,000,000 rows: region mode only, one index segment of 32,000,000 rows; producer peak RSS (sampled by the script after each segment), queries.

Scripts: OQL's own `bench/scale/*.mjs` at `bb5a23f`, unchanged, run in the scratch copy's `bench/scale` (`$OQL/bench/scale`; README.md, "Setting up the OQL copies") with `bench.wasm` built by OQL's own `bench/scale/build.sh` in the scratch copy (not this folder's `src/build.sh`; moc 1.14.0 `--force-gc --default-persistent-actors -fshared-code --max-stable-pages 1638400`); one scratch fix: `bench/scale/main.mo` lacks `import List "mo:core/List"` at `bb5a23f` and does not compile without it under moc 1.14.0. The copy's `src/ImportData.mo` is the instrumented one (`src/ImportData.mo` here), so `bench.wasm` also carries its counter reads around `putSegment`, `putIndexChunk` and `commitIndex`. Node packages in `bench/scale/node_modules`: `@dfinity/pic` 0.22.0 and `@dfinity/candid` 3.4.3, with the dependency versions `src/package-lock.json` pins (OQL's `bench/scale/package.json` asks for `^0.22.0` and `^3.4.3` and has no lockfile); `node_modules/@dfinity/pic/pocket-ic` was a link to `src/pic-ttl`, which starts PocketIC with `--ttl 86400`. @dfinity/pic 0.22.0 always starts that file and ignores the `POCKET_IC_BIN` and `binPath` the scripts set, so without the link PocketIC stops 60 s after its last finished operation, which a long Node-side step (such as building one large index segment) outlasts. Queries are real query calls (`measure`: the counter around `Executor.runWith`).

Command (in `bench/scale`): `REGION_ONLY=1 ROWS=32000000 INDEX_SEG=32000000 node --expose-gc --max-old-space-size=8192 verify-region-index.mjs`

Output:

```
ROWS = 32,000,000, account_from = 123 matches 322 rows

[11:51:02 rss=142MB] data load: 0/32,000,000
[11:51:02 rss=184MB] data load: 36,915/32,000,000
[11:51:51 rss=262MB] data load: 10,003,965/32,000,000
[11:52:52 rss=240MB] data load: 20,007,930/32,000,000
[11:54:03 rss=227MB] data load: 30,011,895/32,000,000
[11:54:19 rss=223MB] data load: done
[11:54:19 rss=223MB] index seg 1/1: building + uploading 32,000,000 rows
[12:02:06 rss=140MB] index seg 1/1: committed (peak rss so far 139MB)
region: loaded + committed 1 segment(s) (index 262.7 MB, INDEX_SEG=32,000,000) in 664.8 s

  metric                                 region
  ------------------------------------------------
  index segments (K)                          1
  producer peak RSS (MB)                  139.4
  build+commit wall (s)                   467.0
  point instrs                       12,799,033
  count(k) instrs                        67,175
  point rows (want 322)                     322
  count value (want 322)                    322
  settled canister heap (MB)               10.0
  index bytes in region (MB)              262.7
  raw data in region (MB)                1300.4
  raw data bytes/row                       40.6

  base probe results: count=322 rows=322 oracle=322 → OK

  index-served query         rows     oracle       instrs
  --------------------------------------------------------
  OK  point hot key           322        322   12,799,033
  OK  count hot key           322        322       67,175
  OK  point absent key          0          0       94,277
  OK  in [3 keys]             909        909   36,276,291

  correctness: base probe count=322 rows=322 oracle=322; index queries OK → OK
```
