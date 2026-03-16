# Float32 Scalar Representation Plan (`float32-scalar.md`)

## Context

Float32 currently uses `UnboxedFloat64` (Wasm `f64`) everywhere in the 64-bit enhanced
backend — wasting both Wasm stack type and heap/array storage. The plan is to give Float32
a proper scalar stack representation (`SR.UnboxedFloat32` / `F32Type`) and pack f32 bit
patterns into the **upper 32 bits of 64-bit in-place storage slots** (typed arrays etc.),
with the fine-grained tag in the lower 32 bits — mirroring how Nat32 uses the upper 32 bits.

TODO comments at `compile_enhanced.ml` lines **6502**, **10656**, **12079** already
anticipate this change (`(* TODO: switch to UnboxedFloat32 (F32Type) for compactness *)`).

Classical codegen is **not touched** in this plan.

## Representation Summary

| Context | Representation |
|---------|----------------|
| Wasm stack (`SR`) | `SR.UnboxedFloat32` → `F32Type` |
| In-place 64-bit slot / array element | `(reinterpret_f32_as_u32(v) << 32) \| tag_of_typ Float32` in I64 |
| Vanilla / polymorphic / Option / Any | inline tagged — same I64 word, like Nat32 |

Float32 **does** get tag bits in the lower 32 bits of the 64-bit slot, exactly like
Nat32 (`0x40000000`) and Int32 (`0xC0000000`). A new distinct tag value must be chosen
(e.g. `0x20000000` — `bits 31:30 = 00` with `bit 29 = 1`) and added to
`TaggingScheme.tag_of_typ` and `TaggingScheme.ubits_of`.

This resolves the `0.0f32` / null ambiguity: `0.0f32` in Vanilla is
`(0x00000000 << 32) | tag = tag` (non-zero, safe). Like Nat32, Float32 Vanilla is
inline — no heap allocation needed.

## Files to Change

### `src/codegen/compile_enhanced.ml` only

---

### 1. `SR.t` — add `UnboxedFloat32` variant

After `UnboxedFloat64` (~line 349):
```ocaml
| UnboxedFloat32
```

Update `SR.to_var_type`:
```ocaml
| UnboxedFloat32 -> F32Type
```

Update any exhaustive match on `SR.t` (e.g. `SR.is_unboxed`, `SR.eq`, pretty-printers).

---

### 2. `StackRep.of_type` and `stackrep_of_type`

Lines 6502 and 10656 — replace the TODO-carrying `UnboxedFloat64` with:
```ocaml
| Prim Float32 -> UnboxedFloat32        (* was UnboxedFloat64 *)
```

---

### 3. Memory layout and `TaggingScheme` — mirrors Nat32

Float32 uses the **same layout as Nat32** in a 64-bit slot:

```
Nat32:   [ tag_u32 | value_u32 ]   I64 = (value << 32) | tag
Float32: [ tag_u32 | f32_bits  ]   I64 = (f32_bits << 32) | tag
```

On little-endian (Wasm), the lower 32 bits (tag) sit at lower memory addresses
(offset 0..3); the upper 32 bits (f32 payload) sit at higher addresses (offset 4..7).

**Read**: `f32.load` at `base + 4` — direct one-instruction float load.

**Write** (two 32-bit ops, avoids reinterpret/extend/shift cost):
```wasm
f32.store  offset=4   ; upper half: f32 payload
i32.store  offset=0   ; lower half: tag (or 0 when fine-grained tagging is off)
```

**When fine-grained tagging is off**: write `0` at offset 0 — identical to Nat32
writing zeros in its lower-32 tag position when tagging is disabled.

Add to `TaggingScheme.tag_of_typ` (~line 94) — tag lives in **lower 32 bits**, like Nat32:
```ocaml
| Float32 -> 0b00100000_00000000_00000000_00000000L  (* 0x20000000L — pick free slot *)
```

Add to `TaggingScheme.ubits_of` (~line 134):
```ocaml
| Float32 -> 32
```

`can_tag_const` works generically from `ubits_of`; no change needed there.
`BitTagged.clear_tag` / `sanity_check_tag` work on the lower 32 bits — same as Nat32, no special-casing needed.

---

### 4. `SR.adjust` — conversion to/from Vanilla and between float widths

Add cases (near the `UnboxedWord64` tag/untag block):

```ocaml
(* Float32 unboxed → Vanilla: mirrors Nat32's msb_adjust + tag *)
| UnboxedFloat32, Vanilla ->
  G.i (Unary  (Wasm_exts.Values.I32 I32Op.ReinterpretFloat)) ^^  (* f32 → i32 *)
  G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32)) ^^        (* i32 → i64 *)
  compile_shl_const 32L ^^                                         (* value to upper 32 *)
  compile_bitor_const (TaggingScheme.tag_of_typ Type.Float32)      (* tag in lower 32 *)

(* Vanilla → Float32 unboxed: mirrors Nat32's sanity_check + lsb_adjust *)
| Vanilla, UnboxedFloat32 ->
  BitTagged.sanity_check_tag __LINE__ env Type.Float32 ^^
  compile_shrU_const 32L ^^                        (* shift f32 bits down, dropping tag *)
  G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
  G.i (Unary  (Wasm_exts.Values.F32 F32Op.ReinterpretInt))

(* Cross-width conversions on the Wasm stack *)
| UnboxedFloat32, UnboxedFloat64 ->
  G.i (Convert (Wasm_exts.Values.F64 F64Op.PromoteF32))
| UnboxedFloat64, UnboxedFloat32 ->
  G.i (Convert (Wasm_exts.Values.F32 F32Op.DemoteF64))
```

---

### 5. In-place array element storage helpers

Float32 slot layout mirrors Nat32: **f32 bits at upper-32 (offset 4..7), tag at lower-32 (offset 0..3)**.
Two-instruction write; one-instruction read.

```ocaml
(* Store: F32 on Wasm stack → two 32-bit stores *)
let store_field_float32 env get_addr get_f =
  get_addr ^^ get_f ^^
  G.i (Store {ty = F32Type; align = 0; offset = 4L; sz = None}) ^^  (* f32 at upper half *)
  get_addr ^^
  ( if TaggingScheme.tagging_enabled ()
    then compile_unboxed_const (TaggingScheme.tag_of_typ Type.Float32)  (* tag as i32 *)
    else compile_unboxed_zero ) ^^                                        (* 0 when tagging off *)
  G.i (Store {ty = I32Type; align = 0; offset = 0L; sz = None})          (* tag at lower half *)

(* Load: plain f32.load at offset 4 — no shift, no reinterpret *)
let load_field_float32 env get_addr =
  get_addr ^^
  G.i (Load {ty = F32Type; align = 0; offset = 4L; sz = None})
```

The array element read/write for `Prim Float32` should use these helpers.
Search for `Array.write` / typed-array codegen (~line 4400) and add Float32 arms.

---

### 6. Float32 arithmetic and comparisons

All Float32 operations currently use Float64 ops. Replace with F32 variants:

- **Comparisons** (~line 11379):
  ```ocaml
  | Prim Float32 -> compile_comparison env SR.UnboxedFloat32 (Wasm_exts.Values.F32 F32Op.Eq) e1 e2
  ```

- **Arithmetic** (`UnOp`, `BinOp` dispatch): add `Float32` arm alongside `Float`, using
  `Wasm_exts.Values.F32 F32Op.*` operations.

- **`debug_show` / `Float32_to_text`** (~line 12079 TODO): update to use `SR.UnboxedFloat32`
  and a new `text_of_float32` call that operates on f32 directly.

---

### 7. `NumConvTrapPrim` — Float ↔ Float32

Update the existing arms (~lines 11800–11809) now that Float32 uses F32Type:

```ocaml
| Float, Float32 ->
  SR.UnboxedFloat32,
  compile_exp_as env ae SR.UnboxedFloat64 e ^^
  G.i (Convert (Wasm_exts.Values.F32 F32Op.DemoteF64))

| Float32, Float ->
  SR.UnboxedFloat64,
  compile_exp_as env ae SR.UnboxedFloat32 e ^^
  G.i (Convert (Wasm_exts.Values.F64 F64Op.PromoteF32))
```

---

### 8. Float32 literals

Line 10847: `| Float32Lit f -> Const.Float64 f`

Since `Const` has no `Float32` variant, emit as `Const.Float64 f` (keeping the full
f64 bit pattern) and add a convert in the constant-to-SR path: when the expected SR is
`UnboxedFloat32`, apply `f64.demote_f32` after loading the constant.

Alternatively, add `Const.Float32 of F32.t` to `const.ml` — but that's a larger change;
the demote-after-load approach is simpler for now.

---

### 9. Candid serialization / deserialization

Serialization (~line 7787): unpack from inline Vanilla to f32, then write 4 bytes:
```ocaml
| Prim Float32 ->
  reserve env get_data_buf 4L ^^
  get_x ^^ SR.adjust env SR.Vanilla SR.UnboxedFloat32 ^^   (* inline I64 → f32 *)
  G.i (Unary  (Wasm_exts.Values.I32 I32Op.ReinterpretFloat)) ^^
  G.i (Store {ty = I32Type; align = 0; offset = 0L; sz = None})
```

Deserialization (~line 8349): `ReadBuf.read_float32` currently promotes to f64 and returns
f64; update it to return `F32Type` directly (drop the `PromoteF32` convert), then tag inline:
```ocaml
| Prim Float32 ->
  ReadBuf.read_float32 env get_data_buf ^^   (* returns F32 *)
  SR.adjust env SR.UnboxedFloat32 SR.Vanilla (* inline-tag as I64 *)
```

---

### 10. Weak-reference guard

Float32 IS inline in Vanilla (like Nat32), so `branch_default` (~line 2316) sees it as
a tagged scalar. `branch_default` dispatches on **heap-object tags** loaded from object
headers — it won't match an inline Float32 (no heap header). Add Float32 to the
`BitTagged`-style scalar guard before the heap-tag dispatch, mirroring how other small
words are excluded from weak referencing:
```ocaml
(* before branch_default: check inline Float32 tag *)
get_x ^^ BitTagged.has_tag env Type.Float32 ^^
E.if_ env [] (E.trap_with env "weak reference of Float32") G.nop ^^
```

---

## Critical Files

- `src/codegen/compile_enhanced.ml` — all changes above
- `src/codegen/const.ml` — optional: add `Float32` constant variant
- `test/run/float32.mo` — extend with array tests once implemented
- `test/run-drun/` — add a Float32-in-array drun test

## Verification

```bash
# Build enhanced backend
nix build .#moc

# Run Float32 tests
make -C test/run float32.only
make -C test/run float32-candid.only

# Run full candid suite
nix build .#'tests.candid'
```

## Commit plan

Two commits:
1. `feat: SR.UnboxedFloat32 (F32Type) for Float32 in enhanced backend`
2. `feat: typed array element storage for Float32 (f32 bits in upper 32 of I64)`
