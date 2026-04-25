//MOC-FLAG --package core $MOTOKO_CORE

// Test that a `decode` field is accepted in the visibility
// parenthetical. The decoder here is a flow pipeline of composed type
// `Blob -> ?Nat`, built by mapping `Nat.fromText` over the result of
// the `Blob -> ?Text` primitive `decodeUtf8`. The frontend just
// records it — no desugaring contract yet. The method itself returns
// a plain `Nat` over standard Candid.
import { decodeUtf8 } "mo:⛔";
import Nat "mo:core/Nat";

persistent actor {
  (with decode = func (b : Blob) : ?Nat = do ? {
    Nat.fromText((decodeUtf8 b)!)!
  })
  public func get(_ : ?Nat) : async Nat = async 42;
}

//CALL ingress get 0x4449444C016E7D0100012A

//SKIP run
//SKIP run-ir
//SKIP run-low
