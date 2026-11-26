import Prim "mo:prim";

// Candid decoding finds the case of a variant value by its position in the variant type of the
// message. It walks past the earlier cases to reach one of the first 8 positions, and looks later
// positions up in an index of the cases, which only variant types with more than 8 cases get.
//
// Every case of variant types with 8, 9 and 20 cases is decoded, and skipped as a record field
// that the receiving type lacks. The cases carry payloads of different types, so that skipping a
// value as the wrong case would read the next record from the wrong place.
// `from_candid` is unimplemented in the interpreters, so only the compiled backend runs this.

type V8 = {
  #c00;
  #c01 : Nat8;
  #c02 : Nat16;
  #c03 : Nat32;
  #c04 : Nat64;
  #c05 : Text;
  #c06 : Bool;
  #c07 : Int
};

type V9 = {
  #c00;
  #c01 : Nat8;
  #c02 : Nat16;
  #c03 : Nat32;
  #c04 : Nat64;
  #c05 : Text;
  #c06 : Bool;
  #c07 : Int;
  #c08 : Float
};

type V20 = {
  #c00;
  #c01 : Nat8;
  #c02 : Nat16;
  #c03 : Nat32;
  #c04 : Nat64;
  #c05 : Text;
  #c06 : Bool;
  #c07 : Int;
  #c08 : Float;
  #c09 : Int8;
  #c10 : Int16;
  #c11 : Int32;
  #c12 : Int64;
  #c13 : Nat;
  #c14 : [Nat8];
  #c15 : (Nat, Text);
  #c16 : ?Nat;
  #c17 : { x : Nat };
  #c18 : [Text];
  #c19 : Blob
};

let v8 : [V8] = [#c00, #c01(1), #c02(2), #c03(3), #c04(4), #c05("five"), #c06(true), #c07(-700)];

let v9 : [V9] = [
  #c00,
  #c01(1),
  #c02(2),
  #c03(3),
  #c04(4),
  #c05("five"),
  #c06(true),
  #c07(-700),
  #c08(8.5)
];

let v20 : [V20] = [
  #c00,
  #c01(1),
  #c02(2),
  #c03(3),
  #c04(4),
  #c05("five"),
  #c06(true),
  #c07(-700),
  #c08(8.5),
  #c09(-9),
  #c10(-10),
  #c11(-11),
  #c12(-12),
  #c13(13_000),
  #c14([1, 4]),
  #c15(15, "fifteen"),
  #c16(?16),
  #c17({ x = 17 }),
  #c18(["eighteen"]),
  #c19("\01\09")
];

// Decoding the records `{ a = i; v = vs[i] }` as records without the field `v` gives `{ a = i }`
func numbered(n : Nat) : [{ a : Nat }] = Prim.Array_tabulate<{ a : Nat }>(n, func(i) = { a = i });

for v in v8.values() {
  let back : ?V8 = from_candid (to_candid (v));
  assert back == ?v
};
let skipped8 : ?[{ a : Nat }] = from_candid (
  to_candid (Prim.Array_tabulate<{ a : Nat; v : V8 }>(v8.size(), func(i) = { a = i; v = v8[i] }))
);
assert skipped8 == ?numbered(v8.size());

for v in v9.values() {
  let back : ?V9 = from_candid (to_candid (v));
  assert back == ?v
};
let skipped9 : ?[{ a : Nat }] = from_candid (
  to_candid (Prim.Array_tabulate<{ a : Nat; v : V9 }>(v9.size(), func(i) = { a = i; v = v9[i] }))
);
assert skipped9 == ?numbered(v9.size());

for v in v20.values() {
  let back : ?V20 = from_candid (to_candid (v));
  assert back == ?v
};
let skipped20 : ?[{ a : Nat }] = from_candid (
  to_candid (Prim.Array_tabulate<{ a : Nat; v : V20 }>(v20.size(), func(i) = { a = i; v = v20[i] }))
);
assert skipped20 == ?numbered(v20.size())

//SKIP run
//SKIP run-ir
//SKIP run-low
