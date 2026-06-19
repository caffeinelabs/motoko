//MOC-FLAG --experimental-multi-value
import Prim "mo:⛔";
persistent actor {
  func pair(n : Nat64) : (Nat64, Nat64) = (n, n + 1);
  public func go() : async () {
    let (a, b) = pair(42);
    Prim.debugPrint(debug_show a # " " # debug_show b);
  };
}
