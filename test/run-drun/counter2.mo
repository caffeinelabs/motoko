import Prim "mo:⛔";
let a = actor {
  flexible var c = 1;
  public func inc() : async () {
    c += 1;
    Prim.debugPrintNat c;
  };
  public func printCounter () : async () {
    Prim.debugPrintNat c;
  };

  public func go() : async () {
    await a.inc();
    await a.inc();
    await a.inc();
    await a.printCounter();

    var i : Int = 10;
    while i > 0 {
      await a.inc();
      i -= 1;
    };
  };
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
