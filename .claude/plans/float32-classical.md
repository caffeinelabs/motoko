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

## Prerequisites (done)

- **EOP `Float32Lit` constant bug fixed** (commit `b6c4a3cf7`):
  `const_lit_of_lit (Float32Lit f)` in `compile_enhanced.ml` was producing `Const.Float64 f`,
  causing `build_constant_aux` to box as `Bits64 F`. Fixed by computing the tagged I64 scalar
  at OCaml level — use `Int64.(...)` local open for clarity:
  ```ocaml
  let f32_bits = Int32.bits_of_float (Numerics.Float32.to_float f) in
  Const.Vanilla Int64.(logor
    (shift_left (logand (of_int32 f32_bits) 0xFFFFFFFFL) 32)
    (TaggingScheme.tag_of_typ Type.Float32))
  ```
  The `adjust` case `Const Const.Lit (Const.Vanilla n), UnboxedFloat32` emits a single
  `f32.const` — again use `Int64.(...)`:
  ```ocaml
  G.i (Const (nr (Wasm_exts.Values.F32 (Wasm.F32.of_bits Int64.(to_int32 (shift_right_logical n 32))))))
  ```
  Classical is unaffected (Float32 stays heap-boxed, adjust uses `Const.Float64` path).

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
- `compile_unboxed_const f` — emit a Wasm F32 const at OCaml level:
  `G.i (Const (nr (Wasm.Values.F32 (Wasm.F32.of_bits (Int32.bits_of_float (Numerics.Float32.to_float f))))))`
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

### Partial scalar representation using the 1-LSBit invariant

Without `--experimental-rtti`, the classical Vanilla tagging scheme uses only **1 bit** to
distinguish scalars from heap pointers: bit 0 = 0 means scalar; bit 0 = 1 means a skewed
heap pointer (all 4-byte-aligned heap addresses have bits 1:0 = 00; the stored Vanilla
pointer is `address - 1`, so bit 0 = 1 always).

#### F32 bit layout reminder

IEEE 754 single-precision (F32):
```
 31  30    23  22                    0
  ┌───┬────────┬──────────────────────┐
  │ S │ exp(8) │    mantissa (23)     │
  └───┴────────┴──────────────────────┘
                                     ╰── bit 0 is the LSBit of the mantissa
```

#### The scheme

- **bit 0 of F32 bit pattern = 0**: store the 32-bit F32 bit pattern directly as the
  I32 Vanilla scalar.  Unbox with a single `f32.reinterpret_i32`.  No heap allocation.
  Covers ±0.0, all powers of 2, and roughly 50 % of all F32 values.

- **bit 0 of F32 bit pattern = 1**: cannot store directly (would look like a heap pointer).
  Box into a `Bits32 F` heap object; the skewed pointer stored in Vanilla is `addr - 1`
  which has bit 0 = 1, consistent with all other heap pointers.

```
box (f32 on Wasm stack):
  i32.reinterpret_f32        → I32 bits
  check bit 0
    = 0: return bits as Vanilla scalar            (no alloc)
    = 1: alloc Bits32 F {bits}, return ptr - 1    (heap, skewed)

unbox (I32 Vanilla):
  check bit 0
    = 0: f32.reinterpret_i32                      (no load)
    = 1: load_forwarding_ptr → load_field_float32  (heap)
```

#### Constant pool implications

`const_lit_of_lit (Float32Lit f)` is the natural split point.  Check bit 0 of the F32 bit
pattern at OCaml compile time:

```ocaml
| Float32Lit f ->
  let bits = Int32.bits_of_float (Numerics.Float32.to_float f) in
  if Int32.logand bits 1l = 0l
  then Const.Vanilla bits   (* scalar: materialises as a Wasm i32.const / f32.const *)
  else Const.Float32 f      (* heap:   shared_static_obj Bits32 F on first use *)
```

- **`Const.Vanilla bits`**: no static heap object; `materialize_lit` returns the bits as-is
  (line 9233: `Const.Vanilla n -> n`).  The existing `adjust` case
  `Const (_, Const.Lit (Const.Vanilla n)), UnboxedFloat32` emits a single `f32.const`.
- **`Const.Float32 f`**: `materialize_lit` calls `Float32.vanilla_lit env f` which allocates
  a shared static `Bits32 F` heap object and returns its skewed pointer.

**Invariant**: in the classical backend `Const.Float32` exclusively represents Float32 values
whose bit pattern has bit 0 = 1 — i.e., every entry in the `Bits32 F` constant-pool sub-set
has mantissa LSBit set.

**Common literals** that become free scalars: `0.0` (0x00000000), `1.0` (0x3F800000),
`2.0` (0x40000000), `0.5` (0x3F000000), `10.0` (0x41200000).
Values like `3.14` (0x4048F5C3, bit 0 = 1) still require a pool entry.

#### Sanity-check hook

`Tagged.sanity_check_tag` (guarded by `Flags.sanity || TaggingScheme.debug`) can enforce
the invariant on the **heap path**: when we follow a Vanilla pointer to a `Bits32 F` object,
the `u32` payload stored inside that object must have bit 0 = 1 (otherwise it should have
been kept as a scalar).  Add this assertion in `Float32.unbox` after loading the payload field.

#### Trade-offs vs. current always-heap implementation

| | Current (`Bits32 F` always) | Scalar-when-possible |
|---|---|---|
| Box cost | always alloc | branch + conditional alloc |
| Unbox cost | always load | branch + conditional load |
| `is_always_scalar` | false | still false (≈50 % box) |
| `Opt.inject` | always needed | still needed |
| GC pressure | 3 words per value | ≈1.5 words avg |

This optimisation is deferred; the present plan establishes the `SR.UnboxedFloat32`
foundation and always-heap `Bits32 F` boxing on which it can be layered.
