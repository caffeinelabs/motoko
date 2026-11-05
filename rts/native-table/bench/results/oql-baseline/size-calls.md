# size() of a core Map and of an IndexedMap

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.

What: a core `Map<Nat, L>` and an `IndexedMap<Nat, L>` with `ts #ordered, kind #hash, amount #ordered`, 10^5 rows L. Each cell runs a loop of 1,000 iterations: `empty_x1000` adds a constant, `size_map_x1000` adds `m.size()`, `size_im_x1000` adds `im.size()`; a call's cost is the difference from `empty_x1000` divided by 1,000.


## moc 1.14.0

- cwd: $OQL
- moc: Motoko compiler 1.14.0 (source 27krqrwk-vmsjiajs-zrcbmnsi-0p7rqmvj)
- cmd: DFX_MOC_PATH=$MOPS_CACHE/moc/1.14.0/moc PATH=$MOPS_CACHE/moc/1.14.0:$PATH CI=1 mops bench oqlbars_size --replica pocket-ic --query --save --verbose
- date: 2026-10-14T16:05:30Z
- pipeline (mops): moc `--release --force-gc` plus OQL's `[moc] args` (`--default-persistent-actors -fshared-code -W M0236`), each cell in a query call (`--query`); collection in every cell: 0 (checked: rts_reclaimed and rts_collector_instructions are 0 in every cell)

Instructions per cell, as `mops bench` reports them: the canister mops builds around the suite calls `InternetComputer.countInstructions` from mo:core (it returns the instructions a function call executes) on the suite's `runCell` for that cell, so each figure is the cell's whole query plus the dispatch to it (mo:bench's `runCell` and the suite's `runner` switch on the cell name):

| cell | 100000 |
|---|---:|
| empty_x1000 | 190,490 |
| size_map_x1000 | 247,782 |
| size_im_x1000 | 290,542 |

Raw: the mops JSON of this run (`.bench/oqlbars_size.bench.json`, `--save`) is kept beside this file in `raw.tar.xz` as `raw/moc1140-oqlbars_size.bench.json`.


## moc 1.16.1

- cwd: $OQL_1161
- moc: Motoko compiler 1.16.1 (source cmnnq83c-j64xzf3y-dbl9326w-qfay9xca)
- cmd: DFX_MOC_PATH=$MOPS_CACHE/moc/1.16.1/moc PATH=$MOPS_CACHE/moc/1.16.1:$PATH CI=1 mops bench oqlbars_size --replica pocket-ic --query --save --verbose
- date: 2026-10-14T16:06:16Z
- pipeline (mops): moc `--release --force-gc` plus OQL's `[moc] args` (`--default-persistent-actors -fshared-code -W M0236`), each cell in a query call (`--query`); collection in every cell: 0 (checked: rts_reclaimed and rts_collector_instructions are 0 in every cell)

Instructions per cell, as `mops bench` reports them: the canister mops builds around the suite calls `InternetComputer.countInstructions` from mo:core (it returns the instructions a function call executes) on the suite's `runCell` for that cell, so each figure is the cell's whole query plus the dispatch to it (mo:bench's `runCell` and the suite's `runner` switch on the cell name):

| cell | 100000 |
|---|---:|
| empty_x1000 | 175,498 |
| size_map_x1000 | 232,790 |
| size_im_x1000 | 275,550 |

Raw: the mops JSON of this run (`.bench/oqlbars_size.bench.json`, `--save`) is kept beside this file in `raw.tar.xz` as `raw/moc1161-oqlbars_size.bench.json`.


Source of the suite: `bench/oqlbars_size.bench.mo` of the scratch copy, kept beside this file as `src/oqlbars_size.bench.mo`.
