//MOC-FLAG --package core $MOTOKO_CORE --all-libs
import Array "mo:core/Array";
import { type Order } "mo:core/Order";

module Pair {
  public func compare<A, B>(
    a : (A, B),
    b : (A, B),
    cmpA : (implicit : (compare : (A, A) -> Order)),
    cmpB : (implicit : (compare : (B, B) -> Order)),
  ) : Order { #equal };
};

// Shallow: leaf suggests importing mo:core/Bool
func needsBoolArrayCompare(
  a : [Bool],
  b : [Bool],
  compare : (implicit : ([Bool], [Bool]) -> Order),
) : Order { compare(a, b) };
ignore needsBoolArrayCompare([true], [false]);

// Deep chain: [[Bool]] → Array<[Bool]> → Array<Bool> → suggest mo:core/Bool
func needsNestedBoolArrayCompare(
  a : [[Bool]],
  b : [[Bool]],
  compare : (implicit : ([[Bool]], [[Bool]]) -> Order),
) : Order { compare(a, b) };
ignore needsNestedBoolArrayCompare([[true]], [[false]]);

type Color = { #red; #green; #blue };

// Multi-branch: Pair<[Color], [Int]> with two failing branches, different reasons
func needsPairCompare(
  a : ([Color], [Int]),
  b : ([Color], [Int]),
  compare : (implicit : (([Color], [Int]), ([Color], [Int])) -> Order),
) : Order { compare(a, b) };
ignore needsPairCompare(([#red], [1]), ([#blue], [2]));
