// Exercise find_unshared via unshareable types in shared functions.
// find_unshared is called to detect what makes a type non-shareable.
// Non-shareable: Mut, Func (local), Module (TODO:1452), Obj with mutable fields.

// Mutable field inside a stable-typed parameter -> not shareable
// Local function type -> not shareable in shared context
actor A {
  // Local function type is not shareable (find_unshared: Func Local)
  public shared func bad(f : Nat -> Nat) : async Nat {
    async f(1)
  };
};
