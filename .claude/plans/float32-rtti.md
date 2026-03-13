# Float32 RTTI Tag Plan

## Problem

`--experimental-rtti` (auto-enabled for enhanced orthogonal persistence) reconstructs
types from heap tags **without** typechecker guidance. Currently Float and Float32 both
use `Bits64 F` (tag 15), so the stable-memory serializer cannot distinguish them:
- `Float`   → should serialize 8 bytes (f64, Candid float64)
- `Float32` → should serialize **4 bytes (f32)** for format stability

A distinct `Bits64 H` tag lets the serializer and debugger tell them apart.

## Heap layout

Float32 is currently stored as `UnboxedFloat64` (f64 with f32-truncated precision)
on **both** enhanced and classical builds. The `Bits64 H` heap object layout is
therefore **identical to `Bits64 F`** — 8 bytes of f64 payload — and
`size_of::<Bits64>()` is correct.

The stable format must serialize Float32 as **4 bytes (f32)** for type fidelity:
demote f64→f32 on serialize, promote f32→f64 on deserialize.
This requires a dedicated `StableFloat32` serialization module (cannot reuse `StableBits64`).

(Future: when Float32 moves to `UnboxedFloat32` / `HType`, the heap object will shrink
to 4 bytes / `Bits32 F`. At that point `size_of` and the stable format will need
updating. The `Bits64 H` tag serves as a stable identifier across that transition.)

## Tag number constraint in the enhanced build

In the enhanced build all odd tags 1–45 are allocated and:

```
#[enhanced_orthogonal_persistence]
pub const TAG_ARRAY_SLICE_MIN: Tag = 46;
```

means **all values ≥ 46 are reserved** for the incremental GC's array-slice encoding
(`slice_tag = (array_type << 62) | slice_start`). Tag 47 is **invalid** as a real
object tag — it would be indistinguishable from a TAG_ARRAY_I slice starting at index 47.

**Fix**: bump `TAG_ARRAY_SLICE_MIN` from 46 to 48, freeing tag 47 for `TAG_BITS64_H`.
This constrains array-slice resume indices to ≥ 48, meaning a single GC increment
must process at least 48 array fields before pausing — which should already hold
given the GC's step granularity.

## Files to Change

### 1. `rts/motoko-rts/src/types.rs`

**Bump `TAG_ARRAY_SLICE_MIN`** (enhanced only):
```rust
#[enhanced_orthogonal_persistence]
pub const TAG_ARRAY_SLICE_MIN: Tag = 48;  // was 46; frees 47 for TAG_BITS64_H
```

**Add `TAG_BITS64_H`** (enhanced only, after `TAG_WEAK_REF = 45`):
```rust
#[enhanced_orthogonal_persistence]
pub const TAG_BITS64_H: Tag = 47; // Float32 (f32-precision f64)
```

**Update size dispatch** (line ~1237):
```rust
TAG_BITS64_U | TAG_BITS64_S | TAG_BITS64_F | TAG_BITS64_H => size_of::<Bits64>(),
```
(classical needs no change; `TAG_BITS64_H` is enhanced-only)

---

### 2. `src/codegen/compile_enhanced.ml` and `compile_classical.ml`

**`bits_sort`** — add `H` variant (enhanced only, or both if shared):
```ocaml
type bits_sort = U | S | F | H
```

**`int_of_tag`** — add (after `Bits64 F -> 15L`):
```ocaml
| Bits64 H -> 47L   (* enhanced only; classical uses TAG_BITS32_F path in future *)
```

**Weak-reference guard** (~line 2316 enhanced) — add Float32 arm so it traps like
Float, rather than silently falling to `branch_default`'s pass-through default:
```ocaml
(Tagged.(Bits64 H), E.trap_with env "weak reference of Float32")
```
Note: the mutable-data serialization guards at ~line 7754 (WeakRef, MutBox, Array M,
Region) concern upgrade-boundary mutability, not boxing — **no change needed there**.

**Float32 alloc** — change `Bits64 F` to `Bits64 H` for Float32 boxing:
```ocaml
Tagged.alloc env size Tagged.(Bits64 H) ^^
...
Tagged.(sanity_check_tag __LINE__ env (Bits64 H)) ^^
```
A new `Float32.box` / `Float32.unbox` pair (wrapping with `Bits64 H`) is needed,
or the existing Float module needs a `box_as` parameter.

---

### 3. `rts/motoko-rts/src/stabilization/layout.rs`

Add variant to `StableObjectKind` (after `Bits64Float = 13`):
```rust
Bits64Float32 = 14,
```

Add import: `TAG_BITS64_H`

Add to tag→kind mapping:
```rust
TAG_BITS64_H => StableObjectKind::Bits64Float32,
```

Add stable-kind constant and mapping:
```rust
const STABLE_TAG_BITS64_FLOAT32: ... = 14; // (follow pattern of others)
STABLE_TAG_BITS64_FLOAT32 => StableObjectKind::Bits64Float32,
```

Add to `scan_serialized` and `serialize` match arms:
```rust
StableObjectKind::Bits64Float32 => StableFloat32::scan_serialized(context, translate),
StableObjectKind::Bits64Float32 => StableFloat32::serialize(stable_memory, main_object),
```

---

### 4. `rts/motoko-rts/src/stabilization/layout/stable_float32.rs` (new file)

Mirrors `stable_bits64.rs` but with 4-byte f32 payload:
- `serialize`: read 8-byte f64 from `Bits64 H` heap object, demote to f32, write 4 bytes
- `deserialize`: read 4 bytes, promote f32→f64, allocate `Bits64 H` heap object
- `scan_serialized`: advance by 4 bytes (not 8)

---

### 5. `rts/motoko-rts/src/visitor/enhanced.rs`

Add `TAG_BITS64_H` to the Bits64 pattern:
```rust
TAG_BITS64_U | TAG_BITS64_S | TAG_BITS64_F | TAG_BITS64_H | ...
```

---

### 6. `rts/motoko-rts/src/debug.rs`

Add `TAG_BITS64_H` to the debug display arm (~line 191):
```rust
TAG_BITS64_U | TAG_BITS64_S | TAG_BITS64_F | TAG_BITS64_H => { ... }
```

---

## Commit plan

Single commit: `feat: distinct Bits64 H RTTI tag for Float32 heap objects`

## Note on Float16 (bfloat16 / IEEE 754 binary16)

Should `Float16` arrive (driven by AI workloads), it would naturally take `Q`
(quarter-width), continuing the sequence:
- `Bits64 F` = 64-bit float (full)
- `Bits64 H` = 32-bit float (half, stored as f64 for now)
- `Bits64 Q` = 16-bit float (quarter, stored as f64 for now)

`TAG_BITS64_Q` would be the next available odd number after 47 in the enhanced build
(bumping `TAG_ARRAY_SLICE_MIN` again if needed).
