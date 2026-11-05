# oql-baseline

OQL's own instruction costs, measured on PocketIC, for comparing this repository's costs against. Each
`.md` file is one run or one group of runs and states its OQL commit, replica, compiler, method and
command; `src/` holds the benchmark and harness sources those files name; `raw.tar.xz` holds the
per-message records, run logs and suite outputs of the runs listed under "Raw data" below (not of every
run).

## What was measured

- OQL at commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype`, compiled by moc 1.14.0 (OQL's
  toolchain pin) and by moc 1.16.1; each file or section names its compiler.
- PocketIC 14.0.0 on an Apple M3 (macOS, 16 KiB host pages); the query-context suites ran under
  `mops bench --replica pocket-ic --query`.
- Three cost windows per operation:
  - page-free: the instructions of the operation and of the collection it causes, with no page fee
    (the "in-method" and "with collection" figures);
  - whole message: that plus the dirty-page fee, 4,000 per dirty 16 KiB host page (`charged16`);
  - `op16`: the whole message less the 3 host pages (12,000) that a message doing nothing dirties.
- Collection is counted: each update message's `rts_collector_instructions`, plus the collection of the
  idle `tick` messages that follow a sequence (the tail), added to its mean. The query-context suites were
  checked to collect nothing.

## Tools and libraries from other projects

- OQL (`github.com/caffeinelabs/oql-prototype`): the Motoko query library measured here: its tables
  (`OQL.Table`, columnar; `IndexedMap`, a heap map with secondary indexes), its query executor
  (`Executor.runWith`) and its bulk-load producer (`tools/oql-ingest/lib.mjs`: `uploadRows`,
  `putIndexSegment`).
- moc: the Motoko compiler, versions 1.14.0 and 1.16.1, as mops downloads them (`mops toolchain bin moc`,
  run in an OQL copy, prints the path of the one that copy pins).
- mops 2.16.1 (npm package `ic-mops`): the Motoko package manager. `mops install` fetches the Motoko
  packages and the tools (`[toolchain]`: moc, PocketIC) that a `mops.toml` lists. `mops bench <filter>`
  takes its argument as part of a file name, not as a suite name: it compiles every file below the project
  root that matches `**/bench?(mark)/**/*<filter>*.mo` into a canister of its own and runs each of its
  cells on a replica. Its runner is the canister mops generates around a suite (`bench-canister.mo` in
  `ic-mops`), which for each cell calls `InternetComputer.countInstructions` from mo:core (it returns the
  instructions a function call executes) on the suite's `runCell` and reports that count as the cell's
  `instructions`, beside the Motoko runtime's `rts_*` counters read before and after. `--save` writes each
  matched suite's results to `.bench/<suite>.bench.json`. So `mops bench oqlbars_extra` also runs
  `oqlbars_extra2`, and `mops bench oqlbars_point` also runs `oqlbars_pointr`; with `--save`, both
  commands also rewrite that other suite's JSON. The runs ran each suite's own command and copied that
  suite's JSON right after it (`mops bench oqlbars_extra.bench` would match one file only). In the
  commands, `DFX_MOC_PATH` is the moc binary mops bench compiles with (the variable is named after dfx; no
  dfx is involved) and `CI=1` makes it print plain, non-interactive output.
- Motoko packages, fetched by `mops install`: `core` 2.6.0, Motoko's standard library (`mo:core`);
  `jayson` 0.1.1, a JSON library for Motoko that OQL depends on; `bench` 2.0.1, the library a suite builds
  its `Bench` object with (`mo:bench`: rows, columns and the `runner` function that runs one cell).
- PocketIC 14.0.0 (`pocket-ic-server`): DFINITY's local test replica of the Internet Computer, the binary
  mops installs for OQL's `[toolchain]` pin `pocket-ic = "14.0.0"`.
- dfx: DFINITY's SDK command-line tool, which runs a local replica of its own. No run used it.
- Node.js runs the drivers in `src/`, with three npm packages: `@dfinity/pic` 0.22.0, the JavaScript
  client that starts a PocketIC server, installs canisters on it and calls them (its
  `PocketIcServer.start()` always runs the file `node_modules/@dfinity/pic/pocket-ic`, which its install
  script downloads from the `pocket-ic-server` 14.0.0 release of `github.com/dfinity/pocketic`; it reads no
  other binary path, neither a `POCKET_IC_BIN` variable nor a `binPath` option); `@dfinity/candid` 3.4.3,
  Candid (the Internet Computer's interface description language and wire format), for the drivers'
  interface definitions and encoded arguments; `@dfinity/principal` 3.4.3, Internet Computer principals
  (canister and user ids), used by `snap.mjs`. `src/package.json` names them and `src/package-lock.json`
  pins them and their dependencies at the versions the runs used (`@icp-sdk/core` 5.4.0 among them).

## Placeholders

The files name these locations by placeholder:

- `$OQL`: the scratch copy the files mention: OQL's files at commit `bb5a23f`, then `mops install`, with
  two edits: `import List "mo:core/List"` added to `bench/scale/main.mo`, which does not compile under
  moc 1.14.0 without it; and `src/ImportData.mo` replaced by `src/ImportData.mo` here, which adds two
  counter reads (`performanceCounter(0)`) around each of `putSegment`, `putIndexChunk` and `commitIndex`
  and a query `oqlbarsLastImportInstr` that returns the last count. The suites `src/oqlbars_*.bench.mo`
  sit in it as `bench/oqlbars_*.bench.mo`, the canister `src/oqlbars.mo` as `bench/scale/oqlbars.mo`
  (the ledger, heap-index and jord runs) and the canister `src/oqlbars-deadkeys.mo` as
  `bench/scale/oqlbars-deadkeys.mo` (the dead-keys run only; it is `oqlbars.mo` plus a table `dkt`, see
  "Other sources"). "Setting up the OQL copies" below makes it.
- `$OQL_1161`: the second copy the files mention, the same as `$OQL` except that its `mops.toml` pins
  moc 1.16.1 (its two `moc = "1.14.0"` lines, under `[requirements]` and `[toolchain]`, read
  `moc = "1.16.1"`) and its `bench/scale/main.mo` is as at `bb5a23f` (only `$OQL` built a `bench.wasm`
  from it); the moc 1.16.1 runs used it.
- `<OQL checkout>`: a git clone of `github.com/caffeinelabs/oql-prototype` that holds commit `bb5a23f`.
- `<oql copy>`: `$OQL` in a moc 1.14.0 run, `$OQL_1161` in a moc 1.16.1 run, as an absolute path (the
  drivers import modules from it).
- `<out>`: any directory for the driver's output files; it is created if missing.
- `<wasm dir built by src/build.sh>` and `<wasm dir built by src/build2.sh>`: the `<out dir>` those scripts
  wrote, below.
- `$MOPS_CACHE`: mops's cache directory, `~/Library/Caches/mops` on macOS, holding each moc as
  `moc/<version>/moc` and PocketIC as `pocket-ic/<version>/pocket-ic`.
- `~`: the home directory.
- `$BASE`: this folder (`bench/results/oql-baseline`), as an absolute path.

## Setting up the OQL copies

In a POSIX shell, in this folder, with `OQL` and `OQL_1161` set to the absolute paths of two new
directories:

```sh
BASE="$(pwd)"
for d in "$OQL" "$OQL_1161"; do
  mkdir -p "$d"
  git -C <OQL checkout> archive bb5a23f | tar -x -C "$d"    # OQL's files at bb5a23f
  cp src/ImportData.mo "$d/src/ImportData.mo"
  cp src/oqlbars_*.bench.mo "$d/bench/"
  cp src/oqlbars.mo src/oqlbars-deadkeys.mo "$d/bench/scale/"
done
perl -pi -e '$_ .= qq(import List "mo:core/List";\n) if $_ eq qq(import Nat64 "mo:core/Nat64";\n)' \
  "$OQL/bench/scale/main.mo"
perl -pi -e 's/^moc = "1\.14\.0"$/moc = "1.16.1"/' "$OQL_1161/mops.toml"
for d in "$OQL" "$OQL_1161"; do
  (cd "$d" && mops install &&    # the Motoko packages, moc and PocketIC, as mops.toml pins them
    for c in oqlbars oqlbars-deadkeys; do
      "$(mops toolchain bin moc)" $(mops sources) --default-persistent-actors -fshared-code \
        --max-stable-pages 1638400 -W M0236 -o "bench/scale/$c.wasm" "bench/scale/$c.mo" || exit 1
    done)
done
bash "$OQL/bench/scale/build.sh"    # OQL's own bench.wasm, for the scale-*.md runs
```

The first `perl` command adds `import List "mo:core/List"` after `import Nat64 "mo:core/Nat64"` in
`$OQL`'s `bench/scale/main.mo`; the second pins moc 1.16.1 in `$OQL_1161`'s `mops.toml`. The second loop
installs each copy's packages and tools and builds its `bench/scale/oqlbars.wasm` and
`bench/scale/oqlbars-deadkeys.wasm` the way OQL's own `bench/scale/build.sh` builds `bench.wasm`, less
`--force-gc` and with `oqlbars.mo` or `oqlbars-deadkeys.mo` in place of `main.mo`; the last line runs
that `build.sh` in `$OQL`, which built the `bench.wasm` of the `scale-*.md` runs (with the instrumented
`src/ImportData.mo`). The `scale-*.md` runs also need the npm packages of the next section, linked into
`$OQL/bench/scale`.

## Setting up the drivers

After the OQL copies, with `BASE`, `OQL` and `OQL_1161` as above:

```sh
cd "$BASE/src"
npm ci --ignore-scripts    # the npm packages above, at the locked versions, without the PocketIC download
printf '#!/bin/sh\nexec "%s" --ttl 86400 "$@"\n' "$(cd "$OQL" && mops toolchain bin pocket-ic)" > pic-ttl
chmod +x pic-ttl           # mops's PocketIC 14.0.0, which stops 86,400 s after its last finished operation
ln -sf "$BASE/src/pic-ttl" node_modules/@dfinity/pic/pocket-ic
ln -s "$BASE/src/node_modules" "$OQL/bench/scale/node_modules"    # the same, for OQL's own scripts
sh build.sh "$OQL" out-1140 && sh build.sh "$OQL_1161" out-1161
sh build2.sh "$OQL" out2-1140 && sh build2.sh "$OQL_1161" out2-1161
```

`@dfinity/pic` 0.22.0 starts PocketIC by running `node_modules/@dfinity/pic/pocket-ic`, whatever
`POCKET_IC_BIN` says. The `printf`, `chmod` and first `ln` lines make that file a link to `pic-ttl`, a
two-line script (not kept here) that runs mops's `pocket-ic-server` 14.0.0, the binary of OQL's
`[toolchain]` pin (`~/Library/Caches/mops/pocket-ic/14.0.0/pocket-ic` on the runs' host), with
`--ttl 86400`. Started without it, PocketIC shuts down 60 s after its last finished operation, which a long
Node-side step outlasts (the producer building one large index segment, for example). The runs had the
same link: `npm install` had downloaded `@dfinity/pic`'s own `pocket-ic-server` 14.0.0 into that file, and
the file had then been replaced by a link to `pic-ttl`; `--ignore-scripts` skips that download, so a
missing link cannot go unnoticed, and `startPic` in `src/lib.mjs` stops with an error unless the file is a
link to `pic-ttl`.

The second `ln` line gives OQL's own `bench/scale` scripts (the `scale-*.md` runs) the same packages and
the same link. In the runs, `npm install` in `$OQL/bench/scale` had installed `@dfinity/pic` 0.22.0 and
`@dfinity/candid` 3.4.3 (OQL's `bench/scale/package.json` asks for `^0.22.0` and `^3.4.3` and has no
lockfile) with the dependency versions `src/package-lock.json` pins, and its
`node_modules/@dfinity/pic/pocket-ic` had been replaced by a link to the same `pic-ttl`. Those scripts set
`POCKET_IC_BIN` (to their own `pocket-ic-ttl` shim or to that file) and `bench.mjs` also passes a
`binPath`, but @dfinity/pic 0.22.0 reads neither, so without the link they too get the 60 s timeout. The
last two lines build the harness canisters (below). Every `node <driver>.mjs` command in the results files
runs in `src/`, with `<oql copy>` standing for `$OQL` or `$OQL_1161`.

## Harness and drivers

Two Motoko canisters in `src/` measure the update-context operations:

- `wh.mo`, the harness canister: one configuration per canister (`setup`), a batched pre-fill
  (`prefill`), lone operations one per message (`op`), batched writes and index backfill in one message
  each (`wBuild`, `wReindex`, `wDelete`, `wBuildT`, `wBackfill`), an empty update message `tick`, a
  query `last` that returns the last update message's mutator and collector instructions, and a query
  `sh(k)`, a hash of the height and the internal node counts along every B-tree path key `k` touches.
  `wh2.mo` is the same canister plus the paid-orders table and query (`ordLoad`, `ordQuery`) and batched
  inserts of the reference row R (`wBuildR`).
- `relay.mo`, the relay canister: it forwards one call to the harness canister (`op`, `calib`, `batch`
  by method name, or `raw` with any method name and Candid argument), so the harness canister's cycles
  balance changes by that call's execution alone and the ingress fee falls on the relay. `wbatch.mjs`
  and `paidorders.mjs` send each measured message through it; `wseq.mjs`, `scale.mjs` and `heapidx.mjs`
  send a no-op `op` through it to calibrate the ingress fee of the messages they send directly.

Two scripts build them, each with the given OQL copy's own moc (`mops toolchain bin moc`) and its `core`,
`jayson` and `oql` sources, `--release --default-persistent-actors`:

- `src/build.sh <oql copy> <out dir>` writes `<out dir>/wh.wasm` from `wh.mo` and `<out dir>/relay.wasm`.
  The runs used `src/out-1140` (built from `$OQL`) and `src/out-1161` (from `$OQL_1161`).
- `src/build2.sh <oql copy> <out dir>` writes `<out dir>/wh.wasm` from `wh2.mo` (the same file name, so
  a driver takes either directory) and `<out dir>/relay.wasm`; the runs kept its output in a directory
  of its own per compiler (`src/out2-1140`, `src/out2-1161`), apart from `src/build.sh`'s.

| driver | results files | canister measured | relay canister |
|---|---|---|---|
| `wseq.mjs` | `lone-writes-*.md` | `wh.wasm` built by `src/build.sh` | `relay.wasm` beside it |
| `wbatch.mjs`, last argument `-buildR` | `batched-writes-gc.md` | `wh.wasm` built by `src/build.sh` | `relay.wasm` beside it |
| `wbatch.mjs`, last argument `buildR` | `batched-writes-refrow-gc.md` | `wh.wasm` built by `src/build2.sh` | `relay.wasm` beside it |
| `paidorders.mjs` | `paid-orders-indexedmap-gc.md` | `wh.wasm` built by `src/build2.sh` | `relay.wasm` beside it |
| `scale.mjs`, `heapidx.mjs` | `ledger-*.md`, `heapidx-*.md` | `<oql copy>/bench/scale/oqlbars.wasm` | `src/out-1140/relay.wasm`, a path fixed in both scripts |
| `jord.mjs` | `jord-*.md` | `<oql copy>/bench/scale/oqlbars.wasm` | none |
| `dk2.mjs` | `dead-keys-group-count-moc1140.md` | `<oql copy>/bench/scale/oqlbars-deadkeys.wasm` | none |
| `textidx.mjs`, `natidx.mjs` | `text-key-counts.md` | `oqltext.wasm`, `oqlnat.wasm` built by `src/build-textidx.sh`, `src/build-natidx.sh` | none |

`wbatch.mjs` takes an optional fourth argument that picks cells by name: a prefix runs only the cells
whose names start with it (`buildR`), and a prefix after a minus sign runs all the others (`-buildR`).
`batched-writes-gc.md` ran before the four `buildR_*` cells were added to `wbatch.mjs`, and `src/build.sh`'s
`wh.wasm` has no `wBuildR`, which those cells call; its command therefore ends in `-buildR`, which leaves
the other eleven cells in the order that run had them. Without it, the run stops at the first `buildR_*`
cell, after the other 10,000-row cells.

`wseq.mjs` sends each lone `op` to the harness canister directly, as an ingress message, and subtracts
the ingress fee it calibrated (`calibrateIngress` in `lib.mjs`). It flags a message structural when the
harness canister's `sh(k)` differs before and after it: a B-tree node split or merge on a touched path, or
for a Table a new segment. As run, `wseq.mjs` also read an optional file `SAMPLES_OVERRIDE` beside it
whose number replaced its `samples` argument; that read is removed here, and each `lone-writes-*.md`
command gives the sample count the run used.

### Ledger runs (`scale.mjs`)

`scale.mjs` bulk-loads OQL's generated ledger into the table `ledger_unindexed`, uploads a region `#hash`
index on it, and measures queries and deletes. Its arguments are `<oql copy> <out> <rows> <K> <deltas>
<stages> [N]`: K, which sets the size of the index segments (the index is uploaded in segments of
ceil(rows / K) rows: K segments, or fewer when the last of K such segments would hold no row, as at
100,000 rows with K = 1024, which gives 1,021 segments, 1,020 of 98 rows and a last one of 40; the file
names carry K); deltas, a comma-separated list of cumulative on-chain append counts (`''` for none);
stages, the comma-separated stages to run; N, the messages per lone delete sequence (8,192 in every run).
Two environment variables: `COL=day` uploads the index on `day` instead of `account_from` and adds `-day`
to the output file names; `DK_KILLS`, for stage `dk`, lists the numbers of dead days at which the group
count is repeated. The stages run in this order, each only when listed:

- `load`: the rows bulk-loaded through OQL's producer (`putSegment` messages).
- `q0`: queries with no index.
- `index`: the region index uploaded in segments of ceil(rows / K) rows (`putIndexChunk`, `commitIndex`
  messages).
- `q1`: queries with the region index.
- `delta`: for each D in deltas, rows appended on chain (`appendBig`, 20,000 per message) until D rows
  in all were added since the load (they go to the index's heap delta, on top of the uploaded segments),
  then three counts; their stage is `delta<D>`, for example `delta1000`.
- `emptydel`: N rows with fresh `account_from` keys appended in one message, then each deleted in its own
  message (sequence `emptying_delete`), then a count of that key (stage `emptydel`).
- `tabledel`: N lone `Table.delete`s of loaded rows (`table_delete_lone_base`), then 10 messages of 1,000
  deletes each (`table_delete_batch1000_base`).
- `dk`: the group count by `day`, with every day live and then with whole days deleted (`DK_KILLS`).
- `dropbase`: `dropIndexBase` on the `account_from` index.
- `tabledelnobase`: `tabledel` again on other rows, after `dropbase` (`table_delete_*_nobase`).

The `stage` column of the ledger results' query tables is the stage a query ran in: `q0`, `q1`,
`delta<D>`, `emptydel` or `dk`. Each ledger result gives its full command, stages included. (`heapidx.mjs`
has no stages, and the heap-index results' query tables have no `stage` column.)

The `src/scale.mjs` kept here sends `tick` messages until the collector is idle (`settle` in
`src/lib.mjs`) before the `q0`, `q1` and `delta<D>` query sets and logs it: before the `q0` and `q1`
queries (`settled before q0`, `settled before q1`) and, for each delta, after the appends and before its
three counts (`delta: <D> rows appended on chain; settled ...`). Only five runs logged these steps
(`settled before q1` and the `settled` delta lines): moc 1.14.0 at 100,000,000 rows with K = 64 and
K = 1024, and moc 1.16.1 at 100,000 rows with K = 1, 64 and 1024. The other runs with a `q0`, `q1` or
`delta` stage were made with an earlier form of the driver that sent no `tick` before those query sets;
their logs print `delta: <D> rows appended on chain` with nothing after it. So a rerun's `q0`, `q1` and
`delta<D>` figures can differ from theirs where the collector was still busy after the load, the upload
or the appends; each of those results files says so above its query table.

### Heap-index runs (`heapidx.mjs`)

`heapidx.mjs` appends the generated ledger on chain into the table `ledger1`, which has one heap
`account_from #hash` index, then measures queries and deletes. Its arguments are `<oql copy> <out> <rows>
[N] [batch]`: N, the messages per lone delete sequence (8,192); batch, the rows appended per message
during the build (50,000). With `QONLY=1` it stops after the queries, before the update-context
operations, and names its output files `heapidx-<rows>-q.*` instead of `heapidx-<rows>.*`
(`heapidx-2000000-q-moc1140.md`). As run, it then ended the process with `process.exit(0)`, which skipped
stopping PocketIC and left the server running (until `--ttl` ran out); the driver kept here leaves
through its `finally` block, which stops it, as the full run does. The driver kept here also measures the
first three counts twice, straight after the build (labeled `(collector as the build left it)`) and after
ticking the collector idle; the moc 1.14.0 runs at 100,000, 2,000,000 and 10,000,000 rows were made with
an earlier form that measured every query straight after the build, without the label (each of those
files says so).

### Text-key count runs (`textidx.mjs`, `natidx.mjs`)

`src/oqltext.mo` appends the same rows on chain into a Table and an IndexedMap, each with a heap `k #hash`
index on one text column, and a mo:core `Map<Text, Nat>`; `textidx.mjs` builds it for one key shape (17
characters, paths of 59 to 199 characters, or 1,999 characters) and counts one key per query call through
each of the six paths of `text-key-counts.md`. `src/oqlnat.mo` and `natidx.mjs` are the same with the
column a `#nat` (the control). `src/build-textidx.sh <oql copy> <out dir>` and `src/build-natidx.sh` build
the two canisters with the copy's own moc, as `oqlbars.wasm` is built; `src/keycheck.mjs` checks the
canister's keys against the crate's key generator. `src/text-key-count-bars.py` reduces the records to one
bar per key, `text-key-count-bars.tsv`, the per-key bars the crate's count checks compare with.

### Other sources

- `snap.mjs`: takes canister snapshots of the harness canister's wasm memory through the management
  canister (`take_canister_snapshot`, `read_canister_snapshot_data`) and compares two of them 4 KiB page
  by 4 KiB page (`diffPages`). `wseq.mjs` uses it in its second pass, when its `samples` argument is above
  0, to count the pages each sampled message changed.
- `lib.mjs`: the drivers' shared code: it starts PocketIC (`startPic`), installs canisters with no Wasm
  memory limit (`install`), ticks until the collector is idle (`settle`), calibrates the ingress fee
  (`calibrateIngress`), writes a query's answer exactly (`natValue`, see "Raw data") and summarizes lists
  of counts (`stats`). Before starting PocketIC, `startPic` checks that
  `node_modules/@dfinity/pic/pocket-ic` is a link to `pic-ttl` (see "Setting up the drivers"); as run, it
  set `POCKET_IC_BIN` to `pic-ttl` instead, which @dfinity/pic 0.22.0 does not read, so the runs relied on
  that link alone. It also holds `structural()`, which compares two lists from the harness canister's
  `sig` query, but no driver calls it: `wseq.mjs` uses `sh` instead. Each PocketIC server gets its own
  temporary directory, `oql-baseline-pocketic/<name>` under the system's temporary directory (Node's
  `os.tmpdir()`), where `<name>` is the argument each driver passes to `startPic`: `<cfg><rows>`
  (`wseq.mjs`, for example `cm100000`), `wbatch`, `paidorders`, `sc<rows>k<K>` (`scale.mjs`), `hx<rows>`
  (`heapidx.mjs`), `jo<orders>` (`jord.mjs`), `dk<rows>s<S>` (`dk2.mjs`).
- `verify-import-longtimeout.mjs`: OQL's `bench/scale/verify-import.mjs` at `bb5a23f` with one change,
  a PocketIC processing timeout (`processingTimeoutMs`, line 55) of 3,600,000 ms instead of 600,000; it
  runs from `<oql copy>/bench/scale` like the original (`scale-verify-import-50M.md`).
- `ImportData.mo`, `oqlbars.mo`, `oqlbars-deadkeys.mo` and `oqlbars_*.bench.mo`: OQL-copy files,
  described under `$OQL` above. In the scratch copies the suite files, the canister file and the
  instrumentation in `ImportData.mo` (its variables and its query) had other names, and the drivers as
  run called the query by its other name; the names here are the ones the commands and the drivers use.
  After the runs, the header comments and `bench.name` strings of all eight suites were rewritten, as
  were the `bench.description` strings of `oqlbars_columnar`, `oqlbars_extra` and `oqlbars_extra2` and
  three comments in `ImportData.mo` and the canister source. Every other line of these files, the cells,
  their data and their queries included, is as run: in `oqlbars.mo` as the ledger, heap-index and jord
  runs had it, in `oqlbars-deadkeys.mo` as the dead-keys run had it.
- `oqlbars.mo` and `oqlbars-deadkeys.mo`: each scratch copy had one canister file, built into one
  `bench/scale/oqlbars.wasm`, and `$OQL`'s changed once during the runs. At 15:50 UTC on 2026-10-14, just
  before the dead-keys run, the table `dkt` was added to it and the canister rebuilt: the table with its
  comment and the method `dktDeleteKey`, `dkt.importTarget("dkt")` in the `ImportData` list and
  `dkt.entityWith("dkt", "Dkt", "id")` in the `Expose` list. Every ledger, heap-index and jord run had
  installed its canister before then: the drivers install it when they start, and the last of those runs
  to start, the one logged in `raw/scale-1140-ledger-1000000-k4-day.log`, started at 15:48 and ended at
  15:49 UTC. `src/oqlbars.mo` is the canister before that change, which the ledger, heap-index and jord
  runs used; `src/oqlbars-deadkeys.mo` is the canister after it, which `dead-keys-group-count-moc1140.md`
  used. The two differ in those lines only, and the difference shows in the counts: built from
  `oqlbars-deadkeys.mo`, the canister answers the queries of `heapidx-100000-moc1140.md`'s command with
  about 500 more instructions each, and those of `ledger-100000-k64-moc1161.md`'s command with counts
  about 150 instructions apart (its index upload's counts differ too), while built from `oqlbars.mo` it
  reproduces both runs' archived query records, and the ledger run's index upload counts, exactly. As
  run, `dk2.mjs` read `bench/scale/oqlbars.wasm`, by then rebuilt from the later source; the `dk2.mjs`
  kept here reads `bench/scale/oqlbars-deadkeys.wasm`, so that one copy holds both canisters.
- `package.json`, `package-lock.json`: the drivers' npm packages, above.

## Raw data

`text-key-counts.md`'s records are in an archive of their own, `raw-text-key-counts.tar.xz`, described in
that file; everything below is about `raw.tar.xz`.

`raw.tar.xz` (70 MB unpacked; `tar -xJf raw.tar.xz` restores `raw/` beside this file) holds these
members, where `<c>` is the compiler (`1140`: moc 1.14.0, `1161`: moc 1.16.1). Every `raw/...` path a
results file names is a member, and each results file listed here names its members.

| results files | members |
|---|---|
| `lone-writes-<cfg>-<rows>-moc<c>.md` (22) | `raw/lone-<c>-<cfg>-<rows>-<sequence>.jsonl`, sequence `insert`, `replace` (not for the Table configurations `tb*`) and `delete`; also `raw/lone-1140-cm-100000-insert.sample.jsonl` |
| `ledger-<rows>-k<K>[-day]-moc<c>.md` (16) | `raw/scale-<c>-ledger-<rows>-k<K>[-day].log`, `.putSegment.jsonl` and `.queries.jsonl`; `.index.jsonl` where the `index` stage ran; `.<sequence>.jsonl` for the update-context sequences of the K = 4 runs at 100,000 and 100,000,000 rows |
| `heapidx-<rows>[-q]-moc<c>.md` (7) | `raw/heapidx-<c>-heapidx-<rows>[-q].log` and `.queries.jsonl`; `.<sequence>.jsonl` for `emptying_delete`, `delete_loaded` and `delete_batch1000`, except in the `-q` run |
| `jord-100000000-1000000-moc1140.md` | `raw/jord-100000000-1000000.log` |
| the eight `mops bench` suite files (`columnar-noindex.md`, `coremap-reads.md`, `deletes-queue-closures.md`, `index-present-shapes.md`, `one-day-through-day-index.md`, `point-pk-rows-L.md`, `point-pk-rows-R.md`, `size-calls.md`) | `raw/moc<c>-oqlbars_<suite>.bench.json`, the results `mops bench --save` wrote |

Each member is a file a driver or `mops bench --save` wrote, with a prefix added to its name that says
what wrote it and under which compiler (none for the one `jord.mjs` log), as below; `<record>` stands for
the rest of the name (`log`, `queries.jsonl`, `putSegment.jsonl`, `index.jsonl` or `<sequence>.jsonl`).
To compare a rerun's file with the archive, add the prefix to its name: for example, `scale.mjs` run with
moc 1.16.1 at 100,000 rows with K = 64 writes `<out>/ledger-100000-k64.queries.jsonl`, archived as
`raw/scale-1161-ledger-100000-k64.queries.jsonl`.

| writer | file it writes | member |
|---|---|---|
| `wseq.mjs` | `<out>/<cfg>-<rows>-<sequence>.jsonl`, `<out>/<cfg>-<rows>-<sequence>.sample.jsonl` | `raw/lone-<c>-<cfg>-<rows>-<sequence>.jsonl`, `raw/lone-<c>-<cfg>-<rows>-<sequence>.sample.jsonl` |
| `scale.mjs` | `<out>/ledger-<rows>-k<K>[-day].<record>` | `raw/scale-<c>-ledger-<rows>-k<K>[-day].<record>` |
| `heapidx.mjs` | `<out>/heapidx-<rows>[-q].<record>` | `raw/heapidx-<c>-heapidx-<rows>[-q].<record>` |
| `jord.mjs` | `<out>/jord-<orders>-<customers>.log` | `raw/jord-<orders>-<customers>.log` (no prefix: one run) |
| `mops bench --save` | `<oql copy>/.bench/oqlbars_<suite>.bench.json` | `raw/moc<c>-oqlbars_<suite>.bench.json` |

The drivers also write files the archive does not hold: the `<out>/<tag>.summary.json` of `wseq.mjs`,
`scale.mjs`, `heapidx.mjs` and `jord.mjs` (`<tag>` the name before the record, for example
`ledger-100000-k64`), `wseq.mjs`'s log `<out>/<cfg>-<rows>.log`, `jord.mjs`'s
`<out>/jord-<orders>-<customers>.queries.jsonl` (its results file lists every query), and every file of
`dk2.mjs`, `wbatch.mjs` and `paidorders.mjs` (their results files list every measured message or query,
below).

The other runs have no raw records: `batched-writes-gc.md`, `batched-writes-refrow-gc.md`,
`paid-orders-indexedmap-gc.md` and `dead-keys-group-count-moc1140.md` list every measured message or query
whole, and the `scale-*.md` files quote their scripts' printed output.

Names changed after the runs: in `raw/scale-1140-ledger-100000-k4.log`,
`raw/scale-1161-ledger-100000-k4.log` and `raw/scale-1140-ledger-100000000-k4.log`, the labels of the four
Table-delete sequences (`table_delete_lone_base`, `table_delete_batch1000_base`,
`table_delete_lone_nobase`, `table_delete_batch1000_nobase`), and the names of the 12 members that hold
those sequences (`raw/scale-<c>-ledger-<rows>-k4.table_delete_*.jsonl`), were renamed: the driver as run
printed other, shorter labels for the same sequences, and called the stages `tabledel` and
`tabledelnobase` by other names. Only the names changed; every number in those lines and members is as the
run wrote it, and every other member's content is byte for byte the file a run wrote. The
`raw/moc<c>-oqlbars_<suite>.bench.json` members are the `.bench/oqlbars_<suite>.bench.json` files that
`mops bench --save` wrote, under the suites' names here.

Record fields: `.log`, the driver's log lines (with the renamed labels above); `.queries.jsonl`, one query
each (`stage` in the ledger runs, `label`, `json` the query, `instrs`, `rows`, `value` the first cell of
the first returned row when it is a `#nat`, else 0, or `trapped` and `error`); lone-write `.jsonl`, one
message each (`i` its index in the sequence, `k` the key, `instr` the in-method count, `mut` and `gc` the
message's mutator and collector instructions, `cyc` the cycles the harness canister paid, `n16` its
pages16, `resid`, `structural`; the `.sample.jsonl` adds `p4diff`, `h16` and `p4lo`, described in its
results file); ledger and heap-index `<sequence>.jsonl`, one `op` message each (`i`, `a` and `b` the `op`
arguments, `r` its return, then `instr`, `cyc`, `mut`, `gc`, `n16`, `resid` as above); `.putSegment.jsonl`
and `.index.jsonl`, one upload message each (`at` or `firstRow`, `segLen` and `off` locate it, `bytes`,
`rows` the method's reply, `m` the method in `.index.jsonl`, then `instr`, `mut`, `gc`, `cyc`).

`value` in the archived records is a JSON number: the drivers as run converted every answer with
JavaScript's `Number`, so an answer above 2^53 - 1 (9,007,199,254,740,991) is stored as the nearest
double. Six records hold such an answer, the `q0` `sum(amount_e8s)` of the K = 4 ledger runs at 100,000
(both compilers), 1,000,000, 2,000,000, 10,000,000 and 100,000,000 rows. The run's `.log` line prints the
exact answer; the results file's table gives the record's value, as for every other answer, and the note
below the table gives the exact one. The drivers kept here write such an answer as its exact decimal
string (`natValue` in `src/lib.mjs`) and every other answer as a JSON number, as before.
