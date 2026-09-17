import Prim "mo:prim";

actor {

  public func test() : async () {
    let myPrincipal = Prim.getSelfPrincipal<system>();
    Prim.debugPrint(debug_show myPrincipal);
  };
};

//SKIP run
//SKIP run-ir
//SKIP run-low

//CALL ingress test "DIDL\x00\x00"
