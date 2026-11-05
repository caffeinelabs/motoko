# Batched IndexedMap writes in update context, collection counted

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.


What: OQL's `index_write` and `backfill` bench shapes run as update messages of a harness canister (built with `moc --release --default-persistent-actors`, no `-fshared-code`; the `IndexedMap` code is OQL's). Rows L = {id, ts = k, kind (10% burn, 30% mint, 60% xfer), amount = 7k mod 1000}; rows T = {id, pair (8 pairs, hot-skewed), price = 7919k mod 10000, ts = k}. Cells: `build_Kidx` a fresh IndexedMap with K indexes (`ts #ordered`, `kind #hash`, `amount #ordered`, in that order) and n inserts, the map kept live; `reindex_2` n updates moving `ts` on a map built with 2 indexes; `delete_2` n deletes on it; `buildT_0idx`/`buildT_price`/`buildT_comp` n inserts of T with no index, `price #ordered`, the composite `[pair, price] #ordered`; `backfill_price`/`backfill_comp` `addIndex`/`addComposite` and `Backfill.step` in 10^4-row steps over a map of n T rows built without an index.

Method per cell: a fresh canister; the pre-state messages (not measured); `tick` messages until the collector spent under 100,000 instructions for three messages in a row; the measured message, sent through a relay canister so the target's cycles carry no ingress fee; the message's collector instructions (`rts_collector_instructions`, read in a query after it); `tick` until idle again, their collector instructions summed as the tail. Per op: in-method = `performanceCounter(0)` around the pass / n; with collection = (in-method + the message's collection + the tail's) / n; charged16 = that plus 4,000 per dirty page as this host charges them (one fee per dirty 16 KiB host page: calibrated, four writes 4 KiB apart cost one fee), divided by n. Pages are derived from the message's cycles: cycles = 5,000,000 + 2 x (mutator + collector + 4,000 x pages + 19); the residue 19 is constant in every message (column resid).


## moc 1.14.0

Command, in `src/`: `node wbatch.mjs <wasm dir built by src/build.sh> <out> 10000,40000,100000 -buildR` (source `src/wbatch.mjs`). The last argument leaves out the four `buildR_*` cells: this run's form of the script did not have them, and they call `wBuildR`, which the `wh.wasm` of `src/build.sh` lacks, so without that argument the run stops at the first `buildR_*` cell, after the other 10,000-row cells. The cells that remain run in this run's order, the order of the table below.

| cell | n | in-method total | collection (message) | collection (tail, ticks) | pages16 | resid | per op in-method | per op with collection | per op charged16 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| build_0idx | 10,000 | 183,594,323 | 330 | 990 (3) | 725 | 19 | 18,359 | 18,360 | 18,650 |
| build_1idx | 10,000 | 425,453,805 | 330 | 990 (3) | 1,660 | 19 | 42,545 | 42,546 | 43,210 |
| build_2idx | 10,000 | 492,497,411 | 330 | 990 (3) | 1,802 | 19 | 49,250 | 49,250 | 49,971 |
| build_3idx | 10,000 | 618,864,296 | 330 | 990 (3) | 2,224 | 19 | 61,886 | 61,887 | 62,776 |
| reindex_2 | 10,000 | 880,966,669 | 106,944,459 | 990 (3) | 5,672 | 19 | 88,097 | 98,791 | 101,060 |
| delete_2 | 10,000 | 671,023,747 | 330 | 990 (3) | 3,618 | 19 | 67,102 | 67,103 | 68,550 |
| buildT_0idx | 10,000 | 183,764,779 | 330 | 990 (3) | 725 | 19 | 18,376 | 18,377 | 18,667 |
| buildT_price | 10,000 | 451,364,771 | 330 | 990 (3) | 1,706 | 19 | 45,136 | 45,137 | 45,819 |
| buildT_comp | 10,000 | 734,839,399 | 330 | 990 (3) | 2,280 | 19 | 73,484 | 73,484 | 74,396 |
| backfill_price | 10,000 | 321,439,186 | 330 | 990 (3) | 1,497 | 19 | 32,144 | 32,144 | 32,743 |
| backfill_comp | 10,000 | 604,954,409 | 330 | 990 (3) | 2,070 | 19 | 60,495 | 60,496 | 61,324 |
| build_0idx | 40,000 | 830,859,609 | 330 | 990 (3) | 3,066 | 19 | 20,771 | 20,772 | 21,078 |
| build_1idx | 40,000 | 1,933,471,665 | 380,148,297 | 1,236 (3) | 8,052 | 19 | 48,337 | 57,841 | 58,646 |
| build_2idx | 40,000 | 2,201,576,556 | 475,486,110 | 990 (3) | 9,110 | 19 | 55,039 | 66,927 | 67,838 |
| build_3idx | 40,000 | 2,678,129,124 | 454,378,142 | 990 (3) | 10,594 | 19 | 66,953 | 78,313 | 79,372 |
| reindex_2 | 40,000 | 3,707,044,354 | 434,844,780 | 1,236 (3) | 17,045 | 19 | 92,676 | 103,547 | 105,252 |
| delete_2 | 40,000 | 2,994,016,787 | 22,563,195 | 990 (3) | 9,656 | 19 | 74,850 | 75,415 | 76,380 |
| buildT_0idx | 40,000 | 831,541,565 | 330 | 990 (3) | 3,066 | 19 | 20,789 | 20,789 | 21,095 |
| buildT_price | 40,000 | 1,479,358,066 | 206,542,926 | 990 (3) | 5,866 | 19 | 36,984 | 42,148 | 42,734 |
| buildT_comp | 40,000 | 2,240,555,032 | 220,433,499 | 990 (3) | 7,465 | 19 | 56,014 | 61,525 | 62,271 |
| backfill_price | 40,000 | 863,196,719 | 203,055,531 | 1,236 (3) | 7,908 | 19 | 21,580 | 26,656 | 27,447 |
| backfill_comp | 40,000 | 1,624,554,529 | 251,798,986 | 990 (3) | 9,640 | 19 | 40,614 | 46,909 | 47,873 |
| build_0idx | 100,000 | 2,192,635,399 | 274,821,210 | 1,236 (3) | 8,289 | 19 | 21,926 | 24,675 | 25,006 |
| build_1idx | 100,000 | 5,132,437,878 | 489,688,734 | 602,561,812 (5) | 20,181 | 19 | 51,324 | 62,247 | 63,054 |
| build_2idx | 100,000 | 5,796,078,387 | 439,033,290 | 727,338,143 (5) | 21,352 | 19 | 57,961 | 69,624 | 70,479 |
| build_3idx | 100,000 | 6,959,923,130 | 383,614,976 | 809,145,156 (5) | 24,958 | 19 | 69,599 | 81,527 | 82,525 |
| reindex_2 | 100,000 | 9,690,164,266 | 350,979,774 | 839,800,287 (5) | 42,229 | 19 | 96,902 | 108,809 | 110,499 |
| delete_2 | 100,000 | 7,817,176,653 | 49,575,435 | 990 (3) | 24,925 | 19 | 78,172 | 78,668 | 79,665 |
| buildT_0idx | 100,000 | 2,194,340,355 | 274,821,218 | 1,236 (3) | 8,289 | 19 | 21,943 | 24,692 | 25,023 |
| buildT_price | 100,000 | 3,656,461,221 | 421,755,531 | 990 (3) | 14,002 | 19 | 36,565 | 40,782 | 41,342 |
| buildT_comp | 100,000 | 5,373,103,168 | 455,603,319 | 990 (3) | 17,747 | 19 | 53,731 | 58,287 | 58,997 |
| backfill_price | 100,000 | 2,000,087,506 | 426,686,901 | 990 (3) | 15,589 | 19 | 20,001 | 24,268 | 24,891 |
| backfill_comp | 100,000 | 3,717,014,356 | 458,577,829 | 990 (3) | 19,294 | 19 | 37,170 | 41,756 | 42,528 |

## moc 1.16.1

Command, in `src/`: `node wbatch.mjs <wasm dir built by src/build.sh> <out> 10000,40000,100000 -buildR` (source `src/wbatch.mjs`). The last argument leaves out the four `buildR_*` cells: this run's form of the script did not have them, and they call `wBuildR`, which the `wh.wasm` of `src/build.sh` lacks, so without that argument the run stops at the first `buildR_*` cell, after the other 10,000-row cells. The cells that remain run in this run's order, the order of the table below.

| cell | n | in-method total | collection (message) | collection (tail, ticks) | pages16 | resid | per op in-method | per op with collection | per op charged16 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| build_0idx | 10,000 | 179,896,400 | 330 | 990 (3) | 725 | 19 | 17,990 | 17,990 | 18,280 |
| build_1idx | 10,000 | 415,926,944 | 330 | 990 (3) | 1,660 | 19 | 41,593 | 41,593 | 42,257 |
| build_2idx | 10,000 | 481,909,003 | 330 | 990 (3) | 1,802 | 19 | 48,191 | 48,191 | 48,912 |
| build_3idx | 10,000 | 605,646,064 | 330 | 990 (3) | 2,224 | 19 | 60,565 | 60,565 | 61,454 |
| reindex_2 | 10,000 | 858,080,773 | 106,945,362 | 990 (3) | 5,672 | 19 | 85,808 | 96,503 | 98,772 |
| delete_2 | 10,000 | 653,582,177 | 330 | 990 (3) | 3,618 | 19 | 65,358 | 65,358 | 66,806 |
| buildT_0idx | 10,000 | 180,066,856 | 330 | 990 (3) | 725 | 19 | 18,007 | 18,007 | 18,297 |
| buildT_price | 10,000 | 440,131,270 | 330 | 990 (3) | 1,706 | 19 | 44,013 | 44,013 | 44,696 |
| buildT_comp | 10,000 | 721,122,467 | 330 | 990 (3) | 2,280 | 19 | 72,112 | 72,112 | 73,024 |
| backfill_price | 10,000 | 311,009,337 | 330 | 990 (3) | 1,497 | 19 | 31,101 | 31,101 | 31,700 |
| backfill_comp | 10,000 | 592,041,129 | 330 | 990 (3) | 2,070 | 19 | 59,204 | 59,204 | 60,032 |
| build_0idx | 40,000 | 815,082,107 | 330 | 990 (3) | 3,066 | 19 | 20,377 | 20,377 | 20,684 |
| build_1idx | 40,000 | 1,891,663,252 | 380,149,152 | 1,236 (3) | 8,052 | 19 | 47,292 | 56,795 | 57,601 |
| build_2idx | 40,000 | 2,155,526,242 | 475,484,931 | 990 (3) | 9,110 | 19 | 53,888 | 65,775 | 66,686 |
| build_3idx | 40,000 | 2,622,763,706 | 454,379,181 | 990 (3) | 10,594 | 19 | 65,569 | 76,929 | 77,988 |
| reindex_2 | 40,000 | 3,612,991,329 | 434,843,699 | 1,236 (3) | 17,045 | 19 | 90,325 | 101,196 | 102,900 |
| delete_2 | 40,000 | 2,918,689,207 | 22,564,266 | 990 (3) | 9,656 | 19 | 72,967 | 73,531 | 74,497 |
| buildT_0idx | 40,000 | 815,764,063 | 330 | 990 (3) | 3,066 | 19 | 20,394 | 20,394 | 20,701 |
| buildT_price | 40,000 | 1,449,076,793 | 206,543,245 | 990 (3) | 5,866 | 19 | 36,227 | 41,391 | 41,977 |
| buildT_comp | 40,000 | 2,203,476,256 | 220,435,418 | 990 (3) | 7,465 | 19 | 55,087 | 60,598 | 61,344 |
| backfill_price | 40,000 | 837,115,310 | 203,056,474 | 1,236 (3) | 7,908 | 19 | 20,928 | 26,004 | 26,795 |
| backfill_comp | 40,000 | 1,591,675,617 | 251,800,080 | 990 (3) | 9,640 | 19 | 39,792 | 46,087 | 47,051 |
| build_0idx | 100,000 | 2,152,701,019 | 274,823,177 | 1,236 (3) | 8,289 | 19 | 21,527 | 24,275 | 24,607 |
| build_1idx | 100,000 | 5,023,906,583 | 489,687,940 | 602,563,129 (5) | 20,181 | 19 | 50,239 | 61,162 | 61,969 |
| build_2idx | 100,000 | 5,677,184,347 | 439,032,851 | 727,337,930 (5) | 21,352 | 19 | 56,772 | 68,436 | 69,290 |
| build_3idx | 100,000 | 6,818,852,426 | 383,614,457 | 809,146,722 (5) | 24,958 | 19 | 68,189 | 80,116 | 81,114 |
| reindex_2 | 100,000 | 9,449,159,345 | 350,979,784 | 839,799,754 (5) | 42,229 | 19 | 94,492 | 106,399 | 108,089 |
| delete_2 | 100,000 | 7,624,461,645 | 49,576,791 | 990 (3) | 24,926 | 19 | 76,245 | 76,740 | 77,737 |
| buildT_0idx | 100,000 | 2,154,405,975 | 274,823,185 | 1,236 (3) | 8,289 | 19 | 21,544 | 24,292 | 24,624 |
| buildT_price | 100,000 | 3,585,906,684 | 421,756,434 | 990 (3) | 14,002 | 19 | 35,859 | 40,077 | 40,637 |
| buildT_comp | 100,000 | 5,287,122,984 | 455,605,502 | 990 (3) | 17,747 | 19 | 52,871 | 57,427 | 58,137 |
| backfill_price | 100,000 | 1,940,523,036 | 426,701,108 | 990 (3) | 15,589 | 19 | 19,405 | 23,672 | 24,296 |
| backfill_comp | 100,000 | 3,642,024,239 | 458,592,508 | 990 (3) | 19,294 | 19 | 36,420 | 41,006 | 41,778 |

Raw records: none kept; each cell is one measured message, listed whole in the tables above.
