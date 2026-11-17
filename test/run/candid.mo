// unit
assert (?() == (from_candid(to_candid ()) : ?()));
assert (?() == (from_candid(to_candid (1)) : ?()));
assert (?() == (from_candid(to_candid (1,2)) :?()));


// singleton
assert (?1 == (from_candid(to_candid(1)) : ?Nat));
assert (null == (from_candid(to_candid(-1)) : ?Nat)); // subtype failure
assert (?-1 == (from_candid(to_candid(-1)) : ?Int));

// pairs
assert (?(1,true) == (from_candid(to_candid(1,true)) : ?(Nat,Bool)));
assert (null == (from_candid(to_candid(-1,true)) : ?(Nat,Bool))); // subtype failure
assert (?(-1,true) == (from_candid(to_candid(-1,true)) : ?(Int,Bool)));

// triples
assert (?(1,true,?'a') == (from_candid(to_candid(1,true,'a')) : ?(Nat,Bool,?Char)));
assert (null == (from_candid(to_candid(-1,true,'a')) : ?(Nat,Bool,?Char))); // subtype failure
assert (?(-1,true,?'a') == (from_candid(to_candid(-1,true,'a')) : ?(Int,Bool,?Char)));

// tuples as a single top-level argument
// Note: this is somewhat tricky and relies on two facts:
//       - asking for a non-tuple type extracts the first argument
//       - there are no tuples in Candid, only records that have keys _0_, _1_, etc.
assert (?{_0_ = 1; _1_ = true} == (from_candid(to_candid((1,true))) : ?{_0_ : Nat; _1_ : Bool}));

// defaulting
assert (?(1,true,null) == (from_candid(to_candid(1,true,'a')) : ?(Nat,Bool,?Text)));
assert (null == (from_candid(to_candid(-1,true,'a')) : ?(Nat,Bool,?Text))); // subtype failure
assert (?(-1,true,null) == (from_candid(to_candid(-1,true,'a')) : ?(Int,Bool,?Text)));

// variants
assert (?(#apple))          == (from_candid(to_candid(#apple)) : ?({#apple}));
assert (?(#apple, #banana)) == (from_candid(to_candid(#apple, #banana)) : ?({#apple}, {#apple; #banana}));
assert null                 == (from_candid(to_candid(#apple, #banana)) : ?({#apple}, {#apple})); // subtype failure

// records
assert ?{a = true; b = 137} == (from_candid(to_candid({a = true; b = 137})) : ?{a : Bool; b : Nat});
assert ?{a = true; b = 137} == (from_candid(to_candid({a = true; b = 137; c = 42})) : ?{a : Bool; b : Nat});
assert null == (from_candid(to_candid({a = true; b = 137})) : ?{a : Bool; b : Nat; c : Nat}); // subtype failure

// arrays
let big = 148894445742041325547806458472397916603026273992795324185271289425213239361064475310309971132180337174752834401423587560;
assert ?[] == (from_candid(to_candid([])) : ?[Nat]);
assert ?[1] == (from_candid(to_candid([1])) : ?[Nat]);
assert ?[1, 2] == (from_candid(to_candid([1, 2])) : ?[Nat]);
assert ?[1, big, 2] == (from_candid(to_candid([1, big, 2])) : ?[Nat]); // really big should work!
assert null == (from_candid(to_candid([-1])) : ?[Nat]);  // subtype failure

// don't test for now as output needs normalization, or test in drun using try-catch instead
// traps on invalid candid (check last)
//let trap = from_candid("") : ?Any;

//SKIP run
//SKIP run-ir
//SKIP run-low
