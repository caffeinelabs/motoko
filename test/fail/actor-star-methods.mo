//MOC-FLAG --experimental-local-actors
// The public fields of an actor* are local methods: an async* computation or a
// synchronous method, monomorphic, over immutable (projectable) values (M0280).
actor* class A() {
  public query func peek() : async Nat { 0 }; // reject: exported shape outside a container
  public func ident<T>(x : T) : async* T { x }; // reject: generic computation
  public func poly<T>(x : T) : T { x }; // reject: generic synchronous method
  public func put(_xs : [var Nat]) : async* () {}; // reject: mutable parameter
  public func get() : [var Nat] { [var 0] }; // reject: mutable result
  public func fine(n : Nat) : async* Nat { n + 1 }; // accept
  public func also(n : Nat) : Nat { n + 1 }; // accept
};
