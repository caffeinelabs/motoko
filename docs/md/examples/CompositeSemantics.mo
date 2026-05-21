persistent actor Composites {

  var state = 0;

  // ...

  public shared query func q() : async Nat {
    let s = state;
    state += 10;
    s
  };

  public shared composite query func cq() : async Nat {
    let s = state;
    state += 100;
    s
  };

  public shared composite query func test() :
    async {s0 : Nat; s1 : Nat; s2 : Nat; s3 : Nat } {
    let s0 = state;
    state += 1000;
    let s1 = await q();
    state += 1000;
    let s2 = await cq();
    state += 1000;
    let s3 = state;
    {s0; s1; s2; s3}
  };

}
