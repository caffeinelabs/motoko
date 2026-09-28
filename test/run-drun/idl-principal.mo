actor {
  public query func fun(id: Principal) : async Principal { return id; };
  public query func fun2(id: ?Principal) : async ?Principal { return id; };
}

//CALL query fun 0x4449444C0001680103CAFFEE
//CALL query fun2 0x4449444C016E680100010103CAFFEE

//SKIP run
//SKIP run-ir
//SKIP run-low
