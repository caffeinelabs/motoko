import Prim "mo:⛔";

// test candid subtype test with higher-order arguments
actor this {

  type Enum1 = {
    #A : Int;
    #B
  };

  type Enum2 = {
    #A : Int;
  };

  public func get_A() : async Enum1 {
    #A(1)
  };

  public func get_B() : async Enum1 {
    #B
  };

  public func go() : async () {

    let t = debug_show (Prim.principalOfActor(this));

    // positive tests
    do {
      let (#A _) : Enum1 = await this.get_A();
      Prim.debugPrint("pass1");
    };

    do {
      let this = actor (t) : actor {
        get_A : () -> async Enum2;
      };
      let (#A _) : Enum2 = await this.get_A();
      Prim.debugPrint("pass1");
    };

    do {
      let this = actor (t) : actor {
        get_B : () -> async Enum2;
      };
      try {
        let _ : Enum2 = await async { await this.get_B(); };
        assert false;
      } catch e {
        Prim.debugPrint("pass3: " # Prim.errorMessage(e));
      };
    };

  };
}
//SKIP run
//SKIP run-ir
//SKIP run-low
//CALL ingress go "DIDL\x00\x00"
