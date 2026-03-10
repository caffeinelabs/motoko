import Prim "mo:⛔";
actor a {
  public func go() : () {
    actor b {
      public func bad() : () {
        Prim.debugPrint ("b.bad() called (should not happen)");
      }
    };
    b.bad();
    assert false;
  }
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"


// disabled, because hard to use Prim from inner actor
// a bit sad, because this test is mostly interesting on ic-ref-run

