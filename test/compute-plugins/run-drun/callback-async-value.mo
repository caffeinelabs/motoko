//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
//MOC-FLAG --sanity-checks --force-gc
import Prim "mo:⛔";

let a = actor {
  func callback<T>() : T -> T {
    func (value : T) : T { value }
  };

  public func check() : async () {
    // The callback returns an existing future as an opaque ordinary value.
    // Only the surrounding Motoko function suspends when it awaits the result.
    let future = async { 41 };
    let result = Prim.fixtureApply<async Nat, async Nat>(
      callback<async Nat>(), future
    );
    assert (await result) == 41;

    var runs = 0;
    let computation = async* { runs += 1; 17 };
    let retained = Prim.fixtureApply<async* Nat, async* Nat>(
      callback<async* Nat>(), computation
    );
    assert runs == 0;
    assert (await* retained) == 17;
    assert runs == 1;
    Prim.debugPrint("plugin callbacks retained async values without awaiting");
  };
};

ignore a.check(); //OR-CALL ingress check "DIDL\x00\x00"
