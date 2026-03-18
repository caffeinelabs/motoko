//MOC-FLAG -A=M0194
import Prim "mo:⛔";
actor a {
  public func go() : async () {
    let o = object {
      public var x = await { async { 1 } };
      let a = Prim.debugPrintNat(x);
      // private b = (x := await { async (x + 1) });
      let b = (x := x + 1);
      let c = Prim.debugPrintNat(x);
      public func foo() { x := x + 1 };
      let e = foo();
      let f = Prim.debugPrintNat(x);
    };
    Prim.debugPrint("done creating\n");
    Prim.debugPrintNat(o.x);
    o.x := o.x + 1;
    Prim.debugPrintNat(o.x);
    o.foo();
    Prim.debugPrintNat(o.x);
  }
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
