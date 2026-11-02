# Stable Variables and Upgrade

To enable persistent state to survive upgrades, a special form of state is needed that we call _stable_.

The compiler needs to take special care to never change the representation of such state.
To make this requirement feasible, certain type restrictions apply to stable state: essentially it can only contain data.

Stable state is introduced in the form of _stable variable definitions_ that are only allowed (as well as required) in actors.


## Language Extension

### Syntax

We require all `let` and `var` declarations in an actor to be declared either stable or flexible.
This is to indicate explicitly that special type restrictions apply.

Concretely, the syntax of `<dec-field>` is extended as follows:
```
<dec-field> ::=
  (public|private)? (stable|flexible)? dec
```

Additional restrictions apply:
* A `stable` or `flexible` modifier _can_ appear on `let` and `var` declarations that are actor fields.
* A `stable` or `flexible` modifier _must not_ appear anywhere else.

Currently, `flexible` is assumed as implicit keyword on actor fields if no keyword is declared.
However, we should revise this design, as it may lead to accidental loss of data on upgrade if programmers accidentally forgot to specify `stable`.
In other languages of orthogonal persistence, pointers are by default persistent, analogous to `stable` in Motoko.

(Note: One possible future use case might be to mark private methods as stable, which would be a requisite that they can be handed out as capabilities, because such methods must also remain backwards compatible.)


### Typing

A stable declaration must have a _stable type_. Stable types are a superset of _shared_ types: specifically, they additionally allow objects or arrays with mutable components.

Concretely, the `stable` predicate has the same definition as the `shared` predicate (cf. `type.ml`), except that the case for `Mut t` is
```
  | Mut t -> go t
```

That is, shared entails stable.
Note that this implies that stable types may contain actors or shared functions, but mutability of course does not extend into those.

Note: This implies that stable state can only contain records, not objects (which contain non-shared functions).
This clearly is a severe restriction.
But we leave the possibility of "stable classes" for later, since it is not at all obvious how to design or implement them.


### Semantics

Installing a new actor runs the initialiser expressions of all flexible and stable variables in sequence, like for ordinary variable definitions in an object.
(In terms of the System API, this happens in the init hook.)

When upgrading an actor, all stable variables that existed in the previous version are pre-initialised with their old values.
Their initialiser expressions are ignored.
After that, the initialiser expressions of flexible and newly added stable variables are executed in sequence, like for ordinary variable definitions.
(In terms of the System API, this happens in the post_upgrade hook.)

This implies that any expression declaration (or any of the form `let _ = <exp>`, for which expressions are a short-hand) will always be run after an upgrade.
They can hence be (ab)used as post-upgrade hooks.

Open Question: What about let declarations with multiple variables, some of which existed before while others didn't? Or should we generally not persist `let`-bound values and always re-initialise them? Would that be a pitfall? Should we forbid it (how?)?

Note: With respect to variable initialisation, installing a new actor behaves like upgrading the actor from an empty actor with no pre-existing stable variables.


## Stable Signatures

The Candid IDL defines the public interface of an actor, listing the methods that an actor provides and their types.
When upgrading an actor, this interface may only be modified in backwards-compatible ways:
* new methods may be added,
* existing methods may be refined to a subtype.
This prevents breaking existing clients assuming the current or an earlier interface.

With stable state, a second dimension is added: the _stable signature_ of an actor lists its stable fields and their types.
When upgrading an actor, this interface may also only be modified in backwards-compatible ways:
* new variables may be added,
* existing variables may be refined to a _supertype_.
This ensures that existing persistent state is still readable with the new version of the program.

The stable signature is not public; its only relevance is to the owner of an actor, as an additional constraint imposed when upgrading the actor.

Stable signatures could also be used to auto-generate interfaces or UI for inspecting or even administering the state of an actor.


### Syntax

The stable signature can't be described in terms of IDL types, because it is specific to Motoko and stable types contain more than what the IDL can express.

The textual representation for stable signatures looks like a Motoko actor type:
```
actor {
  stable x : Nat;
  stable var y : Int;
  stable z : [var Nat];
};
```
Like in Candid, the actor specification may be preceded by a sequence of auxiliary (Motoko) type definitions.

Grammar:
```
<typ> ::= ...   (Motoko types)

<field> ::=
  stable <id> : <typ>
  stable var <id> : <typ>

<dec> ::=
  type <id> <typ-params>? = <typ>

<sig> ::= <dec>;* actor { <field>;* };

```

Note: We could also allow imports if useful.

Question: Should the stable signature become a superset of Candid signatures, i.e., also include methods, but expressed with (richer) Motoko types?


### Compiler and System Support

Like the Candid IDL, the Motoko compiler can produce stable signatures for the actors it compiles.

By using `moc --stable-compatible`, one can compare stable signature and verify that an extension is valid according to the Motoko subtyping rules.

To make that test reliable, the stable signature of an actor should be contained in the Wasm module of a deployed Motoko actor.
That way, it is ensured that accurate signature information is always available for each installed actor.
One way to store it would be in a Motoko-specific custom section;
another alternative is as a separate internal asset.
In either case, it is probably sufficient to use a textual representation.

Like for the IDL, the System would need to provide a way to extract this information from an onchain canister.

For even higher safety, [enhanced orthogonal persistence](OrthogonalPersistence.md) integrates the compatibility check in the runtime system,
such that it is atomically guarded and cannot be bypassed e.g. by skipping a `dfx` stable compatibility warning.

## Upgrade Hooks

The System API provides a number of hooks that a canister can implement.
In particular, this includes the pre & post upgrade hooks.
Motoko allows to define custom pre-/post upgrade hooks, see below.

### Syntax

To this end, we further extend the syntax of `<dec-field>` with _system methods_ of the following form:
```
<dec-field> ::= ...
  (public|private|system)? (flexible|stable)? dec
```
Again, additional restrictions apply:
* A `system` modifier _may only_ appear on `func` declarations that are actor fields.
* A `system` modifier _must not_ appear anywhere else.

Two system methods are recognised by their name:
* `preupgrade`
* `postupgrade`

The set of system functions may be extended in the future.


### Typing

The required type of system methods depends on their name:
* `preupgrade : () -> ()`
* `postupgrade : () -> ()`


### Semantics

Pre-upgrade and post-upgrade methods are executed before or after an upgrade, respectively. (In terms of the System API, they correspond to the respective hooks.)

Moreover, a post-upgrade method is executed after the actor body and its variable initialisers have run (see above).

Note: The post-upgrade method differs from expression declarations in the body of an actor in that they are _only_ run after an upgrade, not when first installing the actor.


## Implementation

See [enhanced orthogonal persistence](OrthogonalPersistence.md).
