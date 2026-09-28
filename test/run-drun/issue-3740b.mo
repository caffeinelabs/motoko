actor {
  type Pair = (Nat, Bool);

  public func f() : async Pair {
    return (0, true);
  };

  public func go() : async () {
    let (0, true) = await f();
  };

}
