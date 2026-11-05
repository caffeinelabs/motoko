# Shape O without an index: columnar Table and heap Map; footer extremes

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.

What: rows O = {id, customerId = k mod 100, amount = k mod 1000, paid = k even} at 10^3, 10^4, 4·10^4, 10^5, no secondary index anywhere. `*_col`: `OQL.Table` (auto-flush at 50,000 rows, then `flush`), so its segment footers serve sum, min and max; `*_heap`: a plain core `Map` entity. `min_col_del`/`max_col_del`: the same Table with position 1 deleted (the segment holding a deleted row is scanned row by row). `sum_col_buffered`: a Table with auto-flush off (`newWith(.., 0)`), every row in the write buffer. Queries: min/max/sum(amount); filter `paid = true` (all matching rows returned); count where `paid = true`; range `amount >= 500`; top-10 by amount desc; group by paid, count; `paid = true and amount >= 500`.


## moc 1.14.0

- cwd: $OQL
- moc: Motoko compiler 1.14.0 (source 27krqrwk-vmsjiajs-zrcbmnsi-0p7rqmvj)
- cmd: DFX_MOC_PATH=$MOPS_CACHE/moc/1.14.0/moc PATH=$MOPS_CACHE/moc/1.14.0:$PATH CI=1 mops bench oqlbars_columnar --replica pocket-ic --query --save --verbose
- date: 2026-10-14T07:47:17Z
- pipeline (mops): moc `--release --force-gc` plus OQL's `[moc] args` (`--default-persistent-actors -fshared-code -W M0236`), each cell in a query call (`--query`); collection in every cell: 0 (checked: rts_reclaimed and rts_collector_instructions are 0 in every cell)

Instructions per cell, as `mops bench` reports them: the canister mops builds around the suite calls `InternetComputer.countInstructions` from mo:core (it returns the instructions a function call executes) on the suite's `runCell` for that cell, so each figure is the cell's whole query plus the dispatch to it (mo:bench's `runCell` and the suite's `runner` switch on the cell name):

| cell | 1000 | 10000 | 40000 | 100000 |
|---|---:|---:|---:|---:|
| min_col | 51,389 | 51,725 | 51,837 | 53,838 |
| max_col | 51,672 | 52,008 | 52,120 | 54,121 |
| min_col_del | 3,612,394 | 35,679,790 | 142,570,114 | 178,201,993 |
| max_col_del | 3,840,213 | 35,907,609 | 142,797,933 | 178,429,816 |
| sum_col_buffered | 1,213,207 | 11,457,958 | 45,434,896 | 113,275,970 |
| sum_col | 47,060 | 47,396 | 47,508 | 49,509 |
| filter_col | 16,805,622 | 167,020,846 | 668,788,406 | 1,702,513,902 |
| filter_heap | 11,095,557 | 110,325,754 | 442,193,894 | 1,103,990,013 |
| count_col | 8,785,106 | 86,617,630 | 346,058,358 | 898,125,439 |
| count_heap | 8,814,116 | 87,524,189 | 350,949,613 | 875,882,389 |
| range_col | 16,639,817 | 165,351,839 | 662,108,739 | 1,685,812,472 |
| range_heap | 11,265,990 | 112,016,989 | 448,954,469 | 1,120,889,268 |
| topn_col | 23,011,158 | 275,303,970 | 1,233,050,417 | 3,383,686,691 |
| topn_heap | 23,108,275 | 277,565,628 | 1,242,597,161 | 3,375,669,877 |
| group_col | 14,876,770 | 145,004,216 | 577,422,576 | 1,474,618,376 |
| group_heap | 16,821,590 | 164,818,605 | 657,917,063 | 1,642,269,034 |
| multi_col | 14,410,337 | 142,802,521 | 570,775,461 | 1,459,994,026 |
| multi_heap | 9,863,081 | 97,856,508 | 392,244,797 | 979,093,662 |

Raw: the mops JSON of this run (`.bench/oqlbars_columnar.bench.json`, `--save`) is kept beside this file in `raw.tar.xz` as `raw/moc1140-oqlbars_columnar.bench.json`.


## moc 1.16.1

- cwd: $OQL_1161
- moc: Motoko compiler 1.16.1 (source cmnnq83c-j64xzf3y-dbl9326w-qfay9xca)
- cmd: DFX_MOC_PATH=$MOPS_CACHE/moc/1.16.1/moc PATH=$MOPS_CACHE/moc/1.16.1:$PATH CI=1 mops bench oqlbars_columnar --replica pocket-ic --query --save --verbose
- date: 2026-10-14T11:11:22Z
- pipeline (mops): moc `--release --force-gc` plus OQL's `[moc] args` (`--default-persistent-actors -fshared-code -W M0236`), each cell in a query call (`--query`); collection in every cell: 0 (checked: rts_reclaimed and rts_collector_instructions are 0 in every cell)

Instructions per cell, as `mops bench` reports them: the canister mops builds around the suite calls `InternetComputer.countInstructions` from mo:core (it returns the instructions a function call executes) on the suite's `runCell` for that cell, so each figure is the cell's whole query plus the dispatch to it (mo:bench's `runCell` and the suite's `runner` switch on the cell name):

| cell | 1000 | 10000 | 40000 | 100000 |
|---|---:|---:|---:|---:|
| min_col | 51,243 | 51,579 | 51,691 | 53,621 |
| max_col | 51,526 | 51,862 | 51,974 | 53,904 |
| min_col_del | 3,622,238 | 35,779,634 | 142,969,958 | 178,701,766 |
| max_col_del | 3,835,087 | 35,992,483 | 143,182,807 | 178,914,619 |
| sum_col_buffered | 1,202,448 | 11,375,619 | 45,130,197 | 112,537,231 |
| sum_col | 46,999 | 47,335 | 47,447 | 49,381 |
| filter_col | 16,812,927 | 167,104,651 | 669,127,211 | 1,703,362,480 |
| filter_heap | 11,103,560 | 110,408,269 | 442,524,789 | 1,104,817,672 |
| count_col | 8,758,412 | 86,361,436 | 345,037,164 | 895,574,018 |
| count_heap | 8,799,326 | 87,376,911 | 350,360,715 | 874,410,255 |
| range_col | 16,647,118 | 165,435,640 | 662,447,540 | 1,686,661,042 |
| range_heap | 11,273,997 | 112,099,508 | 449,285,368 | 1,121,716,931 |
| topn_col | 22,913,657 | 274,046,818 | 1,227,622,264 | 3,370,297,827 |
| topn_heap | 23,021,926 | 276,416,640 | 1,237,600,552 | 3,363,359,556 |
| group_col | 14,788,273 | 144,199,551 | 574,274,683 | 1,466,814,882 |
| group_heap | 16,748,425 | 164,162,284 | 655,360,894 | 1,635,944,263 |
| multi_col | 14,419,446 | 142,908,380 | 571,203,820 | 1,461,067,158 |
| multi_heap | 9,875,712 | 97,986,401 | 392,765,570 | 980,396,199 |

Raw: the mops JSON of this run (`.bench/oqlbars_columnar.bench.json`, `--save`) is kept beside this file in `raw.tar.xz` as `raw/moc1161-oqlbars_columnar.bench.json`.


Source of the suite: `bench/oqlbars_columnar.bench.mo` of the scratch copy, kept beside this file as `src/oqlbars_columnar.bench.mo`.
