//MOC-FLAG -A=M0194
import Prim "mo:⛔";
// test type components are ignored for typ_hash applications (eq, debug, serialization)
actor a {
  private class C(x:Int) {
    public type T = Int;
    public let f = x;
  };

  private class D(x:Int) {
    public type T = Bool;
    public let f = x;
  };

  let eqC = C(0) == C(0);
  let eqD = D(0) == D(0);

  let tc = debug_show(C(1));
  let td = debug_show(D(1));
  Prim.debugPrint(tc);
  Prim.debugPrint(td);

  public func f(c : C) : async C { return C(1) };

  public func g(d : D) : async D { return D(1) };

  public func go() : async () {
    assert ((await f(C(0))) == C(1));
    assert ((await g(D(0))) == D(1));
  };

};

await a.go(); //OR-CALL ingress go "DIDL\x00\x00"

