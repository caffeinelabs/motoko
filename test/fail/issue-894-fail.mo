import Prim "mo:⛔";
actor a {
  public shared func test() : async () {
    do {
      let o = { var l = 0 };
      (await async o).l := await async 1; // type error (o mutable, not shared)
      Prim.debugPrintNat(o.l);
      assert o.l == 1;
    };

    do {
      let a = [var 0];
      (await async a)[0] := await async 1; // type error (a mutable, not shared)
      Prim.debugPrintNat(a[0]);
      assert a[0] == 1;
    };
  }
};

a.test();
