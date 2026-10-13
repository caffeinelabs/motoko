//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
import Prim "mo:⛔";

var stage = 0;
ignore Prim.fixtureSupplied2<Nat, Nat, Nat>(
  func () : ((Nat, Nat) -> Nat) {
    assert stage == 0;
    stage := 1;
    func (_a : Nat, _b : Nat) : Nat {
      Prim.trap("outer callback must not run")
    }
  },
  func () : Nat {
    assert stage == 1;
    Prim.trap("callback argument trap propagates")
  },
  func () : Nat { Prim.trap("later argument must not run") }
);
