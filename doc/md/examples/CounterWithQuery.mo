actor Counter {

  var count = 0;

  // ...

  public shared query func peek() : async Nat {
    count
  };

}
