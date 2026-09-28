// test lexically nested async* expressions
actor a {

  public func bar() : async Int {
    666
  };

  private func foo(n : Nat) : async* Int {
    let _ = await* async* { "hello" };
    if n == 0 {
      await* async* { await bar(); }
    }
    else { await* { async* { await* foo(n - 1); }; } }
  };

  public func go() : async () {
    assert 666 == (await* foo(100));
  };

};

a.go(); //OR-CALL ingress go "DIDL\x00\x00"
