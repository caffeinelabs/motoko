# Float32 RTTI Tag Plan

## Problem

`--experimental-rtti` (auto-enabled for enhanced orthogonal persistence) reconstructs
types from heap tags **without** typechecker guidance. Currently Float and Float32 both
use `Bits64 F` (tag 15), so the stable-memory serializer cannot distinguish them:
- `Float`   → should serialize 8 bytes (f64, Candid float64)
- `Float32` → should serialize **4 bytes (f32)** for format stability

A distinct `Bits64 H` tag (47) lets the serializer and debugger tell them apart.

## Approach

Float32 is stored as `UnboxedFloat64` (f64 with f32-truncated precision) in both the
interpreter and the Wasm heap. So the **heap layout** of `Bits64 H` is identical to
`Bits64 F` — 8 bytes of f64 payload. Only the header tag differs.

The stable format must serialize Float32 as **4 bytes (f32)** for type fidelity and
format stability: demote f64→f32 on serialize, promote f32→f64 on deserialize.
This requires a dedicated `StableFloat32` serialization module (cannot reuse `StableBits64`).

## Files to Change

### 1. `src/codegen/compile_enhanced.ml` and `compile_classical.ml`

**`bits_sort`** — add `H` variant (after `F`):
```ocaml
type bits_sort = U | S | F | H
```

**`int_of_tag`** — add (after `Bits64 F -> 15L`):
```ocaml
| Bits64 H -> 47L   (* 4l in classical *)
```
(47 is next available odd number after `WeakRef = 45`.)

**WeakRef dispatch** (~line 2316 enhanced, similar in classical) — add Float32 arm:
```ocaml
(Tagged.Bits64 Tagged.H, E.trap_with env "weak reference of Float32")
```

**Float32 alloc** — change the two `Bits64 F` references in the Float module
(lines ~2810, ~2819 enhanced; ~2963, ~2972 classical) to use `Bits64 H`:
```ocaml
Tagged.alloc env size Tagged.(Bits64 H) ^^
...
Tagged.(sanity_check_tag __LINE__ env (Bits64 H)) ^^
```
Note: `Float.box` / `Float.unbox` are shared between Float and Float32 currently,
so a new `Float32.box` / `Float32.unbox` pair (wrapping with `Bits64 H`) will be
needed, or the existing Float module needs a `box_as` parameter.

---

### 2. `rts/motoko-rts/src/types.rs`

Add constant (after `TAG_BITS64_F = 15`):
```rust
pub const TAG_BITS64_H: Tag = 47; // Float32 (f32-precision f64)
```

Update size dispatch (line ~1237):
```rust
TAG_BITS64_U | TAG_BITS64_S | TAG_BITS64_F | TAG_BITS64_H => size_of::<Bits64>(),
```

---

### 3. `rts/motoko-rts/src/stabilization/layout.rs`

Add variant to `StableObjectKind` (after `Bits64Float = 13`):
```rust
Bits64Float32 = 14,
```

Add import:
```rust
TAG_BITS64_H,
```

Add to the tag→kind mapping (after `TAG_BITS64_F => StableObjectKind::Bits64Float`):
```rust
TAG_BITS64_H => StableObjectKind::Bits64Float32,
```

Add to stable-kind→tag mapping:
```rust
STABLE_TAG_BITS64_FLOAT32 => StableObjectKind::Bits64Float32,
```
(add `const STABLE_TAG_BITS64_FLOAT32` similarly to the other three)

Add to both `scan_serialized` and `serialize` match arms:
```rust
StableObjectKind::Bits64Float32 => StableFloat32::scan_serialized(context, translate),
StableObjectKind::Bits64Float32 => StableFloat32::serialize(stable_memory, main_object),
```
(4-byte f32 payload: demote f64→f32 on serialize, promote f32→f64 on deserialize.)

---

### 4. `rts/motoko-rts/src/stabilization/layout/stable_float32.rs` (new file)

New module mirroring `stable_bits64.rs` but with 4-byte f32 payload:
- `serialize`: read 8-byte f64 from `Bits64 H` heap object, demote to f32, write 4 bytes
- `deserialize`: read 4 bytes, promote f32→f64, allocate `Bits64 H` heap object
- `scan_serialized`: advance by 4 bytes (not 8)

Also add arm to `stable_bits64.rs`'s `stable_object_kind → tag` (or handle in new file):
```rust
StableObjectKind::Bits64Float32 => TAG_BITS64_H,
```

---

### 5. `rts/motoko-rts/src/visitor/enhanced.rs` and `visitor/classical.rs`

Add `TAG_BITS64_H` to the Bits64 pattern (line ~129 enhanced, ~115 classical):
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
(quarter-width of f64), continuing the geometric sequence `F → H → Q`:
- `Bits64 F` = 64-bit float (full)
- `Bits64 H` = 32-bit float (half)
- `Bits64 Q` = 16-bit float (quarter)

`TAG_BITS64_Q` would be the next available odd number after 47.

## Note on future compactness

When Float32 eventually moves to `UnboxedFloat32` / `HType` (the TODO comments),
the heap object size will shrink from 8 bytes to 4 bytes (a `Bits32`-style box).
At that point the stable format and visitor will also need updating. The `Bits64 H`
tag serves as a stable identifier across that future transition.
