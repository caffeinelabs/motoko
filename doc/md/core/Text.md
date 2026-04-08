# core/Text
Utility functions for `Text` values.

A `Text` value represents human-readable text as a sequence of characters of type `Char`.

```motoko
let text = "Hello!";
let size = text.size();
assert size == 6;
let iter = text.chars();
assert iter.next() == ?'H';
assert iter.next() == ?'e';
assert iter.next() == ?'l';
assert iter.next() == ?'l';
assert iter.next() == ?'o';
assert iter.next() == ?'!';
assert iter.next() == null;
let concat = text # " 👋";
assert concat == "Hello! 👋";
```

The `"mo:core/Text"` module defines additional operations on `Text` values.

Import the module from the core package:

```motoko name=import
import Text "mo:core/Text";
```

Note: `Text` values are represented as ropes of UTF-8 character sequences with O(1) concatenation.


## Type `Text`
``` motoko no-repl
type Text = Prim.Types.Text
```

The type corresponding to primitive `Text` values.

```motoko
let hello = "Hello!";
let emoji = "👋";
let concat = hello # " " # emoji;
assert concat == "Hello! 👋";
```

## Function `fromChar`
``` motoko no-repl
func fromChar(c : Char) : Text
```

Converts the given `Char` to a `Text` value.

```motoko include=import
let text = Text.fromChar('A');
assert text == "A";
```

## Function `fromArray`
``` motoko no-repl
func fromArray(a : [Char]) : Text
```

Converts the given `[Char]` to a `Text` value.

```motoko include=import
let text = Text.fromArray(['A', 'v', 'o', 'c', 'a', 'd', 'o']);
assert text == "Avocado";
```

Runtime: O(a.size())
Space: O(a.size())

## Function `fromVarArray`
``` motoko no-repl
func fromVarArray(a : [var Char]) : Text
```

Converts the given `[var Char]` to a `Text` value.

```motoko include=import
let text = Text.fromVarArray([var 'E', 'g', 'g', 'p', 'l', 'a', 'n', 't']);
assert text == "Eggplant";
```

Runtime: O(a.size())
Space: O(a.size())

## Function `toIter`
``` motoko no-repl
func toIter(self : Text) : Iter.Iter<Char>
```

Iterates over each `Char` value in the given `Text`.

Equivalent to calling the `t.chars()` method where `t` is a `Text` value.

```motoko include=import
let chars = Text.toIter("abc");
assert chars.next() == ?'a';
assert chars.next() == ?'b';
assert chars.next() == ?'c';
assert chars.next() == null;
```

## Function `foldLeft`
``` motoko no-repl
func foldLeft<A>(self : Text, base : A, combine : (A, Char) -> A) : A
```

Collapses the characters in `text` into a single value by starting with `base`
and progessively combining characters into `base` with `combine`. Iteration runs
left to right.

```motoko include=import

let text = "Mississippi";
let count =
  Text.foldLeft<Nat>(
    text,
    0, // start the sum at 0
    func(ss, c) = if (c == 's') ss + 1 else ss
  );
assert count == 4;
```

Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `combine` runs in O(1) time and space.

## Function `toArray`
``` motoko no-repl
func toArray(self : Text) : [Char]
```

Creates a new `Array` containing characters of the given `Text`.

Equivalent to `Iter.toArray(t.chars())`.

```motoko include=import
assert Text.toArray("Café") == ['C', 'a', 'f', 'é'];
```

Runtime: O(t.size())
Space: O(t.size())

## Function `toVarArray`
``` motoko no-repl
func toVarArray(self : Text) : [var Char]
```

Creates a new mutable `Array` containing characters of the given `Text`.

Equivalent to `Iter.toArrayMut(t.chars())`.

```motoko include=import
import VarArray "mo:core/VarArray";
import Char "mo:core/Char";

assert VarArray.equal(Text.toVarArray("Café"), [var 'C', 'a', 'f', 'é'], Char.equal);
```

Runtime: O(t.size())
Space: O(t.size())

## Function `fromIter`
``` motoko no-repl
func fromIter(cs : Iter.Iter<Char>) : Text
```

Creates a `Text` value from a `Char` iterator.

```motoko include=import
let text = Text.fromIter(['a', 'b', 'c'].values());
assert text == "abc";
```

## Function `isEmpty`
``` motoko no-repl
func isEmpty(self : Text) : Bool
```

Returns whether the given `Text` is empty (has a size of zero).

```motoko include=import
let text1 = "";
let text2 = "example";
assert Text.isEmpty(text1);
assert not Text.isEmpty(text2);
```

## Function `size`
``` motoko no-repl
func size(self : Text) : Nat
```

Returns the number of characters in the given `Text`.

Equivalent to calling `t.size()` where `t` is a `Text` value.

```motoko include=import
let size = Text.size("abc");
assert size == 3;
```

## Function `concat`
``` motoko no-repl
func concat(self : Text, other : Text) : Text
```

Returns `t1 # t2`, where `#` is the `Text` concatenation operator.

```motoko include=import
let a = "Hello";
let b = "There";
let together = a # b;
assert together == "HelloThere";
let withSpace = a # " " # b;
assert withSpace == "Hello There";
let togetherAgain = Text.concat(a, b);
assert togetherAgain == "HelloThere";
```

## Function `reverse`
``` motoko no-repl
func reverse(self : Text) : Text
```

Returns a new `Text` with the characters of the input `Text` in reverse order.

```motoko include=import
let text = Text.reverse("Hello");
assert text == "olleH";
```

Runtime: O(t.size())
Space: O(t.size())

## Function `equal`
``` motoko no-repl
func equal(self : Text, other : Text) : Bool
```

Returns true if two text values are equal.

```motoko
import Text "mo:core/Text";

assert Text.equal("hello", "hello");
assert not Text.equal("hello", "world");
```

## Function `notEqual`
``` motoko no-repl
func notEqual(self : Text, other : Text) : Bool
```

Returns true if two text values are not equal.

```motoko
import Text "mo:core/Text";

assert Text.notEqual("hello", "world");
assert not Text.notEqual("hello", "hello");
```

## Function `less`
``` motoko no-repl
func less(self : Text, other : Text) : Bool
```

Returns true if the first text value is lexicographically less than the second.

```motoko
import Text "mo:core/Text";

assert Text.less("apple", "banana");
assert not Text.less("banana", "apple");
```

## Function `lessOrEqual`
``` motoko no-repl
func lessOrEqual(self : Text, other : Text) : Bool
```

Returns true if the first text value is lexicographically less than or equal to the second.

```motoko
import Text "mo:core/Text";

assert Text.lessOrEqual("apple", "banana");
assert Text.lessOrEqual("apple", "apple");
assert not Text.lessOrEqual("banana", "apple");
```

## Function `greater`
``` motoko no-repl
func greater(self : Text, other : Text) : Bool
```

Returns true if the first text value is lexicographically greater than the second.

```motoko
import Text "mo:core/Text";

assert Text.greater("banana", "apple");
assert not Text.greater("apple", "banana");
```

## Function `greaterOrEqual`
``` motoko no-repl
func greaterOrEqual(self : Text, other : Text) : Bool
```

Returns true if the first text value is lexicographically greater than or equal to the second.

```motoko
import Text "mo:core/Text";

assert Text.greaterOrEqual("banana", "apple");
assert Text.greaterOrEqual("apple", "apple");
assert not Text.greaterOrEqual("apple", "banana");
```

## Function `compare`
``` motoko no-repl
func compare(self : Text, other : Text) : Order.Order
```

Compares `t1` and `t2` lexicographically.

```motoko include=import
assert Text.compare("abc", "abc") == #equal;
assert Text.compare("abc", "def") == #less;
assert Text.compare("abc", "ABC") == #greater;
```

## Function `join`
``` motoko no-repl
func join(self : Iter.Iter<Text>, sep : Text) : Text
```

Join an iterator of `Text` values with a given delimiter.

```motoko include=import
let joined = Text.join(["a", "b", "c"].values(), ", ");
assert joined == "a, b, c";
```

## Function `map`
``` motoko no-repl
func map(self : Text, f : Char -> Char) : Text
```

Applies a function to each character in a `Text` value, returning the concatenated `Char` results.

```motoko include=import
// Replace all occurrences of '?' with '!'
let result = Text.map("Motoko?", func(c) {
  if (c == '?') '!'
  else c
});
assert result == "Motoko!";
```

## Function `flatMap`
``` motoko no-repl
func flatMap(self : Text, f : Char -> Text) : Text
```

Returns the result of applying `f` to each character in `ts`, concatenating the intermediate text values.

```motoko include=import
// Replace all occurrences of '?' with "!!"
let result = Text.flatMap("Motoko?", func(c) {
  if (c == '?') "!!"
  else Text.fromChar(c)
});
assert result == "Motoko!!";
```

## Type `Pattern`
``` motoko no-repl
type Pattern = Types.Pattern
```

A pattern `p` describes a sequence of characters. A pattern has one of the following forms:

* `#char c` matches the single character sequence, `c`.
* `#text t` matches multi-character text sequence `t`.
* `#predicate p` matches any single character sequence `c` satisfying predicate `p(c)`.

A _match_ for `p` is any sequence of characters matching the pattern `p`.

```motoko include=import
let charPattern = #char 'A';
let textPattern = #text "phrase";
let predicatePattern : Text.Pattern = #predicate (func(c) { c == 'A' or c == 'B' });
assert Text.contains("A", predicatePattern);
assert Text.contains("B", predicatePattern);
```

## Function `split`
``` motoko no-repl
func split(self : Text, p : Pattern) : Iter.Iter<Text>
```

Splits the input `Text` with the specified `Pattern`.

Two fields are separated by exactly one match.

```motoko include=import
let words = Text.split("This is a sentence.", #char ' ');
assert Text.join(words, "|") == "This|is|a|sentence.";
```

## Function `tokens`
``` motoko no-repl
func tokens(self : Text, p : Pattern) : Iter.Iter<Text>
```

Returns a sequence of tokens from the input `Text` delimited by the specified `Pattern`, derived from start to end.
A "token" is a non-empty maximal subsequence of `t` not containing a match for pattern `p`.
Two tokens may be separated by one or more matches of `p`.

```motoko include=import
let tokens = Text.tokens("this needs\n an   example", #predicate (func(c) { c == ' ' or c == '\n' }));
assert Text.join(tokens, "|") == "this|needs|an|example";
```

## Function `contains`
``` motoko no-repl
func contains(self : Text, p : Pattern) : Bool
```

Returns `true` if the input `Text` contains a match for the specified `Pattern`.

```motoko include=import
assert Text.contains("Motoko", #text "oto");
assert not Text.contains("Motoko", #text "xyz");
```

## Function `startsWith`
``` motoko no-repl
func startsWith(self : Text, p : Pattern) : Bool
```

Returns `true` if the input `Text` starts with a prefix matching the specified `Pattern`.

```motoko include=import
assert Text.startsWith("Motoko", #text "Mo");
```

## Function `endsWith`
``` motoko no-repl
func endsWith(self : Text, p : Pattern) : Bool
```

Returns `true` if the input `Text` ends with a suffix matching the specified `Pattern`.

```motoko include=import
assert Text.endsWith("Motoko", #char 'o');
```

## Function `replace`
``` motoko no-repl
func replace(self : Text, p : Pattern, r : Text) : Text
```

Returns the input text `t` with all matches of pattern `p` replaced by text `r`.

```motoko include=import
let result = Text.replace("abcabc", #char 'a', "A");
assert result == "AbcAbc";
```

## Function `stripStart`
``` motoko no-repl
func stripStart(self : Text, p : Pattern) : ?Text
```

Strips one occurrence of the given `Pattern` from the beginning of the input `Text`.
If you want to remove multiple instances of the pattern, use `Text.trimStart()` instead.

```motoko include=import
// Try to strip a nonexistent character
let none = Text.stripStart("abc", #char '-');
assert none == null;
// Strip just one '-'
let one = Text.stripStart("--abc", #char '-');
assert one == ?"-abc";
```

## Function `stripEnd`
``` motoko no-repl
func stripEnd(self : Text, p : Pattern) : ?Text
```

Strips one occurrence of the given `Pattern` from the end of the input `Text`.
If you want to remove multiple instances of the pattern, use `Text.trimEnd()` instead.

```motoko include=import
// Try to strip a nonexistent character
let none = Text.stripEnd("xyz", #char '-');
assert none == null;
// Strip just one '-'
let one = Text.stripEnd("xyz--", #char '-');
assert one == ?"xyz-";
```

## Function `trimStart`
``` motoko no-repl
func trimStart(self : Text, p : Pattern) : Text
```

Trims the given `Pattern` from the start of the input `Text`.
If you only want to remove a single instance of the pattern, use `Text.stripStart()` instead.

```motoko include=import
let trimmed = Text.trimStart("---abc", #char '-');
assert trimmed == "abc";
```

## Function `trimEnd`
``` motoko no-repl
func trimEnd(self : Text, p : Pattern) : Text
```

Trims the given `Pattern` from the end of the input `Text`.
If you only want to remove a single instance of the pattern, use `Text.stripEnd()` instead.

```motoko include=import
let trimmed = Text.trimEnd("xyz---", #char '-');
assert trimmed == "xyz";
```

## Function `trim`
``` motoko no-repl
func trim(self : Text, p : Pattern) : Text
```

Trims the given `Pattern` from both the start and end of the input `Text`.

```motoko include=import
let trimmed = Text.trim("---abcxyz---", #char '-');
assert trimmed == "abcxyz";
```

## Function `compareWith`
``` motoko no-repl
func compareWith(self : Text, other : Text, compare : (Char, Char) -> Order.Order) : Order.Order
```

Compares `t1` and `t2` using the provided character-wise comparison function.

```motoko include=import
import Char "mo:core/Char";

assert Text.compareWith("abc", "ABC", func(c1, c2) { Char.compare(c1, c2) }) == #greater;
```

## Function `encodeUtf8`
``` motoko no-repl
func encodeUtf8(self : Text) : Blob
```

Returns a UTF-8 encoded `Blob` from the given `Text`.

```motoko include=import
let blob = Text.encodeUtf8("Hello");
assert blob == "\48\65\6C\6C\6F";
```

## Function `decodeUtf8`
``` motoko no-repl
func decodeUtf8(self : Blob) : ?Text
```

Tries to decode the given `Blob` as UTF-8.
Returns `null` if the blob is not valid UTF-8.

```motoko include=import
let text = Text.decodeUtf8("\48\65\6C\6C\6F");
assert text == ?"Hello";
```

## Function `toLower`
``` motoko no-repl
func toLower(self : Text) : Text
```

Returns the text argument in lowercase.
WARNING: Unicode compliant only when compiled, not interpreted.

```motoko include=import
let text = Text.toLower("Good Day");
assert text == "good day";
```

## Function `toUpper`
``` motoko no-repl
func toUpper(self : Text) : Text
```

Returns the text argument in uppercase. Unicode compliant.
WARNING: Unicode compliant only when compiled, not interpreted.

```motoko include=import
let text = Text.toUpper("Good Day");
assert text == "GOOD DAY";
```

## Function `toText`
``` motoko no-repl
func toText(self : Text) : Text
```

Returns the given text value unchanged.
This function is provided for consistency with other modules.

```motoko include=import
assert Text.toText("Hello") == "Hello";
```
