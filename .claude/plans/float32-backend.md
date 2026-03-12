# Float32 Backend Plan

## Context

The Float32 frontend (parse, typecheck, IR lowering) is complete on branch `claude/float32`.
The `nix build .#'tests.candid'` suite has 6 failures because the two codegen backends
(`compile_enhanced.ml`, `compile_classical.ml`) have no Float32 support:
- `type_desc` crashes with `assert false` (Float32 not in `to_idl_prim`)
- Candid serialization/deserialization of Float32 is unimplemented
- `NumConvTrapPrim (Float, Float32)` and `(Float32, Float)` hit the catch-all `todo_trap`

Float32 is stored internally as `UnboxedFloat64` (f64 with f32-truncated precision),
matching the interpreter's `Numerics.Float32 = MakeFloat(Wasm.F64)` approach.

## Files to change

### Both `src/codegen/compile_enhanced.ml` and `src/codegen/compile_classical.ml`
(All changes are parallel; the only differences are `4L`/`4l` literals and
`Wasm_exts.Values` vs `Wasm.Values`)

---

### 1. `to_idl_prim` — add IDL type code 13 for float32

**enhanced.ml ~line 7302**, **classical.ml ~line 6929** — after `| Prim Float -> Some 14l`:
```ocaml
| Prim Float32 -> Some 13l
```
Candid spec: float32 = -13, stored as positive `13l` (negated on use).
This alone fixes the `assert false` crash.

---

### 2. `StackRep.of_type` — map Float32 to UnboxedFloat64

**enhanced.ml ~line 6494**, **classical.ml ~line 6637** — after `| Prim Float -> UnboxedFloat64`:
```ocaml
| Prim Float32 -> UnboxedFloat64
```

---

### 3. `stackrep_of_type` — same for locals

**enhanced.ml ~line 10635**, **classical.ml ~line 10200** — after `| Prim Float -> SR.UnboxedFloat64`:
```ocaml
| Prim Float32 -> SR.UnboxedFloat64
```

---

### 4. `ReadBuf.read_float32` — new helper (4-byte f32 load + promote)

**enhanced.ml** — after `read_float64` (~line 2934):
```ocaml
let read_float32 env get_buf =
  check_space env get_buf (compile_unboxed_const 4L) ^^
  get_ptr get_buf ^^
  G.i (Load {ty = F32Type; align = 0; offset = 0L; sz = None}) ^^
  G.i (Convert (Wasm_exts.Values.F64 F64Op.PromoteF32)) ^^
  advance get_buf (compile_unboxed_const 4L)
```

**classical.ml** — same but `4l` and `Wasm.Values.F64`:
```ocaml
let read_float32 env get_buf =
  check_space env get_buf (compile_unboxed_const 4l) ^^
  get_ptr get_buf ^^
  G.i (Load {ty = F32Type; align = 0; offset = 0L; sz = None}) ^^
  G.i (Convert (Wasm.Values.F64 F64Op.PromoteF32)) ^^
  advance get_buf (compile_unboxed_const 4l)
```

---

### 5. Serialization `write` — Float32: demote f64→f32, store 4 bytes

**enhanced.ml ~line 7778**, **classical.ml ~line 7365** — after `| Prim Float ->` block:
```ocaml
| Prim Float32 ->
  reserve env get_data_buf 4L ^^         (* 4l in classical *)
  get_x ^^ Float.unbox env ^^
  G.i (Convert (Wasm_exts.Values.F32 F32Op.DemoteF64)) ^^   (* Wasm.Values in classical *)
  G.i (Store {ty = F32Type; align = 0; offset = 0L; sz = None})
```

---

### 6. Deserialization `read` — read 4-byte f32, promote, box as f64

**enhanced.ml ~line 8349**, **classical.ml ~line 7891** — after `| Prim Float ->` block:
```ocaml
| Prim Float32 ->
  with_prim_typ t
  begin
    ReadBuf.read_float32 env get_data_buf ^^
    Float.box env
  end
```
`read_float32` already promotes to f64 before returning; `Float.box` wraps it.

---

### 7. `NumConvTrapPrim` — Float↔Float32 conversions

**enhanced.ml ~line 11768**, **classical.ml ~line 11412** — after `| Int64, Float ->` case,
before the catch-all `| _ -> SR.Unreachable`:

```ocaml
| Float, Float32 ->
  SR.UnboxedFloat64,
  compile_exp_as env ae SR.UnboxedFloat64 e ^^
  G.i (Convert (Wasm_exts.Values.F32 F32Op.DemoteF64)) ^^   (* Wasm.Values in classical *)
  G.i (Convert (Wasm_exts.Values.F64 F64Op.PromoteF32))

| Float32, Float ->
  SR.UnboxedFloat64,
  compile_exp_as env ae SR.UnboxedFloat64 e
  (* Float32 is already stored as UnboxedFloat64; no conversion needed *)
```

---

## Test changes

- Remove `//SKIP comp` from `test/run/float32.mo`
- Run `make -C test/run float32.accept` to capture `.comp.ok` output
- Run `nix build .#'tests.candid'` — the 4 type_desc/serialization failures should be gone
  (2 remaining are Float32 literal coercion: `(0.5 : Float32)` — separate future work)

## Commit plan

Single commit: `feat: Float32 backend (Candid serialization and NumConvTrapPrim codegen)`
