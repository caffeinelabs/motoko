# cost_vs_ref

What one typed-map operation costs, in wasmtime fuel, three ways in one wasm64 module built and run by
`scripts/cost_vs_ref.sh`:

- `ref`: `nt-ref`'s transcription of the `PBTreeMap` that Rust EOP, a Rust library for upgrades that keep
  the heap in place, ships today;
- `unc`: the crate's typed tree with `CHECKED` false over `RawTypedNodes`, the host Rust EOP's map will
  use, behind a front end shaped like today's map. It must cost what `ref` costs;
- `chk`: the same with `CHECKED` true, recorded for information.

The three maps of a size are built by the same inserts in their own arenas, so they hold the same tree and
see the same allocator states, and every operation runs on all three; the module checks after each row that
the trees are still equal. Each figure is the mean over the samples of one export call, which includes the
export's own few instructions; the iteration rows give fuel per element. Fuel counts executed wasm
instructions; it is a fast proxy for the replica's instruction counter, not that counter.

## The harness, pinned

- wasmtime 42.0.1, the runner's locked dependency; `Config::new()` with memory64 and fuel consumption on and
  every other setting default; the fuel of a call is the store's fuel before it minus after it.
- 200 samples per row (the `next` rows: 20 iterators of 256 steps each).
- The module built by `nightly-2026-07-25` with `-Z build-std`, release profile, one codegen unit, as a
  `cdylib` only. Both settings matter: with the default sixteen units, which function lands in which unit
  follows symbol hashes and changed inlining, and so the fuel of a row, by 5 to 15 for unchanged code; an
  `rlib` crate type beside the `cdylib` changed the linkage of the instances and with it the inlining.

## The gate

On `unc`: get and insert with room cost exactly what `ref` costs on every sample; every other row costs at
most max(4 fuel, 0.5% of `ref`) more on average. The script exits with status 1 when a row fails. A negative
Δ is recorded as it is; the internal-level removals need a level between the root and the leaves, so they
have no samples at depth 2.

## The rows

Shapes: `<u64, u64>` at 10^3, 10^5 and 10^6; `<u64, [u8; 1024]>` at 10^4, whose values are boxed;
`<PString, u64>` at 10^5, whose comparisons read each string's buffer (capacities 169 and 127); and
`<[u8; 1013], u64>` at 10^5, whose keys are boxed. Operations: get; insert with room and with a split;
`entry(k).or_insert(v)` on a vacant key, one search and an insert along the saved path; removal from a leaf
with no rebalancing, with a merge, with a rotation from the left and from the right at the leaf level, and a
leaf merge followed by a merge or a rotation of the leaf's parent; a seek with each start bound together
with its iterator's first step; one iterator step with each end bound; and, at 10^5 and 10^6 of
`<u64, u64>`, iteration per element over the whole map and over bounded ranges of about 1,000 entries. Each
planned key is checked against the tree before it runs (that it splits or not, which levels its removal
rebalances).

## Result

Every row passes but insert with room in three places:

- `<u64, u64>` at 10^6 (197 of 200 samples equal) and `<u64, [u8; 1024]>` at 10^4 (199 of 200): the samples
  that differ, by 1 fuel each, insert after the last entry of their leaf, where no entry moves. The optimizer
  lays out that path differently around the empty move; every other sample is equal.
- `<PString, u64>`: `unc` costs 35 fuel less on every sample. Today's map calls its descent out of line for
  string keys, where it has three callers and a larger body, and the crate's typed descent is inlined; the
  same saving shows on every string-key insert and removal.

Elsewhere `unc` equals `ref` (get everywhere; insert with room at 10^3, at 10^5 and for boxed keys) or costs
less: removals 3 to 16 fuel less, seeks 3 to 19 less, iterator steps 5 to 9 less, iteration per element 4 to 6
less; splits and `entry` are within ±2 fuel; with string keys every insert and removal is 28 to 40 less, for
the reason above.

The checked instance, for information: get and insert with room +17% to +21% (+5% for string keys and +0.5%
to +11% for boxed keys, whose comparisons dominate), splits +4% to +6%, seeks from a bound +17% to +19% (+1%
to +5% for boxed and string keys), iterator steps and iteration per element +1.5% to +7%, merges and
rotations +33% to +60% (+1.5% to +28% for boxed and string keys).

## Changes to the crate for these rows

None changes a byte moved or the order of host calls.

1. A typed `Cursor::seek_here` is always inlined into its caller: out of line its `Result` came back through
   memory, where today's seek returns nothing. The typed front end calls it from its own seek, kept out of
   line as today's is once a map is iterated from more than one place.
2. `leftmost` and `rightmost` are always inlined, as today's are: out of line, a seek that descended through
   them paid a call and a `Result` in memory.
3. `Cursor::peek`, `advance` and `clear`: the two halves of `next`. The typed front end checks its end bound
   between them on the key reference it hands out, as today's iterator does; with the bound checked inside
   `next`, the reference was formed twice (+6 to +8 fuel per step for string keys).
4. `Fault::text` is inlined, so a front end trapping with a fault's text needs no stack slot for the fault.

`before-fixes/` holds the first round's measurement of the tree before its own cost fixes, in that round's
format and rows.
