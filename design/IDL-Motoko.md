The IDL-Motoko integration
===============================

## Goals

This document specifies the integration of the IDL with the Motoko
language, in particular:

 * How to translate between an Motoko `actor` type and an IDL `service`
   description in both ways (i.e. _exporting_ IDL from Motoko, and _importing_
   IDL into Motoko), and how to translate between the values of these types.

 * The supported work-flows, including a sketch of the involved tools.

We try to achieve the following goals (but do not achieve them completely)

 * We can round-trip all Motoko values of sharable type. More precisely:

   When exporting an Motoko type `ta` into an IDL type `ti`, then
   round-tripping a value `va : ta` through `ti` yields a value that is
   indistinguishable from `va` at type `ta`.

 * Motoko can receive all IDL types: The type export has an inverse, the
   type import mapping, which is injective (up-to IDL type equivalence via
   shorthands).

 * Motoko can receive all IDL values when importing an IDL:

   When importing an IDL type `ti` into `ta`, then every IDL value `vi : ti`
   will successfully translated to an Motoko value of type `ta`.

The following are not necessary true:

 * The type export mapping is not total: there are Motoko types that cannot
   be exported, such as mutable arrays.

 * The type export mapping is not injective: there may be different
   Motoko types that map to the same IDL type, e.g. `Char`, `Nat32` and
   `Nat32`.

   This implies that round-tripping an Motoko type via the IDL can yield
   different types.

 * The type export mapping is not surjective: there are Candid types that
   cannot be imported, in particular types with `service` types with methods
   names that are no valid identifiers in Motoko.

 * For some types, not all IDL values may be accepted: for example, `Char` is
   may be exported as `nat32`, but not all `nat32` values can be read as
   `Char`. This can only be the case for types not in the image of the type
   import mapping.

NB: These properties (and non-properties) are not specific to Motoko, and
we expect that they will hold similarly for interfaces to other typed languages
with seamless serialization (e.g. built-in, macro or type class based
integration). In this sense, this document serves as a blueprint. Untyped
languages or languages with a code-generation workflow may have a different
story.

## The type mappings

We define
 * a partial function `e` from Motoko types to IDL types.
   Types that are not in the domain of `e` cannot be exported.
 * a partial function `i` from IDL types to Motoko types.
   Types that are not in the domain of `i` cannot be imported.

These definition treats Motoko types and IDL types as structural and
infinite; a concrete implementation will have to look through type constructors
in Motoko and introduce type definitions in the IDL as necessary.

It assumes that the IDL short-hands (e.g. named or anonymous fields)
are part of the grammar of types, and that `i` is allowed to make difference
choices for types that are short-hands.

The function is defined with regard to the grammars in [IDL.md](IDL.md) and [Syntax.md](Syntax.md).

### Type export

```
e : <typ> -> <datatype>
e(Null) = null
e(Bool) = bool
e(Nat) = nat
e(Int) = int
e(Nat<n>) = nat<n> for n = 8, 16, 32, 64
e(Int<n>) = int<n> for n = 8, 16, 32, 64
e(Float32) = float32
e(Float) = float64
e(Char) = nat32
e(Text) = text
e(Blob) = blob
e(Principal) = principal
e({ <typ-field>;* }) = record { ef(<typ-field>);* }
e(variant { <typ-field>;* }) = variant { ef(<typ-field>);* }
e([<typ>]) = vec (e(<typ>))
e(? <typ>) = opt (e(<typ>))
e(shared <typ1> -> <typ2>) = func (efn(shared <typ1> -> <typ2>))
e(actor { <typ-field>;* }) = service { em(<typ-field>);* }
e( () ) = null
e( ( <typ>,+ ) ) = record { e(<typ>);+ }
e(Any) = reserved
e(None) = empty

ef : <typ-field> -> <fieldtype>
ef (<id> : <typ>) = unescape(<id>) : e(<typ>)

efn : <typ> -> <functype>
efn(shared <typ> -> ()) = ea(<typ>) -> () oneway
efn(shared query? <typ1> -> async <typ2>) = ea(<typ1>) -> ea(<typ2>) query?

ea : <typ> -> <argtype>,*
ea( ( <typ>,* ) ) = e(<typ>);*
ea(<typ>) = ( e(<typ>) )  otherwise

em : <typ-field> -> <methtype>
em(<id> : <typ>) = unescape_method(<id>) : efn(<typ>)

unescape : <id> -> <nat>|<name>
unescape("_" <nat> "_") = <nat>  if <nat> is 32-bit
unescape(<id> "_") = <id>
unescape(<id>) = <id>

unescape_method : <id> -> <name>
unescape_method(<id> "_") = <id>
unescape_method(<id>) = <id>
```

### Type import

```
i : <datatype> -> <typ>
i(null) = Null
i(bool) = Bool
i(nat) = Nat
i(int) = Int
i(nat<n>) = Nat<n> for n = 8, 16, 32, 64
i(int<n>) = Int<n> for n = 8, 16, 32, 64
i(float32) = Float32
i(float64) = Float
i(text) = Text
i(reserved) = Any
i(empty) = None
i(opt <datatype>) = ? i(<datatype>)
i(vec <datatype>) = [ i(<datatype>) ]
i(blob) = Blob
i(record { <datatype>;^N }) = ( i(<datatype>),^N ) if n > 1 // matches tuple short-hand
i(record { <fieldtype>;* }) = { if(<fieldtype>);* }
i(variant { <fieldtype>;* }) = variant { ivf(<fieldtype>);* }
i(func <functype>) = ifn(<functype>)
i(service { <methtype>;* }) = actor { im(<methtype>);* }
i(principal) = Principal

if : <fieldtype> -> <typ>
if(<name> : <datatype>) = escape(<name>) : i(<datatype>)
if(<nat> : <datatype>) = escape_number(<nat>) : i(<datatype>) // also for implicit labels

ivf : <fieldtype> -> <typ>
ivf(<name> : null) = escape(<name>) : ()
ivf(<nat> : null) = "_" <nat> "_": ()
ivf(<fieldtype> = if(<fieldtype>) otherwise

ifn : <functype> -> <typ>
ifn((<datatype>,*) -> () oneway) = shared ia(<datatype>) -> ()
ifn((<datatype1>,*) -> (<datatype2>,*) query?) = shared query? ia(<datatype1>,*) -> async ia(<datatype2>,*)

ia : <argtype>,* -> <typ>
ia(<argtype>,) = i(<argtype>)
ia(<argtype>,*) = ( i(<argtype>),* )  otherwise

im : <methtype> -> <typ>
im(<name> : <functype>) = escape_method(<name>) : ifn(<functype>)

escape_number <nat> = "_" <nat> "_"

escape : <name> -> <id>
escape <name> = <name> "_"  if <name> is a reserved identifier in Motoko
escape <name> = <name> "_"  if <name> is a valid Motoko <id> ending in "_"
escape <name> = <name>  if <name> is a valid Motoko <id> not ending in "_"
escape <name> = escape_number(hash(<name>))  otherwise

escape_method : <name> -> <id>
escape_method <name> = <name> "_"  if <name> is a reserved identifier in Motoko
escape_method <name> = <name> "_"  if <name> is a valid Motoko <id> ending in "_"
escape_method <name> = <name>  if <name> is a valid Motoko <id> not ending in "_"
escape_method <name> = (* failure, unsupported *)
```

### Notes:

 * Up-to short-hands, `i` is injective and the right-inverse of `e`.

   Formally: For all IDL types `t ∈ dom i`, we have that `e(i(t))` is equivalent to
   `t`, i.e. either they are the same types, or short-hands of each other.

 * Non-empty tuples are exported using the unnamed field short-hand, which is how tuples
   are idiomatically expressed in the IDL:
   ```
   e((Int, Nat)) = record {int;nat}
   e({i:Int, n:Nat}) = record {i:int; n:nat}
   e({_0_:Int, _1_:Nat}) = record {0:int; 1:nat}
   ```

 * The mapping `i` tries to detect types that can be expressed as
   tuples in Motoko.
   ```
   i(record {int;nat}) = (Int, Nat)
   i(record {int; nat; foo:text}) = {_0_:Int; _1_:Nat; foo:Text}
   i(record {0:int, 1:nat}) = {_0_:Int; _1_:Nat}
   ```

   But note that

   * `i(record {}) ≠ ()` because `e(()) = null` and we want
     `e(i(record {})) = record {}`.

   * `i(record {int}) ≠ (int,)` because we do not have unary tuples in AS.
     Instead, `i(record {int}) = { _0_ : int}` so that `e(i(record {int})) =
     record {int}`.

 * The `escape` and `unescape` functions allow round-tripping of IDL field
   names that are not valid Motoko names (fake hash values):
   ```
   i(record {int; if:text; foobar_:nat; "_0_":bool})
     = {_0_:Int; if_:Text; _1234_:Nat; _4321_:Bool}
   ```
   This is another source of partiality for `e`:
   ```
   e({clash_ : Nat; clash : Int})
   ```
   is undefined, because `unescape(clash_) = unescape(clash)`.

 * Similarly, the `escape_method` and `unescape_method` functions append `_` to
   method names in Candid that happen to be reserved keywords in Motoko.

   Candid method names that are _not_ valid identifiers in Motoko are
   unsupported.

 * Motoko functions with type parameters are not in the domain of `e`.

 * Abstract Motoko types are not in the domain of `e`

 * The translation produces IDL functions without parameter names.  But both
   the IDL and Motoko conveniently support non-significant  names in
   parameter lists. These are essentially comments, and do not affect, for
   example, the type section in a message, so it is not necessary to specify
   them here.

   But tooling (e.g. `moc` exporting an IDL from an Motoko type) is of
   course free to use any annotations in the Motoko type (or even names
   from pattern in function definitions) also in the exported IDL.

 * The soundness of the Motoko type system, when it comes to higher-order
   use of actor and function references, relies on
   ```
   ∀ t1 t2 : dom(e), t1 <: t2 ⟹ e(t1) <: e(t2)
   ```
   In other words: Motoko subtyping must be contained in IDL subtyping.

 * The functions `escape`/`unescape` ensure round-tripping of IDL field names
   through Motoko types. See `IDL-Motoko.proofs.md` for details.

## The value mappings

For each Motoko type `t` in the domain of `e`, we need mapping from
Motoko value of type `t` to an IDL value of type `e(t)`, and vice-versa.

Note that decoding may only fail for those `t` that are not in the range of `i`.

These mappings should be straight-forward, given the following clarifications:

* Characters (of type `Char`) are mapped to their Unicode scalar as a `nat32`.
  Decoding a `nat32` that is not a valid Unicode scalar fails.

## Type name mangling

The name of type definition or services are irrelevant with regard to whether
the resulting imported/exported types are correct, as both Motoko and IDL
employ structural typing. Nevertheless, when the type export or import has to
produce such identifiers, it tries to preserve the original name. This is a
best effort approach.

If it happens that such a name is invalid (e.g. reserved) in the target
language, a `"_"` is appended during translation.

Conversely, a name of the form `<id> "_"` is translated  to `<id>`, if the
latter is legal in the target language.

## Work flows

The mapping specified here can be used to support the following use-cases. The
user interfaces (e.g. flag names, or whether `moc`, `didc`, `dfx` is used) are
just suggestions.

* Generating IDL from Motoko

  If `foo.mo` is an Motoko `actor` compilation unit, then

      moc --idl foo.mo -o foo.did

  will type-check `foo.mo` as `t = actor { … }`, map the Motoko type `t`
  to an IDL type `e(t)` of the form `service <actortype>`, and produce a
  textual IDL file `foo.did` that ends with a `service n : <actortype>`,
  where `n` is the name of the actor class, actor, or basename of the source
  file.

* Checking Motoko against a given IDL

  If `foo.mo` is an Motoko `actor` compilation unit and `foo.did` a
  textual IDL file, then

      moc --check-idl foo.did foo.mo -o foo.wasm

  will import the type service `t_spec` specified in `foo.did`, using the
  mapping `i`, will generate an IDL type `e(t)` as in the previous point, and
  and check that `e(t) <: t_spec` (using IDL subtyping).

* Converting IDL types to Motoko types

  If `foo.did` a textual IDL file, then

      didc foo.did -o foo.mo

  will create an Motoko library unit `foo.mo` that consists of type
  definitions.
  All `<def>`s and the final `<actor>` from `foo.did` is turned into a `type`
  declaration in Motoko, according to `i`.
  Imported IDL files are recursively inlined.

  Variant: Imported IDL files are translated separately and included via
  `import` in Motoko.

* Importing IDL types from the Motoko compiler

  If `path/to/foo.did` a textual IDL file, then a declaration

      import Foo "path/to/foo"

  is treated by `moc` by reading `foo.did` as if  the developer had
  run `didc path/to/foo.did -o path/to/foo.mo`.
