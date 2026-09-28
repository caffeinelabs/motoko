import Prim "mo:⛔";

actor a {
  public func go() : async () {
    for check1 in (await async ["effect", "hello", "world"]).values() { Prim.debugPrint check1 };

    for check2 in ["hello", "world", "effect"].values() { await async { Prim.debugPrint check2 } };

    let array = ["hello", "bound", "world"];

    for check3 in (await async array).values() { Prim.debugPrint check3 };

    for check4 in array.values() { await async { Prim.debugPrint check4 } };

    for _ in array.values(await async ()) {}
  }
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
