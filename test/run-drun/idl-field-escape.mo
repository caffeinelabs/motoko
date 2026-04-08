//MOC-FLAG -A=M0194
// This test checks that the IDL encoder/decode implements
// the field escape logic
actor {
  type R = {
    _0_ : Int;
    _1_ : Nat;
    async_ : Text
  };

  public query func out() : async R {
    { _0_ = 0xFFFF;
      _1_ = 0x1000;
      async_ = "XXX"
    }
  };

  public query func foo1() : async {foo_ : ()} { { foo_ = () } };
  public query func foo2() : async {foo : ()} { { foo = () } };

  public query func input(r : R) : async () {
  };
}

//CALL query foo1 0x4449444C0000
//CALL query foo2 0x4449444C0000
//CALL query out 0x4449444C0000
//CALL query input 0x4449444c016c03007c017dbcfef7b102710100ffff03802003585858

//SKIP run
//SKIP run-ir
//SKIP run-low
