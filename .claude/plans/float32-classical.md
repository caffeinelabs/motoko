# Plan: `SR.UnboxedFloat32` in the Classical Backend

Branch: `claude/float32`

## Context

The enhanced backend already has a complete `SR.UnboxedFloat32` implementation where `Float32`
values are stored as bit-tagged scalars in 64-bit Vanilla (f32 bits in upper 32, RTTI tag in
lower 32). This plan mirrors that work for the classical backend, with one key architectural
difference: the classical Vanilla is 32-bit (`I32Type`), so there is no room for both the
f32 bit-pattern AND a tag. **Float32 in classical must remain heap-boxed**, using the already-
allocated `Bits32 F` tag (45).

See also: `.claude/plans/float32.md` (frontend plan) and the enhanced backend scalars plan
already executed in `compile_enhanced.ml`.

## Goal

- Use `F32Type` on the Wasm computation stack for `Float32` (was: `F64Type` via `UnboxedFloat64`)
- Box to/from Vanilla via a new `Float32` module using `Bits32 F` heap objects (4-byte payload)
- Gain actual `f32` arithmetic precision (currently forces round-trip through `f64`)
- Memory savings per boxed value: 3 words (Bits32 F, incr GC) vs 4 words (Bits64 F) = 25% less
- Track enhanced backend code structure as closely as possible

## Key Architectural Difference vs Enhanced

| | Enhanced | Classical |
|---|---|---|
| Vanilla | 64-bit | 32-bit |
| Float32 Vanilla repr | bit-tagged scalar `(f32_bits << 32) \| tag` | heap `Bits32 F` object |
| `is_always_scalar` | Float32 = true | Float32 = false (heap-boxed) |
| `Opt.inject` for Float32 | no-op, skip statically | needed, runtime check |

## Files to Change

### 1. `src/codegen/compile_classical.ml`

All changes are in this file only.

#### 1a. SR module (`line ~290–326`)
- Add `| UnboxedFloat32` after `UnboxedFloat64` in `SR.t`
- Add `| UnboxedFloat32 -> F32Type` to `to_var_type`
- Add `| UnboxedFloat32 -> [F32Type]` to `to_block_type`
- Add `| UnboxedFloat32 -> "UnboxedFloat32"` to `to_string`
- Add `UnboxedFloat32` to the `drop` case (same `G.i Drop`)

#### 1b. New `Float32` module (insert after `Float` module, ~line 2975)
Parallel to `Float` but uses `Bits32 F` tag and 4-byte `f32` payload.

```
  (* Heap layout:
       ┌──────┬─────┬─────┐
       │ obj header │ f32 │
       └──────┴─────┴─────┘
     Incr GC: 3 words; no GC: 2 words.  Tag = Bits32 F (45). *)
```

Functions to add (mirroring `Float`):
- `payload_field env = Tagged.header_size env`
- `compile_unboxed_const f` — emit a Wasm F32 const (demote f64 literal to f32 bits)
- `vanilla_lit env f` — static `Bits32 F` object with `I32 (f32 bits of f)`
- `box env` — `Func.share_code1` boxing into `Bits32 F` heap object
- `unbox env` — load forwarding ptr, sanity check `Bits32 F`, load f32 field

Note: `Tagged.store_field` / `Tagged.load_field` deal in `I32` words; we need
`store_field_float32` / `load_field_float32` helpers (analogous to the existing
`store_field_float64` / `load_field_float64` for `Float`). Add these to the `Tagged` module
as well (~line 2050-2100).

#### 1c. `StackRep.of_type` (`line 9106`)
```ocaml
| Prim Float32 -> UnboxedFloat32   (* was: UnboxedFloat64, TODO *)
```

#### 1d. `StackRep.adjust` (~line 9234)
Add after the `UnboxedFloat64` cases:
```ocaml
| UnboxedFloat32, Vanilla                          -> Float32.box env
| Vanilla, UnboxedFloat32                          -> Float32.unbox env
| UnboxedFloat64, UnboxedFloat32                   -> G.i (Convert (Wasm.Values.F32 F32Op.DemoteF64))
| UnboxedFloat32, UnboxedFloat64                   -> G.i (Convert (Wasm.Values.F64 F64Op.PromoteF32))
| Const (_, Const.Lit (Const.Float64 f)), UnboxedFloat32
                                                   -> Float32.compile_unboxed_const f
```

#### 1e. `StackRep.is_always_scalar` — add, mirroring enhanced, but without Float32
`Nat8/Int8/Nat16/Int16/Char` are always tagged scalars in 32-bit Vanilla (tags have bit 0 = 0,
values always fit in the compact range). `Nat32/Int32` are excluded (values outside 27-bit
compact range spill to heap `Bits32 U/S`). `Float32` excluded (always heap-boxed in classical).

Add after `of_type` (mirroring the enhanced placement, ~line 9110):
```ocaml
  (* True for types whose 32-bit Vanilla encoding is always a bit-tagged scalar (bit 0 = 0),
     so Opt.inject is a no-op and can be omitted at compile time.
     Nat32/Int32 are excluded: values outside the 27-bit compact range are heap-boxed.
     Float32 is excluded: always heap-boxed as Bits32 F in the classical backend. *)
  let is_always_scalar t =
    Type.(match normalize t with
    | Prim (Nat8 | Nat16 | Int8 | Int16 | Char) -> true
    | _ -> false)
```

Update `OptPrim` (~line 11210) to use it:
```ocaml
  | OptPrim, [e] ->
    SR.Vanilla,
    (* TODO: extend to Float32 once classical gains scalar Float32 repr (see future work) *)
    if StackRep.is_always_scalar e.note.Note.typ
    then compile_exp_vanilla env ae e
    else Opt.inject env (compile_exp_vanilla env ae e)
```

#### 1f. `NegOp` for Float32 (~line 10964)
Split the existing Float case to add Float32:
```ocaml
| NegOp, Type.(Prim Float32) ->
  SR.UnboxedFloat32, SR.UnboxedFloat32,
  G.i (Unary (Wasm.Values.F32 F32Op.Neg))
```

#### 1g. Binary ops (~line 10780+)
Add F32 counterparts for Add, Sub, Mul, Div (mirroring Float):
```ocaml
| Type.(Prim Float32), AddOp -> G.i (Binary (Wasm.Values.F32 F32Op.Add))
| Type.(Prim Float32), SubOp -> G.i (Binary (Wasm.Values.F32 F32Op.Sub))
| Type.(Prim Float32), MulOp -> G.i (Binary (Wasm.Values.F32 F32Op.Mul))
| Type.(Prim Float32), DivOp -> G.i (Binary (Wasm.Values.F32 F32Op.Div))
```

#### 1h. Comparison / equality ops (~line 10988)
Split the `Prim (Float | Float32)` joint pattern to add F32 versions:
```ocaml
| Prim Float32 -> G.i (Compare (Wasm.Values.F32 F32Op.Eq)) ^^
                  G.i (Convert (Wasm.Values.I32 I32Op.WrapI64))  (* if vanilla is i32 *)
```
And relational ops (GtOp, GeOp, LtOp, LeOp) similarly.

Note: in classical, compare results are `I32`; check whether an `ExtendUI32` is needed
(unlike enhanced which has `ExtendUI64`).

#### 1i. `NumConvTrapPrim Float ↔ Float32` (~line 11444)
Replace the current round-trip implementation:
```ocaml
| Float, Float32 ->
  SR.UnboxedFloat32,
  compile_exp_as env ae SR.UnboxedFloat64 e ^^
  G.i (Convert (Wasm.Values.F32 F32Op.DemoteF64))

| Float32, Float ->
  SR.UnboxedFloat64,
  compile_exp_as env ae SR.UnboxedFloat32 e ^^
  G.i (Convert (Wasm.Values.F64 F64Op.PromoteF32))
```

#### 1j. `ReadBuf.read_float32` (~line 3075)
Remove the `PromoteF32` instruction; return `F32Type` on stack:
```ocaml
let read_float32 env get_buf =
  check_space env get_buf (compile_unboxed_const 4l) ^^
  get_ptr get_buf ^^
  G.i (Load {ty = F32Type; align = 0; offset = 0L; sz = None}) ^^
  advance get_buf (compile_unboxed_const 4l)
```

#### 1k. Serialization deserialization (~line 7904)
Update deserialization to box Float32 correctly:
```ocaml
| Prim Float32 ->
  with_prim_typ t
  begin
    ReadBuf.read_float32 env get_data_buf ^^
    Float32.box env                        (* was: Float.box after promote *)
  end
```
Serialization (write) side already demotes to F32 before storing — update to use
`SR.UnboxedFloat32` in the adjust call if needed.

#### 1l. `Float32->Text` (~line 11791)
```ocaml
| OtherPrim "Float32->Text", [e] ->
  SR.Vanilla,
  compile_exp_as env ae SR.UnboxedFloat32 e ^^      (* was: UnboxedFloat64 *)
  G.i (Convert (Wasm.Values.F64 F64Op.PromoteF32)) ^^ (* promote for float_fmt RTS *)
  compile_unboxed_const (TaggedSmallWord.vanilla_lit Type.Nat8 6) ^^
  compile_unboxed_const (TaggedSmallWord.vanilla_lit Type.Nat8 0) ^^
  E.call_import env "rts" "float_fmt"
```

### 2. No RTS changes needed

`TAG_BITS32_F = 45` is already defined in `rts/motoko-rts/src/types.rs:555`.
The `Bits32` struct (`header: Obj; bits: u32`) covers it.
`size_of::<Bits32>()` is returned for `TAG_BITS32_F` in the GC size table.
The classical visitor already lists `TAG_BITS32_F` as a leaf (no heap pointers inside).

### 3. No frontend changes needed

Frontend (`prim.mo`, type-checker, etc.) is already complete.

## Verification

1. Build: `make -C src ../src/moc`
2. Run existing Float32 tests:
   - `make -C test/run float32.only` — all stages incl. `[comp]`/`[wasm-run]`
   - `make -C test/run safe-float32.only`
3. Run bench:
   - `make -C test/bench alloc.only`
4. Check disassembly for F32 ops:
   ```
   moc -o /tmp/f32.wasm test/run/float32.mo
   wasm2wat --enable-all /tmp/f32.wasm | grep -E "f32\.|f64\."
   ```
   Should show `f32.demote_f64`, `f32.add`, etc., with **no** spurious `f64.promote_f32`
   round-trips in Float32-only arithmetic paths.
5. Memory check: consider adding a classical equivalent of `test/bench/float32-mem.mo`
   (without `//SKIP run`/`run-ir`/`run-low`) to confirm the 25% boxed-object size reduction.
6. Accept updated `.ok` files if instruction counts change.

## Future Work (not this plan)

The user noted a possible future optimization: represent a large class of Float32 values as
direct tagged scalars in classical Vanilla (similar to how `Nat32` has both a compact scalar
range and a heap fallback). This could use spare bits in the 32-bit Vanilla word (e.g., values
with NaN-tag patterns, or a specific exponent range). This is deferred; the present plan only
establishes the correct `SR.UnboxedFloat32` foundation and `Bits32 F` boxing.
