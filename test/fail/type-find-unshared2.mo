// Exercise find_unshared via various non-shareable type shapes:
// - Obj with mutable field (Object sort with var field)
// - Array of non-shared type
// - Opt of non-shared type
// - Tuple containing non-shared type

actor B {
  // Object with mutable field -> find_unshared hits Obj(Object) and finds Mut
  public shared func bad_obj(o : {var n : Nat}) : async Nat {
    async o.n
  };
};
