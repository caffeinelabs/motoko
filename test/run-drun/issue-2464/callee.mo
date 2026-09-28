actor class Callee() {
  private var a : Nat = 1;

  public func add1() : async Nat {
    a += 1;
    a
  };

  public query func get() : async Nat {
    a
  };
};
