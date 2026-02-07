actor a {

  type Shob = { a : Int; b : { c : ?Nat } };

  flexible let foo : Shob = { a = 17; b = { c = ?25 } };

  // check whether we can pattern match shared objects

  public func baz(sh : Shob) : async Int = async (switch sh {
    case {a; b = {c = null}} a;
    case {a; b = {c = ?c}} (a + c)
  });

  public func go() : () {
    let b = await (baz foo);
    assert (b == 42);
  };
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
