//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
import Prim "mo:⛔";

actor {
  public func notify(_n : Nat) : async () {};
  public func test() : async () {
    ignore Prim.fixtureVisit<Nat>(notify, 0)
  };
};
