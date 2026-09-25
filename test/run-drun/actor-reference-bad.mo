// actor references

import Prim "mo:⛔";

actor a {
  public func go() : async () {

    let tests = [
      "bfozs-kwa73-7nadi", // a good one
      "", // empty principal
      "BFOZS-KWA73-7NADI", // lowercase not allowed
      "bfozskwa737nadi", // missing dashes
      "vpgq", // too short
      "5h74t-uga73-7nadi", // wrong checksum
    ];

    for t in tests.values() {
      Prim.debugPrint(debug_show t # ":");
      try (await async ignore (actor(t) : actor {}))
      catch (e) { Prim.debugPrint(Prim.errorMessage(e)) }
    }
  }
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
