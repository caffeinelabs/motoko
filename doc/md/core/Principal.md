# core/Principal
Module for interacting with Principals (users and canisters).

Principals are used to identify entities that can interact with the Internet
Computer. These entities are either users or canisters.

Example textual representation of Principals:

`un4fu-tqaaa-aaaab-qadjq-cai`

In Motoko, there is a primitive Principal type called `Principal`. As an example
of where you might see Principals, you can access the Principal of the
caller of your shared function.

```motoko no-repl
persistent actor {
  public shared(msg) func foo() {
    let caller : Principal = msg.caller;
  };
}
```

Then, you can use this module to work with the `Principal`.

Import from the core package to use this module.
```motoko name=import
import Principal "mo:core/Principal";
```

## Type `Principal`
``` motoko no-repl
type Principal = Prim.Types.Principal
```


## Function `fromActor`
``` motoko no-repl
func fromActor(a : actor {  }) : Principal
```

Get the `Principal` identifier of an actor.

Example:
```motoko include=import no-repl
persistent actor MyCanister {
  func getPrincipal() : Principal {
    let principal = Principal.fromActor(MyCanister);
  }
}
```

## Function `toActor`
``` motoko no-repl
func toActor<A <: actor {  }>(p : Principal) : A
```

Turns a `Principal` into an actor reference. The presence of the methods
(and their respective types) is not guaranteed.

This is the inverse of `fromActor`. The returned reference is
typed with the given actor type `A`.

Example:
```motoko include=import no-repl
let principal = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
let canister : actor {} = Principal.toActor<actor {}>(principal);
```

## Function `toLedgerAccount`
``` motoko no-repl
func toLedgerAccount(self : Principal, subAccount : ?Blob) : Blob
```

Compute the Ledger account identifier of a principal. Optionally specify a sub-account.

Example:
```motoko include=import no-validate
let principal = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
let subAccount : Blob = "\4A\8D\3F\2B\6E\01\C8\7D\9E\03\B4\56\7C\F8\9A\01\D2\34\56\78\9A\BC\DE\F0\12\34\56\78\9A\BC\DE\F0";
let account = Principal.toLedgerAccount(principal, ?subAccount);
assert account == "\8C\5C\20\C6\15\3F\7F\51\E2\0D\0F\0F\B5\08\51\5B\47\65\63\A9\62\B4\A9\91\5F\4F\02\70\8A\ED\4F\82";
```

## Function `toBlob`
``` motoko no-repl
func toBlob(self : Principal) : Blob
```

Convert a `Principal` to its `Blob` (bytes) representation.

Example:
```motoko include=import
let principal = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
let blob = Principal.toBlob(principal);
assert blob == "\00\00\00\00\00\30\00\D3\01\01";
```

## Function `fromBlob`
``` motoko no-repl
func fromBlob(self : Blob) : Principal
```

Converts a `Blob` (bytes) representation of a `Principal` to a `Principal` value.

Example:
```motoko include=import
let blob = "\00\00\00\00\00\30\00\D3\01\01" : Blob;
let principal = Principal.fromBlob(blob);
assert Principal.toText(principal) == "un4fu-tqaaa-aaaab-qadjq-cai";
```

## Function `toText`
``` motoko no-repl
func toText(self : Principal) : Text
```

Converts a `Principal` to its `Text` representation.

Example:
```motoko include=import
let principal = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
assert Principal.toText(principal) == "un4fu-tqaaa-aaaab-qadjq-cai";
```

## Function `fromText`
``` motoko no-repl
func fromText(t : Text) : Principal
```

Converts a `Text` representation of a `Principal` to a `Principal` value.

Example:
```motoko include=import
let principal = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
assert Principal.toText(principal) == "un4fu-tqaaa-aaaab-qadjq-cai";
```

## Function `anonymous`
``` motoko no-repl
func anonymous() : Principal
```

Constructs and returns the anonymous principal.

## Function `isAnonymous`
``` motoko no-repl
func isAnonymous(self : Principal) : Bool
```

Checks if the given principal represents an anonymous user.

Example:
```motoko include=import
let principal = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
assert not Principal.isAnonymous(principal);
```

## Function `isCanister`
``` motoko no-repl
func isCanister(self : Principal) : Bool
```

Checks if the given principal is a canister.

The last byte for opaque principal ids must be 0x01
https://internetcomputer.org/docs/current/references/ic-interface-spec#principal

Example:
```motoko include=import
let principal = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
assert Principal.isCanister(principal);
```

## Function `isSelfAuthenticating`
``` motoko no-repl
func isSelfAuthenticating(self : Principal) : Bool
```

Checks if the given principal is a self authenticating principal.
Most of the time, this is a user principal.

The last byte for user principal ids must be 0x02
https://internetcomputer.org/docs/current/references/ic-interface-spec#principal

Example:
```motoko include=import
let principal = Principal.fromText("6rgy7-3uukz-jrj2k-crt3v-u2wjm-dmn3t-p26d6-ndilt-3gusv-75ybk-jae");
assert Principal.isSelfAuthenticating(principal);
```

## Function `isReserved`
``` motoko no-repl
func isReserved(self : Principal) : Bool
```

Checks if the given principal is a reserved principal.

The last byte for reserved principal ids must be 0x7f
https://internetcomputer.org/docs/current/references/ic-interface-spec#principal

Example:
```motoko include=import
let principal = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
assert not Principal.isReserved(principal);
```

## Function `isController`
``` motoko no-repl
func isController(self : Principal) : Bool
```

Checks if the given principal can control this canister.

Example:
```motoko include=import
let principal = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
assert not Principal.isController(principal);
```

## Function `hash`
``` motoko no-repl
func hash(self : Principal) : Types.Hash
```

Hashes the given principal by hashing its `Blob` representation.

Example:
```motoko include=import
let principal = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
assert Principal.hash(principal) == 2_742_573_646;
```

## Function `compare`
``` motoko no-repl
func compare(self : Principal, other : Principal) : {#less; #equal; #greater}
```

General purpose comparison function for `Principal`. Returns the `Order` (
either `#less`, `#equal`, or `#greater`) of comparing `principal1` with
`principal2`.

Example:
```motoko include=import
let principal1 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
let principal2 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
assert Principal.compare(principal1, principal2) == #equal;
```

## Function `equal`
``` motoko no-repl
func equal(self : Principal, other : Principal) : Bool
```

Equality function for Principal types.
This is equivalent to `principal1 == principal2`.

Example:
```motoko include=import
let principal1 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
let principal2 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
ignore Principal.equal(principal1, principal2);
assert principal1 == principal2;
```

Note: The reason why this function is defined in this library (in addition
to the existing `==` operator) is so that you can use it as a function
value to pass to a higher order function. It is not possible to use `==`
as a function value at the moment.

Example:
```motoko include=import
let principal1 = Principal.anonymous();
let principal2 = Principal.fromBlob("\04");
assert Principal.equal(principal1, principal2);
```

## Function `notEqual`
``` motoko no-repl
func notEqual(self : Principal, other : Principal) : Bool
```

Inequality function for Principal types.
This is equivalent to `principal1 != principal2`.

Example:
```motoko include=import
let principal1 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
let principal2 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
ignore Principal.notEqual(principal1, principal2);
assert not (principal1 != principal2);
```

Note: The reason why this function is defined in this library (in addition
to the existing `!=` operator) is so that you can use it as a function
value to pass to a higher order function. It is not possible to use `!=`
as a function value at the moment.

## Function `less`
``` motoko no-repl
func less(self : Principal, other : Principal) : Bool
```

"Less than" function for Principal types.
This is equivalent to `principal1 < principal2`.

Example:
```motoko include=import
let principal1 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
let principal2 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
ignore Principal.less(principal1, principal2);
assert not (principal1 < principal2);
```

Note: The reason why this function is defined in this library (in addition
to the existing `<` operator) is so that you can use it as a function
value to pass to a higher order function. It is not possible to use `<`
as a function value at the moment.

## Function `lessOrEqual`
``` motoko no-repl
func lessOrEqual(self : Principal, other : Principal) : Bool
```

"Less than or equal to" function for Principal types.
This is equivalent to `principal1 <= principal2`.

Example:
```motoko include=import
let principal1 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
let principal2 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
ignore Principal.lessOrEqual(principal1, principal2);
assert principal1 <= principal2;
```

Note: The reason why this function is defined in this library (in addition
to the existing `<=` operator) is so that you can use it as a function
value to pass to a higher order function. It is not possible to use `<=`
as a function value at the moment.

## Function `greater`
``` motoko no-repl
func greater(self : Principal, other : Principal) : Bool
```

"Greater than" function for Principal types.
This is equivalent to `principal1 > principal2`.

Example:
```motoko include=import
let principal1 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
let principal2 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
ignore Principal.greater(principal1, principal2);
assert not (principal1 > principal2);
```

Note: The reason why this function is defined in this library (in addition
to the existing `>` operator) is so that you can use it as a function
value to pass to a higher order function. It is not possible to use `>`
as a function value at the moment.

## Function `greaterOrEqual`
``` motoko no-repl
func greaterOrEqual(self : Principal, other : Principal) : Bool
```

"Greater than or equal to" function for Principal types.
This is equivalent to `principal1 >= principal2`.

Example:
```motoko include=import
let principal1 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
let principal2 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
ignore Principal.greaterOrEqual(principal1, principal2);
assert principal1 >= principal2;
```

Note: The reason why this function is defined in this library (in addition
to the existing `>=` operator) is so that you can use it as a function
value to pass to a higher order function. It is not possible to use `>=`
as a function value at the moment.
