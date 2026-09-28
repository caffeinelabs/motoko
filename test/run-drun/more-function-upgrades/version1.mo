actor {
  public shared query func test() : async () { loop {} };
  let shared_function = test;
};
