//MOC-FLAG -A=M0240
shared({caller = creator}) actor class C() {

  let c : Principal = creator;
  let d = creator;

  public shared ctxt func c1 () : async Principal {
     return c;
  };

  public shared ctxt func c2 () : async Principal {
     return d;
  };


};

let c = await C(); ignore c.c1(); //OR-CALL ingress c1 0x4449444C0000
ignore c.c2(); //OR-CALL ingress c2 0x4449444C0000
