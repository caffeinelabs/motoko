# Golden typed nodes

These files hold the exact bytes of `PBTreeMap`'s B-tree nodes, as the shared crate's tree writes them for
the typed maps of Rust EOP, a Rust library for upgrades that keep the heap in place, after chosen steps of
three fixed scripts, together with the log of the nodes each step
allocates and hands back. Every byte of a 4,080-byte node is compared, stale copies past `count` included,
and so is the exact set of bytes nobody wrote.

**The files are frozen.** No test or tool rewrites them. A change to them is a change to `PBTreeMap`'s node
bytes, which needs the sign-off of both the crate's owner and Rust EOP's owner, and the matching change to
Rust EOP's own copy.

## Who checks them

- The crate's side: `cargo test -p nt-testhost --test golden_nodes` (also run by `scripts/check.sh`). It runs
  every script on the crate's tree over the typed test host, once with the fill pattern 0xA5 and once with
  0x5A, and fails on any difference from these files. Setting `NT_GOLDEN_NODES_OUT=<dir>` also writes the
  text it produced to `<dir>` (never to this directory), for inspection.
- The reference side: `crates/nt-eopref` (its own workspace, which builds rust-eop, the repository of Rust EOP,
  from a local clone; `cargo run --release` in that directory, also run by `scripts/check.sh` wherever that crate
  is present).
  It runs the same scripts on today's `PBTreeMap`, rust-eop's own `map.rs` at the revision pinned in its
  `Cargo.toml` (3efbee1a405c554977d55bc35a366dc608f8396b), in Rust EOP's host mode, through `ic-cdk-eop`'s
  public API only, writes its text to `crates/nt-eopref/reference/` and compares it byte for byte with these
  files. At that revision all three files are equal.

Both sides produce the text with the same code, `crates/nt-testhost/src/golden_nodes.rs`, from what each
observes of its own run, so equal observations give equal files.

## The scripts

The scripts are defined in `crates/nt-testhost/src/golden_nodes.rs`. Each starts from an empty map, makes
all its allocations before its first hand-back (an insert phase, then a remove phase), so that within a
script each node reference names exactly one allocation, and ends with the tree emptied.

| File | Map | Capacities (leaf / internal) | Depth | Steps |
|---|---|---|---|---|
| `u64_depth3.txt` | `<u64, u64>` | 254 / 169 | 3 | 88,326 |
| `a16_depth2.txt` | `<[u8; 16], u64>` | 169 / 127 | 2 | 690 |
| `c3_depth3.txt` | `<[u8; 1012], u64>` | 3 / 3 | 3 | 70 |

Between them the imaged steps exercise: the first insert into an empty map; an insert with room; leaf splits
with the new entry's index i below, at and above h = ⌈c/2⌉; internal splits at i < h, i = h and i > h; a root
split of a leaf root and of an internal root; a leaf removal without an underflow; the removal of an entry of
an internal node (its predecessor moves up); merges and rotations from the left and from the right, at the
leaf level and at an internal level; an internal root emptied by a merge (handed back, its child 0 becoming the
root); and the emptied root leaf (the root becomes 0). The depth-3 scripts exercise every one of them at both
levels; the depth-2 script every case at the leaf level. The test checks each step's case list against a
count model of `PBTreeMap`'s rules applied to the tree before the step.

**Keys and values.** A script names keys by labels (u64). The key of label l is made of the base-254 digits of
l, each digit d written as the byte d + 1, skipping 0xA5 (so bytes 0x01..=0xA4, 0xA6..=0xFF), so that no key
byte is 0x00 or 0xA5 and label order is key order:

- `u64` keys: the eight digit bytes, least significant first, as the little-endian bytes of the u64;
- `[u8; N]` keys: the eight digit bytes, most significant first, then, at byte j ≥ 8, the byte of digit
  (131·l + 37·j) mod 254.

The value of label l is the u64 whose little-endian bytes are the bytes of digits (z >> 8i) & 0xFF mod 254,
i = 0..7, z a splitmix64 mix of l (`golden_nodes::value`). No value byte is 0x00 or 0xA5 either.

## Normalization

What differs between hosts by construction is removed, and nothing else:

1. **Written and unwritten bytes.** Production `node_alloc` writes only the 8-byte header; bytes 8..4,080 of a
   fresh node hold whatever the block held. The crate's test host fills them with a pattern P right after
   `alloc`, and every script runs with P = 0xA5 and with P = 0x5A, which differ in every bit: a byte equal in
   both runs is written, a byte that differs is unwritten (and must hold P in each run). The two runs must give
   identical results and identical logs. On the reference side the host region starts zeroed and fresh nodes
   are carved from it, so a zero byte in 8..4,080 is unwritten (no key or value byte is zero), the header is
   written, and a child slot is written whole when its word is non-zero.
2. **Allocation ordinals.** A node reference becomes its allocation ordinal, its 1-based position among the
   script's `alloc` results (the crate: the test host's log; the reference: the address order of nodes as they
   first appear, which is allocation order for blocks carved from dlmalloc's top chunk).
3. **Child slots.** In an internal node, every child slot j = 0..nI that is written holds a whole word; a
   written non-zero word is replaced by the ordinal of the node it names (u64, little-endian). A written word
   that names no allocation of the script fails.
4. **The layout word** at offset 4 must equal the registered leaf or internal layout number (the test host
   writes 1 and 2) and becomes 1 in a leaf, 2 in an internal node.
5. **Reachable nodes only.** An image covers every node reachable from the root after the step, in pre-order.
   A handed-back node is never imaged (Rust EOP's host mode poisons it with 0xA5 and dlmalloc writes into it);
   it appears in the log only. The reference side requires every handed-back node to read 0xA5 from byte 48
   on.

## Format

UTF-8 text, one item per line.

```
# comment
script <name>
map <type> ks <key part bytes> vs 8 leaf cap <nL> voff <voff> internal cap <nI> voff <voff> coff <coff>
steps <number of steps>
```

Then one block per step that allocated, handed back or was imaged; other steps print nothing.

```
step <index, 1-based> insert|remove <label> [<cases>]   the bracket only on imaged steps
alloc <ordinal> leaf|internal                            each node the step allocated, in allocation order
hand_back <ordinal>                                      each node it handed back, ascending (a set)
root <ordinal, 0 for none> len <entries>
image <number of nodes>                                  imaged steps only, then that many node blocks:
node <ordinal> leaf|internal count <count> full|delta|same
<offset> <data>                                          node lines
```

A node line covers 32 bytes from `<offset>` (4 hex digits, a multiple of 32; the last line, at 0fe0, covers
16 bytes). `<data>` gives two lowercase hex digits per written byte and `..` per unwritten byte. The node's
image is:

- `full`: the node's first image: every byte unwritten except the lines given; lines with no written byte are
  omitted;
- `delta`: the node's image at the last step where it was imaged, with the lines given replaced;
- `same`: the node's image at the last step where it was imaged.

`golden_nodes::parse` reads a file back into these images; the test checks that it reproduces what the run
observed.
