// Structural comparison on tuples (lexicographic)
assert((1, 2) < (1, 3));
assert((1, 2) < (2, 0));
assert((1, 2) <= (1, 2));
assert((1, 2) >= (1, 2));
assert(not ((1, 2) > (1, 2)));
assert((2, 0) > (1, 99));

// Structural comparison on records (lexicographic by field name)
assert({ a = 1; b = 2 } < { a = 1; b = 3 });
assert({ a = 1; b = 2 } < { a = 2; b = 0 });
assert({ a = 1; b = 2 } <= { a = 1; b = 2 });
assert({ a = 1; b = 2 } >= { a = 1; b = 2 });
assert(not ({ a = 1; b = 2 } > { a = 1; b = 2 }));

// Records: field ordering is alphabetical by label
// "a" < "z", so {a=2; z=0} vs {a=1; z=99} compares a first => 2 > 1
assert({ a = 2; z = 0 } > { a = 1; z = 99 });

// Structural comparison on options (null < ?x)
assert((null : ?Nat) < ?10);
assert((null : ?Nat) < ?0);
assert(?5 < ?10);
assert(?10 > ?5);
assert((null : ?Nat) <= (null : ?Nat));
assert(?10 <= ?10);
assert(?10 >= ?10);

// Structural comparison on arrays (lexicographic, shorter < longer for prefixes)
assert([1, 2, 3] < [1, 2, 4]);
assert([1, 2] < [1, 2, 3]);
assert([1, 2, 3] > [1, 2]);
assert([1, 2, 3] <= [1, 2, 3]);
assert([1, 2, 3] >= [1, 2, 3]);
assert([] < ([1] : [Nat]));

// Structural comparison on variants (alphabetical by tag, then payload)
assert((#a : { #a; #b }) < #b);
assert((#b : { #a; #b }) > #a);
assert((#ok(1) : { #ok : Nat; #err : Text }) < #ok(2));
assert((#ok(1) : { #ok : Nat; #err : Text }) <= #ok(1));
assert((#err("x") : { #ok : Nat; #err : Text }) < #ok(0)); // "err" < "ok" alphabetically

// Nested structures
assert((1, [2, 3]) < (1, [2, 4]));
assert({ x = ?1; y = [1] } < { x = ?2; y = [0] });

// Bool comparison (false < true)
assert(false < true);
assert(not (true < false));
assert(true >= false);
assert(false <= true);
assert(false <= false);
assert(true >= true);

// Bool inside structured types
assert((false, 1) < (true, 0));
assert((true, 1) > (false, 99));
assert([false, true] < [true, false]);
assert([false] < [true]);
assert({ flag = false; val = 10 } < { flag = true; val = 0 });
assert(?false < ?true);
assert((#off : { #off; #on }) < #on);  // "off" < "on" alphabetically

// Null comparison (singleton: always equal)
assert((null : Null) <= (null : Null));
assert((null : Null) >= (null : Null));
assert(not ((null : Null) < (null : Null)));
assert(not ((null : Null) > (null : Null)));

// Null with subtyping via options (null < ?x)
assert([?1, null] <= [?1, ?0]);
assert([?1, null] < [?1, ?0]);
assert([null, null] < [null, ?0]);
assert([null] < [?0]);
assert((?null : ??Nat) < ??0);
assert(((null, null) : (?Nat, ?Nat)) < (null, ?1));
assert(((null, ?5) : (?Nat, ?Nat)) < (?0, null));

// Char comparison
assert('a' < 'b');
assert('z' > 'a');

// Text comparison
assert("abc" < "abd");
assert("abc" < "abcd");
assert("" < "a");

// Recursive types
type List = ?(Nat, List);

let xs : List = ?(1, ?(2, null));
let ys : List = ?(1, ?(3, null));
assert(xs < ys);
assert(ys > xs);

let zs : List = ?(1, ?(2, ?(3, null)));
assert(xs < zs);  // shorter prefix < longer
