import Prim "mo:prim";
import { migration } "Migration1";

// test merging fields `one` and `two` into field `three`, dropping `one` and `two` and preserving `zero`, using the migration idiom
(with migration)
actor {

  Prim.debugPrint("Version 1");

  var zero : Nat = Prim.trap "unreachable"; // inherited
  assert zero == 0;

  var three : [var (Nat, Text)] = [var];

  public func check(): async() {
    Prim.debugPrint (debug_show { zero; three });
  }
};
