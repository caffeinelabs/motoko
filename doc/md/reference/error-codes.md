---
title: "Error code reference"
description: "Reference for Motoko compiler error codes with examples and explanations."
sidebar:
  order: 2
---

<!-- needs updating or better: autogeneration -->

| Error Code | Description | Example | Explanation |
|------------|-------------------|---------|-------------|
| M0001 | Parsing errors. | `let x = 5 + ;` | Syntax error - missing operand after the `+` operator. |
| M0003 | Module tried to import itself. | `import Self "self.mo"` | A module cannot import itself, creating a circular dependency. |
| M0009 | File not found for import. | `import Lib "wrong/path_to_file.mo"` | The specified file path in the import statement doesn't exist. |
| M0010 | Imported package was not defined. | `import { undefined_module } from "mo:package"` | The imported module doesn't exist in the specified package. |
| M0014 | Non-static expression in library or module. | ```module { let x = Random.blob(); }``` | Modules can only contain static definitions, not runtime expressions. |
| M0029 | Unbound type. | `func transform(x : UnknownType) { ... }` | The type `UnknownType` is referenced but hasn't been defined. |
| M0030 | Type field does not exist in type. | ```type Person = { name : Text }; func getName(p : Person) : Nat { p.age }``` | The field `age` doesn't exist in the `Person` type. |
| M0031 | Shared function has non-shared parameter type. | ```public shared func process(obj : { var count : Nat }) { ... }``` | Shared functions cannot have mutable types as parameters. |
| M0032 | Shared function has non-shared return type. | ```public shared func getData() : { var data : [Nat] } { ... }``` | Shared functions cannot return mutable types. |
| M0033 | Async has non-shared content type. | ```async { var counter = 0 }``` | Async blocks cannot contain mutable [state](../fundamentals/actors/state.md). |
| M0036 | Invalid return type for shared query function. | ```public shared query func getUsers() : async [User] { ... }``` | Query functions cannot return async types. |
| M0038 | Misplaced await. | ```func compute() { let x = await promise; }``` | `await` can only be used in an async function or block. |
| M0045 | Wrong number of type arguments. | ```func process<T>(x : Array<T, Nat>) { ... }``` | `Array` type expects one type parameter, not two. |
| M0047 | Send capability required. | ```actor { public func send() { IC.call(...); } }``` | Functions calling IC management require explicit capability declarations. |
| M0050 | Literal does not have expected type. | ```let x : Text = 42;``` | Cannot assign a number literal to a variable of type [`Text`](https://mops.one/core/docs/Text). |
| M0055 | Cannot infer type of forward variable. | ```func process() { x := 10; var x = 0; }``` | Variable `x` is used before its declaration. |
| M0057 | Unbound variable. | ```func calculate() { return result; }``` | The variable `result` is used but hasn't been defined. |
| M0060 | Operator is not defined for operand types. | ```let result = "text" - 5;``` | The subtraction operator isn't defined for [`Text`](https://mops.one/core/docs/Text) and [`Nat`](https://mops.one/core/docs/Nat) types. |
| M0064 | Misplaced '!' without enclosing do block | ```func test() { someFunction!(); }``` | The `!` operator must be used within a `do` block. |
| M0070 | Expected object type. | ```let name = person.name;``` (where `person` is not an object) | Trying to access a field on a non-object type. |
| M0072 | Field does not exist in type. | ```type User = { id : Nat }; func getName(u : User) { u.name }``` | The field `name` doesn't exist in the `User` type. |
| M0073 | Expected mutable assignment target. | ```let x = 5; x := 10;``` | Cannot reassign to an immutable variable declared with `let`. |
| M0082 | Expected iterable type. | ```for (item in 42) { ... }``` | The value `42` is not an iterable type for a for-loop. |
| M0088 | Expected async type. | ```let result = await 42;``` | Cannot `await` on a non-async value. |
| M0089 | Redundant ignore. | ```ignore (5);``` | The `ignore` is unnecessary for a value that doesn't need to be ignored. |
| M0090 | Actor reference must have an [actor](../fundamentals/actors/actors-async.md) type .| ```let a = actor { ... }; a.someMethod();``` | The variable `a` must have an explicit [actor](../fundamentals/actors/actors-async.md) type to call its methods. |
| M0096 | Expression can't produce expected type. | ```func getValue() : Bool { return "true"; }``` | String cannot be returned where a [`Bool`](https://mops.one/core/docs/Bool) is expected. |
| M0097 | Expected function type. | ```let x = 5; x();``` | Cannot call a non-function value. |
| M0098 | Cannot instantiate function type. | ```type Func = (Nat) -> Nat; let f = Func();``` | Function types cannot be instantiated directly. |
| M0112 | Tuple pattern cannot consume type. | ```let (x, y) = { a = 1; b = 2 };``` | Cannot destructure an object using tuple pattern. |
| M0116 | Variant pattern cannot consume type. | ```switch (value) { case (#ok(x)) { ... } };``` (where `value` is not a variant) | Cannot match a non-variant value with variant patterns. |
| M0126 | Shared function cannot be private. | ```actor { private shared func publicAPI() { ... } }``` | Shared functions must be public in actors. |
| M0137 | A type or class was declared that explicitly or implicitly references an outer type parameter. | ```class Container<T>() { class Inner() { var value : T = ... } }``` | Inner classes cannot reference type parameters from outer classes. |
| M0139 | Inner actor classes are not supported. | ```class Outer() { actor class Inner() { ... } }``` | Actor classes cannot be nested inside other classes. |
| M0141 | Forbidden declaration in program. | ```import Prim "mo:prim";``` | Certain imports/declarations are not allowed in normal user code. |
| M0145 | Pattern does not cover value. | ```switch (option) { case (null) { ... } };``` (missing `?some` case) | Switch statement doesn't handle all possible values. |
| M0149 | An immutable [record](../fundamentals/types/records.md) field (declared without `var`) was supplied where a mutable [record](../fundamentals/types/records.md) field (specified with `var`), was expected. | ```type Mutable = { var x : Nat }; func set(r : Mutable) { ... }; set({ x = 10 });``` | Immutable [record](../fundamentals/types/records.md) provided where mutable was expected. |
| M0150 | A mutable [record](../fundamentals/types/records.md) field (declared with `var`) was supplied where an immutable [record](../fundamentals/types/records.md) field (specified without `var`) was expected. | ```type Immutable = { x : Nat }; let record : Immutable = { var x = 10 };``` | Mutable field provided where immutable was expected. |
| M0151 | A object literal is missing some fields. | ```type Person = { name : Text; age : Nat }; let p : Person = { name = "John" };``` | The `age` field is missing from the object literal. |
| M0153 | An imported Candid file (.did) mentions types that cannot be represented in Motoko. | ```import Types from "types.did";``` (where `types.did` contains incompatible types) | The imported Candid file contains types that don't map to Motoko types. |
| M0154 | Deprecation annotation. | ```@deprecated("Use newFunction instead") func oldFunction() { ... }``` | Using a deprecated function or feature that has been marked with `@deprecated`. |
| M0155 | Inferred type [`Nat`](https://mops.one/core/docs/Nat) for subtraction. | ```let x = 5 - 10;``` | Subtraction with natural numbers can underflow without type annotation. |
| M0156 | A parameterized type definition, or set of type definitions, is too complicated for Motoko to accept. | ```type Complex<A,B,C,D,E,F> = ... (extremely nested recursive types)``` | Type definition too complex for the compiler to process. |
| M0157 | A type definition, or set of type definitions, is ill-defined. | ```type RecursiveType = RecursiveType;``` | A type directly references itself without proper indirection. |
| M0158 | A public class was declared without providing it with a name. | ```public class<T>() { ... }``` | Public parameterized classes must be named explicitly. |
| M0194 | An identifier was defined without referencing it later. | ```func process() { let unused = computeValue(); }``` | The variable `unused` is defined but never used. |
| M0195 | A function that demands elevated (system) capabilities was called without manifestly passing the capability. | ```func splitCycles() { let amount = ExperimentalCycles.balance() / 2; ExperimentalCycles.add(amount);}}``` | System capability function called without proper capability passing. |
| M0197 | A function that requires (system) capabilities was called in a context that does not provide them. | ```func invalidCycleAddition(): () { ExperimentalCycles.add(1000);}``` | Calling a function requiring system capabilities from a context without them. |
| M0198 | A field identifier was specified in an object pattern without referencing this identifier later. | ```func process(obj) { let { id } = obj; }``` | The extracted `id` field is never used in the function body. |

