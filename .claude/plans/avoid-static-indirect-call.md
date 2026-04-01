# Plan: Avoid `call_indirect` for Statically-Known Functions

## Problem

When a top-level Motoko function (compiled as `Const.Fun`) is used as a
first-class value and then called, the compiler may materialise it into a
heap-allocated static closure object and then call it via `call_indirect`.
This is pure overhead: the function index is stored into the closure only to
be loaded back immediately.

The pattern in Wasm:
```wat
i32.const <static_closure_ptr>   ;; materialize_const_v (Const.Fun fi)
...
i32.load offset=5                ;; load funptr_field back out
call_indirect                    ;; call via table
```

Whereas it could be:
```wat
i32.const 0                      ;; dummy $clos
...args...
call $fi                         ;; direct call
```

## Key Code Locations (`compile_classical.ml`)

### Call dispatch — `CallPrim` (line 11197)

All Motoko function calls compile through `CallPrim _, [e1; e2]`.
The match at line 11218 has three branches:

1. **`SR.Const (_, Const.Fun (mk_fi, PrimWrapper prim))`** (line 11219):
   Inlines the primitive directly.

2. **`SR.Const (_, Const.Fun (mk_fi, _))`** (line 11238):
   Emits `compile_unboxed_zero ^^ args ^^ Call fi` — the optimal direct call.
   This fires when `compile_exp env ae e1` returns `SR.Const`.

3. **`_, Type.Local`** (line 11247):
   The fallthrough for runtime closures. Emits:
   `code1 ^^ StackRep.adjust … SR.Vanilla ^^ set_clos ^^ … ^^ call_closure`
   where `call_closure` does `load_field(funptr_field) ^^ CallIndirect`.

   **Invariant**: `fun_sr` here is never `SR.Const (_, Const.Fun _)` — those
   are caught by branch 2. However, `fun_sr` may be `SR.Vanilla` where the
   value on the stack is a *static closure* (produced by `materialize_const_v`
   earlier), meaning the function index was statically known at some earlier
   point but the `SR.Const` rep was lost.

### Materialisation — `materialize_const_v` (line 9251)

```ocaml
| Const.Fun (get_fi, _) -> Closure.static_closure env (get_fi ())
```

`static_closure` (line 2531) writes the function index into a shared static
heap object:
```ocaml
let static_closure env fi : int32 =
  Tagged.shared_static_obj env Tagged.Closure StaticBytes.[
    I32 (E.add_fun_ptr env fi);
    I32 0l
  ]
```

`materialize_const_v` is called **only** from `materialize_const_t`, which is
called from `StackRep.adjust` (line 9318) when converting `SR.Const → Vanilla`.

### Const propagation gate — `compile_exp_with_hint` (line 12732)

```ocaml
if exp.note.Note.const
then let (c, fill) = compile_const_exp env ae exp in fill env ae; (SR.Const c, G.nop)
```

Whether `e1` in `CallPrim` returns `SR.Const` or `SR.Vanilla` depends entirely
on `exp.note.Note.const`. If the const annotation flows through a `let`-binding
to the call site, branch 2 fires (direct call). If not, the value is
materialised and branch 3 fires (indirect call).

## Root Cause

The missed optimisation occurs when the function index is statically known at
the call site but the call still goes via `call_indirect`.  There are three
distinct subcases:

### Subcase 1 — `Const.Fun` materialised to static closure
When `Note.const = true` is **not** propagated to a use site of a function
value (e.g. a top-level function passed as argument), `StackRep.adjust
SR.Const → Vanilla` calls `materialize_const_v (Const.Fun fi)`, which writes
`fi` into a shared static heap object.  The callee then loads it back out via
`call_closure` → `call_indirect`.

### Subcase 2 — Capturing closure with statically-known function index
When a function with captures is compiled (`FuncDec.lit` → `FuncDec.closure`),
the function index `fi` is computed at line 9782 and stored into the closure's
`funptr_field` (line 9792).  At the call site the compiler loads it back out
and does `call_indirect`, even though `fi` was fixed at compile time.

Concrete example — `bar.1` in `test/run/fmodf-forward.mo`:
```motoko
func bar(a : Float32, b : Float, c : Nat32) : Float {
  func quux(a : Float32, b : Float, _ : Nat32) : Float = if (c != 0) …;
  quux(a, b, c)   (* bar.1 allocates quux.1's closure, then call_indirect *)
};
```
`quux.1` captures `c`, so `Note.const = false` and `VarE "quux"` binds as
`VarEnv.Local (SR.Vanilla, …)`.  Yet the call could be `call $quux.1` with
the freshly-built closure passed as `$clos`.

Unlike subcase 1, a **real closure is needed** (to pass captured vars), so we
cannot use `compile_unboxed_zero`.  The optimised emission would be:
```wat
get_clos              ;; closure with captured vars (as $clos argument)
…args…
call $quux.1          ;; direct call instead of call_indirect
```

### Subcase 3 — Function passed as argument to higher-order callee
When a statically-known function is passed as argument to a higher-order
function, it is materialised (subcase 1).  Inside the callee the parameter is
`VarEnv.Local (SR.Vanilla, …)` — the static identity is permanently lost.
Fixing this requires devirtualization / inlining and is out of scope.

## Fix Directions

### Option A — Preserve `Note.const` through `let`-bindings (already working)

The const-analysis pass (`ir_passes/const.ml`) already propagates
`Note.const = true` for inner functions without captures.  `FuncDec.lit`
returns `SR.Const (_, Const.Fun _)` when `captured = []`, and the call goes
through branch 2 (direct `Call` with `i32.const 0` dummy closure).  The
assert in branch 3 confirms `SR.Const (_, Const.Fun _)` never reaches it.

### Option B — Codegen: `SR.StaticClosure fi` (subcase 2) — **DONE (enhanced backend)**

`FuncDec.closure` now returns `SR.StaticClosure fi` instead of `SR.Vanilla`.
A special case in `compile_dec` for `LetD (VarP v, FuncE …)` non-const eagerly
compiles the FuncE with `pre_ae`, extracts `fi` from `SR.StaticClosure fi`, and
binds `v` as `VarEnv.Local (SR.StaticClosure fi, local_i)` so the correct SR
reaches `CallPrim`.

In `CallPrim`, `SR.StaticClosure fi, Type.Local` is matched at the outer level
(before the generic `_, Type.Local` arm), emitting `Call fi` directly:
```ocaml
| SR.StaticClosure fi, Type.Local ->
    … code1 ^^ set_clos ^^ get_clos ^^ prepare_closure_call ^^ args ^^
    G.i (Call (nr fi)) ^^ FakeMultiVal.load …
```

`StackRep.adjust (SR.StaticClosure _) SR.Vanilla` is a no-op (same closure
ptr on stack).  `SR.StaticClosure` is also wired into `to_block_type`,
`to_var_type`, `to_string`, `drop`, `join`, and `adjust`.

#### Gotcha: mutually recursive declarations

The eager `compile_exp env pre_ae e` is only safe when all free variables of
the `FuncE` are already present in `pre_ae`.  In a mutually recursive
declaration group, `pre_ae` grows as each `dec` is processed by `go` in
`compile_decs_public`; a forward-referenced variable (e.g. another function
in the same `rec` block) is not yet in `pre_ae` when the earlier `dec` is
compiled.  Calling `compile_exp` prematurely causes `FuncDec.lit` →
`needs_capture pre_ae var` to hit `| None -> assert false`.

Fix: guard the special case with
`VarEnv.all_in_scope pre_ae (Freevars.captured e)` — only take the eager path
when every free variable of the `FuncE` is already bound in `pre_ae`.
Otherwise fall through to the regular deferred `fun ae -> …` path which
receives the fully-built `ae`.

Verified: `test/run/fmodf-forward.mo` passes; `nix build .#'tests.mo-idl'`
clean.  Both classical and enhanced backends done on branch `gabor/static-closure`.

### Option C — Linker (already done for the 0-forwarder subset)

The `zero_forwarder_target` optimization in `linkModule.ml` already collapses
call chains where a moc top-level function is a pure forwarder
(`i32/i64.const 0; local.get 1…n-1; Call k`). This catches the cases where
the compiler emits a 0-forwarder wrapper, but not the `static_closure +
call_indirect` case.

## Status

- Branch 3 (`_, Type.Local`) assertion: **in place** — confirms the invariant
  holds for subcase 1 (no `SR.Const (_, Const.Fun _)` leaks through).
- Subcase 2 (capturing closure with known `fi`): **DONE** in both backends on
  branch `gabor/static-closure`.  Mutually-recursive gotcha fixed with
  `VarEnv.all_in_scope` guard.
- Subcase 3 (higher-order passing): requires devirtualization — out of scope.
