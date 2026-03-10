//MOC-FLAG -A=M0194
import P "mo:prim";

actor a {

  public func bar() : async Int {
     666
  };

  private func foo(n : Nat) : async* Int {
    if (n == 0) { return await bar();}
    else await* foo(n-1);
  };

  public func go() : async () {
    assert 666 == (await* foo(100));
  };

};

a.go(); //OR-CALL ingress go "DIDL\x00\x00"
