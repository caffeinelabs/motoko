actor {
   public shared query func test(x: ?Nat) : async ?Int { loop {} };
   let shared_function = test;
};
