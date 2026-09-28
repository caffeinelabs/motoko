import { debugPrint } = "mo:⛔";

actor Self {
  let stored : shared () -> async () = Self.method;
  public func method() : async () { debugPrint "Hey!" };
  public func go() : async () {
    assert stored == stored;
    //assert go != stored;
    assert Self.method == Self.method;
    assert Self.go != Self.method;
    //assert stored == method;
    assert stored == Self.method;
    await stored()
  };
};

Self.go(); //OR-CALL ingress go "DIDL\x00\x00"
