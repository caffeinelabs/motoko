# Golden run images

Each `.bin` file is one complete run image exactly as the crate's run builder writes it: the image of one
index's read-only run, every byte defined. Every implementation that builds runs (natively, under wasmtime,
in Node) must produce these bytes from the same input, and every reader must read them back.

## How they are made and checked

The test `golden_runs` in `crates/nt-testhost/tests/runs.rs` rebuilds every case from its generator
(`golden_cases` in the same file), checks that the build covers every byte of the image exactly once, that two
builds into memory prefilled with different patterns (0xA5 and 0x5A) agree, that the bytes equal the file,
and that the run reads back: its distinct keys and their totals, listed through the run's key cursor, equal a
model, and its header and structure pass an upload's checks. The directory holds exactly the cases the test
knows.

To rewrite the files after a deliberate format change:

```
NT_GOLDEN_WRITE=1 cargo test -p nt-testhost --test runs -- golden_runs --nocapture
```

That run also prints the summary rows below.

## Common to every case

- Keys are index-key encodings: `k8(x)` is x as 8 bytes big-endian; text keys are escaped and terminated
  (each 00 byte written 00 FF, then 00 00), so no key is a prefix of another.
- Rids are relative to the run's rid base (header flag bit 0 set). Table uid 0x1111, schema hash 0x2222, key
  spec hash 0x3333.
- (W, V) is forced per case, not chosen, so that every key-area width and value-part size of interest
  appears.
- A count tree is present when the run's distinct keys are at most half its items; a filter when the main
  tree has more than one node.
- A run stores its keys' shared prefix once: the whole words its first and last keys share, short of the
  shorter key's end, at most 16,384 bytes (none in an index with key runs). Its length is at header offset
  2,264, its first 1,824 bytes from 2,272, the rest at the start of the key bytes; entries, fences and
  overflow records hold each key past it. A run with a prefix carries image version 2, one without version 1.

## Cases

| File | Index | W | V | Items | Entries | Nodes | Height | Distinct keys | Overflow records | Count-tree nodes | Filter blocks | Bytes |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `w8_v0_n1.bin` | 1 | 8 | 0 | 1 | 1 | 1 | 1 | 1 | 0 | – | – | 65536 |
| `w8_v0_nl.bin` | 1 | 8 | 0 | 254 | 254 | 1 | 1 | 254 | 0 | – | – | 65536 |
| `w8_v0_nl_plus_1.bin` | 1 | 8 | 0 | 255 | 255 | 3 | 2 | 255 | 0 | – | 1 | 69632 |
| `w8_v0_1e5_primary.bin` | 0 | 8 | 0 | 100000 | 100000 | 397 | 3 | 100000 | 0 | – | 31 | 1765376 |
| `w32_v8_counts.bin` | 2 | 32 | 8 | 5950 | 2219 | 28 | 2 | 300 | 0 | 4 | 1 | 200704 |
| `w8_v128_postings.bin` | 2 | 8 | 128 | 5950 | 300 | 12 | 2 | 300 | 0 | 3 | 1 | 69632 |
| `w8_v256_postings.bin` | 2 | 8 | 256 | 36860 | 163 | 12 | 2 | 40 | 0 | 1 | 1 | 69632 |
| `w1016_v0_long_keys.bin` | 3 | 1016 | 0 | 390 | 415 | 140 | 5 | 60 | 473 | 21 | 1 | 1036288 |
| `w8_v8_key_runs.bin` | 0 | 8 | 8 | 4993 | 8 | 1 | 1 | 4993 | 0 | – | – | 65536 |
| `w32_v16_rows_and_tombstones.bin` | 4 | 32 | 16 | 2100 | 1630 | 24 | 2 | 200 | 0 | 3 | 1 | 135168 |
| `w8_v16_overflowed_count_tree.bin` | 5 | 8 | 16 | 4230 | 500 | 5 | 2 | 500 | 1000 | 3 | 1 | 147456 |
| `w16_v8_prefix_fits.bin` | 6 | 16 | 8 | 900 | 300 | 4 | 2 | 300 | 0 | 3 | 1 | 69632 |
| `w8_v0_prefix_tail.bin` | 7 | 8 | 0 | 100 | 100 | 1 | 1 | 100 | 100 | – | – | 135168 |

What each case pins:

- `w8_v0_n1`: the smallest run, one entry in one leaf that is the root; no filter; a 64 KiB image.
- `w8_v0_nl`: exactly one full leaf (254 entries at W = 8, V = 0); no filter.
- `w8_v0_nl_plus_1`: one entry more than a leaf holds: two leaves under a root, so the plan's split of the
  last two nodes, the trailers of an internal node, and the smallest filter (64 lines, one 4 KiB block).
- `w8_v0_1e5_primary`: 100,000 unique keys of the primary index (header flag bit 2), three levels, UNIT
  leaves (every entry contributes 1), a filter of 31 blocks.
- `w32_v8_counts`: 300 keys of 1 to 40 rows whose rids lie about 2^20 apart, so an 8-byte posting chunk holds
  two rids and every key of 16 rows or more has at least 8 chunks and so a count entry after them; keys with
  fewer chunks have none. A count tree.
- `w8_v128_postings`: 300 keys of 1 to 40 close rids, each key one posting chunk of up to 40 rids; a count
  tree.
- `w8_v256_postings`: 40 keys of 200 to 1,643 rids with gaps of 1 to 9, posting chunks at the largest value
  part, several per key.
- `w1016_v0_long_keys`: the widest key area; 60 text keys of 1,002 to 2,005 encoded bytes, so the shortest
  fits the key area and the rest overflow into overflow records, among them the count entries of keys with 8
  to 12 rows; the count tree's overflowed keys get records after the main tree's.
- `w8_v8_key_runs`: the primary index of a positional table, key i = `k8(i)` with rid i + 10, broken where
  i mod 777 = 5: key-run chunks (kind 1, L = 8) of consecutive keys and rids, one entry per stretch; header
  flag bit 3 and L = 8.
- `w32_v16_rows_and_tombstones`: text keys `key-NNNNN` with rows and tombstones alternating (every third item
  a tombstone), so posting chunks of both signs, negative contributions, count entries whose totals are the
  rows less the tombstones, keys of total 0, and trailers that are not UNIT.
- `w8_v16_overflowed_count_tree`: 500 text keys of 33 encoded bytes sharing their first 19, so a shared prefix
  of 16 bytes, every stored key (17 bytes) longer than W = 8: one overflow record per main-tree entry and one
  per count-tree entry, 1,000 records, keys compared in pieces beyond the key area.
- `w16_v8_prefix_fits`: 300 text keys of 32 encoded bytes sharing their first 27, so a shared prefix of 24
  bytes: every stored key (8 bytes) fits W = 16, with no overflow record.
- `w8_v0_prefix_tail`: 100 text keys of 1,906 encoded bytes sharing their first 1,902, so a shared prefix of
  1,896 bytes, longer than the header's 1,824: its last 72 bytes start the key bytes, before the overflowed
  stored keys' bytes beyond W = 8.
