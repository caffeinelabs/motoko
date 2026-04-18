# Plan: Update Wasm_exts to WebAssembly/spec 2.0.2

## Motivation

The `src/wasm-exts/` module is a fork of the WebAssembly spec interpreter's
OCaml modules. It has diverged significantly — upstream now covers GC, SIMD,
exceptions, tail calls, and multi-memory. Motoko doesn't need all of these,
but staying current avoids bitrot and unblocks future features (e.g. Wasm
exceptions for Motoko try/catch, tail calls for async).

## Strategy: Selective Port

We don't need to support every upstream instruction. The approach:

1. **Import the new type algebra** — this is unavoidable since everything
   depends on `types.ml`
2. **Import the expanded AST** — take all new constructors, even if Motoko
   doesn't emit them. This keeps the AST compatible with upstream tooling
   and allows round-tripping through `wasm-opt` without losing information
3. **constTrack / linkModule**: add `shift_and_evict` stubs for new
   instructions (correct depth tracking, no constant propagation)
4. **Codegen**: only emit the subset Motoko actually needs — no GC instructions,
   no SIMD. New instructions adopted one-by-one as features require them
5. **Re-graft Motoko extensions** onto the new structures

## Files to Update

### Phase 1: Type foundations

| File | Upstream name | Work |
|---|---|---|
| `types.ml` | `types.ml` | Full rewrite. New: `numtype`, `vectype`, `reftype`, `heaptype` (14+ variants), `comptype`, `subtype`, `rectype`, `deftype`, `tagtype`, `localtype`, `instrtype`. Old flat `value_type` becomes `numtype` subset. |
| `values.ml` | `value.ml` (renamed) | Add `V128` for SIMD values. Keep our `op` parameterised type but add the vector case. Rename file or keep as `values.ml` with a note. |

### Phase 2: AST and operators

| File | Upstream name | Work |
|---|---|---|
| `ast.ml` | `ast.ml` | Expand `instr'` from ~25 to 80+ constructors. Key groups: GC (15), SIMD (30), exceptions (3), tail calls (3), multi-memory (memory ops gain `memoryidx`), table ops (8), `Select` gains `valtype list option`. `func'`/`global'` change to constructor form. `module_'` gains `tags`, `elem` type changes. |
| `operators.ml` | `mnemonics.ml` (renamed) | Replace with upstream. Used for pretty-printing, not semantics. |

### Phase 3: Motoko-specific re-grafts

| Extension | Location | Work |
|---|---|---|
| `CustomModule` | `customModule.ml` | Update `module_'` references for new fields (`tags`, new `elem` type). |
| `Meta`/DWARF | `ast.ml` additions | Re-add `Meta` instruction variant to expanded `instr'`. |
| `StableMemory*` ops | `ast.ml` additions | Re-add stable memory instruction variants. |
| Passive segments | `ast.ml` | Verify `segment_mode` still compatible or update. |
| Encode/Decode | `customModuleEncode.ml`, `customModuleDecode.ml` | Update for new instruction opcodes, multi-byte prefixes (0xFB for GC, 0xFD for SIMD, 0x08/0x09 for exceptions). |

### Phase 4: Consumers

| Consumer | Work |
|---|---|
| `linkModule.ml` | Update all `instr'` pattern matches. Multi-memory: `Load`/`Store` etc. gain `memoryidx` arg. `Select` gains optional type. New constructors need wildcard or explicit handling. |
| `constTrack.ml` | Add cases for new instructions in `step`. Most are `shift_and_evict` with the right delta. SIMD vec ops: net -1 (binary), 0 (unary). GC struct/array ops: varies. Table ops: varies. Can bail (`None`) initially for complex ones. |
| Codegen (`compile_*.ml`) | Only update instruction construction that changed shape (e.g. `Load`/`Store` with `memoryidx`, `Select` with type). Don't emit new instructions yet. |
| `mo_ld.ml` | Same as linkModule — pattern match updates. |

## Breaking Changes Requiring Care

### Multi-memory (high impact syntactically, but semantically no-op)
Every `Load`, `Store`, `MemorySize`, `MemoryGrow`, `MemoryFill`, `MemoryCopy`,
`MemoryInit` gains a `memoryidx` parameter. Motoko uses memory 0 everywhere,
so all call sites get `0l @@ no_region` as the memory index. Mechanical but
pervasive — grep for every `Load`/`Store` construction.

Note: the IC does **not** support multi-memory via Wasm instructions.
Stable memory is implemented as a second memory under the hood, but accessed
via system calls (`ic0.stable*`), not Wasm memory instructions. The IC
runtime rewrites these for performance. So `memoryidx` will always be 0 in
Motoko-generated code — the parameter is purely for AST compatibility with
the spec, not for actual multi-memory use.

### Select (low impact)
`Select` becomes `Select of valtype list option`. Motoko emits untyped
`Select` — pass `None`. Pattern matches add the `_` or `None`.

### func'/global' constructors (medium impact)
`{ ftype; locals; body }` becomes `Func { ftype; locals; body }` (or similar).
Search-and-replace across codegen.

### elem segments (medium impact)
`var list segment list` becomes typed `elem` with `const list`. Affects
linking and module construction.

## What We Do NOT Need (initially)

- **GC instructions**: Motoko has its own GC. No `StructNew`/`ArrayNew` etc.
- **SIMD instructions**: No vectorisation in Motoko codegen.
- **Exception instructions**: Future work (Motoko try/catch). Take the AST
  constructors but don't emit.
- **Tail calls**: Future work (async optimisation). Same — take constructors,
  don't emit yet.

These exist in the AST for compatibility but the codegen doesn't touch them.

## Ordering and Dependencies

```
types.ml  →  values.ml  →  ast.ml  →  operators.ml
                                    ↓
                              customModule.ml
                              encode/decode
                                    ↓
                              linkModule.ml
                              constTrack.ml
                              codegen
```

`types.ml` must go first — everything depends on it.

## Risks

- **Upstream keeps moving**: Pin to a specific spec commit (tag `wasm-2.0`
  or the release commit for 2.0.2)
- **Encode/decode complexity**: Multi-byte prefixes for GC/SIMD opcodes
  add complexity to the binary format handlers
- **Test coverage**: Many tests match on compiler stderr/stdout — instruction
  format changes may require baseline updates
- **wasm-opt compatibility**: Ensure the version of `wasm-opt` in nixpkgs
  understands the new instruction set

## Estimate

| Phase | Effort |
|---|---|
| Phase 1 (types + values) | 2–3 days |
| Phase 2 (AST + operators) | 3–5 days |
| Phase 3 (re-grafts) | 2–3 days |
| Phase 4 (consumers) | 3–5 days |
| Testing + baseline updates | 2–3 days |
| **Total** | **~2–3 weeks** |

Can be done incrementally — Phase 1+2 as one PR, Phase 3+4 as another.
