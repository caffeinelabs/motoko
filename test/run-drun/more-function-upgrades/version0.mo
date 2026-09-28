actor {
  public shared query func test(x: ?Nat) : async () { loop {} };
  let shared_function = test;
};
