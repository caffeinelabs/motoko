//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
import Prim "mo:⛔";

type Pair = (Nat, Text);
func identity<T>(value : T) : T { Prim.fixtureIdentity<T>(value) };
func unpack<T>(value : T) : T { Prim.fixtureUnpack<T>(value) };
func pair<A, B>(a : A, b : B) : (A, B) { Prim.fixturePair<A, B>(a, b) };

let direct = Prim.fixturePair<Nat, Text>(17, "retained");
assert direct == (17, "retained");
assert identity<Pair>(direct) == direct;
assert Prim.fixtureUnpack<Pair>(direct) == direct;
assert unpack<Pair>(direct) == direct;
Prim.fixtureUnpack<()>(());
unpack<()>(());
assert identity<Pair>(Prim.fixturePair<Nat, Text>(18, "opaque tuple")) ==
  (18, "opaque tuple");
assert pair<Nat8, ?Nat>(255, null) == (255, null);
assert Prim.fixtureTriple<Nat, Pair, Null>(19, direct, null) ==
  (19, (17, "retained"), null);

// A context retaining a tuple must materialize it as one managed value. Each
// element survives ordinary array storage and generic opaque identity calls.
let retained = Prim.Array_tabulate<Pair>(32, func (index : Nat) : Pair {
  identity<Pair>(Prim.fixturePair<Nat, Text>(index, "stored"))
});
var index = 0;
while (index < retained.size()) {
  assert retained[index] == (index, "stored");
  index += 1;
};

var events = "";
let (first, second) = Prim.fixturePair<Nat, Nat>(
  do {events #= "first;"; 3},
  do {events #= "second;"; 4}
);
assert (first, second) == (3, 4);
ignore Prim.fixtureTriple<Nat, Nat, Nat>(
  do {events #= "a;"; 5},
  do {events #= "b;"; 6},
  do {events #= "c;"; 7}
);
Prim.fixtureIdentity<()>(Prim.fixtureUnit<Nat>(func (value : Nat) {
  assert value == 8;
  events #= "callback;";
}, do {events #= "argument;"; 8}));
assert events == "first;second;a;b;c;argument;callback;";

// Destructuring a known tuple and discarding unit must not allocate a temporary
// managed result. Prepare the callback before measuring its effects.
var visits = 0;
let visit = func (value : Nat) {visits += value};
var iteration = 0;
var total = 0;
let before = Prim.rts_total_allocation();
while (iteration < 1000) {
  let (a, b) = Prim.fixturePair<Nat, Nat>(iteration, iteration + 1);
  let (c, d, e) = Prim.fixtureTriple<Nat, Nat, Nat>(a, b, 1);
  total += c + d + e;
  Prim.fixtureUnit<Nat>(visit, 1);
  iteration += 1;
};
assert total == 1001000;
assert visits == 1000;
assert Prim.rts_total_allocation() == before;
