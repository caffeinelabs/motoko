import Prim "mo:⛔";
actor a {

  var s = 0;

  public func ping(): async () {};

  // this observes how far the trap rolls back
  public func bar(): async () {
    s := 1;
    let f = ping();
    s := 2;
    await f;
    s := 3; // this will not be rolled back!
    await f;
    ignore(0 / 0);
  };

  public func go() : async () {
    try {
      await bar();
      Prim.debugPrint("Huh, bar() replied?");
    } catch _ {
      Prim.debugPrint(debug_show s);
    };
  }
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
