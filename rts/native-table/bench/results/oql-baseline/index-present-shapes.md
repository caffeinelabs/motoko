# Group-bys, all-rows predicates, sort and a 1% range with indexes present

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.

What, at 10^4 and 10^5 rows: `gsa_idx` group by (status, amount), count, over E = {id, status (3 values), amount = 7k mod 1000} in an IndexedMap with `status #hash, amount #ordered`; `gprice_idx` group by price (Float, 1,000 values = (7k mod 1000)/100), count, IndexedMap with `price #ordered`; over M = {id, n = k, i, f, z = 0, t, b, ref} in an IndexedMap with `z #hash, n #ordered`: `eq_all_idx` `z = 0` (every row matches, rows returned), `ge_all_idx` `n >= 0` (every row), `sort1_idx` order by n (every row), `sel1pct_idx` `n < n_rows/100`.


## moc 1.14.0

- cwd: $OQL
- moc: Motoko compiler 1.14.0 (source 27krqrwk-vmsjiajs-zrcbmnsi-0p7rqmvj)
- cmd: DFX_MOC_PATH=$MOPS_CACHE/moc/1.14.0/moc PATH=$MOPS_CACHE/moc/1.14.0:$PATH CI=1 mops bench oqlbars_extra --replica pocket-ic --query --save --verbose
- filter: `mops bench` takes `oqlbars_extra` as part of a file name (README.md, "Tools and libraries from other projects"), so this command also compiled and ran `bench/oqlbars_extra2.bench.mo` and, with `--save`, rewrote `.bench/oqlbars_extra2.bench.json`; `deletes-queue-closures.md` reports that suite from its own command, run separately, whose JSON was copied right after it. `mops bench oqlbars_extra.bench` runs this suite alone.
- date: 2026-10-14T07:47:42Z
- pipeline (mops): moc `--release --force-gc` plus OQL's `[moc] args` (`--default-persistent-actors -fshared-code -W M0236`), each cell in a query call (`--query`); collection in every cell: 0 (checked: rts_reclaimed and rts_collector_instructions are 0 in every cell)

Instructions per cell, as `mops bench` reports them: the canister mops builds around the suite calls `InternetComputer.countInstructions` from mo:core (it returns the instructions a function call executes) on the suite's `runCell` for that cell, so each figure is the cell's whole query plus the dispatch to it (mo:bench's `runCell` and the suite's `runner` switch on the cell name):

| cell | 10000 | 100000 |
|---|---:|---:|
| gsa_idx | 668,484,942 | 4,350,191,162 |
| gprice_idx | 25,233,034 | 25,233,024 |
| eq_all_idx | 262,845,360 | 2,747,252,220 |
| ge_all_idx | 308,705,206 | 3,205,901,287 |
| sort1_idx | 301,762,256 | 3,136,676,711 |
| sel1pct_idx | 3,304,373 | 32,618,840 |

Raw: the mops JSON of this run (`.bench/oqlbars_extra.bench.json`, `--save`) is kept beside this file in `raw.tar.xz` as `raw/moc1140-oqlbars_extra.bench.json`.


## moc 1.16.1

- cwd: $OQL_1161
- moc: Motoko compiler 1.16.1 (source cmnnq83c-j64xzf3y-dbl9326w-qfay9xca)
- cmd: DFX_MOC_PATH=$MOPS_CACHE/moc/1.16.1/moc PATH=$MOPS_CACHE/moc/1.16.1:$PATH CI=1 mops bench oqlbars_extra --replica pocket-ic --query --save --verbose
- filter: `mops bench` takes `oqlbars_extra` as part of a file name (README.md, "Tools and libraries from other projects"), so this command also compiled and ran `bench/oqlbars_extra2.bench.mo` and, with `--save`, rewrote `.bench/oqlbars_extra2.bench.json`; `deletes-queue-closures.md` reports that suite from its own command, run separately, whose JSON was copied right after it. `mops bench oqlbars_extra.bench` runs this suite alone.
- date: 2026-10-14T11:11:50Z
- pipeline (mops): moc `--release --force-gc` plus OQL's `[moc] args` (`--default-persistent-actors -fshared-code -W M0236`), each cell in a query call (`--query`); collection in every cell: 0 (checked: rts_reclaimed and rts_collector_instructions are 0 in every cell)

Instructions per cell, as `mops bench` reports them: the canister mops builds around the suite calls `InternetComputer.countInstructions` from mo:core (it returns the instructions a function call executes) on the suite's `runCell` for that cell, so each figure is the cell's whole query plus the dispatch to it (mo:bench's `runCell` and the suite's `runner` switch on the cell name):

| cell | 10000 | 100000 |
|---|---:|---:|
| gsa_idx | 666,266,940 | 4,342,939,144 |
| gprice_idx | 25,036,319 | 25,036,309 |
| eq_all_idx | 263,882,244 | 2,757,626,608 |
| ge_all_idx | 309,159,952 | 3,210,453,689 |
| sort1_idx | 302,217,049 | 3,141,229,152 |
| sel1pct_idx | 3,309,044 | 32,671,705 |

Raw: the mops JSON of this run (`.bench/oqlbars_extra.bench.json`, `--save`) is kept beside this file in `raw.tar.xz` as `raw/moc1161-oqlbars_extra.bench.json`.


Source of the suite: `bench/oqlbars_extra.bench.mo` of the scratch copy, kept beside this file as `src/oqlbars_extra.bench.mo`.
