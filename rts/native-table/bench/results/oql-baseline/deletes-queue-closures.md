# Top-50 after deletes, queue reads after deletes or updates, closures

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.

What, at 10^4 and 10^5 rows: `top50_amt` the candidate API (`candidatesRange("amount", null, null, #desc)`, typed rows) taking 50, over L in an IndexedMap with `amount #ordered`; `top50_amt_del` the same after deleting every row with amount >= 990 (1%). Queue rows Q = {id, status, amount, ts}, status "pending" when k mod 10 = 0 else "other": `q_pend_*` `status = "pending"` limit 10 after deleting keys 0 .. n/100 - 1; `u_pend_*` the same after updating those keys to status "done"; `*_idx` with `status #hash`, `*_scan` without an index. Closures, over M as a plain entity with `.scopedPerUser()`, read by a scoped subject (`#scoped`): `own_*` `.ownedByWith("t", canSee)` with canSee always true; `view_*` the same plus `.viewWith` returning the row unchanged; `scoped_*` `Entity.newScoped` returning every row; `unr_*` the plain entity read unrestricted. `*_scan` returns every row; `*_none` filters `z = 1`, which no row matches (every row is scanned, none returned).


## moc 1.14.0

- cwd: $OQL
- moc: Motoko compiler 1.14.0 (source 27krqrwk-vmsjiajs-zrcbmnsi-0p7rqmvj)
- cmd: DFX_MOC_PATH=$MOPS_CACHE/moc/1.14.0/moc PATH=$MOPS_CACHE/moc/1.14.0:$PATH CI=1 mops bench oqlbars_extra2 --replica pocket-ic --query --save --verbose
- date: 2026-10-14T11:09:13Z
- pipeline (mops): moc `--release --force-gc` plus OQL's `[moc] args` (`--default-persistent-actors -fshared-code -W M0236`), each cell in a query call (`--query`); collection in every cell: 0 (checked: rts_reclaimed and rts_collector_instructions are 0 in every cell)

Instructions per cell, as `mops bench` reports them: the canister mops builds around the suite calls `InternetComputer.countInstructions` from mo:core (it returns the instructions a function call executes) on the suite's `runCell` for that cell, so each figure is the cell's whole query plus the dispatch to it (mo:bench's `runCell` and the suite's `runner` switch on the cell name):

| cell | 10000 | 100000 |
|---|---:|---:|
| top50_amt | 494,687 | 530,281 |
| top50_amt_del | 487,228 | 521,670 |
| q_pend_idx | 375,269 | 385,796 |
| q_pend_scan | 1,363,679 | 1,363,677 |
| u_pend_idx | 373,075 | 380,450 |
| u_pend_scan | 2,531,239 | 13,036,956 |
| own_scan | 204,852,883 | 2,050,377,744 |
| view_scan | 283,053,344 | 2,834,907,653 |
| scoped_scan | 179,711,975 | 1,798,946,317 |
| unr_scan | 179,708,196 | 1,798,942,538 |
| own_none | 111,316,696 | 1,113,610,443 |
| view_none | 189,517,193 | 1,897,090,491 |
| scoped_none | 86,175,872 | 862,178,789 |
| unr_none | 86,171,597 | 862,174,514 |

Raw: the mops JSON of this run (`.bench/oqlbars_extra2.bench.json`, `--save`) is kept beside this file in `raw.tar.xz` as `raw/moc1140-oqlbars_extra2.bench.json`.


## moc 1.16.1

- cwd: $OQL_1161
- moc: Motoko compiler 1.16.1 (source cmnnq83c-j64xzf3y-dbl9326w-qfay9xca)
- cmd: DFX_MOC_PATH=$MOPS_CACHE/moc/1.16.1/moc PATH=$MOPS_CACHE/moc/1.16.1:$PATH CI=1 mops bench oqlbars_extra2 --replica pocket-ic --query --save --verbose
- date: 2026-10-14T11:13:16Z
- pipeline (mops): moc `--release --force-gc` plus OQL's `[moc] args` (`--default-persistent-actors -fshared-code -W M0236`), each cell in a query call (`--query`); collection in every cell: 0 (checked: rts_reclaimed and rts_collector_instructions are 0 in every cell)

Instructions per cell, as `mops bench` reports them: the canister mops builds around the suite calls `InternetComputer.countInstructions` from mo:core (it returns the instructions a function call executes) on the suite's `runCell` for that cell, so each figure is the cell's whole query plus the dispatch to it (mo:bench's `runCell` and the suite's `runner` switch on the cell name):

| cell | 10000 | 100000 |
|---|---:|---:|
| top50_amt | 494,797 | 530,623 |
| top50_amt_del | 487,330 | 521,996 |
| q_pend_idx | 375,272 | 385,795 |
| q_pend_scan | 1,368,888 | 1,368,886 |
| u_pend_idx | 373,082 | 380,453 |
| u_pend_scan | 2,542,824 | 13,105,941 |
| own_scan | 205,812,608 | 2,059,977,469 |
| view_scan | 284,913,065 | 2,853,507,374 |
| scoped_scan | 180,591,684 | 1,807,746,337 |
| unr_scan | 180,587,893 | 1,807,742,235 |
| own_none | 112,146,255 | 1,121,910,002 |
| view_none | 191,246,748 | 1,914,390,046 |
| scoped_none | 86,925,415 | 869,678,332 |
| unr_none | 86,921,128 | 869,674,045 |

Raw: the mops JSON of this run (`.bench/oqlbars_extra2.bench.json`, `--save`) is kept beside this file in `raw.tar.xz` as `raw/moc1161-oqlbars_extra2.bench.json`.


Source of the suite: `bench/oqlbars_extra2.bench.mo` of the scratch copy, kept beside this file as `src/oqlbars_extra2.bench.mo`.
