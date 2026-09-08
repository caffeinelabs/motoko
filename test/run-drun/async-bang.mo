import Prim "mo:⛔";

actor a {

  public func go() : async () {
    func print(o : ? Nat) { Prim.debugPrint(debug_show(o));};

    let o1 = do ? {
        let oi = ?1;
        let oj = ?2;
        await async {};
        oi! + oj!;
    };
    print(o1);
    assert (o1 == ? 3);


    let o2 = do ? {
        let oi = ?1;
        let oj : ?Nat = null;
        await async {};
        oi! + oj!;
    };
    print(o2);
    assert (o2 == null);


    let o3 = do ? {
       var sum = 0;
       await async {};
       for(o in [?1, ?2, ?3].vals()) {
         sum += o!
       };
       sum
    };
    print(o3);
    assert (o3 == ? 6);

    let o3Values = do ? {
       var sum = 0;
       await async {};
       for(o in [?1, ?2, ?3].values()) {
         sum += o!
       };
       sum
    };
    assert (o3Values == o3);

    let o4 = do ? {
       var sum = 0;
       await async {};
       for(o in [?1, ?2, null].vals()) {
         sum += o!
       };
       sum
    };
    print o4;
    assert (o4 == null);

    let o4Values = do ? {
       var sum = 0;
       await async {};
       for(o in [?1, ?2, null].values()) {
         sum += o!
       };
       sum
    };
    assert (o4Values == o4);

    /* nesting */

    let o5 = do ? {
       let o = ??0;
       await async {};
       o!!
    };
    print o5;
    assert (o5 == ?0);

    let o6 : ? Nat = do ? {
       let o = ?null;
       await async {};
       o!!
    };
    print o6;
    assert (o6 == null);

    let o7 = do ? {
       let o = (null : ? None);
       await async {};
       o!!
    };
    print o7;
    assert (o7 == null);

  }

};

a.go(); //OR-CALL ingress go "DIDL\x00\x00"
//MOC-FLAG -A=M0269
