import Prim "mo:⛔";

persistent actor {
  public func go() : async () {
    ignore Prim.rts_version();
  };
}
