# core/Blob
Module for working with Blobs (immutable sequences of bytes).

Blobs represent sequences of bytes. They are immutable, iterable, indexable and can be empty.

Byte sequences are also often represented as `[Nat8]`, i.e. an array of bytes, but this representation is currently much less compact than `Blob`, taking 4 physical bytes to represent each logical byte in the sequence.
If you would like to manipulate Blobs, it is recommended that you convert
Blobs to `[var Nat8]` or `Buffer<Nat8>`, do the manipulation, then convert back.

Import from the core package to use this module.
```motoko name=import
import Blob "mo:core/Blob";
```

Some built in features not listed in this module:

* You can create a `Blob` literal from a `Text` literal, provided the context expects an expression of type `Blob`.
* `b.size() : Nat` returns the number of bytes in the blob `b`;
* `b.values() : Iter.Iter<Nat8>` returns an iterator to enumerate the bytes of the blob `b`.

For example:
```motoko include=import
import Debug "mo:core/Debug";
import Nat8 "mo:core/Nat8";

let blob = "\00\00\00\ff" : Blob; // blob literals, where each byte is delimited by a back-slash and represented in hex
let blob2 = "charsもあり" : Blob; // you can also use characters in the literals
let numBytes = blob.size();
assert numBytes == 4; // returns the number of bytes in the Blob
for (byte in blob.values()) { // iterator over the Blob
  Debug.print(Nat8.toText(byte))
}
```

## Type `Blob`
``` motoko no-repl
type Blob = Prim.Types.Blob
```


## Function `empty`
``` motoko no-repl
func empty() : Blob
```

Returns an empty `Blob` (equivalent to `""`).

Example:
```motoko include=import
let emptyBlob = Blob.empty();
assert emptyBlob.size() == 0;
```

## Function `isEmpty`
``` motoko no-repl
func isEmpty(self : Blob) : Bool
```

Returns whether the given `Blob` is empty (has a size of zero).

```motoko include=import
let blob1 = "" : Blob;
let blob2 = "\FF\00" : Blob;
assert Blob.isEmpty(blob1);
assert not Blob.isEmpty(blob2);
```

## Function `size`
``` motoko no-repl
func size(self : Blob) : Nat
```

Returns the number of bytes in the given `Blob`.
This is equivalent to `blob.size()`.

Example:
```motoko include=import
let blob = "\FF\00\AA" : Blob;
assert Blob.size(blob) == 3;
assert blob.size() == 3;
```

## Function `fromArray`
``` motoko no-repl
func fromArray(bytes : [Nat8]) : Blob
```

Creates a `Blob` from an array of bytes (`[Nat8]`), by copying each element.

Example:
```motoko include=import
let bytes : [Nat8] = [0, 255, 0];
let blob = Blob.fromArray(bytes);
assert blob == "\00\FF\00";
```
@deprecated M0235

## Function `fromVarArray`
``` motoko no-repl
func fromVarArray(bytes : [var Nat8]) : Blob
```

Creates a `Blob` from a mutable array of bytes (`[var Nat8]`), by copying each element.

Example:
```motoko include=import
let bytes : [var Nat8] = [var 0, 255, 0];
let blob = Blob.fromVarArray(bytes);
assert blob == "\00\FF\00";
```
@deprecated M0235

## Function `toArray`
``` motoko no-repl
func toArray(self : Blob) : [Nat8]
```

Converts a `Blob` to an array of bytes (`[Nat8]`), by copying each element.

Example:
```motoko include=import
let blob = "\00\FF\00" : Blob;
let bytes = Blob.toArray(blob);
assert bytes == [0, 255, 0];
```

## Function `toVarArray`
``` motoko no-repl
func toVarArray(self : Blob) : [var Nat8]
```

Converts a `Blob` to a mutable array of bytes (`[var Nat8]`), by copying each element.

Example:
```motoko include=import
import Nat8 "mo:core/Nat8";
import VarArray "mo:core/VarArray";

let blob = "\00\FF\00" : Blob;
let bytes = Blob.toVarArray(blob);
assert VarArray.equal<Nat8>(bytes, [var 0, 255, 0], Nat8.equal);
```

## Function `hash`
``` motoko no-repl
func hash(self : Blob) : Types.Hash
```

Returns the (non-cryptographic) hash of `blob`.

Example:
```motoko include=import
let blob = "\00\FF\00" : Blob;
let h = Blob.hash(blob);
assert h == 1_818_567_776;
```

## Function `compare`
``` motoko no-repl
func compare(self : Blob, other : Blob) : Order.Order
```

General purpose comparison function for `Blob` by comparing the value of
the bytes. Returns the `Order` (either `#less`, `#equal`, or `#greater`)
by comparing `blob1` with `blob2`.

Example:
```motoko include=import
let blob1 = "\00\00\00" : Blob;
let blob2 = "\00\FF\00" : Blob;
let result = Blob.compare(blob1, blob2);
assert result == #less;
```

## Function `equal`
``` motoko no-repl
func equal(self : Blob, other : Blob) : Bool
```

Equality function for `Blob` types.
This is equivalent to `blob1 == blob2`.

Example:
```motoko include=import
let blob1 = "\00\FF\00" : Blob;
let blob2 = "\00\FF\00" : Blob;
assert Blob.equal(blob1, blob2);
```

Note: The reason why this function is defined in this library (in addition
to the existing `==` operator) is so that you can use it as a function value
to pass to a higher order function.

Example:
```motoko include=import
import List "mo:core/List";

let list1 = List.singleton<Blob>("\00\FF\00");
let list2 = List.singleton<Blob>("\00\FF\00");
assert List.equal(list1, list2, Blob.equal);
```

## Function `notEqual`
``` motoko no-repl
func notEqual(self : Blob, other : Blob) : Bool
```

Inequality function for `Blob` types.
This is equivalent to `blob1 != blob2`.

Example:
```motoko include=import
let blob1 = "\00\AA\AA" : Blob;
let blob2 = "\00\FF\00" : Blob;
assert Blob.notEqual(blob1, blob2);
```

Note: The reason why this function is defined in this library (in addition
to the existing `!=` operator) is so that you can use it as a function value
to pass to a higher order function.

## Function `less`
``` motoko no-repl
func less(self : Blob, other : Blob) : Bool
```

"Less than" function for `Blob` types.
This is equivalent to `blob1 < blob2`.

Example:
```motoko include=import
let blob1 = "\00\AA\AA" : Blob;
let blob2 = "\00\FF\00" : Blob;
assert Blob.less(blob1, blob2);
```

Note: The reason why this function is defined in this library (in addition
to the existing `<` operator) is so that you can use it as a function value
to pass to a higher order function.

## Function `lessOrEqual`
``` motoko no-repl
func lessOrEqual(self : Blob, other : Blob) : Bool
```

"Less than or equal to" function for `Blob` types.
This is equivalent to `blob1 <= blob2`.

Example:
```motoko include=import
let blob1 = "\00\AA\AA" : Blob;
let blob2 = "\00\FF\00" : Blob;
assert Blob.lessOrEqual(blob1, blob2);
```

Note: The reason why this function is defined in this library (in addition
to the existing `<=` operator) is so that you can use it as a function value
to pass to a higher order function.

## Function `greater`
``` motoko no-repl
func greater(self : Blob, other : Blob) : Bool
```

"Greater than" function for `Blob` types.
This is equivalent to `blob1 > blob2`.

Example:
```motoko include=import
let blob1 = "\BB\AA\AA" : Blob;
let blob2 = "\00\00\00" : Blob;
assert Blob.greater(blob1, blob2);
```

Note: The reason why this function is defined in this library (in addition
to the existing `>` operator) is so that you can use it as a function value
to pass to a higher order function.

## Function `greaterOrEqual`
``` motoko no-repl
func greaterOrEqual(self : Blob, other : Blob) : Bool
```

"Greater than or equal to" function for `Blob` types.
This is equivalent to `blob1 >= blob2`.

Example:
```motoko include=import
let blob1 = "\BB\AA\AA" : Blob;
let blob2 = "\00\00\00" : Blob;
assert Blob.greaterOrEqual(blob1, blob2);
```

Note: The reason why this function is defined in this library (in addition
to the existing `>=` operator) is so that you can use it as a function value
to pass to a higher order function.
