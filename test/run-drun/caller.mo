//MOC-FLAG -A=M0194,M0240,M0241
import Prim "mo:⛔";
actor a {

  public shared ctxt func c1 () : async () {
    let c : Principal = ctxt.caller;
    return;
  };

  public shared(c) func c2() : async () {
    let c1 : Principal = c.caller;
    return;
  };

  public shared _ func c3() : async () {
    return;
  };

  public shared({}) func c4() : async () {
  };

  public shared({caller = c}) func c5(c : Bool) : async () { // allow shadowing
    let c1 : Bool = c;
    return;
  };

  public shared query({caller}) func c6() : async () {
    let c1 : Principal = caller;
    return;
  };

  public shared({caller}) func c7() : async Principal {
    return caller;
  };

  public query func c8() : async Principal {
    return Prim.principalOfActor(a);
  };

  // NB: The following tests are more about Blob than Principal
  // Maybe move to their own tests once we have intro forms for blobs

  public shared query({caller}) func c9() : async Nat32 {
    Prim.hashBlob (Prim.blobOfPrincipal caller);
  };

  public shared query({caller}) func c10() : async Nat {
      (Prim.blobOfPrincipal caller).size();
  };

  public shared query({caller}) func c11() : async ?Nat8 {
      (Prim.blobOfPrincipal caller).values().next();
  };

  public shared ({ caller }) func unusedCaller() : async () {};
  public shared ({ caller = c }) func unusedCaller2() : async () {};
};

ignore a.c1(); //OR-CALL ingress c1 0x4449444C0000
ignore a.c2(); //OR-CALL ingress c2 0x4449444C0000
ignore a.c3(); //OR-CALL ingress c3 0x4449444C0000
ignore a.c4(); //OR-CALL ingress c4 0x4449444C0000
ignore a.c6(); //OR-CALL query c6 0x4449444C0000
ignore a.c7(); //OR-CALL ingress c7 0x4449444C0000
ignore a.c8(); //OR-CALL query c8 0x4449444C0000
ignore a.c9(); //OR-CALL query c9 0x4449444C0000
ignore a.c10(); //OR-CALL query c10 0x4449444C0000
ignore a.c11(); //OR-CALL query c11 0x4449444C0000
