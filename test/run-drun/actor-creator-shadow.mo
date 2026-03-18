//MOC-FLAG -A=M0240
shared(c) actor class C () {
  let c = 1;

  public shared ctxt func c1 () : async Nat {
     assert (c == 1);
     return c;
  };

};

ignore (await C()).c1(); //OR-CALL ingress c1 0x4449444C0000




