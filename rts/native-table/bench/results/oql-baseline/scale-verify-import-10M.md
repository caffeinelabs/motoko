# OQL scale script: verify-import-10M

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.

What: OQL's `verify-import.mjs`, 10,000,000 rows loaded as segment images, then `buildIndex` (`account_from #hash`) on chain: messages (rounds) and time of the build, heap after it, queries.

Scripts: OQL's own `bench/scale/*.mjs` at `bb5a23f`, unchanged, run in the scratch copy's `bench/scale` (`$OQL/bench/scale`; README.md, "Setting up the OQL copies") with `bench.wasm` built by OQL's own `bench/scale/build.sh` in the scratch copy (not this folder's `src/build.sh`; moc 1.14.0 `--force-gc --default-persistent-actors -fshared-code --max-stable-pages 1638400`); one scratch fix: `bench/scale/main.mo` lacks `import List "mo:core/List"` at `bb5a23f` and does not compile without it under moc 1.14.0. The copy's `src/ImportData.mo` is the instrumented one (`src/ImportData.mo` here), so `bench.wasm` also carries its counter reads around `putSegment`, `putIndexChunk` and `commitIndex`. Node packages in `bench/scale/node_modules`: `@dfinity/pic` 0.22.0 and `@dfinity/candid` 3.4.3, with the dependency versions `src/package-lock.json` pins (OQL's `bench/scale/package.json` asks for `^0.22.0` and `^3.4.3` and has no lockfile); `node_modules/@dfinity/pic/pocket-ic` was a link to `src/pic-ttl`, which starts PocketIC with `--ttl 86400`. @dfinity/pic 0.22.0 always starts that file and ignores the `POCKET_IC_BIN` and `binPath` the scripts set, so without the link PocketIC stops 60 s after its last finished operation, which a long Node-side step (such as building one large index segment) outlasts. Queries are real query calls (`measure`: the counter around `Executor.runWith`).

Command (in `bench/scale`): `ROWS=10000000 node verify-import.mjs`

Output:

```
OK   count(*): got 10000000, expected 10000000
OK   sum(amount_e8s): got 4999557350999188168, expected 4999557350999188168
OK   day = 7 (zone-map pruned): got 3000, expected 3000
OK   10 <= day <= 12 (pruned): got 9000, expected 9000
FAIL account_from = 123 (scan): Canister call failed: Error from Canister lxzze-o7777-77777-aaaaa-cai: Canister exceeded the limit of 5000000000 instructions for single message execution..
OK   day = 7 & amount < 5e11: got 1516, expected 1516
FAIL spender is null (scan): Canister call failed: Error from Canister lxzze-o7777-77777-aaaaa-cai: Canister exceeded the limit of 5000000000 instructions for single message execution..
OK   materialize: rows of day = 7: got 3000, expected 3000
FAIL top-5 by day desc: Canister call failed: Error from Canister lxzze-o7777-77777-aaaaa-cai: Canister exceeded the limit of 5000000000 instructions for single message execution..
  building: 900,000/10,000,000 (round 25, 18 s, heap 50 MB unsettled, maxLive 67 MB)
  building: 1,600,000/10,000,000 (round 50, 34 s, heap 54 MB unsettled, maxLive 67 MB)
  building: 2,300,000/10,000,000 (round 75, 46 s, heap 71 MB unsettled, maxLive 67 MB)
  building: 3,150,000/10,000,000 (round 100, 65 s, heap 329 MB unsettled, maxLive 324 MB)
  building: 3,700,000/10,000,000 (round 125, 76 s, heap 329 MB unsettled, maxLive 324 MB)
  building: 4,400,000/10,000,000 (round 150, 106 s, heap 105 MB unsettled, maxLive 324 MB)
  building: 5,350,000/10,000,000 (round 175, 119 s, heap 396 MB unsettled, maxLive 391 MB)
  building: 6,100,000/10,000,000 (round 200, 135 s, heap 401 MB unsettled, maxLive 396 MB)
  building: 6,600,000/10,000,000 (round 225, 152 s, heap 388 MB unsettled, maxLive 415 MB)
  building: 7,350,000/10,000,000 (round 250, 172 s, heap 439 MB unsettled, maxLive 434 MB)
  building: 7,900,000/10,000,000 (round 275, 190 s, heap 448 MB unsettled, maxLive 443 MB)
  building: 8,450,000/10,000,000 (round 300, 208 s, heap 417 MB unsettled, maxLive 444 MB)
  building: 9,450,000/10,000,000 (round 325, 227 s, heap 426 MB unsettled, maxLive 444 MB)
index build took 238.6 s over 343 rounds
index build: account_from ready=true 10000000/10000000
OK   account_from = 123 (indexed): got 104, expected 104
  scan: exceeded the query budget -> indexed: 65,729 instrs
  heap retained after build: 146.2 MB
```
