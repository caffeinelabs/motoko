import Prim "mo:⛔";
/* test n-ary async/await */

actor a {
  /* n-ary args */
  public func f0_0() : async () {};
  public func f1_0(x:Int) : async () {
    assert(x == 1);
  };
  public func f2_0(x:Int,y:Bool) : async () {
    assert(x==1);
    assert(y==true);
  };
  public func f3_0(x:Int,y:Bool,z:Text) : async () {
    assert(x == 1);
    assert(y == true);
    assert(z == "a");
  };

  public func go1() : async () {
    do {
      let t = "0_0";
      await async {
        await f0_0();
        Prim.debugPrint t;
      };
    };

    do {
      let t = "1_0";
      await async {
        await f1_0(1);
        Prim.debugPrint t;
      };
    };

    do {
      let t = "2_0";
      await async {
        await f2_0(1,true);
        Prim.debugPrint t;
      };
    };

    do {
    let t = "3_0";
      await async {
        await f3_0(1,true,"a");
        Prim.debugPrint t;
      };
    };
  };

  /* n-ary returns */
  public func g0_0() : async () {};
  public func g0_1() : async Int {
     1;
  };
  public func g0_2() : async (Int,Bool) {
     (1,true);
  };
  public func g0_3() : async (Int,Bool,Text) {
     (1,true,"a");
  };

  public func go2() : async () {
    do {
      let t = "0_0";
      await async {
        await g0_0();
        Prim.debugPrint t;
      };
    };

    do {
      let t = "0_1";
      ignore await async {
        let x = await g0_1();
        assert(x == 1);
        Prim.debugPrint t;
        x;
      };
    };

    do {
      let t = "0_2";
      ignore await async {
        let (x,y) = await g0_2();
        assert(x==1);
        assert(y==true);
        Prim.debugPrint t;
        (x,y);
      };
    };


    do {
      let t = "0_3";
      ignore await async {
        let (x,y,z) = await g0_3();
        assert(x==1);
        assert(y==true);
        assert(z=="a");
        Prim.debugPrint t;
        (x,y,z);
      };
    };
  };

  /* n-ary async values under subtyping */
  type Pair = (Int, Bool);
  type Unit = ();

  func never() : async* None { throw Prim.error("never") };

  public func go3() : async () {
    do {
      let f : async (Int, Bool) = g0_2();
      let fa : async Any = f;
      ignore await fa;
      let (x, y) = await f;
      assert x == 1 and y;
      let u : async Any = g0_0();
      ignore await u;
      Prim.debugPrint("shared")
    };

    do {
      let p : async Pair = async { (1, true) };
      let pa : async Any = p;
      ignore await pa;
      let (x, y) = await p;
      assert x == 1 and y;
      let u : async Unit = async {};
      let ua : async Any = u;
      ignore await ua;
      await u;
      let n : async None = async { throw Prim.error("none") };
      let np : async (Int, Bool) = n;
      try { ignore await np } catch _ { Prim.debugPrint("caught async None") };
      Prim.debugPrint("async")
    };

    do {
      let c : async* Pair = async* { (1, true) };
      let ca : async* Any = c;
      ignore await* ca;
      let (x, y) = await* c;
      assert x == 1 and y;
      let u : async* Unit = async* {};
      let ua : async* Any = u;
      ignore await* ua;
      await* u;
      let np : async* (Int, Bool) = never();
      try { ignore await* np } catch _ { Prim.debugPrint("caught async* None") };
      let nu : async* () = never();
      try { await* nu } catch _ { Prim.debugPrint("caught async* None") };
      Prim.debugPrint("async*")
    }
  }
};

ignore(a.go1()); //OR-CALL ingress go1 "DIDL\x00\x00"
ignore(a.go2()); //OR-CALL ingress go2 "DIDL\x00\x00"
ignore a.go3() //OR-CALL ingress go3 "DIDL\x00\x00"
