# Motoko Evolution White Paper (Draft)

(Note: The status of this document still is very rough. It has more the character of a brain dump. Provided as is.)


## Background

### Introduction

*Motoko* is a general-purpose programming language designed to provide a seamless and modern programming experience on the Internet Computer (IC). To that end, it has built-in support for various features specific to the IC, such as:

* Representation of canisters as *actors* with asynchronous and atomic methods.
* Automatic integration with *Candid* for implicitly de/serialising message arguments and automatic derivation of Candid interfaces.
* *Async/await* support for straight-line coding of asynchronous messaging patterns.
* *Orthogonal persistence* keeping program state alive across messages automatically.
* *Stable variables* for persisting selected program state even across program version upgrades.
* Built-in support for other IC mechanisms, such as cycles, upgrades, heartbeat, etc (some of it still incomplete).

Motoko was initially designed and implemented as a relatively minimal language. This was both to keep it simple and to get off the ground more quickly. Consequently, there is a range of features it still lacks. Some are mere implementation gaps relative to the "complete" semantics provided by the interpreter, some are additional features that were anticipated from the beginning but consciously deferred, some have been discovered as new requirements over time.

The purpose of this document is to lay out the main directions of key features envisioned as future additions. None of these should be expected to materialise immediately. However, taken together, this document ought to provide a decent idea of the intended direction for the language, medium to longer-term.

This document focusses on the language itself. Implementation, tooling, and documentation improvements are a separate discussion altogether and not covered in this document. Likewise, it does not say much about libraries, except where they provide basic primitives of the language.


### Design Philosophy

Motoko tries to follow a set of design principles:

* *Safe.* The language should always default on the side of safety, encourage good code more than bad code, and avoid foot guns and unsafe or error-prone behaviours, unless explicitly requested by the programmer. The type system must be sound.

* *Simple.* The language should avoid complicated, hard-to-understand, or overly bleeding-edge features. It should minimise multiple redundant or overlapping ways to express the same thing (unless one can be expressed as syntactic sugar for another).

* *Understandable.* Where possible, the language should pick syntax and terminology that is intuitive and familiar from mainstream languages. It should avoid niche technical or theoretical jargon.

* *High-level.* The language should be productive and convenient to use, sufficiently expressive, and not bother the programmer with irrelevant details or error-prone routine.

* *Principled.* Both syntax and semantics should be based on well-behaved and well-understood, minimal and orthogonal, mechanisms and constructs and avoid ad-hoc behaviours, extensions, special cases, or leakage of implementation choices. Commonly expected meta-theoretical properties should hold for the language.

* *Definable.* It should be possible to specify the language in a declarative manner, using standard formal methods, without jumping through hoops. The language must never be "defined by implementation", meaning that there would be no way to specify it except by replicating implementation details of the compiler.

Oftentimes, these goals are difficult to achieve fully or can be in conflict with each other. Hence following them sometimes is a judgement call. However, none of these principles should be sacrificed completely.


### Main Concepts

* *Primitive types.* Motoko provides the usual set of built-in data types, such as various number types, Booleans, and text strings. Integral numbers have infinite precision by default, to prevent overflow bugs. Natural numbers are a restricted subtype of integers. Text strings are always assumed to be in Unicode, but without exposing a specific encoding. Motoko also provides (mutable or immutable) arrays and option types, the latter representing "nullable" types (of any form), but in a way that composes properly and enforces handling the null case where necessary.

* *Objects.* Motoko features a simple object and class system. It provides interface inheritance (via structural subtyping), but no implementation inheritance or downcasts. The semantic model for objects is essentially records of closures (closing over the private fields in particular). A class is simply a syntactic shorthands for a function returning an object, plus a type definition of the same name.

* *Actors.* The actor system is designed to be in 1-to-1 correspondence with the object system. Actors are essentially a restricted version of objects, with only asynchronous (and shared) methods. Actor classes look like classes, but producing an actor instead of an object. Typing is similarly symmetric.

* *Futures.* Expressions can be asynchronous, in which case their immediate result is just a future (a value of type `async T`), whose value is determined later. The value can only be accessed through the await operator, which in turn is only allowed inside another asynchronous expression. It suspends evaluation of that expression until the value is available, allowing interleaving of other computations in the same actor. It also commits all state changes performed thus far. Asynchronous functions are simply a shorthand for functions whose body is an asynchronous expression.

* *Functional constructs.* At its heart, Motoko has all features of a modern functional language, such as first-class functions (a.k.a. closures), tuples, algebraic data types (a.k.a. variants and records), polymorphism (a.k.a. generics), checked pattern matching (in the form of a generalised `switch`), and the fact that there are no statements but everything is an expression.

* *Modules.* Motoko also features a simple module system, where modules are essentially a restricted form of object that must be stateless. Modules can contain type definitions, but not (currently) abstract them, i.e., hide their implementation. A Motoko compilation unit is either a module, or a top-level actor or actor class definition that can be imported as if it was contained in a module.

* *Canonical type system.* Motoko's type system, at its core, is a fairly canonical typed lambda calculus with primitive types, labeled products and sums (i.e., objects and variants), first-class function types, polymorphism (i.e., generics), and structural recursive subtyping with top and bottom types. Type theory experts will recognise the close similarity to (Kernel) System F\<:. Type checking mostly follows a strict bidirectional discipline, with only a few exceptions regarding optional inference of polymorphic type instantiation, overloaded operators, and multiple passes to account for recursive declarations.

* *Shared types.* Motoko distinguishes a subset of types that are "sharable". These are those types whose values can be serialised, compared, or otherwise be inspected transparently. They include primitive types, immutable aggregates (arrays, objects, variants) whose components are shared, and references to actors or shared functions.


## Future Extensions

The following sections present various ideas of varying level of complexity, in no particular order and with no particular priorities. Some of them are fleshed out more concretely, while in some cases the details are not obvious. Where applicable, respective tracking issues in the Motoko repository are included, which sometimes contain further discussion.

In some cases, the description contains partial sketches of a formalisation. This can be skipped by readers not interested in the inner workings of a compiler.


### More Checking

Motoko should help detecting common sources of bugs, especially those of the subtle kind. To that end, the language ought to add more checking mechanisms.


#### Unused Identifiers ([#1489](https://github.com/dfinity/motoko/issues/1489))

It often is a symptom of a bug that an identifier is defined but never used. To diagnose these, the compiler should warn about unused identifiers (values, types, or otherwise).

On the other hand, it is occasionally useful to introduce unused identifiers for documentation purposes, e.g., unused function parameters, pattern components, etc.. To that end, an exception is made for identifiers starting with `_`, which are not warned if unused. This way, a programmer can still document the role of a binding, and simultaneously document that it is intentionally unused.

The `_` prefix convention is natural and common in languages that provide pattern matching and already use `_` for anonymous patterns, like Motoko.


#### Atomicity Violations ([#792](https://github.com/dfinity/motoko/issues/792))

Following the actor model, method execution is atomic in Motoko. However, this is only true up to uses of `await`. Semantically, every await splits execution of a method into separate actor-to-actor messages that can be interleaved by other calls. These calls may modify the state, opening up the possiblity of subtle reentrancy bugs if computations after an await depend on state read before the await. For example:
```
actor {
  var list : List<Nat> = null;
  public put(x : Nat) { list := List.push(x, list) };
  public average() : async Nat {
    let size = List.size(list);
    if (size == 0) return 0;
    let cap = await other.cap_value();
    var a = 0;
    List.iterate(list, func(n) {a := a + min(n, cap)});
    return a/size;
  }
}
```
Here, `size` is computed before calling out to `other.cap_value` and awaiting the result. Because of the await, another call to `put` may happen in the meantime, resulting in an incorrect average computation.

The async/await construct, and the ability to suspend a method with it, undermines one of the core properties of the actor model, namely the fact that methods are fully atomic. On the other hand, the practical convenience of async/await is so substantial that it still is a worthwhile trade-off.

To eliminate the disadvantage, the Motoko compiler could detect that there is a dependency on the current state of `list` across an await and warn.

However, sometimes such dependencies are necessary. So there needs to be a way for the programmer to declare intent and prevent the warning. Unfortunately, it is not obvious what a clean and general solution to this problem would be.

A related issue is that `await` also represents a commit point: if execution traps, the current method is aborted, but only up to the last await. Any state changes or messages sent before the last await will remain effective, potentially leaving the actor with inconsistent state. Consider:
```
actor {
  var list : List<Nat> = null;
  var len : Nat = 0;
  public putLogged(x : Nat) : async () {
    let := len + 1;
    let y = await other.process(x);
    list := List.push(y, list);
    log("old head: " # Nat.toText(List.head(List.head(list))));
  }
}
```
Here, if the logging attempt fails because the original list was empty, then the mutation of `list` will be abandoned, but not the mutation of `len`, because that happened before the `await`, and thus has already been committed to the actor's state.

There is no easy general solution to the problem – that would require the implementation of distributed transactions on the level of the IC, which are notoriously hard and expensive. But again it might be desirable to warn against accidental state mutations before an await, unless the programmer declares intent explicitly – and thereby, presumably, was aware of the implications.


### Primitive Types

Primitive types in Motoko still lack basic functionality.

#### Number Conversions

Currently, converting between different number types requires going through `Nat` or `Int` in most cases:
```
func nat8ToNat16(n : Nat8) : Nat16 { Nat18.fromNat(Nat8.toNat(n)) }
```
Obviously, that's both inconvenient and expensive. The base package should directly support the complete conversion matrix for all number types (nats, ints, and floats).


#### Blobs

Blobs are essentially arrays of bytes, but they currently lack basic array functionality, like random access. That ought to be made available analogously to regular arrays.


#### Text

All text strings are in Unicode in Motoko. But there is no proper support for Unicode-aware text processing. For example, there are no Unicode-enabled comparison operators, and no Unicode character classification or conversion functions.

Adding these requires including Unicode libraries and tables into the generated Wasm code, which can easily be several MB in size. Presumably, supporting Unicode would hence depend on some tree shaking ability of the compiler.

Another shortcoming in text processing is the inability to efficiently extract substrings. Motoko provides iterators over strings, but they cannot be used as arguments to other operations, because their type is structural. A solution to this is needed, e.g., by extending text iterator objects in some way. Details tbd.

Finally, it would sometimes be desirable to have extended syntax for multi-line text literals, ideally with interpolation, perhaps based on a generic `toText` function (see [shared generics](#shared-generics)).


### Type System

The Motoko type system has a number of known gaps that stand in the way of certain forms of abstraction and composition.


#### Newtypes

Motoko supports `newtype` declarations that introduce a nominal wrapper around an existing type.
Unlike `type` aliases (which are structurally transparent), a `newtype` is fully opaque: no implicit conversion exists between the newtype and its underlying representation.

```
newtype Time = Int;
let t : Time = Time(123);    // explicit wrapping via constructor
let n : Int = t.unwrap;      // explicit unwrapping via .unwrap
```

The declaration introduces both a type `Time` and a value `Time` — the constructor function of type `Int -> Time`.

Newtypes can have type parameters.
Variance of each type parameter is computed from its usage in the right-hand side, consistent with how `type` aliases work:

```
newtype Map<K, V> = MapInternals<K, V>;
let m : Map<Text, Nat> = Map<Text, Nat>(internal);
let i = m.unwrap;  // : MapInternals<Text, Nat>
```

##### Semantics

- **Opacity.** A newtype `T` and its underlying type `U` are distinct for subtyping and equality. `T </: U` and `U </: T`. Values must be explicitly wrapped/unwrapped.
- **Zero-cost.** At runtime, newtype values have the same representation as the underlying type. The constructor and `.unwrap` are identity operations.
- **Nominal.** Two newtypes with the same underlying type are distinct: `newtype A = Int` and `newtype B = Int` produce incompatible types.
- **Constructor.** The newtype name is bound as both a type and a value (the constructor function). For `newtype T<A> = U<A>`, the constructor has type `<A>(U<A>) -> T<A>`.
- **Unwrap.** Values of newtype are given a virtual `.unwrap` field (analogous to how arrays have `.size`). For a value `v : T<A>`, `v.unwrap : U<A>`.

##### Candid Serialization

Newtypes are a Motoko-only concept. When serializing to Candid, newtypes are transparent: `Time` serializes as `Int`, `Map<K, V>` serializes as its underlying record type.

##### Stable Storage and Migrations

For `.most` files (stable type signatures), newtypes are treated like type aliases. This allows users to freely add, rename, or remove newtypes across upgrades without breaking migration compatibility.

Newtypes are preserved in error messages (e.g., `Time` rather than `Int`) for better diagnostics.

##### Implementation

The `newtype` declaration introduces a new type constructor kind `Newtype(binds, body)` alongside the existing `Def` (transparent alias) and `Abs` (abstract/parameter). Key design choices:

- **`normalize`** does *not* expand `Newtype`, preserving opacity during type checking. This is the fundamental difference from `Def`.
- **`promote`** does *not* expand `Newtype`, preventing implicit promotion in subtyping contexts.
- **Lowering.** During desugaring, `Newtype` kinds are mutated to `Def`, making them transparent for downstream IR passes and codegen. The constructor is emitted as an identity function binding.

##### Future Work

- Pattern matching on newtypes.
- Methods on newtypes.


#### Type Fields ([#760](https://github.com/dfinity/motoko/issues/760))

Motoko allows type members in objects and modules:
```
object o {
  public type T = Nat;
  public let n : T = 0;
};
let n : o.T = o.n;
```
However, there is no way to actually express the type of `o` itself in Motoko's surface syntax, since type fields are not allowed in object _types_.

The following syntax should be available:
```
type O = {
  type T = Nat;
  n : Nat;
}
```
One delicate question here is whether it should be allowed to refer to a type field like `T` from within the record type itself – e.g., whether declaring `n : T`should be allowed to define the second field. The rules should be consistent with the scoping of value fields in plain record syntax, where
```
let x = 0;
let o = {x = 1; y = x};
```
has `x` referring to the outer binding. Consequently, the rules should be the same for types, and `n : T` not be allowed.

Note that all type definitions in an object are still transparent. Furthermore, in order to express the type of objects with recursive type fields, [anonymous recursive types](#anonymous-recursive-types) are also needed.

##### Formalisation

Syntax:
```
<typfld> ::= ... | type <id> < <typbind>,* >? = <typ>
```

Kinding:
```
(C,typbind* ⊢ typbind ok)*
C,typbind* ⊢ typ : Ω
-----------------------------------
C ⊢ type X<typbind*> = typ : X:Ω*→Ω
```


#### Anonymous Recursive Types

In Motoko, types can be recursive or mutually recursive. However, recursive types can only be written as named declarations. It is not possible to write an _anonymous_ recursive type inline, e.g., directly in a type annotation.

That is a problem when there is no place to put a type declaration, or when the compiler has to produce a type (e.g., as part of an error message) in a place where the respective type declarations are no longer in scope. For example:
```
let f = do {
  type A = {#aa; #ab : B};
  type B = {#bb; #ba : A};
  func(x : A) {...};
}
```
Here, the type of `f` can only be expressed in reference to types `A` and `B`, but those are not accessible outside the `do`.

The solution is to add recursive type expressions. In analogy to functions and other term-level recursive expressions, one way to do this would be by allowing type declarations as type expressions:
```
<typ> ::= ... | type <id>? =? <typ>
```
The optional identifier in this phrase is only available within the right-hand side type `<typ>`, and allows recursive references of that type to itself (if omitted, the syntax has the same meaning as plain `<typ>`).

With that, the type of `f` can be written as:
```
f : (x : type A {#aa; #ab : {#bb; #ba : A}}) -> ()
```

##### Formalisation

Syntax:
```
<typ> ::= ... | type <id>? < <typbind>,* >? =? <typ>
```

Kinding:
```
(C,typbind* ⊢ typbind ok)*
C,X:Ω*→Ω,typbind* ⊢ typ : Ω
typ productive
typ uniform wrt X<typbind*>
-------------------------------
C ⊢ type X<typbind*> T : Ω* → Ω
```

Equivalence:
```
C,X<typbind*>≡typ,typbind* ⊢ typ ≡ typ'
=======================================
C ⊢ type X<typbind*> typ ≡ typ'
```


#### Open Type Aliases

Type definitions are currently restricted in that they can only refer to their own type parameters, not to outer ones:
```
func f<A>(x : A) {
  type M<B> = HashMap<A, B>;  // error, can't refer to A
  ...
}
```
The reason for this restriction are mostly technicalities in the compiler and should be lifted.


#### Shared Generics ([#2096](https://github.com/dfinity/motoko/issues/2096))

Motoko defines the notion of _shared_ types. As mentioned above, these are the types whose values can be serialised, compared, or otherwise be inspected transparently.

Unfortunately, type _parameters_ are never deemed shared, because they can be instantiated arbitrarily. That prevents certain useful abstractions:
```
type Id<A> = shared (x : A) -> async A
```
Here, the use of `A` inside a shared function type will be rejected, because it cannot be guaranteed that all instantiations of `F` pick a shared type.

More interestingly, this prevents writing generic abstractions that require values of arbitrary shared type, e.g., to send a message:
```
func sendTwice<A>(x : A, f : shared A -> ()) {
  f(x);
  f(x);
}
```

Allowing this requires expressing a constraint on the type parameter:
```
type Id<shared A> = shared (x : A) -> async A

func sendTwice<shared A>(x : A, f : shared A -> ()) {
  f(x);
  f(x);
}
```

With that, any type parameter can be marked shared. If present, the bound of a shared parameter must itself be a shared type.

This extension is easy to implement for type definitions, but much more involved for generic functions or classes: to implement `sendTwice`, the value `x` must be serialised with type `A`, but that type is not known statically. Hence, every shared type parameter on a generic function or class will require a runtime type representation that the de/serialiser can interpret at runtime. This requires extensions to calling conventions, generic instantiation, as well as the implementation of de/serialisation itself.

On the plus side, the same mechanism can then be used to express any abstractions over shared types. For example, it would be possible to use equality over generic types:
```
func contained<shared A>(x : A, ys : [A]) : Bool {
  for (y in ys.values()) { if (x == y) return true };
  return false;
}
```

It would also allow getting rid of (and generalising) the hack that currently is the `debug_show` keyword, and instead surface it as a proper generic library function:
```
toText : <shared A>(A) -> Text;
```

A possible further extension is providing the programmer to write functions of this sort themselves. That would require adding a form of type-switch for intensional type analysis to the language.
```
func myToText<shared A>(x : A) : Text {
  switch type A {
    case Nat { Nat.toText(x) };
    case Bool { if x "true" else "false" }
    case _ { "?" }
  }
}
```
The challenge would be designing this construct such that it works for variadic types like tuples, objects, or functions, let alone generic types.


##### Formalisation

Syntax:
```
<typbind> ::= shared? <id> (<: <typ>)?
```

Kinding:
```
C,shared X<:typ ⊢ typ : Ω
C,shared X<:typ ⊢ typ shared
----------------------------
C ⊢ shared X <: typ ok
```


### Core Language

#### General `do`-Expressions

Motoko recently introduced `do?` expressions to make handling and threading of option types more convenient. A similar shorthand is desirable for other types, e.g., the type `Result<A, B>`.

However, such a generalisation requires specifying how values are composed for a given type. In semantic terms, the general abstraction is that of a so-called _monad_. A type `M<A*>` in question needs to define two operations:
```
make : <A*>(x : T) -> M<A*>
chain : <B*,A*>(x : M<B*>, f : T -> M<A*>) : M<A*>
```
You will note that the Motoko library already contains these operations for relevant types.

With that, the `do?` expression can be generalised to a form
```
do <id> { ... }
```
where the identifier refers to a module defining the above operations for the type in question.

Roughly, such a `do`-expression desugars into an expression whose last step is a call to `<id>.make`. Each use of `!` will be translated into a call to `<id>.chain`, after CPS-transforming the remainder of the body. For example, assume
```
g : Nat -> Result<Text, Error>
```
Then the expression
```
do Result { f(g(1)!, g(2)!) }
```
becomes
```
Result.chain(g(1),
  func(x) {
    Result.chain(g(2),
      func(y) {
        Result.make(f(x, y))
      }
    )
  }
)
```
Consequently, if `g(1)` produces an error result, that is the result of the whole expression. Otherwise, if `g(2)` produces an error result, that is the result of the whole expression. Otherwise, the result is `#ok(f(x, y))`.

With this generalisation, the expression `do?` effectively becomes a shorthand for `do Option`.

It would be ideal if `async/await` could be interpreted as an instance of this construct as well, though that may be difficult to achieve due to implementation constraints.

##### Formalisation

Syntax:
```
exp_nondec ::= ... | do <exp_nullary> <exp_block>
```

Typing:
```
C ⊢ exp1 :syn T1
C,do:T1 ⊢ exp2 :syn T2
T1 = { make : <A*>(T2') -> T; ... }
C ⊢ T2' <:infer T2[U*/A*]
-----------------------------------
C ⊢ do exp1 exp2 :syn T[U*/A*]

C ⊢ exp :syn T1
C(do) = { chain : <A*>(T1', T1'' -> T'') : T'; ... }
C,A* ⊢ T1' <: T1''
C,A* ⊢ T'' <: T'
C ⊢ T1 <:infer T1'[U*/A*]
-------------------------
C ⊢ exp! :syn T'[U*/A*]
```
The relation `<:infer` infers the rhs type substitution, as for implict type arguments in a generic call. The expression is ill-typed if it can't be inferred.

Challenge: One issue with the above is that `T1` does not generally determine all type parameters of `chain`, particularly the ones only occurring in the result type `T'`. It may be necessary to type-check `exp!` in analysis mode, though that could be inconvenient.


#### Bindings in `or`-Patterns ([#3807](https://github.com/dfinity/motoko/pull/3807))

When using `or`-patterns, it is possible to bind a variable in the alternatives. This is useful to avoid `case` repetitions:
```
func free(x : Text, e : exp) : Nat {
  switch e {
    case (#lit(_)) { 0 };
    case (#var(y)) { if (y == x) 1 else 0 };
    case (#add(e1, e2) or #sub(e1, e2)) { free(x, e1) + free(x, e2) };
  }
```
An or-pattern binding identifiers is well-formed only if both alternatives bind the same set of identifiers. The observed type of each identifier is the lub of the individual binding types.

##### Formalisation

Typing:
```
C ⊢ pat1 :syn T1 ⊣ C1
C ⊢ pat2 :syn T2 ⊣ C2
dom(C1) = dom(C2)
---------------------------------------
C ⊢ pat1 or pat2 :syn T1 ⊔ T2 ⊣ C1 ⊔ C2

C ⊢ pat1 :anal T ⊣ C1
C ⊢ pat2 :anal T ⊣ C2
dom(C1) = dom(C2)
----------------------------------
C ⊢ pat1 or pat2 :anal T ⊣ C1 ⊔ C2
```


#### `and`-Patterns

Analogous to disjunctive `or`-patterns, the language could also provide conjunctive `and`-patterns. they would be a generalisation of the notion of `as`-patterns present in other languages with pattern matching. Their main benefit is that they allow binding a name to a pattern while still matching it:
```
let (x, y) and p = f();
```
Here, `f` returns a pair that will be bound to `p`, while simultaneously, `x` and `y` will be bound to its components.

##### Formalisation

Syntax:
```
<pat_bin> ::= ... | <pat_bin> and <pat_bin>
```

Typing:
```
C ⊢ pat1 :syn T1 ⊣ C1
C ⊢ pat2 :syn T2 ⊣ C2
----------------------------------------
C ⊢ pat1 and pat2 :syn T1 ⊓ T2 ⊣ C1 ⊓ C2

C ⊢ pat1 :anal T ⊣ C1
C ⊢ pat2 :anal T ⊣ C2
-----------------------------------
C ⊢ pat1 and pat2 :anal T ⊣ C1 ⊓ C2
```


### Objects

#### Object Composition ([#3072](https://github.com/dfinity/motoko/issues/3072))

Currently, objects can only be constructed using object literals. It is not possible to construct an object in terms of another, preexisting object with some fields updated or added.

Some functional languages provide forms of _record update_, where a form of object literal is provided that replaces or adds individual fields of a pre-existing object. In Motoko, this could easily be generalised to object _merging_, akin to type intersection:
```
let o1 = {a = 1; b = 2};
let o2 = {a = 3; c = 4};
let o3 = o1 with o2;
```
Unlike type intersection, however, this operator cannot apply recursively. Instead, if the fields of both operands overlap, the right-hand side value takes precedence. Consequently, `o3` would result in the object `{a = 3; b = 2; c = 4}`.

If any of the fields are mutable, their state is duplicated in the new object – as an alternative design choice, they could become immutable; or it may be prohibited to merge stateful objects at all. Either way, the new object will only contain the public fields of the operands. It does _not_ duplicate private state. Likewise, functions/methods copied into the new object are not magically rewired to close over the new state, they'd still close over the original state unchanged.

One challenge is that for Motoko, such a mechanisms would be expected to symmetrically apply not just to regular objects, but also to actor or module objects. For example, this would allow to compose actors:
```
actor a {
  public func f() {}
}
let b = a with actor {
  public func g() {}
}
```
However, it is not clear how to implement this if an actor contains mutable state. Actor composition may only be possible via [mixin classes](#mixin-composition).


##### Formalisation

Syntax:
```
<exp_bin> ::= ... <exp_bin> with <exp_bin>
```

Typing:
```
C ⊢ exp1 :syn {(var? x1:T1)*; (var? x:T1')*}
C ⊢ exp2 :syn {(var? x2:T2)*; (var? x:T2')*}
x1* disjoint x2*
---------------------------------------------------------------------
C ⊢ exp1 with exp2 :syn {(var? x1:T1)*; (var? x2:T2)*; (var? x:T2')*}
```


#### Mixin Composition

Motoko does not provide implementation inheritance for classes. That is intentional: method overriding is a risky and error-prone mechanism, and even experienced programmers often use overridability carelessly, in ways that can easily allow subclasses break the assumed invariants of a class.

So far, there have been surprisingly few complaints about Motoko's lack of class inheritance.

However, should this become an issue, a more elegant and safer alternative is the introduction of a mechanism for _mixin-composition_, which is a form of generalisation of object merging.

Mixins are classes with _abstract_ fields. That is, some of their fields may not have a definition – they are "holes" in the object that are to be filled in later (similar to abstract methods in some OO languages, but not restricted to methods). The holes are filled by merging with another mixin or object that provides a definition for this hole. Only after all holes have been filled through composition, the result defines a proper class. For example:
```
abstract class A(x : Nat) {
  public abstract func f() : Nat;
  public func g() : Nat { f() + x }
}

class B(x : Nat) = A(x) with {
  public func f() : Nat { 7 }
}
```
Mixin composition thereby addresses similar use cases as implementation inheritance. But it only allows defining _abstract_ fields and methods, not overriding ones that were already defined. That way, it merely provides a way to express a form of (recursive) functional abstraction, not a stateful mechanism to "patch" an existing definition. That avoids most risks of general inheritance.

At the same time, mixin composition has the added advantage that it is symmetric, i.e., merging two mixins can define abstract fields in both directions simultaneously. For example:
```
abstract class A() {
  public abstract let x : Nat;
  public let y = 1;
  public func add() : Nat {x + y}
};
abstract class B() {
  public let x = 2;
  public abstract let y : Nat;
  public func mul() : Nat {x * y}
};
class C() = A() with B();
```
In this example, the `with` operator plugs together two abstract objects as if they are two jigsaw pieces. However, the composition may also leave some or all abstract fields alone, producing a new abstract class:
```
class Z() {
  public let zero = 0
};
abstract class D() = A() with Z();
```
Here, `D` still has an abstract field `x`.

Of course, a class can also compose more than 2 objects.

Mixin composition would also work fine for actor classes, and is more easily implementable than merging of actor objects:
```
abstract actor class Ping() {
  var x : Nat = 1;
  public func ping() : Nat { pong() + x };
  public abstract func pong() : Nat;
};
abstract actor class Pong() {
  var x : Nat = 2;
  public func pong() : Nat { ping() * x };
  public abstract func ping() : Nat;
};
actor class Tennis() = Ping() with Pong();
```

Obviously, however, mixins are a highly involved feature, and shouldn't be introduced light-heartedly. Many details are ignored above, and they induce significant complexity. For now the hope is that Motoko can get along without such a feature.


### Actors

Actors are Motoko's representation of IC canisters, but they were designed to allow more flexibility than just being static pieces of code.


#### Local Actors

As mentioned above, actors in Motoko are designed as a restricted form of object, where all public functions have to be defined as asynchronous and shared. Other than that, they ought to have mostly the same "rights" as regular objects. In particular, it is intended to be possible to define actors (or actor classes) in local scope, and to have them refer to definitions from outer scopes.

One crucial restriction, though, is that those outer definitions must be of shared types (and not `var` bindings), such that referencing them does not imply mutable state being shared across multiple actors, which would break the actor model.

One interesting special case of such an "outer" reference is a recursive reference to an actor or actor class itself, which technically lives in the scope surrounding it. For example:
```
actor class A(xx : Nat) {
  let x = xx;
  public func fork() : async A {
    await A(x + 1)
  }
}
```

Local actors and actor (class) closures are implemented in the Motoko interpreter, but they are not yet handled by the compiler. The main reason is that it is not obvious how to compile them to the IC in general. While it isn't too difficult to implement closures for plain actor expressions (e.g., by abusing the canister's init args to initialise the closure environment), it is more difficult to pull it off for actor _classes_, where the init args are visible and used by clients. It may require generating the Wasm binary for a canister representing an actor class closure at runtime, embedding the serialised closure environment in some form.

An additional complication arises from recursion and mutual recursion, such as two actors referring to each other. Interestingly, in the example above, the Wasm module representing actor class `A` would have to be able to reproduce instances of its own code, akin to a quine. Unless the IC enables a way for a canister to reference its own code, this would be difficult in general.


#### Local Shared Functions

The methods of an actor are _shared functions_, meaning that they (technically, references to them) can be passed to another actor. However, like with regular functions, it is occasionally convenient to be able to define shared functions locally, e.g., as shared "lambdas" passed as argument to another actor.

An obvious implementation of this is via hoisting the function to the enclosing actor. However, local functions are primarily useful if they can refer to definitions from their local scopes, as _closures_. The tricky part is to represent such closures on the IC. This would require the implementation to pair a function reference with an additional closure environment that is passed along with both the function and every call back to the function (as an additional parameter).

Such closures could be supported by an extension of Candid (via closure types, which would be a refinement of function types). But in addition, it is highly desirable that closure environments cannot be forged outside the originating actor, since that would easily allow breaking encapsulation properties. Ideally, they also should not be readable by others. Both could be achieved by signing and encrypting them in the originating actor in some fashion, or if the IC itself provided some respective functionality for passing "sealed" values. Details tbd.

Note: the obvious "solution", namely storing closure environments inside the originating actor and merely passing out handles to them, is not sufficient, since malicious actors might still incorrectly reuse handles they receive. Furthermore, storing the data locally induces a life time problem, since the actor cannot know if another actor still holds a reference to the closure, so it could never garbage-collect the data.


#### Upgrades and Memory

The most difficult problem to solve in the programming model of the IC by far is the question of safe and robust upgrades.
For this purpose, Motoko implements powerful and safe persistence, see [Enhanced Orthogonal Persistence](OrthogonalPersistence.md).

#### Upgrades and Methods

Another issue with upgrades on the IC is outstanding replies: if an actor is still waiting for a reply to some message it sent, upgrading might potentially lose the ability to handle the result.

To enable new versions of a canister to handle replies targetted at an earlier version, the IC and the language runtime need to know which code to reroute the reply to. Since the canister's code might have changed arbitrarily after the upgrade, this isn't generally possible without some indication from the programmer. At a minimum, they must somehow name the "entry points" for replies, such that they can be uniquely identified. Details tbd.


### Modules

#### Import Destructuring ([#2354](https://github.com/dfinity/motoko/issues/2354))

In current Motoko, an import declaration binds the imported module to a single identifier, and all accesses to the module have to be qualified with that module name.

An obvious extension is to allow pattern matching in import declarations to destructure the module and bring its components into scope unqualified:
```
import {freeze; size = asize} "mo:base/Array"
```

Mostly, this extension is syntactic sugar.

##### Formalisation

Syntax:
```
impdec ::= import <pat> =? <textlit>
```


#### Separate Compilation

The most serious restriction of the Motoko module system right now is that the compiler does not actually support compiling modules individually. Instead, it is a _whole-program_ compiler that always recompiles the full transitive set of all imports. Obviously, this will cause a bottleneck in turn-around times for large programs. Motoko should support separate compilation and separate linking of produced modules into a complete canister.


#### Foreign Function Interface

The ability to link compiled modules together would also be a first step towards supporting the import of modules written in other languages than Motoko, such as Rust.

Unfortunately, though, this is a more difficult problem than linking modules compiled in the same language, since the data types, memory management, and calling conventions used by different languages are rarely compatible. Supporting cross-language calls requires a suitable ABI agreed upon by different Wasm compilers, which in turn requires some kind of standard. There is work on a proposal ([interface types](https://github.com/WebAssembly/interface-types/blob/main/proposals/interface-types/Explainer.md)) for Wasm that could be the basis of such a mechanism, but it's not ready yet.

Wasm component model offers a solution for secure intra-canister language interop on the IC.


#### Onchain Linking

[Separate compilation](#separate-compilation) avoids the need to _compile_ applications in a monolitic manner, but it does not prevent the need to _deploy_ them in monolithic form. No matter how large an application and how small a change, upgrading always requires redeploying the entire code.

Also, it is not possible to share modules onchain between multiple applications. For features like [unicode support](#text), it would be desirable if respective modules of the language runtime would only have to be installed on the chain once and could be shared between applications.

Supporting this would require a new mechanism in the IC that allows canisters to consist of more than just a single Wasm module, and linking these modules _dynamically_, "onchain", reminiscent of dynamic linking in conventional operating systems. Unfortunately, there currently isn't any mechanism planned for the IC to support that.


### Platform Support

Besides the basic functionality of canisters and message sends between them, the IC has a number of ad-hoc mechanisms that only have basic support in Motoko so far.

#### Cycles ([#1981](https://github.com/dfinity/motoko/issues/1981))

The most important mechanism probably is the ability to send and receive cycles. The Motoko library currently provides an experimental low-level module to handle cycles, but obviously more high-level mechanisms would be desirable.

Ideally, Motoko should be able to represent cycles as a form of _linear resource_, like other smart contract languages do. Such a representation can provide significant aids to the programmer, i.e., the ability to check that funds aren't lost accidentally.

Unfortunately, the implementation and semantics of cycles on the IC is quite a bit more ad-hoc, and it is not obvious how to design a nice language-level abstraction around them that would have enforcable properties.

At a minimum, Motoko needs language extensions to send cycles along with messages, and likewise to receive them. This could either be through ad-hoc syntax, or preferably, through library primitives.

Even less clear is whether such a mechanism could be designed in a general enough fashion that could also model the transfer of ICPs or other custom tokens.

This also is an open question for Candid, which may want means to declare cycle or token transfer for functions.


#### IC library

There are other features of the IC that require support, e.g., access checks, heartbeat, etc. Some of these can hopefully be encapsulated in dedicated libraries, others might need minimal language support, such as new `system` methods an actor can define.

It would be desirable to sperate all IC-specific modules into a new `ic` library separate form `base`, in order to separate platform-neutral and platform-specific concerns.


#### Capabilities

The current authentication mechanisms of the IC, via access checks on caller etc, are very 1980. Access control is known to have fundamental flaws.

We would hope that the IC will eventually adopt a more modern and robust approach using [capabilities](https://en.wikipedia.org/wiki/Capability-based_security) for its programming model. Such a model also maps much more elegantly to a programming language like Motoko.


## Tooling Improvements

Tbd.

### Better Errors

### Logging

### Profiling

### Debugging

### Formalisation
