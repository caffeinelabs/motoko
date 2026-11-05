# OQL scale script: verify-import-50M

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.

What: OQL's `verify-import.mjs`, as in `scale-verify-import-10M.md` but with 50,000,000 rows: rows loaded as segment images, then `buildIndex` (`account_from #hash`) on chain; run as `verify-import-longtimeout.mjs` (kept here as `src/verify-import-longtimeout.mjs`), a copy of OQL's `bench/scale/verify-import.mjs` whose PocketIC processing timeout (`processingTimeoutMs`, line 55) is 3,600,000 ms instead of 600,000 (the first attempt stopped on a polling timeout during the build on this loaded host); nothing else differs. **Not completed**: the on-chain build stopped progressing after 47,250,000 of 50,000,000 rows (round 2,125, 1,443 s); no further progress line appeared for 25 minutes while both processes stayed nearly idle, and the run was stopped. No figure for the on-chain index build (`buildIndex`) at this size.

Scripts: OQL's own `bench/scale/*.mjs` at `bb5a23f`, unchanged apart from that copy, run in the scratch copy's `bench/scale` (`$OQL/bench/scale`; README.md, "Setting up the OQL copies") with `bench.wasm` built by OQL's own `bench/scale/build.sh` in the scratch copy (not this folder's `src/build.sh`; moc 1.14.0 `--force-gc --default-persistent-actors -fshared-code --max-stable-pages 1638400`); one scratch fix: `bench/scale/main.mo` lacks `import List "mo:core/List"` at `bb5a23f` and does not compile without it under moc 1.14.0. The copy's `src/ImportData.mo` is the instrumented one (`src/ImportData.mo` here), so `bench.wasm` also carries its counter reads around `putSegment`, `putIndexChunk` and `commitIndex`. Node packages in `bench/scale/node_modules`: `@dfinity/pic` 0.22.0 and `@dfinity/candid` 3.4.3, with the dependency versions `src/package-lock.json` pins (OQL's `bench/scale/package.json` asks for `^0.22.0` and `^3.4.3` and has no lockfile); `node_modules/@dfinity/pic/pocket-ic` was a link to `src/pic-ttl`, which starts PocketIC with `--ttl 86400`. @dfinity/pic 0.22.0 always starts that file and ignores the `POCKET_IC_BIN` and `binPath` the scripts set, so without the link PocketIC stops 60 s after its last finished operation, which a long Node-side step (such as building one large index segment) outlasts. Queries are real query calls (`measure`: the counter around `Executor.runWith`).

Command (in `bench/scale`, with `src/verify-import-longtimeout.mjs` copied there): `ROWS=50000000 node verify-import-longtimeout.mjs`

Output:

```
OK   count(*): got 50000000, expected 50000000
OK   sum(amount_e8s): got 25001699847298983746, expected 25001699847298983746
OK   day = 7 (zone-map pruned): got 3000, expected 3000
OK   10 <= day <= 12 (pruned): got 9000, expected 9000
FAIL account_from = 123 (scan): Canister call failed: Error from Canister lxzze-o7777-77777-aaaaa-cai: Canister exceeded the limit of 5000000000 instructions for single message execution..
OK   day = 7 & amount < 5e11: got 1516, expected 1516
FAIL spender is null (scan): Canister call failed: Error from Canister lxzze-o7777-77777-aaaaa-cai: Canister exceeded the limit of 5000000000 instructions for single message execution..
OK   materialize: rows of day = 7: got 3000, expected 3000
FAIL top-5 by day desc: Canister call failed: Error from Canister lxzze-o7777-77777-aaaaa-cai: Canister exceeded the limit of 5000000000 instructions for single message execution..
  building: 900,000/50,000,000 (round 25, 12 s, heap 397 MB unsettled, maxLive 71 MB)
  building: 1,600,000/50,000,000 (round 50, 22 s, heap 405 MB unsettled, maxLive 101 MB)
  building: 2,300,000/50,000,000 (round 75, 40 s, heap 417 MB unsettled, maxLive 101 MB)
  building: 2,900,000/50,000,000 (round 100, 52 s, heap 70 MB unsettled, maxLive 311 MB)
  building: 3,700,000/50,000,000 (round 125, 61 s, heap 330 MB unsettled, maxLive 325 MB)
  building: 4,400,000/50,000,000 (round 150, 78 s, heap 106 MB unsettled, maxLive 325 MB)
  building: 5,350,000/50,000,000 (round 175, 90 s, heap 400 MB unsettled, maxLive 395 MB)
  building: 6,000,000/50,000,000 (round 200, 106 s, heap 447 MB unsettled, maxLive 395 MB)
  building: 6,750,000/50,000,000 (round 225, 116 s, heap 397 MB unsettled, maxLive 425 MB)
  building: 7,250,000/50,000,000 (round 250, 133 s, heap 408 MB unsettled, maxLive 425 MB)
  building: 8,000,000/50,000,000 (round 275, 146 s, heap 440 MB unsettled, maxLive 447 MB)
  building: 8,500,000/50,000,000 (round 300, 162 s, heap 414 MB unsettled, maxLive 447 MB)
  building: 9,500,000/50,000,000 (round 325, 174 s, heap 430 MB unsettled, maxLive 447 MB)
  building: 10,250,000/50,000,000 (round 350, 189 s, heap 396 MB unsettled, maxLive 447 MB)
  building: 11,250,000/50,000,000 (round 375, 202 s, heap 447 MB unsettled, maxLive 451 MB)
  building: 11,750,000/50,000,000 (round 400, 213 s, heap 442 MB unsettled, maxLive 451 MB)
  building: 12,250,000/50,000,000 (round 425, 228 s, heap 431 MB unsettled, maxLive 451 MB)
  building: 12,750,000/50,000,000 (round 450, 236 s, heap 445 MB unsettled, maxLive 451 MB)
  building: 13,250,000/50,000,000 (round 475, 251 s, heap 463 MB unsettled, maxLive 458 MB)
  building: 13,750,000/50,000,000 (round 500, 261 s, heap 473 MB unsettled, maxLive 468 MB)
  building: 14,250,000/50,000,000 (round 525, 272 s, heap 485 MB unsettled, maxLive 481 MB)
  building: 14,800,000/50,000,000 (round 550, 291 s, heap 551 MB unsettled, maxLive 546 MB)
  building: 15,700,000/50,000,000 (round 575, 306 s, heap 560 MB unsettled, maxLive 555 MB)
  building: 16,300,000/50,000,000 (round 600, 321 s, heap 551 MB unsettled, maxLive 555 MB)
  building: 16,900,000/50,000,000 (round 625, 333 s, heap 552 MB unsettled, maxLive 555 MB)
  building: 17,500,000/50,000,000 (round 650, 355 s, heap 552 MB unsettled, maxLive 555 MB)
  building: 18,100,000/50,000,000 (round 675, 369 s, heap 552 MB unsettled, maxLive 555 MB)
  building: 18,700,000/50,000,000 (round 700, 383 s, heap 552 MB unsettled, maxLive 555 MB)
  building: 19,300,000/50,000,000 (round 725, 395 s, heap 552 MB unsettled, maxLive 555 MB)
  building: 19,900,000/50,000,000 (round 750, 418 s, heap 552 MB unsettled, maxLive 555 MB)
  building: 20,500,000/50,000,000 (round 775, 446 s, heap 552 MB unsettled, maxLive 555 MB)
  building: 21,100,000/50,000,000 (round 800, 473 s, heap 552 MB unsettled, maxLive 555 MB)
  building: 21,700,000/50,000,000 (round 825, 494 s, heap 553 MB unsettled, maxLive 555 MB)
  building: 22,300,000/50,000,000 (round 850, 516 s, heap 557 MB unsettled, maxLive 555 MB)
  building: 23,150,000/50,000,000 (round 875, 541 s, heap 512 MB unsettled, maxLive 555 MB)
  building: 23,650,000/50,000,000 (round 900, 557 s, heap 517 MB unsettled, maxLive 555 MB)
  building: 24,150,000/50,000,000 (round 925, 568 s, heap 538 MB unsettled, maxLive 555 MB)
  building: 24,650,000/50,000,000 (round 950, 586 s, heap 574 MB unsettled, maxLive 569 MB)
  building: 25,150,000/50,000,000 (round 975, 602 s, heap 584 MB unsettled, maxLive 580 MB)
  building: 25,400,000/50,000,000 (round 1000, 618 s, heap 603 MB unsettled, maxLive 598 MB)
  building: 25,900,000/50,000,000 (round 1025, 627 s, heap 639 MB unsettled, maxLive 634 MB)
  building: 26,400,000/50,000,000 (round 1050, 646 s, heap 656 MB unsettled, maxLive 651 MB)
  building: 26,650,000/50,000,000 (round 1075, 655 s, heap 675 MB unsettled, maxLive 670 MB)
  building: 27,150,000/50,000,000 (round 1100, 672 s, heap 690 MB unsettled, maxLive 685 MB)
  building: 27,650,000/50,000,000 (round 1125, 682 s, heap 702 MB unsettled, maxLive 697 MB)
  building: 27,900,000/50,000,000 (round 1150, 693 s, heap 701 MB unsettled, maxLive 697 MB)
  building: 28,400,000/50,000,000 (round 1175, 708 s, heap 705 MB unsettled, maxLive 700 MB)
  building: 28,700,000/50,000,000 (round 1200, 739 s, heap 762 MB unsettled, maxLive 757 MB)
  building: 29,300,000/50,000,000 (round 1225, 758 s, heap 775 MB unsettled, maxLive 770 MB)
  building: 29,900,000/50,000,000 (round 1250, 786 s, heap 772 MB unsettled, maxLive 770 MB)
  building: 30,200,000/50,000,000 (round 1275, 807 s, heap 773 MB unsettled, maxLive 770 MB)
  building: 30,800,000/50,000,000 (round 1300, 838 s, heap 774 MB unsettled, maxLive 770 MB)
  building: 31,400,000/50,000,000 (round 1325, 874 s, heap 775 MB unsettled, maxLive 770 MB)
  building: 31,700,000/50,000,000 (round 1350, 899 s, heap 775 MB unsettled, maxLive 770 MB)
  building: 32,300,000/50,000,000 (round 1375, 922 s, heap 776 MB unsettled, maxLive 771 MB)
  building: 32,900,000/50,000,000 (round 1400, 946 s, heap 777 MB unsettled, maxLive 772 MB)
  building: 33,200,000/50,000,000 (round 1425, 960 s, heap 777 MB unsettled, maxLive 772 MB)
  building: 33,800,000/50,000,000 (round 1450, 976 s, heap 778 MB unsettled, maxLive 773 MB)
  building: 34,400,000/50,000,000 (round 1475, 993 s, heap 779 MB unsettled, maxLive 774 MB)
  building: 34,700,000/50,000,000 (round 1500, 1011 s, heap 779 MB unsettled, maxLive 774 MB)
  building: 35,300,000/50,000,000 (round 1525, 1022 s, heap 780 MB unsettled, maxLive 775 MB)
  building: 35,900,000/50,000,000 (round 1550, 1041 s, heap 781 MB unsettled, maxLive 776 MB)
  building: 36,200,000/50,000,000 (round 1575, 1051 s, heap 782 MB unsettled, maxLive 776 MB)
  building: 36,800,000/50,000,000 (round 1600, 1067 s, heap 782 MB unsettled, maxLive 777 MB)
  building: 37,400,000/50,000,000 (round 1625, 1084 s, heap 783 MB unsettled, maxLive 778 MB)
  building: 37,700,000/50,000,000 (round 1650, 1107 s, heap 784 MB unsettled, maxLive 778 MB)
  building: 38,300,000/50,000,000 (round 1675, 1126 s, heap 774 MB unsettled, maxLive 779 MB)
  building: 38,600,000/50,000,000 (round 1700, 1139 s, heap 775 MB unsettled, maxLive 779 MB)
  building: 39,200,000/50,000,000 (round 1725, 1159 s, heap 775 MB unsettled, maxLive 779 MB)
  building: 39,800,000/50,000,000 (round 1750, 1171 s, heap 776 MB unsettled, maxLive 779 MB)
  building: 40,100,000/50,000,000 (round 1775, 1198 s, heap 777 MB unsettled, maxLive 779 MB)
  building: 40,700,000/50,000,000 (round 1800, 1208 s, heap 778 MB unsettled, maxLive 779 MB)
  building: 41,300,000/50,000,000 (round 1825, 1224 s, heap 778 MB unsettled, maxLive 779 MB)
  building: 41,600,000/50,000,000 (round 1850, 1238 s, heap 779 MB unsettled, maxLive 779 MB)
  building: 42,200,000/50,000,000 (round 1875, 1255 s, heap 779 MB unsettled, maxLive 779 MB)
  building: 42,800,000/50,000,000 (round 1900, 1273 s, heap 780 MB unsettled, maxLive 779 MB)
  building: 43,100,000/50,000,000 (round 1925, 1287 s, heap 781 MB unsettled, maxLive 779 MB)
  building: 43,700,000/50,000,000 (round 1950, 1300 s, heap 782 MB unsettled, maxLive 779 MB)
  building: 44,300,000/50,000,000 (round 1975, 1322 s, heap 783 MB unsettled, maxLive 779 MB)
  building: 44,600,000/50,000,000 (round 2000, 1346 s, heap 784 MB unsettled, maxLive 779 MB)
  building: 45,200,000/50,000,000 (round 2025, 1358 s, heap 776 MB unsettled, maxLive 779 MB)
  building: 45,800,000/50,000,000 (round 2050, 1384 s, heap 782 MB unsettled, maxLive 779 MB)
  building: 46,100,000/50,000,000 (round 2075, 1404 s, heap 784 MB unsettled, maxLive 779 MB)
  building: 46,700,000/50,000,000 (round 2100, 1420 s, heap 792 MB unsettled, maxLive 787 MB)
  building: 47,250,000/50,000,000 (round 2125, 1443 s, heap 744 MB unsettled, maxLive 787 MB)
```
