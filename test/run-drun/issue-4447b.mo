import { cyclesAdd } = "mo:⛔";
// test <system> classes with varying generic arities
actor {

  class C<system>() {

    public func add<system>() {
      cyclesAdd<system>(100);
    };

    add<system>();

  };

  class D<system, T>(t : T) {
    public func add<system>(_t : T) {
      cyclesAdd<system>(100);
    };

    add<system>(t);

  };

  class E<system, T, U>(t : T, u: U) {

    public func add<system>(_t : T, _u : U) {
      cyclesAdd<system>(100);
    };

    add<system>(t, u);

  };

  public func go() : async () {

    let c = C<system>();
    c.add<system>();

    let d = D<system, Nat>(0);
    d.add<system>(1);

    let e = E<system, Nat, Bool>(0, false);
    e.add<system>(1, true);

  }
}
//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress go "DIDL\x00\x00"
