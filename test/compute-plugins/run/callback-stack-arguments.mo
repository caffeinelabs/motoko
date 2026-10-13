//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
import Prim "mo:⛔";

type Cell = { var value : Nat };
type Payload = ([Nat], Cell);
type Result = (Nat, Text);

// These suppliers execute inside the generated Call.local argument code.
// The outer closure and first argument stay on the Wasm operand stack while
// later suppliers reenter the plugin and allocate managed values.
var events = "";
let cell : Cell = { var value = 7 };
let numbers = [17, 19];
let result = Prim.fixtureSupplied2<Payload, [Nat], Result>(
  func () : ((Payload, [Nat]) -> Result) {
    events #= "factory;";
    let captured = cell;
    func (first : Payload, second : [Nat]) : Result {
      events #= "outer;";
      assert first.0 == numbers;
      assert first.1.value == 44;
      assert captured.value == 44;
      assert second.size() == 65536;
      assert second[65535] == 65536;
      first.1.value += 1;
      (first.0[0] + second[0], events)
    }
  },
  func () : Payload {
    events #= "first;";
    Prim.fixtureApply<Payload, Payload>(func (value : Payload) : Payload {
      events #= "nested;";
      value
    }, (numbers, cell))
  },
  func () : [Nat] {
    events #= "second;";
    cell.value := 44;
    Prim.fixtureTabulate<Nat>(65536, func (n : Nat) : Nat { n + 1 })
  }
);
assert result == (18, "factory;first;nested;second;outer;");
assert cell.value == 45;
