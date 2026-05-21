shared({caller = owner}) persistent actor class Counter(init : Nat) {

  var count : Nat = init;

  public shared({caller}) func inc() : async () {
    assert (owner == caller);
    count += 1;
  };

  // ...
}
