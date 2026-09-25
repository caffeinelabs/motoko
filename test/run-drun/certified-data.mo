import Prim "mo:⛔";
actor {

  public shared func set() : async () {
    Prim.setCertifiedData("Hello");
  };

  public shared query func get() : async Bool {
    switch Prim.getCertificate() {
      case null { return false; }
      case ?_ { return true; }
    }
  };
};

//CALL query get 0x4449444C0000
//CALL ingress get 0x4449444C0000

//CALL ingress set 0x4449444C0000

//CALL query get 0x4449444C0000
//CALL ingress get 0x4449444C0000

//SKIP run
//SKIP run-ir
//SKIP run-low
