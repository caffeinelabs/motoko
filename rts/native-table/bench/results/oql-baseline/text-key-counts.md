# Text-key counts through a heap index written on chain, 10^4 and 10^5 rows, moc 1.14.0 and 1.16.1

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into the copies `$OQL` and `$OQL_1161` (README.md, "Setting up the OQL copies"); dependencies core 2.6.0, jayson 0.1.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in the second copy, whose `mops.toml` pins moc 1.16.1, moc 1.16.1.
Date: 2026-10-14.

## What was measured

OQL's count of one text key, `count(*) where k = <key>`, answered from a heap `#hash` index whose entries were written on chain, one row at a time, by update messages. OQL answers it from the index alone: the executor's index-served aggregate path (`aggPlan` in `src/Executor.mo`) calls the entity's `stats.count`, which is `SecondaryIndex.count` (`src/SecondaryIndex.mo`): one mo:core `Map.get` on the column's `Map<Value, Posting>` under `Predicate.compare`, which for two `#text` values is `Text.compare`, then the posting's size. The number of rows holding the key does not enter the count, except through where the key sits in the Map's B-tree (a key found in an upper node ends the descent early).

The keys are those of the native-table crate's text-key count rows, byte for byte, less the zero byte that ends each of the crate's keys (its key encoding terminates every text; OQL holds the text itself):

| keys | text of value u | crate key |
|---|---|---|
| 17 characters | `b`, then u's 16 hexadecimal digits written as the letters `a`..`p` | the 18-byte keys |
| paths, 59-199 characters | 40 shared characters (`a` + i mod 23 at position i), u's 16 digits as letters, then filler (`q` + i mod 7) up to 59 + (splitmix64's finalizer of u) mod 141 characters | the paths of 60-200 bytes |
| 1,999 characters | 1,500 shared characters, u's 16 digits, filler up to 1,999 characters | the 2,000-byte keys |

The index here holds only the 17-character keys, as the crate's count rows of 18-byte keys do.

Every key is a flat text (one blob, as Candid decodes one): built from its pieces, then passed through `Text.encodeUtf8` and `Text.decodeUtf8`. A check (`src/keycheck.mjs`) compared the canister's key of 10 values per shape with the crate's generator, transcribed: all 30 equal.

Rows: n rows, row j holding the key of value splitmix64-finalizer(j + 0x5EED) mod 100,000 with its low bit cleared, so the values are even and below 100,000, drawn as the crate's text-key rows draw theirs (from another generator); three planted keys of odd values, which no other row holds: 39,595 in 1 row, 34,623 in 6 rows and 74,037 in 1,000 rows (positions in `src/oqltext.mo`, `valueOf`). The 17-character keys, the paths and the 1,999-character keys were each built at 10^5 rows (43,038 distinct keys); the 1,999-character keys also at 10^4 rows (8,236 distinct keys), the size of the crate's 2,000-byte rows. 10^5 rows of 1,999-character keys fit: 100,000 rows, 50 Table segments, the build in 40 messages.

Canister: `src/oqltext.mo`, one per key shape and size. The same rows, by on-chain appends (`gen`, 25,000 rows per message, 2,500 for the 1,999-character keys), go into three structures that hold the same Text objects:

- `ttext`: an `OQL.Table` (OQL's columnar table) with one `#text` column `k` and a heap `k #hash` index, the container of the baseline's heap-index runs (`heapidx-*.md`); the rows go into Region segments as the Table's byte-based auto-flush cuts them, then `flush`;
- `mtext`: an `IndexedMap<Nat, R>` (OQL's heap table) with a heap `k #hash` index, the container of OQL's own `agg` suite's count (`count where status = 'a'`, `mops bench agg`);
- `mp`: a mo:core `Map<Text, Nat>` (key to rows holding it), for reference.

Built as `src/oqlbars.mo` is (README.md, "Setting up the OQL copies"): moc `--default-persistent-actors -fshared-code --max-stable-pages 1638400` (no `--force-gc`), with the copy's own moc, core, jayson and OQL sources (`src/build-textidx.sh`). Installed with no Wasm memory limit, 10^18 cycles.

Method: after the build, `tick` messages until the collector spent under 100,000 instructions in each of three messages in a row (`settle` in `src/lib.mjs`), so every query ran with the collection idle. Then, for each counted key, real query calls, one per measured operation, each reading the performance counter around one call:

| column | query | counter around |
|---|---|---|
| Table, `measure` (JSON) | `measure(qJson)` on `ttext`, `{"start":"ttext","where":{"eq":{"field":"k","value":"<key>"}},"aggregate":[{"fn":"count"}]}`: the baseline's `measure` (JSON parsed outside the counter) | `Executor.runWith` |
| Table, flat key | `measureFlat("ttext", key)`: the same query built in Motoko around the key as Candid decodes it | `Executor.runWith` |
| IndexedMap, JSON / flat key | the same two on `mtext` | `Executor.runWith` |
| Map.get, flat key | `mapGet(key)` | `Map.get(Text.compare, key)` |
| Map.get, JSON key | `mapGetJson(qJson)`: the key taken from the parsed query | `Map.get(Text.compare, key)` |

Counted keys per build: the three planted keys and one absent odd value (75,701); then two sweeps of 300 values: the even values (s x 7,919) mod 100,000 with the low bit cleared, s = 0..299 (held by the rows that drew them, or absent), and the odd values ((1,000 + s) x 7,919 mod 100,000) with the low bit set (absent). The even sweep holds the values the crate's count rows count at key positions 5, 17 and 123 (39,594, 34,622, 74,036); the crate's rows of keys held in none count the odd sweep's. Every query's answer was checked: the six paths agree on every key, the planted keys count 1, 6 and 1,000 and the absent ones 0 (mismatches: 0 in all 10 runs). Instruction counts are deterministic: a second run of the paths build under moc 1.14.0 gave the same 604 records.

A control, `src/oqlnat.mo`, is the same canister with the column `k` a `#nat` holding the value itself, and `Map<Nat, Nat>`, at the same rows: it ties this canister to the baseline's 8-byte figures.

## Bars

Each figure is the lower of moc 1.14.0 and 1.16.1; moc 1.16.1 was the lower in every one of the 18,120 figure pairs (by 1,298-1,604 on the count paths, by 22-48 on `Map.get`). The bar is the lowest of the four count paths: the IndexedMap with a flat key in every row. The Table's count is 3,046-3,214 above the IndexedMap's in every text-key query, flat or JSON (moc 1.16.1: 3,046-3,062). "Most" is the highest of the three keys; "median" and "worst" are the median and the highest of a sweep's keys.

| keys | key | Table, `measure` (JSON) | Table, flat key | IndexedMap, `measure` (JSON) | IndexedMap, flat key | **bar** (lowest) | Map.get, flat key | Map.get, JSON key |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| 17 characters (18-byte keys) | held in 1 row (17 chars) | 85,219 | 55,734 | 82,161 | 52,676 | **52,676** | 17,740 | 46,910 |
| 17 characters (18-byte keys) | held in 6 rows (17 chars) | 78,845 | 52,998 | 75,791 | 49,944 | **49,944** | 15,884 | 41,416 |
| 17 characters (18-byte keys) | held in 1,000 rows (17 chars) | 62,641 | 45,942 | 59,583 | 42,884 | **42,884** | 10,990 | 27,374 |
| 17 characters (18-byte keys) | absent (17 chars) | 82,221 | 54,502 | 79,163 | 51,444 | **51,444** | 17,018 | 44,422 |
| 17 characters (18-byte keys) | values 39,594, 34,622, 74,036 (sweep keys 5, 17, 123; 2 of 3 held): most | 82,086 | 54,367 | 79,032 | 51,313 | **51,313** | 16,819 | 44,223 |
| 17 characters (18-byte keys) | sweep, held, n = 254: median | 81,443 | 54,279 | 78,389 | 51,221 | **51,221** | 16,722 | 43,576 |
| 17 characters (18-byte keys) | sweep, held, n = 254: worst | 87,809 | 56,995 | 84,759 | 53,945 | **53,945** | 18,571 | 49,070 |
| 17 characters (18-byte keys) | sweep, absent even, n = 46: median | 82,143 | 54,436 | 79,085 | 51,378 | **51,378** | 16,944 | 44,340 |
| 17 characters (18-byte keys) | sweep, absent even, n = 46: worst | 87,667 | 56,796 | 84,613 | 53,742 | **53,742** | 18,436 | 49,000 |
| 17 characters (18-byte keys) | sweep, odd (absent), n = 300: median | 82,191 | 54,459 | 79,133 | 51,401 | **51,401** | 16,963 | 44,388 |
| 17 characters (18-byte keys) | sweep, odd (absent), n = 300: worst | 90,333 | 58,058 | 87,279 | 55,004 | **55,004** | 19,268 | 51,228 |
| paths, 59-199 characters (60-200-byte keys) | held in 1 row (120 chars) | 350,137 | 76,086 | 347,079 | 73,028 | **73,028** | 38,092 | 312,143 |
| paths, 59-199 characters (60-200-byte keys) | held in 6 rows (111 chars) | 299,333 | 70,914 | 296,275 | 67,856 | **67,856** | 33,804 | 262,223 |
| paths, 59-199 characters (60-200-byte keys) | held in 1,000 rows (134 chars) | 239,487 | 59,042 | 236,429 | 55,984 | **55,984** | 24,086 | 204,531 |
| paths, 59-199 characters (60-200-byte keys) | absent (83 chars) | 271,626 | 71,970 | 268,572 | 68,916 | **68,916** | 34,482 | 234,138 |
| paths, 59-199 characters (60-200-byte keys) | values 39,594, 34,622, 74,036 (sweep keys 5, 17, 123; 2 of 3 held): most | 359,476 | 74,019 | 356,418 | 70,961 | **70,961** | 36,475 | 322,422 |
| paths, 59-199 characters (60-200-byte keys) | sweep, held, n = 254: median | 324,589 | 72,758 | 321,531 | 69,700 | **69,700** | 35,290 | 287,363 |
| paths, 59-199 characters (60-200-byte keys) | sweep, held, n = 254: worst | 435,080 | 79,943 | 432,022 | 76,885 | **76,885** | 41,515 | 397,082 |
| paths, 59-199 characters (60-200-byte keys) | sweep, absent even, n = 46: median | 330,421 | 71,908 | 327,363 | 68,850 | **68,850** | 34,408 | 292,933 |
| paths, 59-199 characters (60-200-byte keys) | sweep, absent even, n = 46: worst | 409,990 | 76,448 | 406,932 | 73,390 | **73,390** | 38,084 | 372,068 |
| paths, 59-199 characters (60-200-byte keys) | sweep, odd (absent), n = 300: median | 328,506 | 71,923 | 325,448 | 68,865 | **68,865** | 34,431 | 291,018 |
| paths, 59-199 characters (60-200-byte keys) | sweep, odd (absent), n = 300: worst | 450,893 | 78,810 | 447,835 | 75,752 | **75,752** | 40,012 | 412,533 |
| 1,999 characters (2,000-byte keys), 10^4 rows | held in 1 row (1999 chars) | 5,781,909 | 652,235 | 5,778,851 | 649,177 | **649,177** | 615,543 | 5,745,217 |
| 1,999 characters (2,000-byte keys), 10^4 rows | held in 6 rows (1999 chars) | 2,938,558 | 348,546 | 2,935,500 | 345,488 | **345,488** | 314,896 | 2,904,908 |
| 1,999 characters (2,000-byte keys), 10^4 rows | held in 1,000 rows (1999 chars) | 5,375,663 | 608,919 | 5,372,605 | 605,861 | **605,861** | 572,669 | 5,339,413 |
| 1,999 characters (2,000-byte keys), 10^4 rows | absent (1999 chars) | 5,310,943 | 595,291 | 5,307,885 | 592,233 | **592,233** | 559,105 | 5,274,757 |
| 1,999 characters (2,000-byte keys), 10^4 rows | values 39,594, 34,622, 74,036 (sweep keys 5, 17, 123; 0 of 3 held): most | 5,716,962 | 638,486 | 5,713,904 | 635,428 | **635,428** | 601,862 | 5,680,338 |
| 1,999 characters (2,000-byte keys), 10^4 rows | sweep, held, n = 45: median | 5,375,547 | 608,909 | 5,372,489 | 605,851 | **605,851** | 572,651 | 5,339,289 |
| 1,999 characters (2,000-byte keys), 10^4 rows | sweep, held, n = 45: worst | 6,187,659 | 695,479 | 6,184,601 | 692,421 | **692,421** | 658,353 | 6,150,533 |
| 1,999 characters (2,000-byte keys), 10^4 rows | sweep, absent even, n = 255: median | 5,716,513 | 638,444 | 5,713,462 | 635,386 | **635,386** | 601,821 | 5,679,896 |
| 1,999 characters (2,000-byte keys), 10^4 rows | sweep, absent even, n = 255: worst | 6,122,541 | 681,771 | 6,119,483 | 678,713 | **678,713** | 644,717 | 6,085,487 |
| 1,999 characters (2,000-byte keys), 10^4 rows | sweep, odd (absent), n = 300: median | 5,716,548 | 638,429 | 5,713,490 | 635,372 | **635,372** | 601,806 | 5,679,927 |
| 1,999 characters (2,000-byte keys), 10^4 rows | sweep, odd (absent), n = 300: worst | 6,122,663 | 681,785 | 6,119,605 | 678,727 | **678,727** | 644,727 | 6,085,605 |
| 1,999 characters (2,000-byte keys), 10^5 rows | held in 1 row (1999 chars) | 7,001,136 | 782,782 | 6,998,082 | 779,728 | **779,728** | 744,796 | 6,963,150 |
| 1,999 characters (2,000-byte keys), 10^5 rows | held in 6 rows (1999 chars) | 6,188,706 | 696,106 | 6,185,648 | 693,048 | **693,048** | 658,992 | 6,151,592 |
| 1,999 characters (2,000-byte keys), 10^5 rows | held in 1,000 rows (1999 chars) | 4,157,348 | 479,186 | 4,154,290 | 476,128 | **476,128** | 444,234 | 4,122,396 |
| 1,999 characters (2,000-byte keys), 10^5 rows | absent (1999 chars) | 6,530,398 | 726,066 | 6,527,340 | 723,008 | **723,008** | 688,570 | 6,492,902 |
| 1,999 characters (2,000-byte keys), 10^5 rows | values 39,594, 34,622, 74,036 (sweep keys 5, 17, 123; 2 of 3 held): most | 6,594,977 | 739,447 | 6,591,919 | 736,389 | **736,389** | 701,899 | 6,557,429 |
| 1,999 characters (2,000-byte keys), 10^5 rows | sweep, held, n = 254: median | 6,594,338 | 739,359 | 6,591,280 | 736,301 | **736,301** | 701,803 | 6,556,782 |
| 1,999 characters (2,000-byte keys), 10^5 rows | sweep, held, n = 254: worst | 7,406,760 | 826,015 | 7,403,702 | 822,957 | **822,957** | 787,591 | 7,368,340 |
| 1,999 characters (2,000-byte keys), 10^5 rows | sweep, absent even, n = 46: median | 6,530,312 | 725,992 | 6,527,254 | 722,934 | **722,934** | 688,504 | 6,492,824 |
| 1,999 characters (2,000-byte keys), 10^5 rows | sweep, absent even, n = 46: worst | 7,341,900 | 812,296 | 7,338,842 | 809,238 | **809,238** | 773,932 | 7,303,544 |
| 1,999 characters (2,000-byte keys), 10^5 rows | sweep, odd (absent), n = 300: median | 6,530,360 | 726,007 | 6,527,302 | 722,953 | **722,953** | 688,519 | 6,492,872 |
| 1,999 characters (2,000-byte keys), 10^5 rows | sweep, odd (absent), n = 300: worst | 7,747,592 | 855,530 | 7,744,534 | 852,472 | **852,472** | 816,740 | 7,708,802 |
| control: `#nat` keys (the value itself) | held in 1 row | 43,692 | 43,696 | 40,327 | 40,331 | **40,327** | 8,608 | 8,604 |
| control: `#nat` keys (the value itself) | held in 6 rows | 42,432 | 42,436 | 39,067 | 39,071 | **39,067** | 7,854 | 7,850 |
| control: `#nat` keys (the value itself) | held in 1,000 rows | 39,140 | 39,144 | 35,775 | 35,779 | **35,775** | 5,771 | 5,767 |
| control: `#nat` keys (the value itself) | absent | 43,205 | 43,209 | 39,840 | 39,844 | **39,840** | 8,438 | 8,434 |
| control: `#nat` keys (the value itself) | values 39,594, 34,622, 74,036 (sweep keys 5, 17, 123; 2 of 3 held): most | 43,025 | 43,029 | 39,660 | 39,664 | **39,660** | 8,202 | 8,198 |
| control: `#nat` keys (the value itself) | sweep, held, n = 254: median | 43,034 | 43,038 | 39,669 | 39,673 | **39,669** | 8,202 | 8,198 |
| control: `#nat` keys (the value itself) | sweep, held, n = 254: worst | 44,369 | 44,373 | 41,004 | 41,008 | **41,004** | 9,040 | 9,036 |
| control: `#nat` keys (the value itself) | sweep, absent even, n = 46: median | 43,280 | 43,284 | 39,915 | 39,919 | **39,915** | 8,405 | 8,401 |
| control: `#nat` keys (the value itself) | sweep, absent even, n = 46: worst | 44,207 | 44,211 | 40,842 | 40,846 | **40,842** | 8,950 | 8,946 |
| control: `#nat` keys (the value itself) | sweep, odd (absent), n = 300: median | 43,222 | 43,226 | 39,860 | 39,864 | **39,860** | 8,377 | 8,373 |
| control: `#nat` keys (the value itself) | sweep, odd (absent), n = 300: worst | 44,717 | 44,721 | 41,352 | 41,356 | **41,352** | 9,211 | 9,207 |

What the figures show:

- The rows holding a key do not set the count's cost: the posting's size is read, not walked. The 6-row and 1,000-row keys are cheaper than the 1-row key at some sizes and not at others, by where they sit in the tree.
- The key's length sets it. The executor's work over the `Map.get` is 30,592-37,148 in every query (IndexedMap, flat key, both compilers; 29,269-33,553 with `#nat` keys). `Map.get` with a flat key takes 8,068-19,268 for 17 characters, 18,340-41,515 for paths and 315,400-816,740 for 1,999 characters at 10^5 rows (314,896-658,353 at 10^4; every counted key, lower compiler): each comparison of the descent runs the Motoko runtime's `text_compare`, a byte comparison (`memcmp`) of the two texts up to the first difference, here past the 1,500 shared characters.
- A key from JSON costs more: jayson's `parseString` builds the string one character at a time (`result #= char.toText()`), so the key `measure` compares with is a chain of concatenation nodes, and `text_compare` walks it node by node in every comparison. `measure` minus the flat-key count, same container: 11,719-32,275 at 17 characters (x1.27-1.56), 134,514-374,492 for paths (x3.0-5.9), 2,590,008-6,892,062 at 1,999 characters (x8.4-9.1); the same with both compilers, as it is runtime code.
- Control, `#nat` keys at the same 43,038 distinct values (every counted key, lower compiler): Table `measure` 37,080-44,717, IndexedMap with a flat key 33,715-41,356. The baseline's heap-index count at 10^5 rows (`heapidx-100000-moc1161.md`: the ledger's `account_from`, `measure` on a Table of five columns) is 46,792-48,665, and OQL's own `agg` suite's count of `status = 'a'` (an IndexedMap, a text key of 3 values, `mops bench agg`) is 39,631: this canister's Table count is below the first, its IndexedMap count 14.9% below to 4.4% above the second, whose Map holds 3 keys. The canister does not raise the figures.

## Per compiler

Every figure as measured. Sweep cells: min / p50 / mean / max over the sweep's keys. "value": the count every path answered.

### moc 1.14.0

| keys | key | value | Table, JSON | Table, flat | IndexedMap, JSON | IndexedMap, flat | Map.get, flat | Map.get, JSON |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| 17 characters (18-byte keys) | held in 1 row (17 chars) | 1 | 86,801 | 57,316 | 83,591 | 54,106 | 17,784 | 46,954 |
| 17 characters (18-byte keys) | held in 6 rows (17 chars) | 6 | 80,405 | 54,558 | 77,199 | 51,352 | 15,928 | 41,460 |
| 17 characters (18-byte keys) | held in 1,000 rows (17 chars) | 1,000 | 64,135 | 47,436 | 60,925 | 44,226 | 11,023 | 27,407 |
| 17 characters (18-byte keys) | absent (17 chars) | 0 | 83,792 | 56,073 | 80,582 | 52,863 | 17,062 | 44,466 |
| 17 characters (18-byte keys) | values 39,594, 34,622, 74,036 (sweep keys 5, 17, 123; 2 of 3 held): most | | 83,657 | 55,938 | 80,451 | 52,732 | 16,863 | 44,267 |
| 17 characters (18-byte keys) | sweep, held, n = 254: min / p50 / mean / max | | 54,898 / 83,014 / 81,249 / 89,402 | 43,179 / 55,850 / 55,037 / 58,588 | 51,688 / 79,808 / 78,041 / 86,200 | 39,969 / 52,640 / 51,828 / 55,386 | 8,101 / 16,766 / 16,245 / 18,615 | 19,505 / 43,620 / 42,142 / 49,114 |
| 17 characters (18-byte keys) | sweep, absent even, n = 46: min / p50 / mean / max | | 77,174 / 83,714 / 84,547 / 89,260 | 53,199 / 56,007 / 56,360 / 58,389 | 73,964 / 80,504 / 81,338 / 86,054 | 49,989 / 52,797 / 53,151 / 55,183 | 15,070 / 16,988 / 17,151 / 18,480 | 38,730 / 44,384 / 45,023 / 49,044 |
| 17 characters (18-byte keys) | sweep, odd (absent), n = 300: min / p50 / mean / max | | 77,206 / 83,762 / 84,640 / 91,937 | 53,019 / 56,030 / 56,391 / 59,662 | 73,996 / 80,552 / 81,432 / 88,731 | 49,809 / 52,820 / 53,182 / 56,456 | 14,906 / 17,007 / 17,170 / 19,312 | 38,778 / 44,432 / 45,105 / 51,272 |
| paths, 59-199 characters (60-200-byte keys) | held in 1 row (120 chars) | 1 | 351,719 | 77,668 | 348,509 | 74,458 | 38,136 | 312,187 |
| paths, 59-199 characters (60-200-byte keys) | held in 6 rows (111 chars) | 6 | 300,893 | 72,474 | 297,683 | 69,264 | 33,848 | 262,267 |
| paths, 59-199 characters (60-200-byte keys) | held in 1,000 rows (134 chars) | 1,000 | 240,981 | 60,536 | 237,771 | 57,326 | 24,119 | 204,564 |
| paths, 59-199 characters (60-200-byte keys) | absent (83 chars) | 0 | 273,197 | 73,541 | 269,991 | 70,335 | 34,526 | 234,182 |
| paths, 59-199 characters (60-200-byte keys) | values 39,594, 34,622, 74,036 (sweep keys 5, 17, 123; 2 of 3 held): most | | 361,036 | 75,590 | 357,826 | 72,380 | 36,519 | 322,466 |
| paths, 59-199 characters (60-200-byte keys) | sweep, held, n = 254: min / p50 / mean / max | | 188,472 / 326,182 / 324,300 / 436,662 | 53,451 / 74,329 / 73,832 / 81,536 | 185,262 / 322,972 / 321,090 / 433,452 | 50,241 / 71,119 / 70,622 / 78,326 | 18,373 / 35,334 / 35,038 / 41,559 | 153,394 / 287,407 / 285,506 / 397,126 |
| paths, 59-199 characters (60-200-byte keys) | sweep, absent even, n = 46: min / p50 / mean / max | | 233,483 / 331,992 / 327,358 / 411,572 | 68,483 / 73,479 / 74,302 / 78,041 | 230,273 / 328,782 / 324,149 / 408,362 | 65,273 / 70,269 / 71,093 / 74,831 | 30,354 / 34,452 / 35,091 / 38,128 | 194,913 / 292,977 / 288,147 / 372,112 |
| paths, 59-199 characters (60-200-byte keys) | sweep, odd (absent), n = 300: min / p50 / mean / max | | 227,332 / 330,077 / 329,872 / 452,486 | 68,303 / 73,494 / 74,361 / 80,414 | 224,122 / 326,867 / 326,662 / 449,276 | 65,097 / 70,284 / 71,152 / 77,204 | 30,190 / 34,475 / 35,139 / 40,056 | 188,758 / 291,062 / 290,649 / 412,577 |
| 1,999 characters (2,000-byte keys), 10^4 rows | held in 1 row (1999 chars) | 1 | 5,783,447 | 653,773 | 5,780,237 | 650,563 | 615,576 | 5,745,250 |
| 1,999 characters (2,000-byte keys), 10^4 rows | held in 6 rows (1999 chars) | 6 | 2,940,008 | 349,996 | 2,936,798 | 346,786 | 314,918 | 2,904,930 |
| 1,999 characters (2,000-byte keys), 10^4 rows | held in 1,000 rows (1999 chars) | 1,000 | 5,377,190 | 610,446 | 5,373,980 | 607,236 | 572,702 | 5,339,446 |
| 1,999 characters (2,000-byte keys), 10^4 rows | absent (1999 chars) | 0 | 5,312,470 | 596,818 | 5,309,260 | 593,608 | 559,138 | 5,274,790 |
| 1,999 characters (2,000-byte keys), 10^4 rows | values 39,594, 34,622, 74,036 (sweep keys 5, 17, 123; 0 of 3 held): most | | 5,718,500 | 640,024 | 5,715,290 | 636,814 | 601,895 | 5,680,371 |
| 1,999 characters (2,000-byte keys), 10^4 rows | sweep, held, n = 45: min / p50 / mean / max | | 3,751,795 / 5,377,074 / 5,196,537 / 6,189,208 | 436,532 / 610,436 / 591,152 / 697,028 | 3,748,585 / 5,373,864 / 5,193,328 / 6,185,998 | 433,322 / 607,226 / 587,943 / 693,818 | 400,560 / 572,684 / 553,599 / 658,386 | 3,715,823 / 5,339,322 / 5,158,985 / 6,150,566 |
| 1,999 characters (2,000-byte keys), 10^4 rows | sweep, absent even, n = 255: min / p50 / mean / max | | 4,905,906 / 5,718,051 / 5,586,061 / 6,124,090 | 553,482 / 639,982 / 625,945 / 683,320 | 4,902,696 / 5,714,848 / 5,582,851 / 6,120,880 | 550,272 / 636,772 / 622,735 / 680,110 | 516,243 / 601,854 / 587,962 / 644,750 | 4,868,667 / 5,679,929 / 5,548,077 / 6,085,520 |
| 1,999 characters (2,000-byte keys), 10^4 rows | sweep, odd (absent), n = 300: min / p50 / mean / max | | 4,905,698 / 5,718,086 / 5,595,073 / 6,124,212 | 553,472 / 639,967 / 626,901 / 683,334 | 4,902,488 / 5,714,876 / 5,591,863 / 6,121,002 | 550,262 / 636,758 / 623,691 / 680,124 | 516,233 / 601,839 / 588,908 / 644,760 | 4,868,459 / 5,679,960 / 5,557,080 / 6,085,638 |
| 1,999 characters (2,000-byte keys), 10^5 rows | held in 1 row (1999 chars) | 1 | 7,002,718 | 784,364 | 6,999,512 | 781,158 | 744,840 | 6,963,194 |
| 1,999 characters (2,000-byte keys), 10^5 rows | held in 6 rows (1999 chars) | 6 | 6,190,266 | 697,666 | 6,187,056 | 694,456 | 659,036 | 6,151,636 |
| 1,999 characters (2,000-byte keys), 10^5 rows | held in 1,000 rows (1999 chars) | 1,000 | 4,158,842 | 480,680 | 4,155,632 | 477,470 | 444,267 | 4,122,429 |
| 1,999 characters (2,000-byte keys), 10^5 rows | absent (1999 chars) | 0 | 6,531,969 | 727,637 | 6,528,759 | 724,427 | 688,614 | 6,492,946 |
| 1,999 characters (2,000-byte keys), 10^5 rows | values 39,594, 34,622, 74,036 (sweep keys 5, 17, 123; 2 of 3 held): most | | 6,596,548 | 741,018 | 6,593,338 | 737,808 | 701,943 | 6,557,473 |
| 1,999 characters (2,000-byte keys), 10^5 rows | sweep, held, n = 254: min / p50 / mean / max | | 2,940,519 / 6,595,909 / 6,329,155 / 7,408,353 | 350,511 / 740,930 / 712,520 / 827,608 | 2,937,309 / 6,592,699 / 6,325,945 / 7,405,143 | 347,301 / 737,720 / 709,310 / 824,398 | 315,433 / 701,847 / 673,729 / 787,635 | 2,905,441 / 6,556,826 / 6,290,365 / 7,368,384 |
| 1,999 characters (2,000-byte keys), 10^5 rows | sweep, absent even, n = 46: min / p50 / mean / max | | 5,719,279 / 6,531,883 / 6,707,946 / 7,343,493 | 640,807 / 727,563 / 746,164 / 813,889 | 5,716,069 / 6,528,673 / 6,704,737 / 7,340,283 | 637,597 / 724,353 / 742,955 / 810,679 | 602,686 / 688,548 / 706,955 / 773,976 | 5,681,158 / 6,492,868 / 6,668,737 / 7,303,588 |
| 1,999 characters (2,000-byte keys), 10^5 rows | sweep, odd (absent), n = 300: min / p50 / mean / max | | 5,719,323 / 6,531,935 / 6,718,202 / 7,749,196 | 640,639 / 727,578 / 747,253 / 857,134 | 5,716,113 / 6,528,725 / 6,714,992 / 7,745,986 | 637,429 / 724,372 / 744,043 / 853,924 | 602,518 / 688,563 / 708,034 / 816,784 | 5,681,202 / 6,492,916 / 6,678,983 / 7,708,846 |
| control: `#nat` keys (the value itself) | held in 1 row | 1 | 45,274 | 45,278 | 41,757 | 41,761 | 8,652 | 8,648 |
| control: `#nat` keys (the value itself) | held in 6 rows | 6 | 43,992 | 43,996 | 40,475 | 40,479 | 7,898 | 7,894 |
| control: `#nat` keys (the value itself) | held in 1,000 rows | 1,000 | 40,634 | 40,638 | 37,117 | 37,121 | 5,804 | 5,800 |
| control: `#nat` keys (the value itself) | absent | 0 | 44,776 | 44,780 | 41,259 | 41,263 | 8,482 | 8,478 |
| control: `#nat` keys (the value itself) | values 39,594, 34,622, 74,036 (sweep keys 5, 17, 123; 2 of 3 held): most | | 44,596 | 44,600 | 41,079 | 41,083 | 8,246 | 8,242 |
| control: `#nat` keys (the value itself) | sweep, held, n = 254: min / p50 / mean / max | | 38,541 / 44,605 / 44,285 / 45,962 | 38,545 / 44,609 / 44,289 / 45,966 | 35,020 / 41,088 / 40,768 / 42,445 | 35,024 / 41,092 / 40,772 / 42,449 | 4,479 / 8,246 / 8,088 / 9,084 | 4,475 / 8,242 / 8,084 / 9,080 |
| control: `#nat` keys (the value itself) | sweep, absent even, n = 46: min / p50 / mean / max | | 43,454 / 44,851 / 44,800 / 45,800 | 43,458 / 44,855 / 44,804 / 45,804 | 39,937 / 41,334 / 41,283 / 42,283 | 39,941 / 41,338 / 41,287 / 42,287 | 7,668 / 8,449 / 8,392 / 8,994 | 7,664 / 8,445 / 8,388 / 8,990 |
| control: `#nat` keys (the value itself) | sweep, odd (absent), n = 300: min / p50 / mean / max | | 43,138 / 44,793 / 44,807 / 46,321 | 43,142 / 44,797 / 44,811 / 46,325 | 39,621 / 41,276 / 41,290 / 42,804 | 39,625 / 41,280 / 41,294 / 42,808 | 7,368 / 8,421 / 8,393 / 9,255 | 7,364 / 8,417 / 8,389 / 9,251 |

### moc 1.16.1

| keys | key | value | Table, JSON | Table, flat | IndexedMap, JSON | IndexedMap, flat | Map.get, flat | Map.get, JSON |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| 17 characters (18-byte keys) | held in 1 row (17 chars) | 1 | 85,219 | 55,734 | 82,161 | 52,676 | 17,740 | 46,910 |
| 17 characters (18-byte keys) | held in 6 rows (17 chars) | 6 | 78,845 | 52,998 | 75,791 | 49,944 | 15,884 | 41,416 |
| 17 characters (18-byte keys) | held in 1,000 rows (17 chars) | 1,000 | 62,641 | 45,942 | 59,583 | 42,884 | 10,990 | 27,374 |
| 17 characters (18-byte keys) | absent (17 chars) | 0 | 82,221 | 54,502 | 79,163 | 51,444 | 17,018 | 44,422 |
| 17 characters (18-byte keys) | values 39,594, 34,622, 74,036 (sweep keys 5, 17, 123; 2 of 3 held): most | | 82,086 | 54,367 | 79,032 | 51,313 | 16,819 | 44,223 |
| 17 characters (18-byte keys) | sweep, held, n = 254: min / p50 / mean / max | | 53,437 / 81,443 / 79,686 / 87,809 | 41,718 / 54,279 / 53,473 / 56,995 | 50,379 / 78,389 / 76,629 / 84,759 | 38,660 / 51,221 / 50,417 / 53,945 | 8,068 / 16,722 / 16,201 / 18,571 | 19,472 / 43,576 / 42,099 / 49,070 |
| 17 characters (18-byte keys) | sweep, absent even, n = 46: min / p50 / mean / max | | 75,625 / 82,143 / 82,971 / 87,667 | 51,650 / 54,436 / 54,784 / 56,796 | 72,567 / 79,085 / 79,914 / 84,613 | 48,592 / 51,378 / 51,727 / 53,742 | 15,026 / 16,944 / 17,107 / 18,436 | 38,686 / 44,340 / 44,979 / 49,000 |
| 17 characters (18-byte keys) | sweep, odd (absent), n = 300: min / p50 / mean / max | | 75,657 / 82,191 / 83,064 / 90,333 | 51,470 / 54,459 / 54,814 / 58,058 | 72,599 / 79,133 / 80,008 / 87,279 | 48,412 / 51,401 / 51,758 / 55,004 | 14,862 / 16,963 / 17,126 / 19,268 | 38,734 / 44,388 / 45,061 / 51,228 |
| paths, 59-199 characters (60-200-byte keys) | held in 1 row (120 chars) | 1 | 350,137 | 76,086 | 347,079 | 73,028 | 38,092 | 312,143 |
| paths, 59-199 characters (60-200-byte keys) | held in 6 rows (111 chars) | 6 | 299,333 | 70,914 | 296,275 | 67,856 | 33,804 | 262,223 |
| paths, 59-199 characters (60-200-byte keys) | held in 1,000 rows (134 chars) | 1,000 | 239,487 | 59,042 | 236,429 | 55,984 | 24,086 | 204,531 |
| paths, 59-199 characters (60-200-byte keys) | absent (83 chars) | 0 | 271,626 | 71,970 | 268,572 | 68,916 | 34,482 | 234,138 |
| paths, 59-199 characters (60-200-byte keys) | values 39,594, 34,622, 74,036 (sweep keys 5, 17, 123; 2 of 3 held): most | | 359,476 | 74,019 | 356,418 | 70,961 | 36,475 | 322,422 |
| paths, 59-199 characters (60-200-byte keys) | sweep, held, n = 254: min / p50 / mean / max | | 187,011 / 324,589 / 322,736 / 435,080 | 51,990 / 72,758 / 72,268 / 79,943 | 183,953 / 321,531 / 319,678 / 432,022 | 48,932 / 69,700 / 69,211 / 76,885 | 18,340 / 35,290 / 34,995 / 41,515 | 153,361 / 287,363 / 285,462 / 397,082 |
| paths, 59-199 characters (60-200-byte keys) | sweep, absent even, n = 46: min / p50 / mean / max | | 231,923 / 330,421 / 325,783 / 409,990 | 66,934 / 71,908 / 72,727 / 76,448 | 228,865 / 327,363 / 322,725 / 406,932 | 63,876 / 68,850 / 69,669 / 73,390 | 30,310 / 34,408 / 35,047 / 38,084 | 194,869 / 292,933 / 288,103 / 372,068 |
| paths, 59-199 characters (60-200-byte keys) | sweep, odd (absent), n = 300: min / p50 / mean / max | | 225,772 / 328,506 / 328,296 / 450,893 | 66,754 / 71,923 / 72,785 / 78,810 | 222,714 / 325,448 / 325,238 / 447,835 | 63,700 / 68,865 / 69,728 / 75,752 | 30,146 / 34,431 / 35,095 / 40,012 | 188,714 / 291,018 / 290,605 / 412,533 |
| 1,999 characters (2,000-byte keys), 10^4 rows | held in 1 row (1999 chars) | 1 | 5,781,909 | 652,235 | 5,778,851 | 649,177 | 615,543 | 5,745,217 |
| 1,999 characters (2,000-byte keys), 10^4 rows | held in 6 rows (1999 chars) | 6 | 2,938,558 | 348,546 | 2,935,500 | 345,488 | 314,896 | 2,904,908 |
| 1,999 characters (2,000-byte keys), 10^4 rows | held in 1,000 rows (1999 chars) | 1,000 | 5,375,663 | 608,919 | 5,372,605 | 605,861 | 572,669 | 5,339,413 |
| 1,999 characters (2,000-byte keys), 10^4 rows | absent (1999 chars) | 0 | 5,310,943 | 595,291 | 5,307,885 | 592,233 | 559,105 | 5,274,757 |
| 1,999 characters (2,000-byte keys), 10^4 rows | values 39,594, 34,622, 74,036 (sweep keys 5, 17, 123; 0 of 3 held): most | | 5,716,962 | 638,486 | 5,713,904 | 635,428 | 601,862 | 5,680,338 |
| 1,999 characters (2,000-byte keys), 10^4 rows | sweep, held, n = 45: min / p50 / mean / max | | 3,750,323 / 5,375,547 / 5,195,016 / 6,187,659 | 435,060 / 608,909 / 589,631 / 695,479 | 3,747,265 / 5,372,489 / 5,191,958 / 6,184,601 | 432,002 / 605,851 / 586,573 / 692,421 | 400,538 / 572,651 / 553,567 / 658,353 | 3,715,801 / 5,339,289 / 5,158,952 / 6,150,533 |
| 1,999 characters (2,000-byte keys), 10^4 rows | sweep, absent even, n = 255: min / p50 / mean / max | | 4,904,390 / 5,716,513 / 5,584,526 / 6,122,541 | 551,966 / 638,444 / 624,411 / 681,771 | 4,901,332 / 5,713,462 / 5,581,468 / 6,119,483 | 548,908 / 635,386 / 621,353 / 678,713 | 516,210 / 601,821 / 587,929 / 644,717 | 4,868,634 / 5,679,896 / 5,548,044 / 6,085,487 |
| 1,999 characters (2,000-byte keys), 10^4 rows | sweep, odd (absent), n = 300: min / p50 / mean / max | | 4,904,182 / 5,716,548 / 5,593,539 / 6,122,663 | 551,956 / 638,429 / 625,366 / 681,785 | 4,901,124 / 5,713,490 / 5,590,481 / 6,119,605 | 548,898 / 635,372 / 622,309 / 678,727 | 516,200 / 601,806 / 588,875 / 644,727 | 4,868,426 / 5,679,927 / 5,557,047 / 6,085,605 |
| 1,999 characters (2,000-byte keys), 10^5 rows | held in 1 row (1999 chars) | 1 | 7,001,136 | 782,782 | 6,998,082 | 779,728 | 744,796 | 6,963,150 |
| 1,999 characters (2,000-byte keys), 10^5 rows | held in 6 rows (1999 chars) | 6 | 6,188,706 | 696,106 | 6,185,648 | 693,048 | 658,992 | 6,151,592 |
| 1,999 characters (2,000-byte keys), 10^5 rows | held in 1,000 rows (1999 chars) | 1,000 | 4,157,348 | 479,186 | 4,154,290 | 476,128 | 444,234 | 4,122,396 |
| 1,999 characters (2,000-byte keys), 10^5 rows | absent (1999 chars) | 0 | 6,530,398 | 726,066 | 6,527,340 | 723,008 | 688,570 | 6,492,902 |
| 1,999 characters (2,000-byte keys), 10^5 rows | values 39,594, 34,622, 74,036 (sweep keys 5, 17, 123; 2 of 3 held): most | | 6,594,977 | 739,447 | 6,591,919 | 736,389 | 701,899 | 6,557,429 |
| 1,999 characters (2,000-byte keys), 10^5 rows | sweep, held, n = 254: min / p50 / mean / max | | 2,939,058 / 6,594,338 / 6,327,592 / 7,406,760 | 349,050 / 739,359 / 710,956 / 826,015 | 2,936,000 / 6,591,280 / 6,324,534 / 7,403,702 | 345,992 / 736,301 / 707,898 / 822,957 | 315,400 / 701,803 / 673,686 / 787,591 | 2,905,408 / 6,556,782 / 6,290,321 / 7,368,340 |
| 1,999 characters (2,000-byte keys), 10^5 rows | sweep, absent even, n = 46: min / p50 / mean / max | | 5,717,730 / 6,530,312 / 6,706,370 / 7,341,900 | 639,258 / 725,992 / 744,588 / 812,296 | 5,714,672 / 6,527,254 / 6,703,313 / 7,338,842 | 636,200 / 722,934 / 741,531 / 809,238 | 602,642 / 688,504 / 706,911 / 773,932 | 5,681,114 / 6,492,824 / 6,668,693 / 7,303,544 |
| 1,999 characters (2,000-byte keys), 10^5 rows | sweep, odd (absent), n = 300: min / p50 / mean / max | | 5,717,774 / 6,530,360 / 6,716,626 / 7,747,592 | 639,090 / 726,007 / 745,677 / 855,530 | 5,714,716 / 6,527,302 / 6,713,568 / 7,744,534 | 636,032 / 722,953 / 742,619 / 852,472 | 602,474 / 688,519 / 707,990 / 816,740 | 5,681,158 / 6,492,872 / 6,678,939 / 7,708,802 |
| control: `#nat` keys (the value itself) | held in 1 row | 1 | 43,692 | 43,696 | 40,327 | 40,331 | 8,608 | 8,604 |
| control: `#nat` keys (the value itself) | held in 6 rows | 6 | 42,432 | 42,436 | 39,067 | 39,071 | 7,854 | 7,850 |
| control: `#nat` keys (the value itself) | held in 1,000 rows | 1,000 | 39,140 | 39,144 | 35,775 | 35,779 | 5,771 | 5,767 |
| control: `#nat` keys (the value itself) | absent | 0 | 43,205 | 43,209 | 39,840 | 39,844 | 8,438 | 8,434 |
| control: `#nat` keys (the value itself) | values 39,594, 34,622, 74,036 (sweep keys 5, 17, 123; 2 of 3 held): most | | 43,025 | 43,029 | 39,660 | 39,664 | 8,202 | 8,198 |
| control: `#nat` keys (the value itself) | sweep, held, n = 254: min / p50 / mean / max | | 37,080 / 43,034 / 42,722 / 44,369 | 37,084 / 43,038 / 42,726 / 44,373 | 33,711 / 39,669 / 39,357 / 41,004 | 33,715 / 39,673 / 39,361 / 41,008 | 4,446 / 8,202 / 8,045 / 9,040 | 4,442 / 8,198 / 8,041 / 9,036 |
| control: `#nat` keys (the value itself) | sweep, absent even, n = 46: min / p50 / mean / max | | 41,905 / 43,280 / 43,225 / 44,207 | 41,909 / 43,284 / 43,229 / 44,211 | 38,540 / 39,915 / 39,859 / 40,842 | 38,544 / 39,919 / 39,863 / 40,846 | 7,624 / 8,405 / 8,348 / 8,950 | 7,620 / 8,401 / 8,344 / 8,946 |
| control: `#nat` keys (the value itself) | sweep, odd (absent), n = 300: min / p50 / mean / max | | 41,589 / 43,222 / 43,231 / 44,717 | 41,593 / 43,226 / 43,235 / 44,721 | 38,224 / 39,860 / 39,866 / 41,352 | 38,228 / 39,864 / 39,870 / 41,356 | 7,324 / 8,377 / 8,349 / 9,211 | 7,320 / 8,373 / 8,345 / 9,207 |

## Builds

| keys | rows | compiler | distinct keys | Table segments | build messages | build, wall | settle: ticks, collector instructions | mismatches |
|---|---:|---|---:|---:|---:|---:|---|---:|
| 17 characters (18-byte keys) | 100,000 | moc 1.14.0 | 43,038 | 2 | 4 x 25,000 rows | 4 s | 3, 1,236 | 0 |
| 17 characters (18-byte keys) | 100,000 | moc 1.16.1 | 43,038 | 2 | 4 x 25,000 rows | 4 s | 3, 1,236 | 0 |
| paths, 59-199 characters (60-200-byte keys) | 100,000 | moc 1.14.0 | 43,038 | 4 | 4 x 25,000 rows | 6 s | 4, 105,250,209 | 0 |
| paths, 59-199 characters (60-200-byte keys) | 100,000 | moc 1.16.1 | 43,038 | 4 | 4 x 25,000 rows | 6 s | 4, 105,250,228 | 0 |
| 1,999 characters (2,000-byte keys), 10^4 rows | 10,000 | moc 1.14.0 | 8,236 | 5 | 4 x 2,500 rows | 3 s | 3, 990 | 0 |
| 1,999 characters (2,000-byte keys), 10^4 rows | 10,000 | moc 1.16.1 | 8,236 | 5 | 4 x 2,500 rows | 3 s | 3, 990 | 0 |
| 1,999 characters (2,000-byte keys), 10^5 rows | 100,000 | moc 1.14.0 | 43,038 | 50 | 40 x 2,500 rows | 40 s | 3, 1,236 | 0 |
| 1,999 characters (2,000-byte keys), 10^5 rows | 100,000 | moc 1.16.1 | 43,038 | 50 | 40 x 2,500 rows | 38 s | 3, 1,236 | 0 |
| control: `#nat` keys (the value itself) | 100,000 | moc 1.14.0 | 43,038 | 2 | 4 x 25,000 rows | 3 s | 3, 990 | 0 |
| control: `#nat` keys (the value itself) | 100,000 | moc 1.16.1 | 43,038 | 2 | 4 x 25,000 rows | 3 s | 3, 990 | 0 |

The settle column: `tick` messages sent after the build and the collector instructions they spent; each run ended with three messages under 100,000. The path builds' first tick finished a collection (105 million instructions); every query ran after it.

## Per-key bars

`text-key-count-bars.tsv` lists, for every counted key of every build, the key's figure on each count path
reduced to one bar: the lowest of the four count paths (Table and IndexedMap, JSON and flat key) under the
lower of the two compilers (moc 1.16.1 for every key), beside the IndexedMap's flat-key figure. Its columns:
`class` (`nat` the control, `text17` the 17-character keys, `paths`, `text1999-1e4` and `text1999-1e5` the
1,999-character keys at 10^4 and 10^5 rows); `set` (`even` the even sweep, `odd` the odd sweep, or the planted
key's set); `pos` (the sweep position s; -1 for a planted key); `value` (u); `count` (the rows holding the
key, every path's answer); `bar`; `path` and `moc` (the path and compiler that gave it); `indexedmap_flat`.
`src/text-key-count-bars.py` writes it from the records below. Counting a key at sweep position s, one
compares it with that key's figure here: OQL's figure for the same key over the same rows.

## Raw data and commands

`raw-text-key-counts.tar.xz` (`tar -xJf raw-text-key-counts.tar.xz` restores `raw/` beside this file) holds
the drivers' records of every run: per run `raw/<driver>-<c>/<tag>.log` (the driver's log lines),
`<tag>.queries.jsonl` (one record per counted key: `set`, `u` the value, `len` the key's characters, the six
paths' instructions under the column names `tJson`, `tFlat`, `mJson`, `mFlat`, `gFlat`, `gJson`, `value` the
count) and `<tag>.summary.json` (the build, the settle, the sweeps' statistics), with `<driver>` `textidx` or
`natidx`, `<c>` 1140 or 1161, and tag `moc<c>-s<shape>-<rows>` (shape 0: 17 characters, 1: paths, 2: 1,999
characters) or `moc<c>nat-s0-100000` for the control. Every file is as the driver wrote it, except the
`wasm` field of each `summary.json`, which named the canister by the absolute path of the build directory and
now names it by that directory (`out-1140` or `out-1161`) and the file; those files were written out again
with that one field changed.

The drivers run in `src/` (README.md, "Setting up the drivers"), with `<out>` any directory:

```sh
sh build-textidx.sh "$OQL" out-1140 && sh build-textidx.sh "$OQL_1161" out-1161    # oqltext.wasm
sh build-natidx.sh "$OQL" out-1140 && sh build-natidx.sh "$OQL_1161" out-1161      # oqlnat.wasm
for c in 1140 1161; do
  node textidx.mjs out-$c/oqltext.wasm <out>/textidx-$c moc$c 0 100000 25000
  node textidx.mjs out-$c/oqltext.wasm <out>/textidx-$c moc$c 1 100000 25000
  node textidx.mjs out-$c/oqltext.wasm <out>/textidx-$c moc$c 2 10000 2500
  node textidx.mjs out-$c/oqltext.wasm <out>/textidx-$c moc$c 2 100000 2500
  node natidx.mjs out-$c/oqlnat.wasm <out>/natidx-$c moc${c}nat 0 100000 25000
done
node keycheck.mjs out-1140/oqltext.wasm
```

As run, the two build scripts were named `build.sh` and `buildnat.sh` (the second made from the first with
`sed 's/oqltext/oqlnat/g'`), the drivers imported `startPic`, `settle` and `stats` from an earlier form of
`src/lib.mjs` in another folder (the same `settle` and `stats`; its `startPic` put PocketIC's temporary files
in another directory and did not check the `pocket-ic` link, which was in place), and a comment in each
canister, each driver and each build script named the canister they follow by another name; the files here
import `./lib.mjs` and name `src/oqlbars.mo`. Every other line is as run. The text-key runs of the two
compilers ran at the same time, each on its own PocketIC server.
