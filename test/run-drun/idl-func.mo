//MOC-FLAG -A=M0194
actor {
  public type Func = shared Int -> async Func;
  public type Query = shared query () -> async ?Func;
  public type Oneway = shared () -> ();

  public func oneway() : () {};
  public query func fun() : async ?Func { null };
  public query func fun2(arg : ?Func) : async () { };
  public query func fun3() : async Query { fun };
  public query func fun4() : async Oneway { oneway };
}


//CALL query fun 0x4449444C0000
//CALL query fun2 0x4449444c026e016a017c0000010000
//CALL query fun3 0x4449444C0000
//CALL query fun4 0x4449444C0000

//SKIP run
//SKIP run-ir
//SKIP run-low
