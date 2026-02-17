//MOC-FLAG -A=M0194
// test legacy syntax works too
import {debugPrint} "mo:prim";

module Nat {
  public func toText(n : Nat) : Text { debug_show n };
};


module Int {
  public func toText(i : Int) : Text { debug_show i };
};


module Array {

  public func toText<T>(as : [T], toText : (implicit : T -> Text)) : Text {
     var t = "";
     for (a in as.vals()) {
       t := t # (toText(a));
     };
     t
  }

};


module Pair {

  public func toText<T,U>(
    toTextT : (implicit : (toText : T -> Text)),
    toTextU : (implicit : (toText : U -> Text)),
    p : (T, U))
    : Text {
      "(" # toTextT(p.0) # "," # toTextU(p.1) # ")"
    };

};


func test () {

  debugPrint(Array.toText([1,2,3], Nat.toText)); // explicit arguments
  debugPrint(Array.toText([1,2,3])); // implicit arguments

  debugPrint(Pair.toText(Nat.toText, Nat.toText, (1,2)));
  let p = (0,1);
  debugPrint(let _ = Pair.toText((1,2))); // implicit arguments
  debugPrint(let _ = Pair.toText((1,2))); // implicit arguments

};

test();
