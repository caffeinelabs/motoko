// find_unshared: a mutable object field disqualifies a shared-function param.
actor B {
  public shared func bad_obj(o : {var n : Nat}) : async Nat {
    async o.n
  };
};
