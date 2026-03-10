import Prim "mo:prim";

module X {
   public func f() {
    type Callbacks = {
      var call : Nat -> ();
    };
    let callbacks : Callbacks = {
      var call = func(_) { Prim.debugPrint "called"};
    };
    callbacks.call(1);
  };
};

X.f();
