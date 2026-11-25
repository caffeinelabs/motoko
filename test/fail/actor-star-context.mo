//MOC-FLAG --experimental-local-actors
// actor* rules beyond the method shapes.
actor Container {
  persistent actor* class _P() { // warn: an actor* is persistent by default (M0217)
    var n = 0;
    public func get() : Nat { n };
  };

  actor* class _Q() {
    shared func work() : async* () {}; // reject: a computation context needs a public method
    public func run() : async* () {};
  };

  actor* class _R() {
    // reject: an actor* method's context is its local caller, not a Principal
    public shared ({ caller }) func who() : async* Principal { caller };
  };
};
