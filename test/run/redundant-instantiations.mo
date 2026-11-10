//MOC-FLAG -W M0223
import Prim "mo:prim";
// Empty instantiations, no warning
func empty<>(x : Int): Int = x;
assert empty<>(1) == 1;
assert empty(1) == 1;

let vaNat: [var Nat] = [var 1];
let vaNat8: [var Nat8] = [var 1];
let vaInt: [var Int] = [var 1];
let vaInt8: [var Int8] = [var 1];
let vaFloat: [var Float] = [var 1];
let vaText: [var Text] = [var "abc"];
let vaBlob: [var Blob] = [var "abc"];
func inferred<T>(x : T): T = x;

// Inferred instantiations, no warning
let n1 = inferred(1); // defaults to Nat
vaNat[0] := n1; // Check that it's a Nat
let n2 = inferred<Nat>(1); // Redundant!
vaNat[0] := n2;
let n3 = inferred<Nat8>(1); // Not redundant, Nat8 case
vaNat8[0] := n3;

let i1 = inferred<Int>(1);
vaInt[0] := i1;
let i2 = inferred(-1);
vaInt[0] := i2;
let i3 = inferred<Int>(-1); // Redundant
vaInt[0] := i3;
let i4 = inferred<Int8>(-1);
vaInt8[0] := i4;

let f1 = inferred(1.0);
vaFloat[0] := f1;
let f2 = inferred<Float>(1.0); // Redundant
vaFloat[0] := f2;
let f3 = inferred<Float>(1);
vaFloat[0] := f3;

let t1 = inferred("abc");
vaText[0] := t1;
let t2 = inferred<Text>("abc"); // Redundant
vaText[0] := t2;
let t3 = inferred<Blob>("blob");
vaBlob[0] := t3;

module Nested {
  type R = { x : Int; y : Nat };
  public func testVarTab0() {
    var _grid = Prim.Array_tabulate<[var ?R]>( // Redundant
      3,
      func _ = Prim.Array_tabulateVar<?R>(5, func _ = null), // Should NOT be redundant. Only one of these two is redundant.
    );
  };
  public func testVarTab1() {
    var _grid = Prim.Array_tabulate<[var ?R]>(
      3,
      func _ = Prim.Array_tabulateVar(5, func _ = null),
    );
  };
  public func testVarTab2() {
    var _grid = Prim.Array_tabulate(
      3,
      func _ = Prim.Array_tabulateVar<?R>(5, func _ = null),
    );
  };
};