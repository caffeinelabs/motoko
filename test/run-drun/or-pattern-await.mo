import Prim "mo:⛔";

// `let` or-patterns that bind variables, in blocks containing an `await`
// (the await pass renames and declares them)
actor {
  func show(t : ?Text) : Text {
    switch t {
      case null { "null" };
      case ?s { s }
    }
  };

  public func same(x : {#a : Nat; #b : Nat}) : async Nat {
    let (#a v) or (#b v) = x;
    await async ();
    v
  };

  // arms bind `v` at different types: `Null` and `?Text`
  public func mixed(x : {#a : Null; #b : ?Text}) : async Text {
    let (#a v) or (#b v) = x;
    await async ();
    show(v)
  };

  // unrelated arm types: `v : ?Any`, not `Any`
  public func unrelated(x : {#a : ?Nat64; #b : ?Text}) : async Bool {
    let (#a v) or (#b v) = x;
    await async ();
    switch v {
      case null { false };
      case ?_ { true }
    }
  };

  // several names, nested in a tuple, and a closure capturing them
  public func nested(x : ({#a : (Nat, Text); #b : (Nat, Text)}, Int)) : async Text {
    let ((#a (n, t)) or (#b (n, t)), i) = x;
    let f = func() : Text { t # " " # debug_show(n, i) };
    await async ();
    f() # " / " # t # " " # debug_show(n + 1)
  };

  public func go() : async () {
    Prim.debugPrint(debug_show(await same(#a(1)), await same(#b(2))));
    Prim.debugPrint(await mixed(#a(null)));
    Prim.debugPrint(await mixed(#b(?"text")));
    Prim.debugPrint(debug_show(await unrelated(#a(?7)), await unrelated(#b(null))));
    Prim.debugPrint(await nested((#a(3, "left"), -1)));
    Prim.debugPrint(await nested((#b(4, "right"), -2)))
  }
}

//CALL ingress go "DIDL\x00\x00"
