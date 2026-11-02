import Prim "mo:⛔";
let a = actor {
  transient var c = 0;
  public func incn(n : Nat) : async () {
    c += n;
    Prim.debugPrintNat(c);
  };
  public func readCounter(f : shared Nat -> async ()) : async () {
    await f(c);
  };

  public func printNat(n: Nat) : async () { Prim.debugPrintNat n };

  public func go() : async () {
    await incn(1);
    await readCounter(printNat);
    await incn(2);
    await readCounter(printNat);
    await incn(3);
    await readCounter(printNat);
    await incn(4);
    await readCounter(printNat);
    await incn(5);
    await readCounter(printNat);
  };
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
