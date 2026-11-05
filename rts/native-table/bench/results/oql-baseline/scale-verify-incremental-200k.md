# OQL scale script: verify-incremental-200k

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.

What: OQL's `verify-incremental.mjs` (180,000 + 20,000 rows, region indexes on `account_from` and `day`): the point counts while the second batch's index is not yet uploaded (base prefix + tail scan), after the delta upload, and the full-scan reference.

Scripts: OQL's own `bench/scale/*.mjs` at `bb5a23f`, unchanged, run in the scratch copy's `bench/scale` (`$OQL/bench/scale`; README.md, "Setting up the OQL copies") with `bench.wasm` built by OQL's own `bench/scale/build.sh` in the scratch copy (not this folder's `src/build.sh`; moc 1.14.0 `--force-gc --default-persistent-actors -fshared-code --max-stable-pages 1638400`); one scratch fix: `bench/scale/main.mo` lacks `import List "mo:core/List"` at `bb5a23f` and does not compile without it under moc 1.14.0. The copy's `src/ImportData.mo` is the instrumented one (`src/ImportData.mo` here), so `bench.wasm` also carries its counter reads around `putSegment`, `putIndexChunk` and `commitIndex`. Node packages in `bench/scale/node_modules`: `@dfinity/pic` 0.22.0 and `@dfinity/candid` 3.4.3, with the dependency versions `src/package-lock.json` pins (OQL's `bench/scale/package.json` asks for `^0.22.0` and `^3.4.3` and has no lockfile); `node_modules/@dfinity/pic/pocket-ic` was a link to `src/pic-ttl`, which starts PocketIC with `--ttl 86400`. @dfinity/pic 0.22.0 always starts that file and ignores the `POCKET_IC_BIN` and `binPath` the scripts set, so without the link PocketIC stops 60 s after its last finished operation, which a long Node-side step (such as building one large index segment) outlasts. Queries are real query calls (`measure`: the counter around `Executor.runWith`).

Command (in `bench/scale`): `node --expose-gc verify-incremental.mjs`

Output:

```
batch 1 = 180,000 rows, batch 2 = 20,000 rows, total 200,000
probe keys (chosen for gaining rows in batch 2): account_from = 2889 (7 → 9 rows), account_from = 45541 (9 → 10 rows), account_from = 82170 (10 → 11 rows)
         day = 60: 0 rows after batch 1, 3,000 after batch 2
counterfactual: re-uploading the whole index over all 200,000 rows would move 9.84 MB

[11:43:27 rss=185MB] phase 1: loading rows [0, 180,000)
[11:43:28 rss=265MB]   index 'account_from' segment 1: rows [0, 180,000)
[11:43:30 rss=287MB]   index 'day' segment 1: rows [0, 180,000)
[11:43:30 rss=287MB] phase 1: done — 180,000 rows in 1.0s, index 9.51 MB

  OK   rows() after batch 1                       got             180000  want             180000
  OK   indexCoverage('account_from') after batch 1 got             180000  want             180000
  OK   indexCoverage('day') after batch 1         got             180000  want             180000

  ── answers: indexed over batch 1 ─────────────────────────────────────────
  OK   count(account_from = 2889)                 got                  7  want                  7  64,101 instrs
  OK   count(account_from = 45541)                got                  9  want                  9  64,101 instrs
  OK   count(account_from = 82170)                got                 10  want                 10  64,779 instrs
  OK   rows(account_from = 2889)                  got                  7  want                  7  344,202 instrs
  OK   rows(account_from = absent key)            got                  0  want                  0
  OK   rows(account_from in [3 keys])             got                 26  want                 26
  OK   count(day = 60)                            got                  0  want                  0  59,773 instrs
  OK   count(*)                                   got             180000  want             180000
  OK   sum(amount_e8s)                            got  90045038775424465  want  90045038775424465

  ── phase 2: reopen, then load the second batch ───────────────
  OK   putSegment refused while indexed           trapped: …reopenIndex…
  OK   reopenIndex('account_from')                READY → pending
  OK   reopenIndex('account_from') again          no-op: a retried message is not a trap
  OK   reopenIndex('amount_e8s')                  no committed index on that column
  OK   indexCoverage('account_from') after reopen got             180000  want             180000
  OK   putSegment still refused: day is ready     trapped: …reopenIndex…
  OK   reopenIndex('day')                         READY → pending
  OK   buildIndex refused over a partial region base trapped: …has a region index covering…
  OK   extension segment past appended            trapped: …coverage guard…
[11:43:31 rss=287MB] phase 2: loading rows [180,000, 200,000)
  OK   rows() after batch 2                       got             200000  want             200000
  OK   indexCoverage('account_from') is still batch 1 got             180000  want             180000
  OK   indexCoverage('day') is still batch 1      got             180000  want             180000

  ── answers: reopened: base prefix + tail scan ─────────────────────────────────────────
  OK   count(account_from = 2889)                 got                  9  want                  9  240,027,356 instrs
  OK   count(account_from = 45541)                got                 10  want                 10  239,820,494 instrs
  OK   count(account_from = 82170)                got                 11  want                 11  239,646,116 instrs
  OK   rows(account_from = 2889)                  got                  9  want                  9  240,374,766 instrs
  OK   rows(account_from = absent key)            got                  0  want                  0
  OK   rows(account_from in [3 keys])             got                 30  want                 30
  OK   count(day = 60)                            got               3000  want               3000  240,273,103 instrs
  OK   count(*)                                   got             200000  want             200000
  OK   sum(amount_e8s)                            got 100033269073230347  want 100033269073230347
  OK   extension segment overlapping the base     trapped: …no gap or overlap…

  ── phase 3: index segments covering only the new rows ────────
  OK   'account_from' extension starts at indexCoverage got             180000  want             180000
[11:43:32 rss=287MB]   index 'account_from' segment 1: rows [180,000, 200,000)
  OK   'day' extension starts at indexCoverage    got             180000  want             180000
[11:43:32 rss=276MB]   index 'day' segment 1: rows [180,000, 200,000)

  OK   indexCoverage('account_from') after the delta got             200000  want             200000
  OK   indexCoverage('day') after the delta       got             200000  want             200000

  ── answers: indexed over everything ─────────────────────────────────────────
  OK   count(account_from = 2889)                 got                  9  want                  9  82,644 instrs
  OK   count(account_from = 45541)                got                 10  want                 10  82,644 instrs
  OK   count(account_from = 82170)                got                 11  want                 11  83,322 instrs
  OK   rows(account_from = 2889)                  got                  9  want                  9  432,209 instrs
  OK   rows(account_from = absent key)            got                  0  want                  0
  OK   rows(account_from in [3 keys])             got                 30  want                 30
  OK   count(day = 60)                            got               3000  want               3000  108,931 instrs
  OK   count(*)                                   got             200000  want             200000
  OK   sum(amount_e8s)                            got 100033269073230347  want 100033269073230347
  OK   every probed answer differs between phases account_from = 2889: 7 → 9 rows; day = 60: 0 → 3,000 rows
  OK   reopened window costs the tail, not the table 240,027,356 instrs with 20,000 of 200,000 rows uncovered, vs 1,582,321,098 to scan the table
  OK   post-delta point query is index-served     82,644 instrs vs 1,582,321,098 for a scan of the same rows
  OK   jcust: three rows loaded                   positions 0,1,2 from the file
  OK   jcust: rows() counts the ordinary append   got                  4  want                  4
  OK   load refused once a row arrived by append  trapped: …appended rather than loaded…
  OK   jcust: rows() is the position cursor       got                  4  want                  4
  OK   jcust: loadedRows() is the file cursor     got                  3  want                  3
  OK   jord: index complete over A,B,C            ready
  OK   jord: rows() after the append              got                  4  want                  4
  OK   jord: loadedRows() unchanged               got                  3  want                  3
  OK   jord: a ready index is still reported ready after an append coverage 3 vs rows 4 — comparing those would call it unfinished
  OK   jord: coverage after absorbing the appended row got                  4  want                  4
  OK   jord: index complete again                 ready
  OK   jord: count(key 100)                       got                  1  want                  1
  OK   jord: count(key 101)                       got                  1  want                  1
  OK   jord: count(key 102)                       got                  1  want                  1
  OK   jord: count(key 103)                       got                  1  want                  1
  OK   jord: count(key 104)                       got                  1  want                  1
  OK   jord: count(the appended row key 999)      got                  1  want                  1
  OK   jord: count(*)                             got                  6  want                  6
  OK   a new off-chain index over appended rows is refused names buildIndex as the way out
  OK   re-running an index over a complete one is a no-op returned coverage 6
  OK   a reopened (incomplete) index is still reported as needing this run indexedColumns -> [customerId]
  OK   jord: amount has a partial base            covers 2 of 7 rows
  OK   the off-chain path is closed and names the recovery points at dropIndexBase + buildIndex
  OK   dropIndexBase clears the committed base    state is now none
  OK   buildIndex now runs, so the table has a way back on-chain walk accepted
  OK   a retried dropIndexBase reports nothing to drop so the replacement index survives
  OK   the replacement heap index is still declared trapped: …already…
  OK   dropping one column leaves another's staging alone 'status' still had an upload to abort
  OK   day has a base to discard                  covered 2
  OK   a replacement base committed after the drop covers 3, was 2
  OK   a drop naming stale coverage is refused    trapped: …read indexCoverage again…
  OK   the replacement base survived the stale drop still covers 3
  OK   partial region commit refused over a heap index trapped: …already has a heap index…

  index upload                       rows
  first load: account_from        180,000      8.07 MB      1 seg    1.3 s
  first load: day                 180,000      1.44 MB      1 seg    0.4 s
  delta: account_from              20,000      1.81 MB      1 seg    0.2 s
  delta: day                       20,000      0.16 MB      1 seg    0.1 s
  FIRST index upload              180,000      9.51 MB
  DELTA index upload               20,000      1.97 MB
  data load: 1.0 s for batch 1, 0.1 s for batch 2
  OK   delta index upload moves fewer bytes than the first 1.97 MB vs 9.51 MB — 4.8× less
  OK   delta beats re-uploading the whole index   1.97 MB vs 9.84 MB for all 200,000 rows — 20.0% of it

  85 checks, all OK
```
