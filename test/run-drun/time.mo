import Prim "mo:⛔";
actor a {
  public func go() : () {
    // Time should be constant within the function execution
    // (drun currently returns 0 anyways)
    assert(Prim.time() == Prim.time());
  }
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
