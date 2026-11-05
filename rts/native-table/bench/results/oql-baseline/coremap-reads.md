# core Map and IndexedMap reads, reference row R

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.

What: over 10^4 and 10^5 rows of R (as `point-pk-rows-R.md`, without the id field), keyed by Nat (`Map<Nat, R>`), by a 27-byte Text key (`Map<Text, R>`), and in an `IndexedMap<Nat, R>` with three secondary indexes. Cells: one `get` (`*_1`), 1,000 `get`s of keys (7919 i + 13) mod n (`*_x1000`, divide by 1,000 for a per-get figure), `values()` and `entries()` over every row (divide by n), `size()`, and `empty` (the runner with no work: the cell overhead). Text keys: `get_text_*` = "account-" + 19 zero-padded digits (a 20-byte shared prefix); `get_text2_*` = 19 digits of k x 2,654,435,761 then "-account" (keys differing in their first bytes).


## moc 1.14.0

- cwd: $OQL
- moc: Motoko compiler 1.14.0 (source 27krqrwk-vmsjiajs-zrcbmnsi-0p7rqmvj)
- cmd: DFX_MOC_PATH=$MOPS_CACHE/moc/1.14.0/moc PATH=$MOPS_CACHE/moc/1.14.0:$PATH CI=1 mops bench oqlbars_coremap --replica pocket-ic --query --save --verbose
- date: 2026-10-14T11:08:49Z
- pipeline (mops): moc `--release --force-gc` plus OQL's `[moc] args` (`--default-persistent-actors -fshared-code -W M0236`), each cell in a query call (`--query`); collection in every cell: 0 (checked: rts_reclaimed and rts_collector_instructions are 0 in every cell)

Instructions per cell, as `mops bench` reports them: the canister mops builds around the suite calls `InternetComputer.countInstructions` from mo:core (it returns the instructions a function call executes) on the suite's `runCell` for that cell, so each figure is the cell's whole query plus the dispatch to it (mo:bench's `runCell` and the suite's `runner` switch on the cell name):

| cell | 10000 | 100000 |
|---|---:|---:|
| empty | 1,653 | 1,647 |
| get_text2_1 | 26,224 | 32,345 |
| get_text2_x1000 | 23,765,353 | 32,736,086 |
| get_nat_1 | 10,550 | 10,776 |
| get_nat_x1000 | 7,657,153 | 8,824,941 |
| get_text_1 | 69,882 | 78,134 |
| get_text_x1000 | 55,866,870 | 63,341,983 |
| im_get_1 | 11,183 | 11,409 |
| im_get_x1000 | 7,702,689 | 8,870,469 |
| values_all | 29,109,683 | 281,716,707 |
| entries_all | 27,999,245 | 270,616,269 |
| size_map | 4,234 | 4,228 |
| size_im | 4,205 | 4,195 |

Raw: the mops JSON of this run (`.bench/oqlbars_coremap.bench.json`, `--save`) is kept beside this file in `raw.tar.xz` as `raw/moc1140-oqlbars_coremap.bench.json`.


## moc 1.16.1

- cwd: $OQL_1161
- moc: Motoko compiler 1.16.1 (source cmnnq83c-j64xzf3y-dbl9326w-qfay9xca)
- cmd: DFX_MOC_PATH=$MOPS_CACHE/moc/1.16.1/moc PATH=$MOPS_CACHE/moc/1.16.1:$PATH CI=1 mops bench oqlbars_coremap --replica pocket-ic --query --save --verbose
- date: 2026-10-14T11:10:57Z
- pipeline (mops): moc `--release --force-gc` plus OQL's `[moc] args` (`--default-persistent-actors -fshared-code -W M0236`), each cell in a query call (`--query`); collection in every cell: 0 (checked: rts_reclaimed and rts_collector_instructions are 0 in every cell)

Instructions per cell, as `mops bench` reports them: the canister mops builds around the suite calls `InternetComputer.countInstructions` from mo:core (it returns the instructions a function call executes) on the suite's `runCell` for that cell, so each figure is the cell's whole query plus the dispatch to it (mo:bench's `runCell` and the suite's `runner` switch on the cell name):

| cell | 10000 | 100000 |
|---|---:|---:|
| empty | 1,661 | 1,655 |
| get_text2_1 | 26,229 | 32,354 |
| get_text2_x1000 | 23,762,105 | 32,736,846 |
| get_nat_1 | 10,559 | 10,785 |
| get_nat_x1000 | 7,657,897 | 8,825,685 |
| get_text_1 | 69,891 | 78,143 |
| get_text_x1000 | 55,867,618 | 63,342,735 |
| im_get_1 | 11,192 | 11,418 |
| im_get_x1000 | 7,703,437 | 8,871,237 |
| values_all | 28,557,489 | 276,194,657 |
| entries_all | 27,447,043 | 265,094,211 |
| size_map | 4,227 | 4,221 |
| size_im | 4,198 | 4,188 |

Raw: the mops JSON of this run (`.bench/oqlbars_coremap.bench.json`, `--save`) is kept beside this file in `raw.tar.xz` as `raw/moc1161-oqlbars_coremap.bench.json`.


Source of the suite: `bench/oqlbars_coremap.bench.mo` of the scratch copy, kept beside this file as `src/oqlbars_coremap.bench.mo`.
