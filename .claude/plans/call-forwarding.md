# Plan: Forwarding-Function Chase Optimisation in the Motoko Linker

## Motivation

The Motoko RTS exposes `fmod` (and other math functions) as thin `#[no_mangle]`
wrappers around the `libm` crate, e.g.:

```rust
#[no_mangle]
pub fn fmod(a: f64, b: f64) -> f64 {
    libm::fmod(a, b)
}
```

This wrapper is necessary because Rust mangles all names by default; the wrapper
provides a stable, predictable export name that `moc`-generated code can import as
`"rts".fmod`.  At runtime, every call to `fmod` goes through an extra Wasm function
boundary before reaching the real implementation.

Inspecting the compiled RTS Wasm confirms the two-level structure:

```
func[82]  <fmod>                                    12 bytes  ← wrapper
func[224] <libm::math::fmod::fmod::heab6209219ea8a9f>  472 bytes  ← real impl
```

The mangled name (`heab6209219ea8a9f`) is unstable across recompiles, so we cannot
hardcode it.  `wasm-opt` would eventually inline the wrapper, but only after linking.
Doing the elimination *inside* the linker removes the indirection before module
merging and reduces function-index pressure.

The same pattern applies to all the other math wrappers (`pow`, `sin`, `cos`, `exp`,
`log`, `tan`, `asin`, `acos`, `atan`, `atan2`).

---

## Architecture of the Existing Linker

File: `src/linking/linkModule.ml`

The relevant pipeline (lines ~998–1003):

1. `find_imports` — collects `("rts", name) → import_index` pairs from the
   `moc`-generated module.
2. `find_exports` — builds a `name → func_index` map from the RTS module's exports.
3. `resolve` — matches each import name against the exports map, producing an
   `(import_index, rts_func_index)` renumbering list.
4. The renumbering is applied during module merging so that `call import[i]` in the
   generated code becomes a direct call to the merged function.

The full Wasm AST is available: function bodies are `instr list` values (defined in
`src/wasm-exts/ast.ml`), so body inspection is cheap and already used in several
rename passes.

---

## Proposed Change: `chase_forwarders`

Insert a single step between `find_exports` and `resolve` that rewrites the exports
map so that any forwarding wrapper is replaced by its callee's index.

### Forwarding-function pattern

A function is a *pure forwarder* if:
- It has **no extra locals** (only the implicit parameter locals).
- Its body is exactly `LocalGet 0 ; LocalGet 1 ; … ; LocalGet (n-1) ; Call k`
  where `n` is the parameter count of its type.
- The callee `k` has the **same func type** (guaranteed by Rust's call convention
  for this pattern, but should be checked).

`fmod`'s wrapper (12 bytes) matches perfectly: two `LocalGet`s followed by one
`Call func[224]`.

### Implementation sketch

```ocaml
(* True iff the body is exactly: LocalGet 0 … LocalGet (n-1); Call k *)
let forwarding_target (funcs : func array) (types : type_ list)
    (import_count : int) (fi : int32) : int32 option =
  let idx = Int32.to_int fi - import_count in
  if idx < 0 then None  (* fi is itself an import, skip *)
  else
    let f = funcs.(idx) in
    let param_count =
      match (List.nth types (Int32.to_int f.it.ftype.it)).it with
      | FuncType (ps, _) -> List.length ps
    in
    let expected =
      List.init param_count (fun i ->
        LocalGet (Int32.of_int i @@ no_region) @@ no_region)
    in
    match f.it.locals, f.it.body with
    | [], body ->
      (match List.rev body with
       | { it = Call k; _ } :: rest when List.rev rest = expected ->
         Some k.it
       | _ -> None)
    | _ -> None

let chase_forwarders funcs types import_count (exports : exports) : exports =
  NameMap.map (fun fi ->
    match forwarding_target funcs types import_count fi with
    | Some fi' -> fi'   (* redirect to real implementation *)
    | None     -> fi
  ) exports
```

Apply iteratively until fixpoint to handle multi-hop chains (unlikely in practice,
but correct):

```ocaml
let rec fixpoint f x = let x' = f x in if x' = x then x else fixpoint f x'

let fun_exports2_chased =
  fixpoint
    (chase_forwarders
       (Array.of_list dm2.funcs)
       dm2.types
       (List.length (List.filter is_fun_import dm2.imports)))
    fun_exports2
```

Then use `fun_exports2_chased` instead of `fun_exports2` in the `resolve` call.

---

## Considerations

| Concern | Assessment |
|---|---|
| Hash instability of mangled names | Irrelevant — we chase by func index, not name |
| Type safety | Enforced: check callee type = wrapper type before chasing |
| Multi-hop chains | Fixpoint iteration handles them |
| Dead wrapper in output | wasm-opt DCE removes the now-unreferenced wrapper |
| wasm-opt already does this | Yes, but only post-link; linker-level is earlier and cleaner |
| Scope | ~20 lines in `linkModule.ml`, no interface change required |
| Affected functions | `fmod`, `pow`, `sin`, `cos`, `exp`, `log`, `tan`, `asin`, `acos`, `atan`, `atan2` |

---

## Files to Change

- `src/linking/linkModule.ml` — add `chase_forwarders` and `fixpoint`, apply to
  `fun_exports2` before the `resolve` step (~20 lines).

No changes needed to `linkModule.mli`, the codegen, or the RTS.

---

## Impedance: Why Motoko-Level Forwarders Are Not Chased

`chase_forwarders` operates on `fun_exports2` — the **RTS module's** exports.
It does not apply to the moc-generated module (`em1`).  Even if it did, Motoko
functions compiled by `moc` do **not** match the pure-forwarder pattern, for a
structural reason: every Motoko function receives an implicit **closure
pointer** as its first Wasm parameter (`$clos : i32`).

When a Motoko function `foo` forwards to `bar`, the generated body is:

```wat
(func $foo (param $clos i32) (param $a i32) (param $b i32) (param $c i32) (result i32)
  i32.const 0      ;; ← fresh null closure, NOT local.get $clos
  local.get $a
  local.get $b
  local.get $c
  call $bar)
```

The first argument to `$bar` is `i32.const 0` — a freshly synthesised null
closure — rather than `local.get 0` (the forwarded `$clos`).  The
`forwarding_target` pattern requires all arguments to be `local.get i` for
`i = 0 … n-1`, so this body is correctly rejected as a non-forwarder.

The same structural mismatch applies to `baz` (which additionally has
post-call work: unboxing + `f64.add`), confirming that both the closure
substitution and any trailing computation each independently disqualify a
function from the forwarder check.

### Consequence

`chase_forwarders` is the right tool **only** for the RTS shim layer, where
functions are plain `#[no_mangle]` Rust wrappers with no closure convention.
Eliminating Motoko-level call indirections requires a closure-aware extension
described below.

---

## Future Work: 0-Forwarder Chase for Motoko-Level Indirections

### Definition

A **0-forwarder** is a function whose body is exactly:

```wat
i32.const 0      ;; fresh null closure (not local.get $clos)
local.get 1
local.get 2
…
local.get n-1
call $k
```

i.e. it is a pure forwarder except that it substitutes a null closure for
the received `$clos`.  This is precisely what `moc` emits for a Motoko
function that simply delegates to another top-level function.

### Matching call sites

A call site **matches** a 0-forwarder `$foo` when it supplies `i32.const 0`
as the closure argument:

```wat
i32.const 0      ;; ← already the null closure $k will need
<a1>
<a2>
…
call $foo        ;; $foo is a 0-forwarder to $k
```

Because the call site already carries the correct closure value (`0`), the
intermediate `$foo` can be skipped: replace `call $foo` with `call $k`.  The
`i32.const 0` remains in place and becomes `$k`'s closure directly.  No
argument rewriting is required.

### Safety

The replacement is safe at any call site that supplies **any `i32.const k`**
as the closure argument — not just `i32.const 0`.

In practice, `moc` emits `i32.const 0` for calls to top-level named functions,
but for let-bound closure values it emits a **static closure object pointer**
(e.g. `i32.const 2097251`).  The worker function ignores its received `$clos`
and synthesises its own `i32.const 0` for the callee — so the caller's
constant is irrelevant to the callee's behaviour.

The condition is therefore: the closure argument at the call site must be a
compile-time constant (`i32.const k` for any `k`).  A call site that computes
the closure dynamically (loads it from memory, receives it as a parameter,
etc.) cannot be safely redirected without a full alias/escape analysis.

### Implementation sketch

1. **`zero_forwarder_target`** — variant of `forwarding_target` that accepts
   `i32.const 0; local.get 1 … n-1; call k` instead of `local.get 0 … n-1; call k`.
2. **Build a map** `zero_fwds : int32 → int32` over `em1`'s defined functions.
3. **Body rewriter** — scan every instruction list in `em1`; when a `call fi`
   is found where `fi ∈ zero_fwds` and the preceding instruction (at the right
   stack depth) is `i32.const 0`, replace `call fi` with `call zero_fwds[fi]`.
4. Run to **fixpoint** to handle chains (`foo → bar → quux`).
