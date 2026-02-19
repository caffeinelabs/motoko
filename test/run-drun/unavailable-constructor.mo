//MOC-FLAG -A=M0240
actor class C() {

  let _ : Any = C;

  public shared ctxt func m () : async () {
     let _ : actor {} = await C();
  };

};

ignore (await C()).m() //OR-CALL ingress m 0x4449444C0000

