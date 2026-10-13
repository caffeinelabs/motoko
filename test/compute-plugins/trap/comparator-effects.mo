//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/comparators
import Prim "mo:⛔";

type Order = { #less; #equal; #greater };
func compare(x : Nat64, y : Nat64) : Order {
  if (x < y) Prim.trap("custom comparator effect");
  if (x < y) #less else if (x == y) #equal else #greater
};
ignore Prim.fixtureAutoCompare<Nat64>(compare, 1, 9);
