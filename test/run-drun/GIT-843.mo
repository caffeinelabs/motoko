import Prim "mo:⛔";
// Both check_ir and ir_interpreter would failed
// if an actor field name (legally) shadowed the actor name
actor Bad {
  public func Bad () : (){ Prim.debugPrint "ok"};
};
Bad.Bad(); //OR-CALL ingress Bad "DIDL\x00\x00"

