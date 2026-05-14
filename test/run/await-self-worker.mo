// Self-actor worker exception: `await*` on a call to a `public` (shared)
// method of this same actor that returns `async T` is permitted; the
// AST-interpreter treats it the same as a plain `await`.

persistent actor a {
  public func foo() : async Int { 42 };

  public func go() : async () {
    let n = await* foo();
    assert n == 42;
  };
};

a.go();

//SKIP comp
//SKIP run-ir
//SKIP run-low
