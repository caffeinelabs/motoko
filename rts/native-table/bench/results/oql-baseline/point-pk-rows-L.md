# Point read by primary key through the executor, rows L

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.

What: `where id = n/2` run through `Executor.runWith` over rows L = {id, ts, kind (Text, 3 values), amount} at 10^3, 10^4, 10^5 rows, three ways: `L_heap_noidx`, an `IndexedMap` with `ts #ordered, kind #hash, amount #ordered` and no index on `id` (the planner serves declared indexes only, so this scans); `L_heap_idx`, the same map with an added `id #hash` index; `L_table`, an `OQL.Table` with the same three heap indexes, `id` being the store position (`Table.entityWith(.., "id")`). Every way returns exactly one row (asserted at init).


## moc 1.14.0

- cwd: $OQL
- moc: Motoko compiler 1.14.0 (source 27krqrwk-vmsjiajs-zrcbmnsi-0p7rqmvj)
- cmd: DFX_MOC_PATH=$MOPS_CACHE/moc/1.14.0/moc PATH=$MOPS_CACHE/moc/1.14.0:$PATH CI=1 mops bench oqlbars_point --replica pocket-ic --query --save --verbose
- filter: `mops bench` takes `oqlbars_point` as part of a file name (README.md, "Tools and libraries from other projects"), so this command also compiled and ran `bench/oqlbars_pointr.bench.mo` and, with `--save`, rewrote `.bench/oqlbars_pointr.bench.json`; `point-pk-rows-R.md` reports that suite from its own command, run separately, whose JSON was copied right after it. `mops bench oqlbars_point.bench` runs this suite alone.
- date: 2026-10-14T07:45:32Z
- pipeline (mops): moc `--release --force-gc` plus OQL's `[moc] args` (`--default-persistent-actors -fshared-code -W M0236`), each cell in a query call (`--query`); collection in every cell: 0 (checked: rts_reclaimed and rts_collector_instructions are 0 in every cell)

Instructions per cell, as `mops bench` reports them: the canister mops builds around the suite calls `InternetComputer.countInstructions` from mo:core (it returns the instructions a function call executes) on the suite's `runCell` for that cell, so each figure is the cell's whole query plus the dispatch to it (mo:bench's `runCell` and the suite's `runner` switch on the cell name):

| cell | 1000 | 10000 | 100000 |
|---|---:|---:|---:|
| L_heap_noidx | 11,623,422 | 116,462,249 | 1,155,700,952 |
| L_heap_idx | 127,367 | 131,953 | 132,794 |
| L_table | 3,671,567 | 35,648,347 | 495,173,492 |

Raw: the mops JSON of this run (`.bench/oqlbars_point.bench.json`, `--save`) is kept beside this file in `raw.tar.xz` as `raw/moc1140-oqlbars_point.bench.json`.


## moc 1.16.1

- cwd: $OQL_1161
- moc: Motoko compiler 1.16.1 (source cmnnq83c-j64xzf3y-dbl9326w-qfay9xca)
- cmd: DFX_MOC_PATH=$MOPS_CACHE/moc/1.16.1/moc PATH=$MOPS_CACHE/moc/1.16.1:$PATH CI=1 mops bench oqlbars_point --replica pocket-ic --query --save --verbose
- filter: `mops bench` takes `oqlbars_point` as part of a file name (README.md, "Tools and libraries from other projects"), so this command also compiled and ran `bench/oqlbars_pointr.bench.mo` and, with `--save`, rewrote `.bench/oqlbars_pointr.bench.json`; `point-pk-rows-R.md` reports that suite from its own command, run separately, whose JSON was copied right after it. `mops bench oqlbars_point.bench` runs this suite alone.
- date: 2026-10-14T11:09:42Z
- pipeline (mops): moc `--release --force-gc` plus OQL's `[moc] args` (`--default-persistent-actors -fshared-code -W M0236`), each cell in a query call (`--query`); collection in every cell: 0 (checked: rts_reclaimed and rts_collector_instructions are 0 in every cell)

Instructions per cell, as `mops bench` reports them: the canister mops builds around the suite calls `InternetComputer.countInstructions` from mo:core (it returns the instructions a function call executes) on the suite's `runCell` for that cell, so each figure is the cell's whole query plus the dispatch to it (mo:bench's `runCell` and the suite's `runner` switch on the cell name):

| cell | 1000 | 10000 | 100000 |
|---|---:|---:|---:|
| L_heap_noidx | 11,686,704 | 117,099,543 | 1,162,078,390 |
| L_heap_idx | 127,025 | 131,627 | 132,472 |
| L_table | 3,675,602 | 35,697,382 | 495,672,233 |

Raw: the mops JSON of this run (`.bench/oqlbars_point.bench.json`, `--save`) is kept beside this file in `raw.tar.xz` as `raw/moc1161-oqlbars_point.bench.json`.


Source of the suite: `bench/oqlbars_point.bench.mo` of the scratch copy, kept beside this file as `src/oqlbars_point.bench.mo`.
