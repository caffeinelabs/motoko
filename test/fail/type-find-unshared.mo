// find_unshared: a local function type is not shareable as a shared-function param.
actor A {
  public shared func bad(f : Nat -> Nat) : async Nat {
    async f(1)
  };
};
