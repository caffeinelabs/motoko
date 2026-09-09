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
     for (a in as.values()) {
       t := t # (toText(a));
     };
     t
  }

};


module Pair {

  public func toText<T,U>(
    p : (T, U),
    toTextT : (implicit : (toText : T -> Text)),
    toTextU : (implicit : (toText : U -> Text)),
    )
    : Text {
      "(" # toTextT(p.0) # "," # toTextU(p.1) # ")"
    };

};


func test () {

  Array.toText([1,2,3], Nat.toText) // explicit arguments
  |> debugPrint _;

  Array.toText([1,2,3]) // implicit arguments
  |> debugPrint _;

  Pair.toText((1,2), Nat.toText, Nat.toText) // explicit arguments
  |> debugPrint _;

  Pair.toText((1,2))
  |> debugPrint _ ; // implicit arguments

};

test();
