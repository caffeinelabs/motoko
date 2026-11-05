# Count and sum(amount) where paid and amount >= 500 over an IndexedMap, update context, collection counted

OQL: commit `bb5a23f` of `github.com/caffeinelabs/oql-prototype` ("fix: scope compiled predicates to one row stream (#107)", 2026-10-09), extracted with `git archive bb5a23f` into a scratch copy (README.md, "Setting up the OQL copies", gives the commands); dependencies core 2.6.0, jayson 0.1.1, bench 2.0.1 (`mops install`).
Replica: PocketIC 14.0.0 (`pocket-ic-server 14.0.0`, DFINITY's local test replica of the Internet Computer, the binary mops installs for OQL's `[toolchain]` pin), on an Apple M3 (macOS, 16 KiB host pages); dfx, DFINITY's SDK command-line tool, which runs a local replica of its own, was not used.
Compiler: `mops toolchain bin moc` of the copy: moc 1.14.0 (OQL's `[toolchain]` pin) and, in a second copy whose `mops.toml` pins moc 1.16.1, moc 1.16.1.


What: the paid-orders query: orders {id, customerId = i mod 10,000, amount = 7919 i mod 1,000, paid = i mod 3 != 0} in an `IndexedMap<Nat, Order>` with no secondary index, or with `paid #hash`; the query value (`where paid = true and amount >= 500`, aggregates count and sum(amount)) built once, run through `Executor.runWith` in an update message of the harness canister (`src/wh2.mo`, `ordQuery`). Method: a fresh canister per row count and index choice; the orders loaded in 50,000-row messages (`ordLoad`, not measured); `tick` messages (empty update messages) until the collector spent under 100,000 instructions in each of three messages in a row, the collection of that warm-up not counted. Then five `ordQuery` messages back to back, each its own update message sent through the relay canister (`src/relay.mo`). For each, the in-method count is `performanceCounter(0)` read right before and right after `Executor.runWith` (the message's Candid decoding and reply, and the registry built once in the first message, fall outside it), and its collection is that message's `rts_collector_instructions`, read by the query `last` right after it. After the fifth, `tick` until idle again by the same rule, the ticks' collector instructions summed as the tail. Mean in the method = the five in-method counts / 5; mean with collection = (the five in-method counts + the five messages' collection + the tail) / 5. Nothing is subtracted: each measured message's and each tail tick's collection is added whole, an idle message's own few hundred collector instructions included, and no page fee is counted. Per row = per table row (divide by 2/3 for a row of the `paid` posting).


## moc 1.14.0, 100,000 rows

| rows | index on paid | in-method per run | collection per run | tail collection (ticks) | mean in the method | mean with collection | per row in the method | per row with collection |
|---:|---|---|---|---|---:|---:|---:|---:|
| 100,000 | no | 928,435,217, 928,434,938, 928,434,958, 928,434,930, 928,434,705 | 329,096,057, 332,228,805, 329,413,757, 332,207,441, 329,308,643 | 1,236 (3) | 928,434,950 | 1,258,886,137 | 9,284 | 12,589 |
| 100,000 | yes | 1,105,448,073, 1,105,751,450, 1,105,446,717, 1,105,446,887, 1,105,446,753 | 339,167,059, 412, 347,134,905, 412, 347,889,897 | 990 (3) | 1,105,507,976 | 1,312,346,711 | 11,055 | 13,123 |

## moc 1.14.0, 1,000,000 rows

| rows | index on paid | in-method per run | collection per run | tail collection (ticks) | mean in the method | mean with collection | per row in the method | per row with collection |
|---:|---|---|---|---|---:|---:|---:|---:|
| 1,000,000 | no | 9,281,924,766, 13,391,272,703, 9,278,425,435, 13,387,979,098, 9,278,152,062 | 221,856,340, 10,910,535,824, 52,938,060, 10,939,491,159, 52,938,060 | 2,029,443,823 (8) | 10,923,550,813 | 15,764,991,466 | 10,924 | 15,765 |
| 1,000,000 | yes | 12,156,736,968, 14,907,012,685, 12,155,341,839, 14,907,679,848, 12,155,360,511 | 289,410,495, 8,064,476,460, 165,542,943, 8,412,161,437, 165,542,943 | 2,485,992,903 (9) | 13,256,426,370 | 17,173,051,806 | 13,256 | 17,173 |

## moc 1.16.1

| rows | index on paid | in-method per run | collection per run | tail collection (ticks) | mean in the method | mean with collection | per row in the method | per row with collection |
|---:|---|---|---|---|---:|---:|---:|---:|
| 100,000 | no | 889,839,340, 889,839,061, 889,839,081, 889,839,053, 889,838,828 | 329,097,211, 332,229,892, 329,414,820, 332,208,488, 329,309,674 | 1,236 (3) | 889,839,073 | 1,220,291,337 | 8,898 | 12,203 |
| 100,000 | yes | 1,079,437,561, 1,079,740,938, 1,079,436,205, 1,079,436,375, 1,079,436,241 | 339,167,977, 412, 347,135,958, 412, 347,890,943 | 990 (3) | 1,079,497,464 | 1,286,336,802 | 10,795 | 12,863 |
| 1,000,000 | no | 8,896,374,095, 13,424,916,517, 8,892,874,764, 13,421,622,912, 8,892,601,391 | 221,856,350, 10,910,536,380, 52,938,070, 10,939,491,675, 52,938,070 | 2,029,444,339 (8) | 10,705,677,936 | 15,547,118,913 | 10,706 | 15,547 |
| 1,000,000 | yes | 11,889,704,873, 14,949,377,040, 11,888,309,744, 14,950,044,203, 11,888,328,416 | 289,410,505, 8,064,477,289, 165,542,953, 8,412,160,860, 165,542,953 | 2,485,993,932 (9) | 13,113,152,855 | 17,029,778,554 | 13,113 | 17,030 |

Command: `node paidorders.mjs <wasm dir built by src/build2.sh> <out> 100000,1000000 0,1` (source `src/paidorders.mjs`), one run per compiler; its third argument lists the row counts, its fourth the index choices (0 no index, 1 `paid #hash`). At moc 1.14.0 that run stopped at 1,000,000 rows, when the harness canister reached the Wasm memory limit a new canister gets by default (3,221,225,472 bytes), so only its 100,000-row rows are reported; the 1,000,000-row rows come from a second run, `node paidorders.mjs <wasm dir built by src/build2.sh> <out> 1000000 0,1`, made once `install` in `src/lib.mjs` set no Wasm memory limit (`wasmMemoryLimit: 0n`, as kept here). The moc 1.16.1 run, with that `install`, gave all four rows. Each run writes `paidorders.log` and `paidorders.jsonl` in `<out>`.

Raw records: none kept; the tables above list every measured message (five per row) whole.
