//MOC-FLAG -A=M0194
// test endpoints taking a single arg that is a tuple
actor class TestAPI() {
  public func endpoint1(x: (Nat, Nat)) : async () {}; // compiler bug
  public func endpoint2(x:Nat, y: Nat) : async () {}; // works

  system func inspect(
     {
       caller : Principal; // unused, could be omitted
       arg : Blob;
       msg : {
         #endpoint1 : () -> ((Nat, Nat));
         #endpoint2 : () -> (Nat, Nat);
       }
     }) : Bool {
    switch (msg) {
      case (#endpoint1 f1) { let ((_,_),) = f1(); true };
      case (#endpoint2 f2) { let (_,_) = f2(); true };
    }
  }
};
