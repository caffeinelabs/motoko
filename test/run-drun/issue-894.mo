import Prim "mo:⛔";
actor a {
  public shared func id(i : Nat) : async Nat { i };

  public shared func go() : async () {

    do {
      let o = { var l = 0 };
      o.l := 1;
      Prim.debugPrintNat(o.l);
      assert o.l == 1;
    };

    do {
      let o = { var l = 0 };
      o.l := await async 1;
      Prim.debugPrintNat(o.l);
      assert o.l == 1;
    };

    do {
      let a = [var 0];
      a[await async 0] := 1;
      Prim.debugPrintNat(a[0]);
      assert a[0] == 1;
    };

    do {
      let a = [var 0];
      a[await async 0] := await async 1;
      Prim.debugPrintNat(a[0]);
      assert a[0] == 1;
    };

    do {
      let a = Prim.Array_init<Nat>(5, 0);
      for i in a.keys() {
        a[await id(i)] := await id(i);
        Prim.debugPrintNat(a[i]);
        assert a[i] == i;
      };
    };

  };

};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
