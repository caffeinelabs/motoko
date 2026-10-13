//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
import Prim "mo:⛔";

// Opaque type parameters carry ordinary values through generic wrappers.
func identity<T>(value : T) : T = Prim.fixtureIdentity<T>(value);
func apply<A, B>(f : A -> B, value : A) : B {
  Prim.fixtureApply<A, B>(f, value)
};
func twice<T>(f : T -> T, value : T) : T {
  Prim.fixtureTwice<T>(f, value)
};

assert identity<Nat8>(255) == 255;
assert identity<Blob>("opaque blob") == "opaque blob";
assert identity<??Nat>(?null) == ?null;
assert identity<[Nat]>([]) == [];
let any : Any = identity<Any>({ hidden = 19 });
ignore any;

type Pair = (Nat, Text);
func pair(value : Pair) : Pair = (value.0 + 1, value.1 # "!");
assert apply<Pair, Pair>(pair, (17, "tuple")) == (18, "tuple!");
assert Prim.fixtureApply0<Pair>(func () : Pair { (9, "one result") }) ==
  (9, "one result");
assert Prim.fixtureApply2<Nat, Text, Pair>(
  func (number : Nat, text : Text) : Pair { (number, text) }, 7, "two args"
) == (7, "two args");
assert Prim.fixtureSplit<Text, Nat, Text>(
  func (text : Text) : (Nat, Text) { (text.size(), text) }, "two results"
) == (11, "two results");

// Shared payload references include mutable objects, closures and polymorphic
// functions. Keeping an opaque value does not clone or inspect its contents.
let cell = { var count = 4 };
let retained = identity<{ var count : Nat }>(cell);
retained.count += 1;
assert cell.count == 5;
let bump = identity<() -> Nat>(func () : Nat {
  cell.count += 1;
  cell.count
});
assert bump() == 6;
type Generic = <T>(T) -> T;
let generic = identity<Generic>(func <T>(value : T) : T { value });
assert generic<Text>("generic payload") == "generic payload";

var events = "";
let dispatch = Prim.fixtureApply;
let aliases = { dispatch };
let value = (do { events #= "callee;"; aliases.dispatch })<Nat, Nat>(
  do {
    events #= "closure;";
    func (n : Nat) : Nat { events #= "callback;"; n + 1 }
  },
  do { events #= "argument;"; 40 }
);
assert value == 41;
assert events == "callee;closure;argument;callback;";

events := "";
assert Prim.fixtureVisit<Nat>(
  func (n : Nat) { events #= debug_show n }, 8
) == 8;
assert events == "8";
ignore Prim.fixtureApply<Nat, Nat>(func (n : Nat) : Nat {
  events #= ";discarded";
  n + 1
}, 0);
assert events == "8;discarded";

// Nested callbacks can reenter the plugin, capture mutable values, allocate
// managed payloads and return values that the outer invocation retains.
var calls = 0;
let result = twice<[Nat]>(func (numbers : [Nat]) : [Nat] {
  calls += 1;
  let nested = apply<Nat, Nat>(func (n : Nat) : Nat { n + 1 }, calls);
  Prim.Array_tabulate<Nat>(numbers.size() + 1, func (i : Nat) : Nat {
    if (i < numbers.size()) numbers[i] else nested
  })
}, [13]);
assert calls == 2;
assert result == [13, 2, 3];

// Contextual fixed schemas must type empty arrays and numeric literals.
assert Prim.fixtureApply<[Nat8], Nat8>(
  func (values : [Nat8]) : Nat8 { if (values.size() == 0) 255 else values[0] },
  []
) == 255;
assert apply<Nat, Nat>(func (n : Any) : Nat { ignore n; 23 }, 8) == 23;

// A multiargument intrinsic and its scalar-returning callback must not create
// an implicit managed argument tuple per call. Prepare the closure first.
let increment = func (n : Nat) : Nat { n + 1 };
var total = 0;
var iteration = 0;
let allocationBefore = Prim.rts_total_allocation();
while (iteration < 1000) {
  total += Prim.fixtureApply<Nat, Nat>(increment, iteration);
  iteration += 1;
};
let allocationAfter = Prim.rts_total_allocation();
assert total == 500500;
assert allocationAfter == allocationBefore;
