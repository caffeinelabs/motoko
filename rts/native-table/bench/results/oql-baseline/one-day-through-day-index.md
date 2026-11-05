# One day of a ledger-like table through an index on `day`

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.

What: rows D = {id, day = floor(k / 3,000), status = k mod 2, amount = 7k mod 1000}, 10^5 rows (3,000 per day), in an IndexedMap with `day #ordered` (`*_ord`), with `day #hash` (`*_hash`), or a plain core Map entity (`*_none`). Queries: `range_day_*` count where day >= 17 and day <= 17 (OQL serves only a lone `#eq` count from an index's statistics, so the range is streamed); `eq_day_hash` count where day = 17 (the posting's size); `group_day_*` where day = 17, group by status, count and sum(amount).


## moc 1.14.0

- cwd: $OQL
- moc: Motoko compiler 1.14.0 (source 27krqrwk-vmsjiajs-zrcbmnsi-0p7rqmvj)
- cmd: DFX_MOC_PATH=$MOPS_CACHE/moc/1.14.0/moc PATH=$MOPS_CACHE/moc/1.14.0:$PATH CI=1 mops bench oqlbars_dayidx --replica pocket-ic --query --save --verbose
- date: 2026-10-14T16:03:06Z
- pipeline (mops): moc `--release --force-gc` plus OQL's `[moc] args` (`--default-persistent-actors -fshared-code -W M0236`), each cell in a query call (`--query`); collection in every cell: 0 (checked: rts_reclaimed and rts_collector_instructions are 0 in every cell)

Instructions per cell, as `mops bench` reports them: the canister mops builds around the suite calls `InternetComputer.countInstructions` from mo:core (it returns the instructions a function call executes) on the suite's `runCell` for that cell, so each figure is the cell's whole query plus the dispatch to it (mo:bench's `runCell` and the suite's `runner` switch on the cell name):

| cell | 100000 |
|---|---:|
| range_day_ord | 60,616,651 |
| eq_day_hash | 48,295 |
| group_day_hash | 94,030,705 |
| group_day_ord | 94,031,165 |
| range_day_none | 1,179,467,002 |
| group_day_none | 1,168,621,638 |

Raw: the mops JSON of this run (`.bench/oqlbars_dayidx.bench.json`, `--save`) is kept beside this file in `raw.tar.xz` as `raw/moc1140-oqlbars_dayidx.bench.json`.


## moc 1.16.1

- cwd: $OQL_1161
- moc: Motoko compiler 1.16.1 (source cmnnq83c-j64xzf3y-dbl9326w-qfay9xca)
- cmd: DFX_MOC_PATH=$MOPS_CACHE/moc/1.16.1/moc PATH=$MOPS_CACHE/moc/1.16.1:$PATH CI=1 mops bench oqlbars_dayidx --replica pocket-ic --query --save --verbose
- date: 2026-10-14T16:04:08Z
- pipeline (mops): moc `--release --force-gc` plus OQL's `[moc] args` (`--default-persistent-actors -fshared-code -W M0236`), each cell in a query call (`--query`); collection in every cell: 0 (checked: rts_reclaimed and rts_collector_instructions are 0 in every cell)

Instructions per cell, as `mops bench` reports them: the canister mops builds around the suite calls `InternetComputer.countInstructions` from mo:core (it returns the instructions a function call executes) on the suite's `runCell` for that cell, so each figure is the cell's whole query plus the dispatch to it (mo:bench's `runCell` and the suite's `runner` switch on the cell name):

| cell | 100000 |
|---|---:|
| range_day_ord | 60,734,963 |
| eq_day_hash | 48,276 |
| group_day_hash | 93,993,204 |
| group_day_ord | 93,993,664 |
| range_day_none | 1,185,649,750 |
| group_day_none | 1,174,648,016 |

Raw: the mops JSON of this run (`.bench/oqlbars_dayidx.bench.json`, `--save`) is kept beside this file in `raw.tar.xz` as `raw/moc1161-oqlbars_dayidx.bench.json`.


Source of the suite: `bench/oqlbars_dayidx.bench.mo` of the scratch copy, kept beside this file as `src/oqlbars_dayidx.bench.mo`.
