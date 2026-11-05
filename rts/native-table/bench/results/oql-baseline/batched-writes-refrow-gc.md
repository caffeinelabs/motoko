# Batched inserts of the reference row R into an IndexedMap with 0-3 indexes, collection counted

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.


What: as `batched-writes-gc.md` (same method), rows R = {ts : Int, amount : Nat, kind : Text (3 values), owner : Principal (1,000 distinct), note : Text (10-60 B), flag : Bool}, built before the counter as the message's input; indexes added in the order `ts #ordered`, `kind #hash`, `amount #ordered`; OQL's implicit `_toRow` derivation (a Principal becomes `#text` of its text form). Harness `src/wh2.mo` (`wBuildR`).

Command: `node wbatch.mjs <wasm dir built by src/build2.sh> <out> 10000,40000,100000 buildR` (source `src/wbatch.mjs`; the last argument keeps only the cells whose names start with `buildR`).


## moc 1.14.0

| cell | n | in-method total | collection (message) | tail (ticks) | pages16 | per op in-method | per op with collection | per op charged16 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| buildR_0idx | 10,000 | 264,505,569 | 330 | 990 (3) | 1,184 | 26,451 | 26,451 | 26,924 |
| buildR_1idx | 10,000 | 515,122,495 | 330 | 990 (3) | 2,119 | 51,512 | 51,512 | 52,360 |
| buildR_2idx | 10,000 | 582,166,071 | 330 | 990 (3) | 2,261 | 58,217 | 58,217 | 59,121 |
| buildR_3idx | 10,000 | 708,532,943 | 330 | 990 (3) | 2,683 | 70,853 | 70,853 | 71,927 |
| buildR_0idx | 40,000 | 1,154,504,747 | 150,190,937 | 990 (3) | 5,329 | 28,863 | 32,617 | 33,150 |
| buildR_1idx | 40,000 | 2,295,445,209 | 462,761,820 | 990 (3) | 10,292 | 57,386 | 68,955 | 69,984 |
| buildR_2idx | 40,000 | 2,563,549,821 | 456,249,799 | 990 (3) | 10,852 | 64,089 | 75,495 | 76,580 |
| buildR_3idx | 40,000 | 3,040,102,376 | 450,707,204 | 1,236 (3) | 12,342 | 76,003 | 87,270 | 88,504 |
| buildR_0idx | 100,000 | 3,001,748,184 | 379,377,075 | 990 (3) | 13,425 | 30,017 | 33,811 | 34,348 |
| buildR_1idx | 100,000 | 6,043,152,315 | 425,977,160 | 717,477,684 (5) | 24,517 | 60,432 | 71,866 | 72,847 |
| buildR_2idx | 100,000 | 6,706,793,141 | 394,384,641 | 808,607,807 (5) | 25,656 | 67,068 | 79,098 | 80,124 |
| buildR_3idx | 100,000 | 7,870,637,899 | 390,197,164 | 838,573,694 (5) | 29,553 | 78,706 | 90,994 | 92,176 |

## moc 1.16.1

| cell | n | in-method total | collection (message) | tail (ticks) | pages16 | per op in-method | per op with collection | per op charged16 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| buildR_0idx | 10,000 | 260,037,646 | 330 | 990 (3) | 1,184 | 26,004 | 26,004 | 26,477 |
| buildR_1idx | 10,000 | 504,825,634 | 330 | 990 (3) | 2,119 | 50,483 | 50,483 | 51,330 |
| buildR_2idx | 10,000 | 570,807,663 | 330 | 990 (3) | 2,261 | 57,081 | 57,081 | 57,985 |
| buildR_3idx | 10,000 | 694,544,711 | 330 | 990 (3) | 2,683 | 69,454 | 69,455 | 70,528 |
| buildR_0idx | 40,000 | 1,135,647,245 | 150,192,656 | 990 (3) | 5,329 | 28,391 | 32,146 | 32,679 |
| buildR_1idx | 40,000 | 2,250,556,796 | 462,759,451 | 990 (3) | 10,292 | 56,264 | 67,833 | 68,862 |
| buildR_2idx | 40,000 | 2,514,419,507 | 456,257,350 | 990 (3) | 10,852 | 62,860 | 74,267 | 75,352 |
| buildR_3idx | 40,000 | 2,981,656,958 | 450,708,275 | 1,236 (3) | 12,342 | 74,541 | 85,809 | 87,043 |
| buildR_0idx | 100,000 | 2,954,113,804 | 379,377,610 | 990 (3) | 13,425 | 29,541 | 33,335 | 33,872 |
| buildR_1idx | 100,000 | 5,926,921,020 | 425,977,391 | 717,483,720 (5) | 24,517 | 59,269 | 70,704 | 71,685 |
| buildR_2idx | 100,000 | 6,580,199,101 | 394,384,651 | 808,604,988 (5) | 25,656 | 65,802 | 77,832 | 78,858 |
| buildR_3idx | 100,000 | 7,721,867,195 | 390,197,174 | 838,574,026 (5) | 29,553 | 77,219 | 89,506 | 90,689 |

Raw records: none kept; each cell is one measured message, listed whole in the tables above.
