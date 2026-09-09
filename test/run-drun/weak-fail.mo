//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//MOC-FLAG --enhanced-orthogonal-persistence
import Prim = "mo:prim";

// test failure on illegal payloads
actor a {

  transient let weak_ = Prim.allocWeakRef;

  let _ok = [
    weak_ "abc",
    weak_ ("" : Blob),
    weak_ {},
    weak_ {x = 1},
    weak_ ([]),
    weak_ ([var]),
    weak_ (#ok),
    weak_ (#ok "hello"),
    weak_ (?"Hello"),
    weak_ (?null),
    weak_ (1,2),
    weak_ (func(){}),
    weak_ (func(){}),
  ];

  public func go() : async () {

    try {
      await async {
        let v = ();
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v = 'a';
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v = true;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v = false;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};


    try {
      await async {
        let v : Nat = 1;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v : Nat = 0xFFFF_FFFF_FFFF_FFFF;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v : Nat8 = 0;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v : Nat16 = 0;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v : Nat32 = 0;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v : Nat64 = 0;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v : Nat64 = 0xFFFF_FFFF_FFFF_FFFF;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};


    try {
      await async {
        let v : Int = 1;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v : Int = 0xFFFF_FFFF_FFFF_FFFF;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v : Int8 = 0;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v : Int16 = 0;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v : Int32 = 0;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v : Int64 = 0;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v : Int64 = 9223372036854775807;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v : Float = 0.0;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

    try {
      await async {
        let v = null;
        ignore weak_(v);
      };
      assert false;
    } catch e {Prim.debugPrint (Prim.errorMessage(e))};

  };

};
a.go();//OR-CALL ingress go "DIDL\x00\x00"
//SKIP run
//SKIP run-ir
//SKIP run-low

