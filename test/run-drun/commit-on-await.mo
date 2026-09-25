import Prim "mo:⛔";
actor a {

  var s = 0;
  public func ping0(): async () {};

  // this observes how far the trap rolls back
  public func bar0(): async () {
    s := 1;
    let f = ping0();
    s := 2;
    await f;
    s := 3; // this will not be rolled back!
    await f;
    ignore(0 / 0);
  };

  public func ping1(x:Nat): async Nat { x };

  // this observes how far the trap rolls back
  public func bar1(): async () {
    s := 1;
    let f = ping1(1);
    s := 2;
    let 1 = await f;
    s := 3; // this will not be rolled back!
    let 1 = await f;
    ignore(0 / 0);
  };

  public func ping2(x:Nat, y:Nat): async (Nat, Nat) {
    (x, y);
  };

  // this observes how far the trap rolls back
  public func bar2(): async () {
    s := 1;
    let f = ping2(1, 2);
    s := 2;
    let (1, 2) = await f;
    s := 3; // this will not be rolled back!
    let (1, 2) = await f;
    ignore(0 / 0);
  };

  public func ping3(): async () {
    throw (Prim.error("fail"));
  };

  // this observes how far the trap rolls back
  public func bar3(): async () {
    s := 1;
    let f = ping3();
    s := 2;
    try {
      await f;
      assert false
    }
    catch e { assert Prim.errorMessage e == "fail"; };
    s := 3; // this will not be rolled back!
    try {
      await f;
      assert false
    } catch e { assert Prim.errorMessage e == "fail"; };
    ignore(0 / 0);
  };

  public func go() : async () {
    try {
      await bar0();
      Prim.debugPrint("Huh, bar() replied?");
    } catch _ {
      Prim.debugPrint(debug_show s);
    };

    try {
      await bar1();
      Prim.debugPrint("Huh, bar1() replied?");
    } catch _ {
      Prim.debugPrint(debug_show s);
    };

    try {
      await bar2();
      Prim.debugPrint("Huh, bar2() replied?");
    } catch _ {
      Prim.debugPrint(debug_show s);
    };

    try {
      await bar3();
      Prim.debugPrint("Huh, bar3() replied?");
    } catch _ {
      Prim.debugPrint(debug_show s);
    };
  }
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
