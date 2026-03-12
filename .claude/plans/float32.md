# Plan: Add `Float32` Primitive Type (Frontend Only)

Branch: `claude/float32`

## Goal

Add `Float32` as a new primitive type to Motoko, analogous to `Float` (f64), with:
- Type parsing via `prim "Float32"` (no new literal syntax — values created via conversion)
- Type checking
- IR lowering
- Conversion primitives: `floatToFloat32 : Float -> Float32` and `float32ToFloat : Float32 -> Float`
- Candid/IDL binding: maps to Candid `float32`
- No codegen (backend) yet

## Files to Change

### 1. `src/mo_types/type.ml`

- Add `| Float32` to `prim` variant (after `Float`)
- Add `| Float32 -> 19` to `tag_prim` function (after `Region -> 18`)
- Add `| Float32 -> None` to `span` function (unbounded, like Float)
- Add `"Float32" -> Float32` to `prim` (string→type) function
- Add `| Float32 -> "Float32"` to `string_of_prim`
- Add `| Float32` to the exhaustive subtype match (alongside Float/Char/etc.)

### 2. `src/mo_values/numerics.ml` + `.mli`

- Add `module Float32 = MakeFloat(Wasm.F64)` to `.ml`
  (same backing as Float for now; codegen will handle actual 32-bit truncation)
- Add `module Float32 : FloatType with type bits = int64 and type t = Wasm.F64.t` to `.mli`

### 3. `src/mo_values/value.ml`

- Add `| Float32 of Numerics.Float32.t` to the value variant (after `Float`)
- Add `let as_float32 = function Float32 f -> f | _ -> invalid "as_float32"` accessor

### 4. `src/mo_values/show.ml`

- Add `| T.(Prim Float32) -> true` to the `needs_parens` predicate (alongside Float)
- Add `| T.(Prim Float32), Value.Float32 i -> Numerics.Float32.to_string i` match arm

### 5. `src/mo_values/prim.ml`

- Add Float32 conversion cases to `num_conv_trap_prim`:
  ```ocaml
  | T.Float, T.Float32 -> fun v -> Float32 (as_float v)
  | T.Float32, T.Float -> fun v -> Float (as_float32 v)
  ```
  (Lossless round-trip at IR interpreter level; backend will apply f64→f32→f64 truncation)

### 6. `src/mo_idl/mo_to_idl.ml`

- Add `| Float32 -> PrimT Float32` in the `prim_typ` mapping function (after `Float -> PrimT Float64`)

### 7. `src/mo_idl/idl_to_mo.ml`

- Replace the `UnsupportedCandidFeature` error for `Float32` with `M.Prim M.Float32`:
  ```ocaml
  | Float32 -> M.Prim M.Float32
  ```

### 8. `src/prelude/prim.mo`

- Add type alias: `public type Float32 = prim "Float32";`
- Add conversion primitives in the float section:
  ```motoko
  func floatToFloat32(f : Float) : Float32 = (prim "num_conv_Float_Float32" : Float -> Float32) f;
  func float32ToFloat(f : Float32) : Float = (prim "num_conv_Float32_Float" : Float32 -> Float) f;
  ```

## Desugaring Note

The `num_conv_Float_Float32` / `num_conv_Float32_Float` prim strings are automatically
handled by the existing `desugar.ml` pattern at line 152 which calls `Type.prim s1`
and `Type.prim s2`. Once `"Float32"` is registered in `type.ml`, no change to
`desugar.ml` is needed.

## Tests

- `test/run/float32.mo` — basic type-check, conversion round-trips (//SKIP run etc. for now)
- Possibly `test/fail/float32-lit.mo` — verify Float32 literals are rejected
- Accept with `make -C test/run float32.accept`

## Tags / Ordering Notes

- Tag 19 is free (Region is 18); use it for Float32
- The `prim` variant order in `type.ml` doesn't have to match the numeric tag order,
  but place `Float32` after `Float` for readability
- `Float32` is NOT a subtype of `Float` or vice versa (they are disjoint primitive types);
  conversions are explicit
