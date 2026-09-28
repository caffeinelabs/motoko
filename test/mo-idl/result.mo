actor {
  public type Result<Ok, Err> = {
    #ok:Ok;
    #err:Err;
  };
  public type Result2<Ok> = {
    #ok:Ok;
    #err:Result<Ok,Text>;
  };
  public type Result3<Ok> = {
    #ok:Ok;
    #err:Result2<Ok>;
  };
  public func f(x:?Nat):async Result<Nat,Text> {
    switch x {
      case ? x { #ok x }
      case null { #err "error" }
    }
  };
  public func g(_x:Result3<()>):async Result2<Nat> {
    #ok(1);
  };
}
