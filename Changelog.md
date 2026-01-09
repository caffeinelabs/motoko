# Motoko compiler changelog


* motoko (`moc`)

  * Allow `break` and `continue` in loops without labels (#5702).

  * Print type constructors using available type paths (#5698).

  * Explain subtype failures (#5643).

  * Removes the Viper support (#5751).

  * Allows resolving local definitions for context-dot (#5731).

  Extends contextual-dot resolution to consider local definitions first, to make the following snippet type check. Local definitions take precedence over definitions that are in scope via modules.

  ```motoko
  func first<A>(self : [A]) : A {
    return self[0]
  };
  assert [1, 2, 3].first() == 1
  ```

  * Add privileged primitive for setting Candid type table cutoff (#5642).

  * Split unused identifier warnings into separate warnings for shared and non-shared contexts: `M0194` for general declarations, `M0240` for identifiers in shared pattern contexts (e.g. `c` in `shared({caller = c})`), `M0198` for unused fields in object patterns, and `M0241` for unused fields in shared patterns (e.g. `caller` in `shared({caller})`) (#5779).

## 1.0.0 (2025-12-11)

* motoko (`moc`)

  * Shorter, simpler error messages for generic functions (#5650).
    The compiler now tries to point to the first problematic expression in the function call, rather than the entire function call with type inference details.
    Simple errors only mention closed types; verbose errors with unsolved type variables are only shown when necessary.

  * Improved error messages for context dot: only the receiver type variables are solved, remaining type variables stay unsolved, not solved to `Any` or `Non` (#5634).

  * Fixed the type instantiation hint to have the correct arity (#5634).

  * Fix for #5618 (compiling dotted `await`s) (#5622).

  * Improved type inference of the record update syntax (#5625).

  * New flag `--error-recovery` to enable reporting of multiple syntax errors (#5632).

  * Improved solving and error messages for invariant type parameters (#5464).
    Error messages now include suggested type instantiations when there is no principal solution.

    Invariant type parameters with bounds like `Int  <:  U  <:  Any` (when the upper bound is `Any`) can now be solved by choosing the lower bound (`Int` here) when it has no proper supertypes other than `Any`.
    This means that when choosing between **exactly two** solutions: the lower bound and `Any` as the upper bound, the lower bound is chosen as the solution for the invariant type parameter (`U` here).
    Symmetrically, with bounds like `Non  <:  U  <:  Nat` (when the lower bound is `Non`), the upper bound (`Nat` here) is chosen when it has no proper subtypes other than `Non`.

    For example, the following code now compiles without explicit type arguments:

    ```motoko
    import VarArray "mo:core/VarArray";
    let varAr = [var 1, 2, 3];
    let result = VarArray.map(varAr, func x = x : Int);
    ```

    This compiles because the body of `func x = x : Int` has type `Int`, which implies that `Int  <:  U  <:  Any`.
    Since `Int` has no proper supertypes other than `Any`, it can be chosen as the solution for the invariant type parameter `U`, resulting in the output type `[var Int]`.

    However, note that if the function body was of type `Nat`, it would not compile, because `Nat` is a proper subtype of `Int` (`Nat <: Int`).
    In this case, there would be no principal solution with `Nat  <:  U  <:  Any`, and the error message would suggest:

    ```
    Hint: Add explicit type instantiation, e.g. <Nat, Nat>
    ```

    The suggested type instantiation can be used to fix the code:

    ```motoko
    let result = VarArray.map<Nat, Nat>(varAr, func x = x);
    ```

    Note that the error message suggests only one possible solution, but there may be alternatives. In the example above, `<Nat, Int>` would also be a valid instantiation.

  * Fixes type inference of deferred funcs that use `return` in their body (#5615).
    Avoids confusing errors like `Bool does not have expected type T` on `return` expressions. Should type check successfully now.

  * Add warning `M0239` that warns when binding a unit `()` value by `let` or `var` (#5599).

  * Use `self` parameter, not `Self` type, to enable contextual dot notation (#5574).

  * Add (caffeine) warning `M0237` (#5588).
    Warns if explicit argument could have been inferred and omitted,
    e.g. `a.sort(Nat.compare)` vs `a.sort()`.
    (allowed by default, warn with `-W 0237`).

  * Add (caffeine) warning `M0236` (#5584).
    Warns if contextual dot notation could have been used,
    e.g. `Map.filter(map, ...)` vs `map.filter(...)`.
    Does not warn for binary `M.equals(e1, e2)` or `M.compareXXX(e1, e2)`.
    (allowed by default, warn with `-W 0236`).

  * Add (caffeine) deprecation code `M0235` (#5583).
    Deprecates any public types and values with special doc comment
    `/// @deprecated M0235`.
    (allowed by default, warn with `-W 0235`).

  * Experimental support for `implicit` argument declarations (#5517).

  * Experimental support for Mixins (#5459).

  * bugfix: importing of `blob:file:` URLs in subdirectories should work now (#5507, #5569).

  * bugfix: escape `composite_query` fields on the Candid side, as it is a keyword (#5617).

  * bugfix: implement Candid spec improvements (#5504, #5543, #5505).
    May now cause rejection of certain type-incorrect Candid messages that were accepted before.

## 0.16.3 (2025-09-29)

* motoko (`moc`)

  * Added `Prim.getSelfPrincipal<system>() : Principal` to get the principal of the current actor (#5518).

  * Contextual dot notation (#5458):

    Writing `e0.f(<Ts>)(e1,...,en)` is short-hand for `M.f(<Ts>)(e0, e1, ..., en)`, provided:

    * `f` is not already a field or special member of `e0`.
    * There is a unique module `M` with member `f` in the context that declares:
      * a public type `Self<T1,..., Tn>` that can be instantiated to the type of `e0`;
      * a public field `f` that accepts `(e0, e2, ..., en)` as arguments.

  * Added an optional warning for **redundant type instantiations** in generic function calls (#5468).
    Note that this warning is off by default.
    It can be enabled with the `-W M0223` flag.

  * Added `-A` and `-W` flags to disable and enable warnings given their message codes (#5496).

    For example, to disable the warning for redundant `stable` keyword, use `-A M0217`.
    To enable the warning for redundant type instantiations, use `-W M0223`.
    Multiple warnings can be disabled or enabled by comma-separating the message codes, e.g. `-A M0217,M0218`.
    Both flags can be used multiple times.

  * Added `-E` flag to treat specified warnings as errors given their message codes (#5502).

  * Added `--warn-help` flag to show available warning codes, current lint level (**A**llowed, **W**arn or **E**rror), and descriptions (#5502).

  * `moc.js` : Added `setExtraFlags` method for passing some of the `moc` flags (#5506).

## 0.16.2 (2025-09-12)

* motoko (`moc`)

  * Added primitives to access canister environment variables (#5443):

    ```motoko
    Prim.envVarNames : <system>() -> [Text]
    Prim.envVar : <system>(name : Text) -> ?Text
    ```

    These require `system` capability to prevent supply-chain attacks.

  * Added ability to import `Blob`s from the local file system by means of the `blob:file:` URI scheme (#4935).

## 0.16.1 (2025-08-25)

* motoko (`moc`)

  * bugfix: fix compile-time exception showing `???` type when using 'improved type inference' (#5423).

  * Allow inference of invariant type parameters, but only when the bound/solution is an 'isolated' type (meaning it has no proper subtypes nor supertypes other than `Any`/`None`) (#5359).
    This addresses the limitation mentioned in #5180.
    Examples of isolated types include all primitive types except `Nat` and `Int`, such as `Bool`, `Text`, `Blob`, `Float`, `Char`, `Int32`, etc.
    `Nat` and `Int` are not isolated because `Nat` is a subtype of `Int` (`Nat <: Int`).

    For example, the following code now works without explicit type arguments:

    ```motoko
    import VarArray "mo:core/VarArray";
    let varAr = [var 1, 2, 3];
    let result = VarArray.map(varAr, func x = debug_show (x) # "!"); // [var Text]
    ```

  * `ignore` now warns when its argument has type `async*`, as it will have no effect (#5419).

  * bugfix: fix rare compiler crash when using a label and identifier of the same name in the same scope (#5283, #5412).

  * bugfix: `moc` now warns about parentheticals on `async*` calls, and makes sure that they get discarded (#5415).

## 0.16.0 (2025-08-19)

* motoko (`moc`)

  * Breaking change: add new type constructor `weak T` for constructing weak references.

    ```motoko
        Prim.allocWeakRef: <T>(value : T) -> weak T
        Prim.weakGet: <T>weak T -> ?(value : T)
        Prim.isLive: weak Any -> Bool
    ```

    A weak reference can only be allocated from a value whose type representation is always a heap reference; `allowWeakRef` will trap on values of other types.
    A weak reference does not count as a reference to its value and allows the collector to collect the value once no other references to it remain.
    `weakGet` will return `null`, and `isLive` will return false once the value of the reference has been collected by the garbage collector.
    The type constructor `weak T` is covariant.

    Weak reference operations are only supported with --enhanced-orthogonal-persistence and cannot be used with the classic compiler.

  * bugfix: the EOP dynamic stable compatibility check incorrectly rejected upgrades from `Null` to `?T` (#5404).

  * More explanatory upgrade error messages with detailing of cause (#5391).

  * Improved type inference for calling generic functions (#5180).
    This means that type arguments can be omitted when calling generic functions in _most common cases_.
    For example:

    ```motoko
    let ar = [1, 2, 3];
    Array.map(ar, func x = x * 2);  // Needs no explicit type arguments anymore!
    ```

    Previously, type arguments were usually required when there was an anonymous not annotated function in arguments.
    The reason being that the type inference algorithm cannot infer the type of `func`s in general,
    e.g. `func x = x * 2` cannot be inferred without knowing the type of `x`.

    Now, the improved type inference can handle such `func`s when there is enough type information from other arguments.
    It works by splitting the type inference into two phases:
    1. In the first phase, it infers part of the instantiation from the non-`func` arguments.
       The goal is to infer all parameters of the `func` arguments, e.g. `x` in the example above.
       The `ar` argument in the example above is used to infer the partial instantiation `Array.map<Nat, O>`, leaving the second type argument `O` to be inferred in the second phase.
       With this partial instantiation, it knows that `x : Nat`.
    2. In the second phase, it completes the instantiation by inferring the bodies of the `func` arguments; assuming that all parameters were inferred in the first phase.
       In the example above, it knows that `x : Nat`, so inferring the body `x * 2` will infer the type `O` to be `Nat`.
       With this, the full instantiation `Array.map<Nat, Nat>` is inferred, and the type arguments can be omitted.

    Limitations:
    - Invariant type parameters must be explicitly provided in most cases.
      e.g. `VarArray.map` must have the return type annotation:
      ```motoko
      let result = VarArray.map<Nat, Nat>(varAr, func x = x * 2);
      ```
      Or the type of the result must be explicitly provided:
      ```motoko
      let result : [var Nat] = VarArray.map(varAr, func x = x * 2);
      ```

    - When there is not enough type information from the non-`func` arguments, the type inference will not be able to infer the `func` arguments.
      However this is not a problem in most cases.

## 0.15.1 (2025-07-30)

* motoko (`moc`)

  * bugfix: `persistent` imported actor classes incorrectly rejected as non-`persistent` (#5667).

  * Allow matching type fields of modules and objects in patterns (#5056)
    This allows importing a type from a module without requiring an indirection or extra binding.

    ```
    // What previously required indirection, ...
    import Result "mo:core/Result";
    type MyResult<Ok> = Result.Result<Ok, Text>;

    // or rebinding, ...
    import Result "mo:core/Result";
    type Result<Ok, Err> = Result.Result<Ok, Err>;
    type MyResult<Ok> = Result<Ok, Text>;

    // can now be written more concisely as:
    import { type Result } "mo:core/Result";
    type MyResult<Ok> = Result<Ok, Text>;
    ```

## 0.15.0 (2025-07-25)

* motoko (`moc`)

  * Breaking change: the `persistent` keyword is now required on actors and actor classes (#5320, #5298).
    This is a transitional restriction to force users to declare transient declarations as `transient` and actor/actor classes as `persistent`.
    New error messages and warnings will iteratively guide you to insert `transient` and `persistent` as required, after which any `stable` keywords can be removed. Use the force.

    In the near future, the `persistent` keyword will be made optional again, and `let` and `var` declarations within actor and actor classes will be `stable` (by default) unless declared `transient`, inverting the previous default for non-`persistent` actors.
    The goal of this song and dance is to *always* default actor declarations to stable unless declared `transient` and make the `persistent` keyword redundant.

  * Breaking change: enhanced orthogonal persistence is now the default compilation mode for `moc` (#5305).
    Flag `--enhanced-orthogonal-persistence` is on by default.
    Users not willing or able to migrate their code can opt in to the behavior of moc prior to this release with the new flag `--legacy-persistence`.
    Flag `--legacy-persistence` is required to select the legacy `--copying-gc` (the previous default), `--compacting-gc`,  or `generational-gc`.

    As a safeguard, to protect users from unwittingly, and irreversibly, upgrading from legacy to enhanced orthogonal persistence, such upgrades will fail unless the new code is compiled with flag `--enhanced-orthogonal-persistence` explicitly set.
    New projects should not require the flag at all (#5308) and will simply adopt enhanced mode.

    To recap, enhanced orthogonal persistence implements scalable and efficient orthogonal persistence (stable variables) for Motoko:
    * The Wasm main memory (heap) is retained on upgrade with new program versions directly picking up this state.
    * The Wasm main memory has been extended to 64-bit to scale as large as stable memory in the future.
    * The runtime system checks that data changes of new program versions are compatible with the old state.

    Implications:
    * Upgrades become extremely fast, only depending on the number of types, not on the number of heap objects.
    * Upgrades will no longer hit the IC instruction limit, even for maximum heap usage.
    * The change to 64-bit increases the memory demand on the heap, in worst case by a factor of two.
    * For step-wise release handling, the IC initially only offers a limited capacity of the 64-bit space (e.g. 4GB or 6GB), that will be gradually increased in future to the capacity of stable memory.
    * There is moderate performance regression of around 10% for normal execution due to combined related features (precise tagging, change to incremental GC, and handling of compile-time-known data).
    * The garbage collector is fixed to incremental GC and cannot be chosen.
    * `Float.format(#hex prec, x)` is no longer supported (expected to be very rarely used in practice).
    * The debug print format of `NaN` changes (originally `nan`).

  * Fixed file indices in the DWARF encoding of the `debug_line` section. This change is only relevant when using the `-g` flag (#5281).

  * Improved large array behavior under the incremental GC (#5314)

## 0.14.14 (2025-06-30)

* motoko (`moc`)

  * Lazy WASM imports: avoids unnecessary function imports from the runtime, improving compatibility with more runtime versions (#5276).

  * Improved stable compatibility error messages to be more concise and clear during canister upgrades (#5271).

## 0.14.13 (2025-06-17)

* motoko (`moc`)

  * Introduce `await?` to synchronize `async` futures, avoiding the commit point when already fulfilled (#5215).

  * Adds a `Prim.Array_tabulateVar` function, that allows faster initialization of mutable arrays (#5256).

  * optimization: accelerate IR type checking with caching of sub, lub and check_typ tests (#5260).
    Reduces need for `-no-check-ir` flag.

## 0.14.12 (2025-06-12)

  * Added the `rootKey` primitive (#4994).

  * optimization: for `--enhanced-orthogonal-persistence`, reduce code-size and compile-time by sharing more static allocations (#5233, #5242).

  * bugfix: fix `-fshared-code` bug (#5230).

  * bugfix: avoid stack overflow and reduce code complexity for large EOP canisters (#5218).

## 0.14.11 (2025-05-16)

* motoko (`moc`)

  * Enhance syntax error messages with examples and support _find-references_ and _go-to-definition_
    functionality for fields in the language server (Serokell, Milestone-3) (#5076).

  * bugfix: `mo-doc` now correctly extracts record-patterned function arguments (#5128).

## 0.14.10 (2025-05-12)

* motoko (`moc`)

  * Added new primitives for cost calculation:
    `costCall`, `costCreateCanister`, `costHttpRequest`, `costSignWithEcdsa`, `costSignWithSchnorr` (#5001).

## 0.14.9 (2025-04-25)

* motoko (`moc`)

  * Added new primitives for exploding fixed-width numbers to bytes:
    `explodeNat16`, `explodeInt16`, `explodeNat32`, `explodeInt32`, `explodeNat64`, `explodeInt64` (#5057).

## 0.14.8 (2025-04-17)

* motoko (`moc`)

  * Add random-access indexing to `Blob`, support special methods `get` and `keys` (#5018).

  * Officializing **enhanced orthogonal persistence** (EOP) after a successful beta testing phase (#5035).

    EOP needs to be explicitly enabled by the `--enhanced-orthogonal-persistence` compiler flag or via `args` in `dfx.json`:
    ```
      "type" : "motoko"
      ...
      "args" : "--enhanced-orthogonal-persistence"
    ```

  * Add support for parser error recovery to improve LSP (Serokell, Milestone-2) (#4959).

  * We now provide a proper `motoko-mode` for `emacs` (#5043).

  * bugfix: Avoid generating new Candid `type`s arising from equal homonymous Motoko `type` (if possible)
    in service definitions (#4309, #5013).

  * bugfix: Provide a more consistent framework for dealing with internally generated type indentifiers,
    fixing caching bugs, e.g. in the VSCode plugin (#5055).

## 0.14.7 (2025-04-04)

* motoko (`moc`)

  * Preserve and infer named types both to improve displayed types in error messages, and to preserve function signatures when deriving Candid types (#4943).
    The names remain semantically insignificant and are ignored when comparing types for subtyping and equality.

    For example,
    ``` motoko
    func add(x : Int, y : Int) : (res : Int) = x + y;
    ```
    now has inferred type:
    ``` motoko
    (x : Int, y: Int) -> (res : Int)
    ```
    Previously, the type would be inferred as:
    ``` motoko
    (Int, Int) -> Int
    ```

  * Refine the `*.most` stable signature file format to distinguish stable variables that are strictly required by the migration function rather than propagated from the actor body (#4991).
    This enables the stable compatibility check to verify that a migration function will not fail due to missing required fields.
    Required fields are declared `in`, not `stable`, in the actor's pre-signature.

  * Added improved LSP cache for typechecking (thanks to Serokell) (#4931).

  * Reduce enhanced-orthogonal-persistence memory requirements using incremental allocation within partitions (#4979).

* motoko-base

  * Deprecated `ExperimentalCycles.add`, use a parenthetical `(with cycles = <amount>) <send>` instead (dfinity/motoko-base#703).

## 0.14.6 (2025-04-01)

* motoko (`moc`)

  * To prevent implicit data loss due to upgrades, stable fields may no longer be dropped or promoted to lossy supertypes (#4970).
    Removing a stable variable, or promoting its type to a lossy supertype by, for example, dropping nested record fields,
    now requires an explicit migration expression.
    Promotion to non-lossy supertypes, such as `Nat` to `Int` or `{#version1}` to `{#version1; #version2}`, is still supported.

  * Now we detect (and warn for) fields in literal objects and record extensions,
    (as well as `public` fields or types in `object` and `class`) that are inaccessible
    due to a user-specified type constraint (#4978, #4981).

  * We now provide release artefacts for `Darwin-arm64` and `Linux-aarch64` platforms (#4952).

## 0.14.5 (2025-03-25)

* motoko (`moc`)

  * Performance improvements to the default timer mechanism (#3872, #4967).

* documentation (`mo-doc`)

  * Changed extracted `let` bindings with manifest function type to appear as `func`s (#4963).

## 0.14.4 (2025-03-18)

* motoko (`moc`)

  * Added `canisterSubnet` primitive (#4857).

* motoko-base

  * Added `burn : <system>Nat -> Nat` to `ExperimentalCycles` (dfinity/motoko-base#699).

  * Added `ExperimentalInternetComputer.subnet` (dfinity/motoko-base#700).

## 0.14.3 (2025-03-04)

* motoko (`moc`)

  * Added primitive predicate `isReplicatedExecution` (#4929).

* motoko-base

  * Added `isRetryPossible : Error -> Bool` to `Error` (dfinity/motoko-base⁠#692).

  * Made `ExperimentalInternetComputer.replyDeadline` to return
    an optional return type (dfinity/motoko-base⁠#693).
    _Caveat_: Breaking change (minor).

  * Added `isReplicated : () -> Bool` to `ExperimentalInternetComputer` (dfinity/motoko-base#694).

## 0.14.2 (2025-02-26)

* motoko (`moc`)

  * Added support for sending `cycles` and setting a `timeout` in parentheticals.
    This is an **experimental feature**, with new syntax, and now also allowing best-effort
    message sends. The legacy call `Cycles.add<system>` is still supported (#4608).

    For example, if one wants to attach cycles to a message send, one can prefix it with a parenthetical
    ``` motoko
    (with cycles = 5_000) Coins.mine(forMe);
    ```
    Similarly a timeout for _best-effort_ execution (also called _bounded-wait_) can be specified like
    ``` motoko
    let worker = (with timeout = 15) async { /* worker loop */ };
    ```
    A common base for fields optionally goes before the `with` and can be customised with both fields
    after it. Please consult the documentation for more usage information.

  * bugfix: `mo-doc` will now generate documentation for `actor`s and `actor class`es (#4905).

  * bugfix: Error messages now won't suggest privileged/internal names (#4916).

## 0.14.1 (2025-02-13)

* motoko (`moc`)

  * bugfix: Be more precise when reporting type errors in `migration` fields (#4888).

## 0.14.0 (2025-02-05)

* motoko (`moc`)

  * Add `.values()` as an alias to `.vals()` for arrays and `Blob`s (#4876).

  * Support explicit, safe migration of persistent data allowing arbitrary
    transformations on a selected subset of stable variables.
    Additional static checks warn against possible data loss (#4812).

    As a very simple example:
    ``` motoko
    import Nat32 "mo:base/Nat32";

    (with migration =
      func (old : { var size : Nat32 }) : { var length : Nat } =
        { var length = Nat32.toNat(old.size) }
    )
    persistent actor {
      var length : Nat = 0;
    }
    ```
    may be used during an upgrade to rename the stable field `size` to `length`,
    and change its type from `Nat32` to `Nat`.

    See the documentation for full details.

## 0.13.7 (2025-02-03)

* motoko (`moc`)

  * Support passing cycles in primitive `call_raw` (resp. `ExperimentalInternetComputer.call`) (#4868).

## 0.13.6 (2025-01-21)

* motoko (`moc`)

  * Support the low Wasm memory hook: `system func lowmemory() : async* () { ... }` (#4849).

  * Breaking change (minor) (#4854):

    * For enhanced orthogonal persistence: The Wasm persistence modes used internally for canister upgrades have been changed to lower case names,
      `keep` and `replace` and instead of `Keep` and `Replace`:

      If using actor class instances with enhanced orthogonal persistence, you would need to recompile the program and upgrade with latest `moc` and `dfx`.
      Otherwise, no action is needed.

  * bugfix: Checks and mitigations that timer servicing works (#4846).

  * bugfix: Some valid upgrades deleting a stable variable could fail the `--enhanced-orthogonal-persistence` stable compatibility check due to a bug (#4855).

## 0.13.5 (2024-12-06)

* motoko (`moc`)

  * Breaking change (minor) (#4786):

    * Add new keyword `transient` with exactly the same meaning as the old keyword `flexible` (but a more familiar reading).

    * Add keyword `persistent`.

      When used to modify the `actor` keyword in an actor or actor class definition, the keyword declares that the default stability of a
      `let` or `var` declaration is `stable` (not `flexible` or `transient`).

      For example, a stateful counter can now be declared as:
      ``` motoko
      persistent actor {
        // counts increments since last upgrade
        transient var invocations = 0;

        // counts increments since first installation
        var value = 0; 	// implicitly `stable`

        public func inc() : async () {
          value += 1;
          invocations += 1;
        }
      }
      ```
      On upgrade, the transient variable `invocations` will be reset to `0` and `value`, now implicitly `stable`, will retain its current value.

      Legacy actors and classes declared without the `persistent` keyword have the same semantics as before.

  * Added new primitive `replyDeadline : () -> Nat64` to obtain when a response for a best-effort message is due (#4795).

  * bugfix: fail-fast by limiting subtyping depth to avoid reliance on unpredictable stack overflow (#3057, #4798).

* motoko-base

  * Added `Text.fromList` and `Text.toList` functions (dfinity/motoko-base#676).

  * Added `Text.fromArray/fromVarArray` functions (dfinity/motoko-base#674).

  * Added `replyDeadline` to `ExperimentalInternetComputer` (dfinity/motoko-base⁠#677).

## 0.13.4 (2024-11-29)

* motoko (`moc`)

  * refactoring: Updating and simplifying the runtime system dependencies (#4677).

* motoko-base

  * Breaking change (minor): `Float.format(#hex)` is no longer supported.
    This is because newer versions of Motoko (such as with enhanced orthogonal persistence)
    rely on the Rust-native formatter that does not offer this functionality.
    It is expected that this formatter is very rarely used in practice (dfinity/motoko-base⁠#589).

  * Formatter change (minor): The text formatting of `NaN`, positive or negative,
    will be `NaN` in newer Motoko versions, while it was `nan` or `-nan` in older versions (dfinity/motoko-base⁠#589).

## 0.13.3 (2024-11-13)

* motoko (`moc`)

  * typing: suggest conversions between primitive types from imported libraries
    and, with `--ai-errors`, all available package libraries (#4747).

* motoko-base

  * Add modules `OrderedMap` and `OrderedSet` to replace `RBTree` with improved functionality, performance
    and ergonomics avoiding the need for preupgrade hooks (thanks to Serokell) (dfinity/motoko-base⁠#662).

## 0.13.2 (2024-10-18)

* motoko (`moc`)

  * Made the `actor`'s _self_ identifier available in the toplevel block. This also allows using
    functions that refer to _self_ from the initialiser (e.g. calls to `setTimer`) (#4719).

  * bugfix: `actor <exp>` now correctly performs definedness tracking (#4731).

## 0.13.1 (2024-10-07)

* motoko (`moc`)

  * Improved error messages for unbound identifiers and fields that avoid reporting large types and use an edit-distance based metric to suggest alternatives (#4720).

  * Flag `--ai-errors` to tailor error messages to AI clients (#4720).

  * Compilation units containing leading type definitions are now rejected with an improved error message (#4714).

  * bugfix: `floatToInt64` now behaves correctly in the interpreter too (#4712).

## 0.13.0 (2024-09-17)

* motoko (`moc`)

  * Added a new primitive `cyclesBurn : <system> Nat -> Nat` for burning the canister's cycles
    programmatically (#4690).

  * **For beta testing:** Support __enhanced orthogonal persistence__, enabled with new `moc` flag `--enhanced-orthogonal-persistence` (#4193).

    This implements scalable and efficient orthogonal persistence (stable variables) for Motoko:
    * The Wasm main memory (heap) is retained on upgrade with new program versions directly picking up this state.
    * The Wasm main memory has been extended to 64-bit to scale as large as stable memory in the future.
    * The runtime system checks that data changes of new program versions are compatible with the old state.

    Implications:
    * Upgrades become extremely fast, only depending on the number of types, not on the number of heap objects.
    * Upgrades will no longer hit the IC instruction limit, even for maximum heap usage.
    * The change to 64-bit increases the memory demand on the heap, in worst case by a factor of two.
    * For step-wise release handling, the IC initially only offers a limited capacity of the 64-bit space (e.g. 4GB or 6GB), that will be gradually increased in future to the capacity of stable memory.
    * There is moderate performance regression of around 10% for normal execution due to combined related features (precise tagging, change to incremental GC, and handling of compile-time-known data).
    * The garbage collector is fixed to incremental GC and cannot be chosen.
    * `Float.format(#hex prec, x)` is no longer supported (expected to be very rarely used in practice).
    * The debug print format of `NaN` changes (originally `nan`).

    To activate enhanced orthogonal persistence under `dfx`, the following command-line argument needs to be specified in `dfx.json`:

    ```
    ...
      "type" : "motoko"
      ...
      "args" : "--enhanced-orthogonal-persistence"
    ...
    ```
    BREAKING CHANGE (Minor): changes some aspects of `Float` formatting.

    For more information, see:
    * The Motoko design documentation `design/OrthogonalPersistence.md`
    * The Motoko user documentation `doc/md/canister-maintenance/upgrades.md`.

  * Candid decoding: impose an upper limit on the number of values decoded or skipped in a single candid payload,
    as a linear function, `max_values`, of binary payload size.

    ```
      max_values(blob) = (blob.size() * numerator)/denominator + bias
    ```

    The current default settings are `{numerator = 1; denominator = 1; bias = 1024 }`, allowing a maximum
    of 1024 values plus one additional value per byte in the payload.

    While hopefully not required, the constant factors can be read/modified using system functions:
    *  Prim.setCandidLimits: `<system>{numerator : Nat32;  denominator : Nat32; bias : Nat32 } -> ()`
    *  Prim.getCandidLimits: `<system>() -> {numerator : Nat32; denominator : Nat32; bias : Nat32 }`

## 0.12.1 (2024-08-08)

* motoko (`moc`)

  * Added a new command-line flag `--print-source-on-error` to print source code context on error (#4650).

  * debugging: `__motoko_runtime_information()` as privileged query for runtime statistics (#4635).

    Exposing a privileged system-level query function `__motoko_runtime_information()` 
    that reports the current runtime statistics of the canister, such as the heap size, 
    the total number of allocated objects, the total amount of reclaimed memory and more.
    This is useful because several statistics of the reported information cannot be 
    inspected on the IC replica dashboard as they are internal to the Motoko runtime system. 
    This query is only authorized to the canister controllers and self-calls of the canister.

    ``` Motoko
    __motoko_runtime_information : () -> {
        compilerVersion : Text;
        rtsVersion : Text;
        garbageCollector : Text;
        sanityChecks : Nat;
        memorySize : Nat;
        heapSize : Nat;
        totalAllocation : Nat;
        reclaimed : Nat;
        maxLiveSize : Nat;
        stableMemorySize : Nat;
        logicalStableMemorySize : Nat;
        maxStackSize : Nat;
        callbackTableCount : Nat;
        callbackTableSize : Nat;
    }
    ```

* motoko-base

  * Added `Iter.concat` function (thanks to AndyGura) (dfinity/motoko-base⁠#650).

## 0.12.0 (2024-07-26)

* motoko (`moc`)

  * feat: `finally` clauses for `try` expressions (#4507).

    A trailing `finally` clause to `try`/`catch` expressions facilitates structured
    resource deallocation (e.g. acquired locks, etc.) and similar cleanups in the
    presence of control-flow expressions (`return`, `break`, `continue`, `throw`).
    Additionally, in presence of `finally` the `catch` clause becomes optional and
    and any uncaught error from the `try` block will be propagated, after executing the `finally` block.

    _Note_: `finally`-expressions that are in scope will be executed even if an execution
    path _following_ an `await`-expression traps. This feature, formerly not available in Motoko,
    allows programmers to implement cleanups even in the presence of traps. For trapping 
    execution paths prior to any `await`, the replica-provided state roll-back mechanism 
    ensures that no cleanup is required.

    The relevant security best practices are accessible at
    https://internetcomputer.org/docs/current/developer-docs/security/security-best-practices/inter-canister-calls#recommendation

    BREAKING CHANGE (Minor): `finally` is now a reserved keyword,
    programs using this identifier will break.

  * bugfix: `mo-doc` will now generate correct entries for `public` variables (#4626).

## 0.11.3 (2024-07-16)

* motoko (`moc`)

  * feat: `motoko-san` contributions by Serokell. Now able to verify some simple but non-trivial actors
    (thanks to the entire Serokell team) (#4500).

  * bugfix: Corrects the interpreter (and compiler) to recognise certain type parameters as callable function types (#4617).

## 0.11.2 (2024-07-06)

* motoko (`moc`)

  * deprecation: Deprecate the use of base library's `ExperimentalStableMemory` (ESM) (#4573).
    New `moc` flag `--experimental-stable-memory <n>` controls the level of deprecation:
    * n < 0: error on use of stable memory primitives.
    * n = 0: warn on use of stable memory primitives.
    * n > 1: warning-less use of stable memory primitives (for legacy applications).
    Users of ESM should consider migrating their code to use isolated regions (library `Region.mo`) instead.

  * bugfix: Fix the detection of unused declarations in `switch` and `catch` alternatives (#4560).

  * improvement: Only warn on unused identifiers if type checking is error-free (#4561).

## 0.11.1 (2024-03-15)

* motoko (`moc`)

  * feat: Custom error message for unused, punned field bindings (#4454).

  * feat: Don't report top-level identifiers as unused (#4452).

  * bugfix: Declaring `<system, ...>` capability on a class enables system capabilities in its body (#4449).

  * bugfix: Fix crash compiling actor reference containing an `await` (#4418, #4450).

  * bugfix: Fix crash when compiling with flag `--experimental-rtti` (#4434).

## 0.11.0 (2024-03-05)

* motoko (`moc`)

  * Warn on detection of unused identifiers (code `M0194`) (#4377).

    - By design, warnings are not emitted for code imported from a package.
    - A warning can be suppressed by replacing the identifier entirely by a wildcard `_`,
      or by prefixing it with an `_`, e.g. replace `x` by `_x`.

    **Limitations**: recursive and mutually recursive definitions are considered used,
    even if never referenced outside the recursive definition.

  * Remove `__get_candid_interface_tmp_hack` endpoint. Candid interface is already stored as canister metadata, this temporary endpoint is redundant, thus removed. (#4386)

  * Improved capability system, introducing a synchronous (`system`) capability (#4406).

    `actor` initialisation body, `pre`/`postupgrade` hooks, `async` function bodies (and
    blocks) possess this capability. Functions (and classes) can demand it by prepending `system`
    to the type argument list. The capability can be forwarded in calls by mentioning `<system, …>`
    in the instantiation parameter list.

    BREAKING CHANGE (Minor): A few built-in functions have been marked with demand
    for the `system` capability. In order to call these, the full call hierarchy needs to be
    adapted to pass the `system` capability.

  * Introduced the feature for precise tagging of scalar values (#4369).

    Controlled by flag `--experimental-rtti` (off by default). Minor performance changes for
    arithmetic expected. We advise to only turn on the feature for testing, as currently no
    productive upsides exist (though future improvements will depend on it), and performance
    of arithmetic will degrade somewhat. See the PR for the whole picture.

* motoko-base

  * Added `Option.equal` function (thanks to ByronBecker) (dfinity/motoko-base⁠#615).

## 0.10.4 (2024-01-10)

* motoko (`moc`)

  * Officializing the new **incremental garbage collector** after a successful beta testing phase.
    The incremental GC can be enabled by the `moc` flag `--incremental-gc` (#3837) and is designed to scale for large program heap sizes.

    **Note**: While resolving scalability issues with regard to the instruction limit of the GC work, it is now possible to hit other scalability limits:
    - _Out of memory_: A program can run out of memory if it fills the entire memory space with live objects.
    - _Upgrade limits_: When using stable variables, the current mechanism of serialization and deserialization to and from stable memory can exceed the instruction limit or run out of memory.

    **Recommendations**:
    - _Test the upgrade_: Thoroughly test the upgrade mechanism for different data volumes and heap sizes and conservatively determine the amount of stable data that is supported when upgrading the program.
    - _Monitor the heap size_: Monitor the memory and heap size (`Prim.rts_memory_size()` and `Prim.rts_heap_size()`) of the application in production.
    - _Limit the heap size_: Implement a custom limit in the application to keep the heap size and data volume below the scalability limit that has been determined during testing, in particular for the upgrade mechanism.
    - _Avoid large allocations per message_: Avoid large allocations of 100 MB or more per message, but rather distribute larger allocations across multiple messages. Large allocations per message extend the duration of the GC increment. Moreover, memory pressure may occur because the GC has a higher reclamation latency than a classical stop-the-world collector.
    - _Consider a backup query function_: Depending on the application case, it can be beneficial to offer an privileged _query_ function to extract the critical canister state in several chunks. The runtime system maintains an extra memory reserve for query functions. Of course, such a function has to be implemented with a check that restricts it to authorized callers only. It is also important to test this function well. 
    - _Last resort if memory would be full_: Assuming the memory is full with objects that have shortly become garbage before the memory space has been exhausted, the canister owner or controllers can call the system-level function `__motoko_gc_trigger()` multiple times to run extra GC increments and complete a GC run, for collecting the latest garbage in a full heap. Up to 100 calls of this function may be needed to complete a GC run in a 4GB memory space. The GC keeps an specific memory reserve to be able to perform its work even if the application has exhausted the memory. Usually, this functionality is not needed in practice but is only useful in such exceptional cases.

  * Allow type annotations on free-standing `object`/`module`/`actor` blocks, in order to perform a conformity check with an interface type (#4324).

## 0.10.3 (2023-12-20)

* motoko (`moc`)

  * Include doc comments to Candid interfaces generated via the `--idl` flag (#4334).

  * bugfix: fix broken implementations of `Region.loadNat32`, `Region.storeNat32`, `Region.loadInt32`, `Region.storeInt32` (#4335).
    Values previously stored with the broken 32-bit operations must be loaded with care.
    If bit 0 is clear, the original value can be obtained by an arithmetic shift right by 1 bit.
    If bit 0 is set, the value cannot be trusted and should be ignored
    (it encodes some transient address of a boxed value).

* motoko-base

  * Added `ExperimentalInternetComputer.performanceCounter` function to get the raw performance
    counters (dfinity/motoko-base⁠#600).

  * Added `Array.take` function to get some prefix of an array (dfinity/motoko-base⁠#587).

  * Deprecated `TrieSet.mem` in favor of `TrieSet.has` (dfinity/motoko-base⁠#576).

  * bugfix: `Array.chain(as, f)` was incorrectly trapping when `f(a)` was an empty array
    (dfinity/motoko-base⁠#599).

## 0.10.2 (2023-11-12)

* motoko (`moc`)

  * bugfix: separate tag from underscore in coverage warnings (#4274).

  * Code compiled for targets WASI (`-wasi-system-api`) and pure Wasm (`-no-system-api`) can now
    use up to 4GB of (efficiently emulated) stable memory, enabling more offline testing of, for example,
    stable data structures built using libraries `Regions.mo` and `ExperimentalStableMemory.mo`.
    Note that any Wasm engine (such as `wasmtime`), used to execute such binaries, must support and enable
    Wasm features `multi-memory` and `bulk-memory` (as well as the standard NaN canonicalization) (#4256).

  * bugfix: fully implement `Region.loadXXX/storeXXX` for `Int8`, `Int16` and `Float` (#4270).

  * BREAKING CHANGE (Minor): values of type [`Principal`](doc/md/base/Principal.md) are now constrained to contain
    at most 29 bytes, matching the IC's notion of principal (#4268).

    In particular:

    * An actor `import` will be statically rejected if the binary representation of the (aliased) textually encoded
      principal contains strictly more than 29 bytes.

    * `Principal.fromBlob(b)` will trap if `b` contains strictly more than 29 bytes.

    * The actor literal, `actor <exp>`, will trap if the binary representation of
      of the textually encoded principal `<exp>` contains strictly more than 29 bytes.

* motoko-base

  * bugfix: fix `Array.tabulateVar` to avoid repeated side-effects (dfinity/motoko-base⁠#596)

## 0.10.1 (2023-10-16)

* motoko (`moc`)

  * bugfix: fix assertion failure renaming `or`-patterns (#4236, #4224).

  * bugfix: unsuccessful Candid decoding of an optional array now defaults to null instead of crashing (#4240).

  * bugfix: Candid decoding of an optional, unknown variant with a payload now succeeds instead of crashing (#4238).

  * Implement Prim.textLowercase and Prim.textUppercase (via Rust) (#4216).

  * perf: inline sharable low-level functions in generated coded,
    trading code size for reduced cycle count (#4212).
    Controlled by flags:
      * `-fno-shared-code` (default)
      * `-fshared-code` (legacy)
    (Helps mitigate the effect of the IC's new cost model, that increases
	the cost of function calls).

* motoko-base

  * Added `Principal.toLedgerAccount` (dfinity/motoko-base⁠#582).

  * Added `Text.toLowercase` and `Text.toUppercase` (dfinity/motoko-base⁠#590).

## 0.10.0 (2023-09-11)

* motoko (`moc`)

  * Added a new stable `Region` type of dynamically allocated, independently growable and
    isolated regions of IC stable memory (#3768). See documentation.
    BREAKING CHANGE: stable memory changes may occur that can prevent returning
    to previous `moc` versions.

  * Added doc comments in generated Candid files (#4178).

* motoko-base

  * Exposed conversions between adjacent fixed-width types (dfinity/motoko-base⁠#585).

  * Added library `Region.mo` offering isolated regions of IC stable memory (dfinity/motoko-base⁠#580).

## 0.9.8 (2023-08-11)

* motoko (`moc`)

  * Added numerical type conversions between adjacent fixed-width types (#4139).

  * Administrative: legacy-named release artefacts are no longer created (#4111).

## 0.9.7 (2023-07-18)

* motoko (`moc`)

  * Performance improvement: lower the default allocation for bignums (#4102).

  * Performance improvement: generate better code for pattern matches on some small variants (#4093).

  * bugfix: don't crash on import of Candid composite queries (#4128).

## 0.9.6 (2023-07-07)

* motoko (`moc`)

  * Allow canister controllers to call the `__motoko_stable_var_info` query endpoint (#4103).
    (Previously only self-queries were permitted.)

  * Performance improvement: reduced cycle consumption for allocating objects (#4095).

  * bugfix: reduced memory consumption in the Motoko Playground (#4106).

## 0.9.5 (2023-07-05)

* motoko (`moc`)

  * Allow identifiers in `or`-patterns (#3807).
    Bindings in alternatives must mention the same identifiers and have compatible types:
    ``` Motoko
    let verbose = switch result {
      case (#ok) "All is good!";
      case (#warning why or #error why) "There is some problem: " # why;
    }
    ```

  * Performance improvement: improved cycle consumption allocating fixed-size objects (#4064).
    Benchmarks indicate up to 10% less cycles burned for allocation-heavy code,
    and 2.5% savings in realistic applications.

  * Administrative: binary build artefacts are now available according to standard naming
    conventions (thanks to EnzoPlayer0ne) (#3997).
    Please consider transitioning to downloading binaries following the new scheme,
    as legacy naming will be discontinued at some point in the future.

## 0.9.4 (2023-07-01)

* motoko (`moc`)

  * Allow multiline text literals (#3995).
    For example,
    ```
    "A horse walks into a bar.
    The barman says: `Why the long face?`"
    ```

    parses as:
    ```
    "A horse walks into a bar.\nThe barman says: `Why the long face?`"
    ```

  * Added pipe operator `<exp1> |> <exp2>` and placeholder expression `_`  (#3987).
    For example:
    ``` motoko
    Iter.range(0, 10) |>
      Iter.toList _ |>
        List.filter<Nat>(_, func n { n % 3 == 0 }) |>
          { multiples = _ };
    ```

    may, according to taste, be a more readable rendition of:
    ``` motoko
    { multiples =
       List.filter<Nat>(
         Iter.toList(Iter.range(0, 10)),
           func n { n % 3 == 0 }) };
    ```

    However, beware the change of evaluation order for code with side-effects.

  * BREAKING CHANGE (Minor):

    New keyword `composite` allows one to declare Internet Computer *composite queries* (#4003).

    For example,
    ``` motoko
    public shared composite query func sum(counters : [Counter]) : async Nat {
      var sum = 0;
      for (counter in counters.vals())  {
        sum += await counter.peek();
      };
      sum
    }
    ```

    has type:
    ``` motoko
    shared composite query [Counter] -> async Nat
    ```

    and can call both `query` and other `composite query` functions.

    See the documentation for full details.

  * Allow canister imports of Candid service constructors, ignoring the service arguments to
    import the instantiated service instead (with a warning) (#4041).

  * Allow optional terminal semicolons in Candid imports (#4042).

  * bugfix: allow signed float literals as static expressions in modules (#4063).

  * bugfix: improved reporting of patterns with record types in error messages (#4002).

* motoko-base

  * Added more `Array` (and `Text`) utility functions (thanks to roman-kashitsyn) (dfinity/motoko-base⁠#564).

## 0.9.3 (2023-06-19)

* motoko (`moc`)

  * Added fields `sender_canister_version` for actor class version tracking (#4036).

## 0.9.2 (2023-06-10)

* motoko (`moc`)

  * BREAKING CHANGE (Minor):

    `or`-patterns in function definitions cannot be inferred any more. The new error
    message suggests to add a type annotation instead. This became necessary in order
    to avoid potentially unsound types (#4012).

  * Added implementation for `ic0.canister_version` as a primitive (#4027).

  * Added a more efficient `Prim.blobCompare` (thanks to nomeata) (#4009).

  * bugfix: minor error in grammar for `async*` expressions (#4005).

* motoko-base

  * Add `Principal.isController` function (dfinity/motoko-base#558).

## 0.9.1 (2023-05-15)

* motoko (`moc`)

  * Added implementation for `ic0.is_controller` as a primitive (#3935).

  * Added ability to enable the new incremental GC in the Motoko Playground (#3976).

## 0.9.0 (2023-05-12)

* motoko (`moc`)

  * **For beta testing:** Add a new _incremental_ GC, enabled with new moc flag `--incremental-gc` (#3837).
    The incremental garbage collector is designed to scale for large program heap sizes.

    The GC distributes its workload across multiple steps, called increments, that each pause the mutator
    (user's program) for only a limited amount of time. As a result, the GC work can fit within the instruction-limited
    IC messages, regardless of the heap size and the object structures.

    According to GC benchmark measurements, the incremental GC is more efficient than the existing copying, compacting,
    and generational GC in the following regards:
    * Scalability: Able to use the full heap space, 3x more object allocations on average.
    * Shorter interruptions: The GC pause has a maximum limit that is up to 10x shorter.
    * Lower runtimes: The number of executed instructions is reduced by 10% on average (compared to the copying GC).
    * Less GC overhead: The amount of GC work in proportion to the user's program work drops by 10-16%.

    The GC incurs a moderate memory overhead: The allocated WASM memory has been measured to be 9% higher
    on average compared to the copying GC, which is the current default GC.

    To activate the incremental GC under `dfx`, the following command-line argument needs to be specified in `dfx.json`:

    ```
    ...
      "type" : "motoko"
      ...
      "args" : "--incremental-gc"
    ...
    ```

  * bugfix: `array.vals()` now returns a working iterator for mutable arrays (#3497, #3967).

## 0.8.8 (2023-05-02)

* motoko (`moc`)

  * Performance improvement: optimised code generation for pattern matching that cannot fail (#3957).

## 0.8.7 (2023-04-06)

* motoko (`moc`)

  * Added ability to `mo-doc` for rendering documentation of nested modules (#3918).

  * bugfix: when re-adding recurrent timers, skip over past expirations (#3871).

  * bugfix: eliminated crash compiling local `async` functions that pattern match on arguments (#3910, #3916).

## 0.8.6 (2023-04-01)

* motoko (`moc`)

  * bugfix: avoid compiler crash (regression) when `let`-matching on constant variants (#3901, #3903).

  * Performance improvement: improved cycle usage when receiving messages (#3893).

## 0.8.5 (2023-03-20)

* motoko (`moc`)

  * Performance improvement: Values of variant type that are compile-time known
    are relegated to the static heap now and don't get allocated each time (#3878).

  * bugfix: the global timer expiration callback was called unnecessarily in the
    default mechanism (#3883).

## 0.8.4 (2023-03-11)

* motoko (`moc`)

  * Performance improvement: UTF-8 coding and validation is now properly tail recursive (#3842).

  * Performance improvement: eliminated bounds checking for certain array accesses (thanks to nomeata) (#3853).

  * Performance improvement: optimized `{array, blob, text}.size()` operations (thanks to nomeata) (#3863).

  * Performance improvement: efficient tuple results in `switch` statements (thanks to nomeata) (#3865).

  * Performance improvement: more efficient untagging operation (#3873).

  * bugfix: restored a grammar regression caused by `let-else` (#3869).

* motoko-base

  * Add `Array.subArray` function (dfinity/motoko-base#445).

  * BREAKING CHANGE (Minor)

    Optimized `AssocList.{replace, find}` to avoid unnecessary allocation (dfinity/motoko-base#535, dfinity/motoko-base#539).
    Note: this subtly changes the order in which the key-value pairs occur after replacement. May affect other containers that use `AssocList`.

  * Performance improvement: Optimized deletion for `Trie`/`TrieMap` (dfinity/motoko-base#525).

## 0.8.3 (2023-02-24)

* motoko (`moc`)

  * new 'let-else' construct for handling pattern-match failure (#3836).
    This is a frequently asked-for feature that allows to change the control-flow
    of programs when pattern-match failure occurs, thus providing a means against
    the famous "pyramid of doom" issue. A common example is look-ups:
    ``` Motoko
    shared func getUser(user : Text) : async Id {
      let ?id = Map.get(users, user) else { throw Error.reject("no such user") };
      id
    }
    ```
    Similarly, an expression like
    ``` Motoko
    (label v : Bool { let <pat> = <exp> else break v false; true })
    ```
    evaluates to a `Bool`, signifying whether `<pat>` matches `<exp>`.

  * Improve recursive deserialization capacity to match recursive serialization capacity by reducing
    Wasm stack consumption (#3809).
    Because of the bounds on recursion depth imposed by fixed-size stack, the
    advice remains the same: avoid deeply nested recursive data structures.
    Think "shallow trees good, very long lists bad".

  * bugfix: stack overflow in UTF-8 encode/decode for `moc.js` (#3825).

* motoko-base

  * add missing `unshare : Tree<K, V> -> ()` method to class `RBTree<K, V>`
    to restore objects from saved state (dfinity/motoko-base#532).

## 0.8.2 (2023-02-17)

* motoko (`moc`)

  * Add compiler flag `--rts-stack-pages <n>` to override default number of
    pages dedicated to fixed runtime system stack. Now defaults to 32 pages
    (2MiB) (up from previous 2 pages/128KiB) (#3782).
    In emergencies, increasing this setting may improve your ability to deserialize
    deeply nested Candid or stable variable data.

  * Add stack overflow detection utilising reserved page (#3793).

  * Performance improvement: heap allocator speedup (#3090, #3790).

  * bugfix: avoid more heap-out-bounds errors during deserialization of stable variables
    by increasing default runtime system stack from 128KiB to 2MiB (#3782).
    _Note_: this is a partial fix, as issues with stack growth remain.

* motoko-base

  * bugfix: non-leaky deletion for `RBTree` (dfinity/motoko-base#524).

## 0.8.1 (2023-02-03)

* motoko (`moc`)

  * Performance improvement: faster heap allocation (#3765).

  * bugfix: `async` returns involving abbreviated tuple types no longer crash the compiler (#3740, #3741).

  * bugfix: avoid quadratic code expansion due to imported, but unused, actor classes (#3758).

## 0.8.0 (2023-01-27)

* motoko (`moc`)

  * BREAKING CHANGE

    Motoko now implements Candid 1.4 (dfinity/candid#311).

    In particular, when deserializing an actor or function reference,
    Motoko will now first check that the type of the deserialized reference
    is a subtype of the expected type and act accordingly.

    Very few users should be affected by this change in behaviour.

  * BREAKING CHANGE

    Failure to send a message no longer traps but, instead, throws a catchable `Error` with new error code `#call_error` (#3630).

    On the IC, the act of making a call to a canister function can fail, so that the call cannot (and will not be) performed.
    This can happen due to a lack of canister resources, typically because the local message queue for the destination canister is full,
    or because performing the call would reduce the current cycle balance of the calling canister to a level below its freezing threshold.
    Such call failures are now reported by throwing an `Error` with new `ErrorCode` `#call_error { err_code = n }`,
    where `n` is the non-zero `err_code` value returned by the IC.
    Like other errors, call errors can be caught and handled using `try ... catch ...` expressions, if desired.

    The constructs that now throw call errors, instead of trapping as with previous version of Motoko are:
    * calls to `shared` functions (including oneway functions that return `()`).
      These involve sending a message to another canister, and can fail when the queue for the destination canister is full.
    * calls to local functions with return type `async`. These involve sending a message to self, and can fail when the local queue for sends to self is full.
    * `async` expressions. These involve sending a message to self, and can fail when the local queue for sends to self is full.
    * `await` expressions. These can fail on awaiting an already completed future, which requires sending a message to self to suspend and commit state.

    (On the other hand, `async*` (being delayed) cannot throw, and evaluating `await*` will at most propagate an error from its argument but not, in itself, throw.)

    Note that exiting a function call via an uncaught throw, rather than a trap, will commit any state changes and currently queued messages.
    The previous behaviour of trapping would, instead, discard, such changes.

    To appreciate the change in semantics, consider the following example:

    ``` motoko
    actor {
      var count = 0;
      public func inc() : async () {
        count += 1;
      };
      public func repeat() : async () {
        loop {
          ignore inc();
        }
      };
      public func repeatUntil() : async () {
        try {
          loop {
           ignore inc();
          }
        } catch (e) {
        }
      };
    }
    ```

    In previous releases of Motoko, calling `repeat()` and `repeatUntil()` would trap, leaving `count` at `0`, because
    each infinite loop would eventually exhaust the message queue and issue a trap, rolling back the effects of each call.
    With this release of Motoko, calling `repeat()` will enqueue several `inc()` messages (around 500), then `throw` an `Error`
    and exit with the error result, incrementing the `count` several times (asynchronously).
    Calling `repeatUntil()` will also enqueue several `inc()` messages (around 500) but the error is caught so the call returns,
    still incrementing `count` several times (asynchronously).

    The previous semantics of trapping on call errors can be enabled with compiler option `--trap-on-call-error`, if desired,
    or selectively emulated by forcing a trap (e.g. `assert false`) when an error is caught.

    For example,

    ``` motoko
      public func allOrNothing() : async () {
        try {
          loop {
           ignore inc();
          }
        } catch (e) {
          assert false; // trap!
        }
      };
    ```

    Calling `allOrNothing()` will not send any messages: the loop exits with an error on queue full,
    the error is caught, but `assert false` traps so all queued `inc()` messages are aborted.

  * bugfix: system method `inspect` involving message with single tuple argument no longer crashes the compiler (#3732, #3733).

## 0.7.6 (2023-01-20)

* motoko (`moc`)

  * Added support for `ManagementCanister.raw_rand` in interpreters (#3693).

  * Added preliminary Viper support for `old` expressions in specifications and calls to private methods (#3675).

  * bugfix: in the default timer mechanism `cancelTimer` sometimes wouldn't actually stop a recurring timer (#3695).

  * bugfix: zero negation for floating point numbers in compiled code (#3676).

* motoko-base

  * Add user-facing timer functionality (dfinity/motoko-base#474).

  * Add `Array.size` (dfinity/motoko-base#486, dfinity/motoko-base#494).

  * Add `TrieSet` methods `isEmpty`, `isSubset` (dfinity/motoko-base#503).

  * BREAKING CHANGES (Minor):
    - renamed `Float.neq` to `Float.neg` (this was a misspelling)
    - renamed `Nat.neq` to `Nat.neg` (this was a misspelling)
    - removed second argument from `bitnot` (this was an oversight)

  * bugfix: `Random.Finite.coin` didn't use entropy correctly (dfinity/motoko-base#500).

  * bugfix: `Trie.mergeDisjoint` (dfinity/motoko-base#505).

  * bugfix: `TrieSet.equals` (dfinity/motoko-base#503).

  * Various documentation fixes and API usage examples.

## 0.7.5 (2022-12-23)

* motoko (`moc`)

  * Add new primitives for a default timer mechanism (#3542). These are
    ``` Motoko
    setTimer : (delayNanos : Nat64, recurring : Bool, job : () -> async ()) -> (id : Nat)
    cancelTimer : (id : Nat) -> ()
    ```
    By defining a `system func timer` the default mechanism can now be overridden by a custom
    implementation. Additionally by supplying the command-line flag `-no-timer` all aspects
    of timers can be suppressed, e.g. for space- or security-sensitive purposes, thus effectively
    reverting canisters to the pre-timers era.

  * bugfix: silence bogus cascading errors in stable compatibility check (#3645).

## 0.7.4 (2022-12-07)

* motoko (`moc`)

  * Add new keywords `async*` and `await*` (note the `*`) for efficient abstraction of asynchronous code (#3609).
    ```
      <typ> ::= ...
        async* <typ>             delayed, asynchronous computation
      <exp> ::= ...
        async* <block-or-exp>    delay an asynchronous computation
        await* <block-or-exp>    execute a delayed computation (only in async, async*)
    ```
    This avoids the resource consumption and latency of `async`/`await` by only committing state and suspending execution
    when necessary in the `await*`-ed computation, not necessarily at the `await*` itself.

    WARNING: Unlike `async`/`await`:
    *  an `async*` value has no effect unless `await*`-ed;
    *  each `await*` of the same `async*` value repeats its effects.

    This feature is experimental and may evolve in future. Use with discretion.
    See the [manual](doc/md/reference/language-manual.md) for details.

  * Suppress GC during IC `canister_heartbeat`, deferring any GC to the scheduled Motoko `heartbeat` `system` method (#3623).
    This is a temporary workaround, to be removed once DTS is supported for `canister_heartbeat` itself (#3622).

  * Add a new _generational_ GC, enabled with new moc flag `--generational-gc` (#3495).
    The generational garbage collector optimizes for fast reclamation of short-lived objects.
    New objects are allocated in a young generation that is more frequently collected than the older objects
    that have already survived a GC run.

    For many cases, the generational GC is more efficient than the existing compacting GC and copying GCs:
    * Lower runtimes: Less number of executed instructions on average.
    * Shorter interruptions: Young generation collection entails shorter program interruptions.

    To activate the generational GC under `dfx`, the following command-line argument needs to be specified in `dfx.json`:

    ```
    ...
      "type" : "motoko"
      ...
      "args" : "--generational-gc"
    ...
    ```

  * `moc.js` : add trampoline and step limiter to interpreter, avoiding (some) stackoverflows and
    hangs (#3618, #3541).
    Enables execution of larger examples on web pages.

  * BREAKING CHANGE (Minor):

    Consider records with mutable fields as non-static (#3586).
    Consequently, an imported library declaring a mutable record is now
    rejected, not accepted, to be consistent with the declarations of
    mutable fields and mutable objects.

  * Experimental Viper integration by compiling a very narrow subset of
    Motoko to the verification intermediate language. See `src/viper/README.md`
    and the PR for details. (#3477).

* motoko-base

  * Unit tests for Trie and fix for `disj` (dfinity/motoko-base#438).

  * Respect Trie structure in `filter` (dfinity/motoko-base#431, dfinity/motoko-base#438).

  * Array module reimplementation, tests and documentation (dfinity/motoko-base#425,dfinity/motoko-base#432).

## 0.7.3 (2022-11-01)

* motoko (`moc`)

  * Statically reject shared functions and function types with type parameters (#3519, #3522).

  * Performance improvement: `Array.init` and `Array.tabulate` (#3526).

* motoko-base

  * Add some examples to `Buffer` library documentation (dfinity/motoko-base#420).

  * Fix another bug in `Buffer` library affecting `filterEntries` (dfinity/motoko-base#422).

## 0.7.2 (2022-10-25)

* motoko-base

  * Fix bugs in `Buffer` library affecting `remove` and `filterEntries` (dfinity/motoko-base#419).

## 0.7.1 (2022-10-24)

* motoko (`moc`)

    * Halve (default ir-checking) compilation times by optimizing type comparison and hashing (#3463)

    * Add support for type components in object type syntax (#3457, also fixes #3449)
    ``` motoko
      type Record = { type T = Nat; x : Nat};
    ```
    is now legal.
    Note the definition of `T` is neither recursive, nor bound in `x : Nat`,
    but can refer to an existing recursive type declared in an outer scope.

* motoko-base

  * Optimized and extended `Buffer` library (dfinity/motoko-base#417).

## 0.7.0 (2022-08-25)

* motoko (`moc`)

  * BREAKING CHANGE (Minor):
    Adds new syntax for merging records (objects) and
    adding/overwriting fields. The expression
    ``` motoko
    { baseA and baseB with field1 = val1; field2 = val2 }
    ```
    creates a new record by joining all (statically known) fields from
    `baseA/B` and the explicitly specified `field1/2`.
    This is a _breaking change_, as a new keyword `with` has been added.
    Restrictions for ambiguous and `var` fields from bases apply. (#3084)

  * Add new support for installing actor class instances on the IC,
    enabling specification of canister settings, install, upgrade and
    reinstall. (#3386)

    A new expression

    ``` bnf
      (system <exp> . <id>)
    ```
    where `<exp>` is an imported library and `<id>` is the name of
    an actor class, accesses a secondary constructor of the class
    that takes an additional argument controlling the installation.

    For example,
    ``` motoko
      await (system Lib.Node)(#upgrade a)(i);
    ```
    upgrades actor `a` with the code for a new instance of class `Lib.Node`,
    passing constructor argument `(i)`.

  * Performance improvements for assigment-heavy code (thanks to nomeata) (#3406)

## 0.6.30 (2022-08-11)

* motoko (`moc`)

  * add primitives
    ```motoko
    shiftLeft : (Nat, Nat32) -> Nat
    shiftRight : (Nat, Nat32) -> Nat
    ```
    for efficiently multiplying/dividing a `Nat` by a power of 2
    (#3112)

  * add primitives
    ```motoko
    rts_mutator_instructions : () -> Nat
    rts_collector_instructions : () -> Nat
    ```
    to report approximate IC instruction costs of the last message
    due to mutation (computation) and collection (GC), respectively (#3381)

* motoko-base

  * Add
    ```motoko
    Buffer.fromArray
    Buffer.fromVarArray
    ```
    for efficiently adding an array to a `Buffer`
    (dfinity/motoko-base#389)

  * Add
    ```motoko
    Iter.sort : (xs : Iter<A>, compare : (A, A) -> Order) : Iter<A>
    ```
    for sorting an `Iter` given a comparison function
    (dfinity/motoko-base#406)

  * Performance: `HashMap` now avoids re-computing hashes on `resize`
    (dfinity/motoko-base#394)

## 0.6.29 (2022-06-10)

* motoko (`moc`)

  * The language server now supports explicit symbol imports (thanks
    to rvanasa) (#3282)
  * The language server now has improved support for navigating to
    definitions in external modules (thanks to rvanasa)  (#3263)
  * Added a primitive `textCompare` allowing more efficient three-way
    `Text` comparisons (#3298)
  * Fixed a typing bug with annotated, recursive records (#3268)

* motoko-base

  * Add
    ```motoko
    ExperimentalInternetComputer.countInstruction : (comp : () -> ()) -> Nat64
    ```
    to count the Wasm instructions performed during execution of `comp()` (dfinity/motoko-base#381)

  * Add
    ```motoko
    ExperimentalStableMemory.stableVarQuery : () -> (shared query () -> async {size : Nat64})
    ```
    for estimating stable variable storage requirements during upgrade
    (dfinity/motoko-base#365)
  * Performance improvement to `Text.compare` (dfinity/motoko-base#382)

## 0.6.28 (2022-05-19)

* motoko (`moc`)

  * Add `to_candid`, `from_candid` language constructs for Candid serialization to/from Blobs (#3155)
  * New `system` field 'inspect' for accepting/declining canister ingress messages (see doc) (#3210)

## 0.6.27 (2022-05-04)

* motoko (`moc`)

  * Importing modules by relative path is now more robust (#3215).
  * Performance: persisting stable variables to stable memory is now
    performed in streaming fashion, reducing heap consumption and
    copying during an upgrade (#3149).
  * Performance: local 32- and 64-bit numeric values are now stored in
    using unboxed form when possible (thanks to nomeata) (#3207).

* motoko-base

  * Fixed a bug in `Trie.filter` (and `Trie.mapFilter`) which could
    lead to missing matches in some cases (dfinity/motoko-base#371).

## 0.6.26 (2022-04-20)

* motoko (`moc`)

  * Performance: inline prim-wrapping functions (thanks to nomeata) (#3159)
  * Improve type pretty printer to mirror type parser (avoids producing unparseable stable variable signatures) (#3190)
  * Adds new flag `--omit-metadata` to omit certain metadata sections from `actor` (and `actor class`) Wasm (#3164)
  * Performance: avoid redundant heap allocation when deserializing compact Candid `int` and  `nat` values (#3173)
  * Added a primitive to obtain stable variable memory footprint (#3049)

* motoko-base

  * Fixed the 32-bit range limitation of `Hash.hash: Nat -> Nat32` and
    deprecate most functions in `Hash` (dfinity/motoko-base#366).
  * Add `List.toIter` (thanks to hoosan) (dfinity/motoko-base#336).

## 0.6.25 (2022-03-07)

* motoko (`moc`)

  * bugfix: fix bogus elision of type constructors sharing names with primitive types in `--stable-types` section and `.most` file (#3140)

## 0.6.24 (2022-03-06)

* motoko (`moc`)

  * bugfix: fix bogus identification of distinct type constructors
    in --stable-types section and .most file (#3140)

## 0.6.23 (2022-03-05)

* motoko (`moc`)

  * bugfix: fix pretty printing of (stable) types and #3128 (#3130)

    * Collect constructors  *transitively* before emitting a .most file.
    * Modifies type pretty printer to produce well-formed types and stable type signatures.

## 0.6.22 (2022-02-24)

* motoko (`moc`)

  * Fix: remove bogus error when transitively importing module with
    selective field imports (#3121)
  * Fix: Treating eponymous types from separate candid files (#3103)

* Various reports from CI are now pushed to
  https://dfinity.github.io/motoko (#3113)

## 0.6.21 (2022-01-31)

* motoko (`moc`)

  * Emit new ICP metadata custom section 'motoko:compiler' with compiler release or revision in UTF8 (e.g. "0.6.21"). Default is `icp:private` (#3091).
  * Generalized `import` supporting pattern matching and selective field imports (#3076).
  * Fix: insert critical overflow checks preventing rare heap corruptions
    in out-of-memory allocation and stable variable serialization (#3077).
  * Implement support for 128-bit Cycles-API (#3042).

* motoko-base

  * `ExperimentalInternetComputer` library, exposing low-level, binary `call` function (a.k.a. "raw calls") (dfinity/motoko-base#334, Motoko #3806).
  * `Principal.fromBlob` added (dfinity/motoko-base#331).

## 0.6.20 (2022-01-11)

* motoko

  * Implement support for `heartbeat` system methods (thanks to ninegua) (#2971)

* motoko-base

  * Add `Iter.filter : <A>(Iter<A>, A -> Bool) -> Iter<A>` (thanks to jzxchiang1) (dfinity/motoko-base#328).

## 0.6.19 (2022-01-05)

* motoko-base

  * Fixed a bug in the `RBTree.size()` method.

## 0.6.18 (2021-12-20)

* moc

  * Add runtime support for low-level, direct access to 64-bit IC stable memory, including documentation.
  * Add compiler flag `--max-stable-pages <n>` to cap any use of `ExperimentalStableMemory.mo` (see below), while reserving space for stable variables.
  Defaults to 65536 (4GiB).

* motoko-base

  * (Officially) add `ExperimentalStableMemory.mo` library, exposing 64-bit IC stable memory

* BREAKING CHANGE (Minor):
  The previously available (but unadvertised) `ExperimentalStableMemory.mo` used
  `Nat32` offsets. This one uses `Nat64` offsets to (eventually) provide access to more address space.

## 0.6.17 (2021-12-10)

* Improved handling of one-shot messages facilitating zero-downtime
  upgrades (#2938).
* Further performance improvements to the mark-compact garbage
  collector (#2952, #2973).
* Stable variable checking for `moc.js` (#2969).
* A bug was fixed in the scoping checker  (#2977).

## 0.6.16 (2021-12-03)

* Minor performance improvement to the mark-compact garbage collector


## 0.6.15 (2021-11-26)

* Fixes crash when (ill-typed) `switch` expression on non-variant
  value has variant alternatives (#2934)

## 0.6.14 (2021-11-19)

* The compiler now embeds the existing Candid interface  and  new
  _stable signature_ of a canister in additional Wasm custom sections,
  to be selectively exposed by the IC, and to be used by tools such as `dfx`
  to verify upgrade compatibility (see extended documentation).

  New compiler options:

    * `--public-metadata <name>`: emit ICP custom section `<name>` (`candid:args` or `candid:service` or `motoko:stable-types`) as `public` (default is `private`)
    * `--stable-types`: emit signature of stable types to `.most` file
    * `--stable-compatible <pre> <post>`: test upgrade compatibility between stable-type signatures  `<pre>` and `<post>`

  A Motoko canister upgrade is safe provided:

    * the canister's Candid interface evolves to a Candid subtype; and
    * the canister's Motoko stable signature evolves to a _stable-compatible_ one.

  (Candid subtyping can be verified using tool `didc` available at:
   https://github.com/dfinity/candid.)

* BREAKING CHANGE (Minor):
  Tightened typing for type-annotated patterns (including function parameters)
  to prevent some cases of unintended and counter-intuitive type propagation.

  This may break some rare programs that were accidentally relying on that
  propagation. For example, the indexing `xs[i]` in the following snippet
  happend to type-check before, because `i` was given the more precise
  type `Nat` (inferred from `run`'s parameter type), regardless of the
  overly liberal declaration as an `Int`:

  ```motoko
  func run(f : Nat -> Text) {...};
  let xs = ["a", "b", "c"];
  run(func(i : Int) { xs[i] });
  ```
  This no longer works, `i` has to be declared as `Nat` (or the type omitted).

  If you encounter such cases, please adjust the type annotation.

* Improved garbage collection scheduling

* Miscellaneous performance improvements
  - code generation for `for`-loops over arrays has improved
  - slightly sped up `Int` equality comparisons

## 0.6.13 (2021-11-19)

*Pulled*

## 0.6.12 (2021-10-22)

* `for` loops over arrays are now converted to more efficient
  index-based iteration (#2831). This can result in significant cycle
  savings for tight loops, as well as slightly less memory usage.

* Add type union and intersection. The type expression

  ```motoko
  T and U
  ```
  produces the greatest lower bound of types `T` and `U`, that is,
  the greatest type that is a subtype of both. Dually,

  ```motoko
  T or U
  ```
  produces the least upper bound of types `T` and `U`, that is,
  the smallest type that is a supertype of both.

  One use case of the former is "extending" an existing object type:

  ``` motoko
  type Person = {name : Text; address : Text};
  type Manager = Person and {underlings : [Person]};
  ```
  Similarly, the latter can be used to "extend" a variant type:
  ```motoko
  type Workday = {#mon; #tue; #wed; #thu; #fri};
  type Weekday = Workday or {#sat; #sun};
  ```

## 0.6.11 (2021-10-08)

* Assertion error messages are now reproducible (#2821)

## 0.6.10 (2021-09-23)

* moc

  * documentation changes

* motoko-base

  * documentation changes

## 0.6.9 (2021-09-15)

* motoko-base

  * add Debug.trap : Text -> None (dfinity/motoko-base#288)

## 0.6.8 (2021-09-06)

* Introduce primitives for `Int` ⇔ `Float` conversions (#2733)
* Bump LLVM toolchain to version 12 (#2542)
* Support extended name linker sections (#2760)
* Fix crashing bug for formatting huge floats (#2737)

## 0.6.7 (2021-08-16)

* moc

  *  Optimize field access by exploiting field ordering (#2708)
  *  Fix handling of self references in mark-compact GC (#2721)
  *  Restore CI reporting of perf-regressions (#2643)

* motoko-base:

  * Fix bug in `AssocList.diff` (dfinity/motoko-base#277)
  * Deprecate unsafe or redundant functions in library `Option` ( `unwrap`, `assertSome`, `assertNull`) (#275)

## 0.6.6 (2021-07-30)

* Vastly improved garbage collection scheduling: previously Motoko runtime would do GC
  after every update message. We now schedule a GC when

  1. Heap grows more than 50% and 10 MiB since the last GC, or
  2. Heap size is more than 3 GiB

  (1) is to make sure we don't do GC on tiny heaps or after only small amounts of allocation.
  (2) is to make sure that on large heaps we will have enough allocation space during the next message.

  This scheduling reduces cycles substantially, but may moderately increase memory usage.

  New flag `--force-gc` restores the old behavior.

* Fix bug in compacting gc causing unnecessary memory growth (#2673)

* Trap on attempt to upgrade when canister not stopped and there are outstanding callbacks.
  (This failure mode can be avoided by stopping the canister before upgrade.)

* Fix issue #2640 (leaked `ClosureTable` entry when awaiting futures fails).

## 0.6.5 (2021-07-08)

* Add alternative, _compacting_ gc, enabled with new moc flag `--compacting-gc`.
  The compacting gc supports larger heap sizes than the default, 2-space copying collector.

  NOTE: Dfx 0.7.6 adds optional field `"args"` to `dfx.json` files,
  so Motoko canisters can specify `moc` command-line arguments. E.g.,

  ```json
  ...
     "type" : "motoko"
     ...
     "args" : "--compacting-gc"
  ...
  ```

* Documentation fixes.
* Command line tools: `--help` option provides better documentation of command line
  options that have arguments.
* Fix issue #2319 (crash on import of Candid class).

## 0.6.4 (2021-06-12)

* For release builds, the banner (`moc --version`) now includes the release
  version.

* Fix MacOS release builds (the 0.6.3 tarball for MacOS contained the linux binaries)

## 0.6.3 (2021-06-10)

* Motoko is now open source!

* Better internal consistency checking of the intermediate representation

## 0.6.2 (2021-05-24)

* motoko-base:

  * reformat to style guidelines
  * add type bindings `Nat.Nat`, `Nat8.Nat8` etc. to libraries for primitive types.

* Bugfix: generation of candid from Motoko:

  * no longer confused by distinct, but eponymous, type definitions (Bug: #2529);
  * numbers eponymous types and specializations from 1 (not 2);
  * avoids long chains of type equalities by normalizing before translation.

## 0.6.1 (2021-04-30)

* Internal: Update to IC interface spec 0.17 (adapt to breaking change to signature of `create_canister`)

## 0.6.0 (2021-04-16)

* BREAKING CHANGE:
  The old-style object and block syntax deprecated in 0.5.0 is finally removed.

* Record punning: As already supported in patterns, short object syntax in
  expressions now allows omitting the right-hand side if it is an identifier
  of the same name as the label. That is,

  ```motoko
  {a; b = 1; var c}
  ```

  is short for

  ```motoko
  {a = a; b = 1; var c = c}
  ```

  assuming respective variables are in scope.

* BREAKING CHANGE:
  The types `Word8`, `Word16`, `Word32` and `Word64` have been removed.
  This also removed the `blob.bytes()` iterator.

  Motoko base also dropped the `Word8`, `Word16`, `Word32` and `Word64`
  modules.

  This concludes the transition to the other fixed-width types that began with
  version 0.5.8

* BREAKING CHANGE (Minor):
 `await` on a completed future now also commits state and suspends
  computation, to ensure every await, regardless of its future's state,
  is a commit point for state changes and tentative message sends.

  (Previously, only awaits on pending futures would force a commit
   and suspend, while awaits on completed futures would continue
   execution without an incremental commit, trading safety for speed.)

* motoko-base: fixed bug in `Text.compareWith`.

## 0.5.15 (2021-04-13)

* Bugfix: `Blob.toArray` was broken.

## 0.5.14 (2021-04-09)

* BREAKING CHANGE (Minor): Type parameter inference will no longer default
  under-constrained type parameters that are invariant in the result, but
  require an explicit type argument.
  This is to avoid confusing the user by inferring non-principal types.

  For example, given (invariant) class `Box<A>`:

  ```motoko
    class Box<A>(a : A) { public var value = a; };
  ```

  the code

  ```motoko
    let box = Box(0); // rejected
  ```

  is rejected as ambiguous and requires an instantiation, type annotation or
  expected type. For example:

  ```motoko
    let box1 = Box<Int>(0); // accepted
    let box2 : Box<Nat> = Box(0); // accepted
  ```

  Note that types `Box<Int>` and `Box<Nat>` are unrelated by subtyping,
  so neither is best (or principal) in the ambiguous, rejected case.

* Bugfix: Type components in objects/actors/modules correctly ignored
  when involved in serialization, equality and `debug_show`, preventing
  the compiler from crashing.

* motoko-base: The `Text.hash` function was changed to a better one.
  If you stored hashes as stable values (which you really shouldn't!)
  you must rehash after upgrading.

* motoko-base: Conversion functions between `Blob` and `[Nat8]` are provided.

* When the compiler itself crashes, it will now ask the user to report the
  backtrace at the DFINITY forum

## 0.5.13 (2021-03-25)

* The `moc` interpreter now pretty-prints values (as well as types) in the
  repl, producing more readable output for larger values.

* The family of `Word` types are deprecated, and mentioning them produces a warning.
  These type will be removed completely in a subsequent release.
  See the user’s guide, section “Word types”, for a migration guide.

* motoko base: because of this deprecation, the `Char.from/toWord32()`
  functions are removed. Migrate away from `Word` types, or use
  `Word32.from/ToChar` for now.

## 0.5.12 (2021-03-23)

* The `moc` compiler now pretty-prints types in error messages and the repl,
  producing more readable output for larger types.

* motoko base: fixed bug in `Text.mo` affecting partial matches in,
  for example, `Text.replace` (GH issue #234).

## 0.5.11 (2021-03-12)

* The `moc` compiler no longer rejects occurrences of private or
  local type definitions in public interfaces.

  For example,

  ```motoko
  module {
    type List = ?(Nat, List); // private
    public func cons(n : Nat, l : List) : List { ?(n , l) };
  }
  ```

  is now accepted, despite `List` being private and appearing in the type
  of public member `cons`.

* Type propagation for binary operators has been improved. If the type of one of
  the operands can be determined locally, then the other operand is checked
  against that expected type. This should help avoiding tedious type annotations
  in many cases of literals, e.g., `x == 0` or `2 * x`, when `x` has a special
  type like `Nat8`.

* The `moc` compiler now rejects type definitions that are non-_productive_ (to ensure termination).

  For example, problematic types such as:

  ```motoko
  type C = C;
  type D<T, U> = D<U, T>;
  type E<T> = F<T>;
  type F<T> = E<T>;
  type G<T> = Fst<G<T>, Any>;
  ```

  are now rejected.

* motoko base: `Text` now contains `decodeUtf8` and `encodeUtf8`.

## 0.5.10 (2021-03-02)

* User defined deprecations

  Declarations in modules can now be annotated with a deprecation comment, which make the compiler emit warnings on usage.

  This lets library authors warn about future breaking changes:

  As an example:

  ```motoko
  module {
    /// @deprecated Use `bar` instead
    public func foo() {}

    public func bar() {}
  }
  ```

  will emit a warning whenever `foo` is used.

* The `moc` compiler now rejects type definitions that are _expansive_, to help ensure termination.
  For example, problematic types such as `type Seq<T> = ?(T, Seq<[T]>)` are rejected.

* motoko base: `Time.Time` is now public

## 0.5.9 (2021-02-19)

* The `moc` compiler now accepts the `-Werror` flag to turn warnings into errors.

* The language server now returns documentation comments alongside
  completions and hover notifications

## 0.5.8 (2021-02-12)

* Wrapping arithmetic and bit-wise operations on `NatN` and `IntN`

  The conventional arithmetic operators on `NatN` and `IntN` trap on overflow.
  If wrap-around semantics is desired, the operators `+%`, `-%`, `*%` and `**%`
  can be used. The corresponding assignment operators (`+%=` etc.) are also available.

  Likewise, the bit fiddling operators (`&`, `|`, `^`, `<<`, `>>`, `<<>`,
  `<>>` etc.) are now also available on `NatN` and `IntN`. The right shift
  operator (`>>`) is an unsigned right shift on `NatN` and a signed right shift
  on `IntN`; the `+>>` operator is _not_ available on these types.

  The motivation for this change is to eventually deprecate and remove the
  `WordN` types.

  Therefore, the wrapping arithmetic operations on `WordN` are deprecated and
  their use will print a warning. See the user’s guide, section “Word types”,
  for a migration guide.

* For values `x` of type `Blob`, an iterator over the elements of the blob
  `x.vals()` is introduced. It works like `x.bytes()`, but returns the elements
  as type `Nat8`.

* `mo-doc` now generates cross-references for types in signatures in
  both the Html as well as the Asciidoc output. So a signature like
  `fromIter : I.Iter<Nat> -> List.List<Nat>` will now let you click on
  `I.Iter` or `List.List` and take you to their definitions.

* Bugfix: Certain ill-typed object literals are now prevented by the type
  checker.

* Bugfix: Avoid compiler aborting when object literals have more fields than
  their type expects.

## 0.5.7 (2021-02-05)

* The type checker now exploits the expected type, if any,
  when typing object literal expressions.
  So `{ x = 0 } : { x : Nat8 }` now works as expected
  instead of requiring an additional type annotation on `0`.

## 0.5.6 (2021-01-22)

* The compiler now reports errors and warnings with an additional _error code_
  This code can be used to look up a more detailed description for a given error by passing the `--explain` flag with a code to the compiler.
  As of now this isn't going to work for most codes because the detailed descriptions still have to be written.
* Internal: The parts of the RTS that were written in C have been ported to Rust.

## 0.5.5 (2021-01-15)

* new `moc` command-line arguments `--args <file>` and `--args0 <file>` for
  reading newline/NUL terminated arguments from `<file>`.
* motoko base: documentation examples are executable in the browser

## 0.5.4 (2021-01-07)

* _Option blocks_ `do ? <block>` and _option checks_ `<exp> !`.
  Inside an option block, an option check validates that its operand expression is not `null`.
  If it is, the entire option block is aborted and evaluates to `null`.
  This simplifies consecutive null handling by avoiding verbose `switch` expressions.

  For example, the expression `do? { f(x!, y!) + z!.a }` evaluates to `null` if either `x`, `y` or `z` is `null`;
  otherwise, it takes the options' contents and ultimately returns `?r`, where `r` is the result of the addition.

* BREAKING CHANGE (Minor):
  The light-weight `do <exp>` form of the recently added, more general `do <block-or-exp>` form,
  is no longer legal syntax.
  That is, the argument to a `do` or `do ?` expression *must* be a block `{ ... }`,
  never a simple expression.

## 0.5.3 (2020-12-10)

* Nothing new, just release moc.js to CDN

## 0.5.2 (2020-12-04)

* Bugfix: gracefully handle importing ill-typed actor classes

## 0.5.1 (2020-11-27)

* BREAKING CHANGE: Simple object literals of the form `{a = foo(); b = bar()}`
  no longer bind the field names locally. This enables writing expressions
  like `func foo(a : Nat) { return {x = x} }`.

  However, this breaks expressions like `{a = 1; b = a + 1}`. Such object
  shorthands now have to be written differently, e.g., with an auxiliary
  declaration, as in `let a = 1; {a = a; b = a + 1}`, or by using the "long"
  object syntax `object {public let a = 1; public let b = a + 1}`.

## 0.5.0 (2020-11-27)

* BREAKING CHANGE: Free-standing blocks are disallowed

  Blocks are only allowed as sub-expressions of control flow expressions like
  `if`, `loop`, `case`, etc. In all other places, braces are always considered
  to start an object literal.

  To use blocks in other positions, the new `do <block>` expression can be
  used.

  The more liberal syntax is still allowed for now but deprecated, i.e.,
  produces a warning.

* BREAKING CHANGE: actor creation is regarded as asynchronous:

  * Actor declarations are asynchronous and can only be used in asynchronous
    contexts.
  * The return type of an actor class, if specified, must be an async actor
    type.
  * To support actor declaration, the top-level context of an interpreted
    program is an asynchronous context, allowing implicit and explicit await
    expressions.

  (Though breaking, this change mostly affects interpreted programs and
  compiled programs with explicate actor class return types)

* Candid support is updated to latest changes of the Candid spec, in particular
  the ability to extend function with optional parameters in a backward
  compatible way.

  Motoko passes the official Candid compliance test suite.

* RTS: Injecting a value into an option type (`? <exp>`) no longer
  requires heap allocation in most cases. This removes the memory-tax
  of using iterators.

* Bugfix: Passing cycles to the instantiation of an actor class works now.

* Various bug fixes and documentation improvements.

## 0.4.6 (2020-11-13)

* Significant documentation improvements
* Various bugfixes
* Improved error messages
* Initial DWARF support
* Candid compliance improvements:
  * Strict checking of utf8 strings
  * More liberal parsing of leb128-encoded numbers
* New motoko-base:
  * The Random library is added

## 0.4.5 (2020-10-06)

* BREAKING CHANGE: a library containing a single actor class is
  imported as a module, providing access to both the class type and
  class constructor function as module components. Restores the
  invariant that imported libraries are modules.
* Backend: Compile captured actor class parameters statically (#2022)
* flip the default for -g (#1546)
* Bug fix: reject array indexing as non-static (could trap) (#2011)
* Initialize tuple length fields (#1992)
* Warns for structural equality on abstract types (#1972)
* Funds Imperative API (#1922)
* Restrict subtyping (#1970)
* Continue labels always have unit codomain (#1975)
* Compile.ml: target and use new builder call pattern (#1974)
* fix scope var bugs (#1973)

## 0.4.4 (2020-09-21)

* Actor class export
* Accept unit installation args for actors
* Reject platform actor (class) programs with additional decs
* Handle IO exceptions at the top-level
* RTS: Remove duplicate array and blob allocation code
* RTS: Fix pointer arithmetic in BigInt collection function

## 0.4.3 (2020-09-14)

* Preliminary support for actor class import and dynamic canister installation.
  Surface syntax may change in future.
* BREAKING CHANGE: a compilation unit/file defining an actor or actor class may *only* have leading `import` declarations; other leading declarations (e.g. `let` or `type`) are no longer supported.
* Rust GC

## 0.4.2 (2020-08-18)

* Polymorphic equality.  `==` and `!=` now work on all shareable types.

## 0.4.1 (2020-08-13)

* Switching to bumping the third component of the version number
* Bugfix: clashing declarations via function and class caught (#1756)
* Bugfix: Candid `bool` decoding rejects invalid input (#1783)
* Canisters can take installation arguments (#1809)
  NB: Communicating the type of the canister installation methods is still
  missing.
* Optimization: Handling of `Bool` in the backend.

## 0.4 (2020-08-03)

* Candid pretty printer to use shorthand when possible (#1774)
* fix candid import to use the new id format (#1787)

## 0.3 (2020-07-31)

* Fixes an issue with boolean encoding to Candid
* Converts the style guide to asciidocs

## 0.2 (2020-07-30)

* The `Blob` type round-trips through candid type export/import (#1744)
* Allow actor classes to know the caller of their constructor (#1737)
* Internals: `Prim.time()` provided (#1747)
* Performance: More dead code removal (#1752)
* Performance: More efficient arithmetic with unboxed values (#1693, #1757)
* Canister references are now parsed and printed according to the new
  base32-based textual format (#1732).
* The runtime is now embedded into `moc` and need not be distributed separately
  (#1772)

## 0.1 (2020-07-20)

* Beginning of the changelog. Released with dfx-0.6.0.
