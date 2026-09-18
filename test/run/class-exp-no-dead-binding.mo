import Prim "mo:⛔";

type It = { next : () -> ?Nat };

// A `class` in expression position must not leave a binding for its anonymous
// constructor or object behind: both were bound and never read, costing two
// dead `local.tee`s per class. The emitted code should match the equivalent
// `func () : It = object { ... }`.
func mk() : () -> It = class() {
  var i = 0;
  public func next() : ?Nat { let j = i; i += 1; ?j };
};

let it = mk()();
ignore it.next();
switch (it.next()) {
  case (?n) Prim.debugPrint("second=" # debug_show n);
  case null Prim.debugPrint("none");
};

// A class that *does* mention its own object still needs the binding, and must
// keep working.
class Counter() = self {
  public var hits = 0;
  public func bump() : Counter { hits += 1; self };
};
let c = Counter();
ignore c.bump().bump();
Prim.debugPrint("hits=" # debug_show c.hits);

//CHECK-NOT: local $@anon-class
//CHECK-NOT: local $@anon-object
