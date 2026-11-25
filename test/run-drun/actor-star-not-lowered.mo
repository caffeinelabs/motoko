//MOC-FLAG --experimental-local-actors
// `actor*` type-checks under --experimental-local-actors, but nothing lowers it yet:
// the interpreter only defines the declarations, and compilation stops with M0279.
import Prim "mo:prim";

actor Container {
  type CounterRef = actor* {
    add : Nat -> async* Nat;
    get : () -> Nat;
  };

  // an internal actor*: async* computations and synchronous local methods
  actor* class Counter(initial : Nat) {
    var n = initial;
    public func add(delta : Nat) : async* Nat { n += delta; n };
    public func get() : Nat { n };
  };

  // a public actor* class: its `async` and `query` methods are exported
  // through the container
  public actor* class Account() {
    var balance = 0;
    public func deposit(amount : Nat) : async Nat { balance += amount; balance };
    public query func peek() : async Nat { balance };
  };

  // does not follow upgrades
  transient actor* class _Scratch() {
    var hits = 0;
    public func hit() : Nat { hits += 1; hits };
  };

  func _reconstitute(owner : ?Principal, id : Principal) : CounterRef = actor* (owner, id);

  func _address(c : CounterRef) : { container : Principal; slot : Nat64; generation : Nat64 } =
    Prim.localActorAddress(c);

  public func total() : async Nat {
    let c = Counter(1);
    let a = Account();
    (await* c.add(2)) + (await a.deposit(3))
  };
};
