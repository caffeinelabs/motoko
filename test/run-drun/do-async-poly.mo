import P "mo:prim";
// test polymorphic async compilation
// Currently rejected because type parameters aren't shared.
// If we (in future) allow type parameters in async types, then we should enforce that  `T` in async* `T` is a manifest tuple
// so that async arity is invariant under substition.
actor a {

  private func doUnit<T>(t : T) : async* T {
    let t = await async null; // await at unit type
    return t;
  };

  private func doText<T>(t : T) : async* T {
    let t1 = await async "text"; // await at different type
    return t;
  };

  private func doReturn<T>(t : T) : async* T {
    return t;
  };

  private func doExit<T>(t : T) : async* T {
    null;
  };

  private func doThrow<T>(t : T) : async* T {
    throw P.error("oops");
  };

  public func go() : async () {
    let u = ();
    await* doUnit<()>(u);
    await* doReturn<()>(u);
    await* doExit<()>(u);
    try {
      await* doThrow<()>();
      assert(false);
    } catch (e) { assert P.errorMessage(e) == "oops";};
  }

};


a.go(); //OR-CALL ingress go "DIDL\x00\x00"
