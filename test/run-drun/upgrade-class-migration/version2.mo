import Prim "mo:prim";
import Migration "Migration2";

// Rename stable field `three` to `four`
(with migration = Migration.run)
actor class C() {

  Prim.debugPrint("Version 2");

  var zero : Nat = Prim.trap "unreachable"; // inherited
  assert zero == 0;

  var four : [var (Nat, Text)] = [var];

  public func check(): async() {
    Prim.debugPrint(debug_show{ zero; four });
  }
};
