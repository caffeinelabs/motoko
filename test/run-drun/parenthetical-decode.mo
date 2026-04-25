//MOC-FLAG --package core $MOTOKO_CORE

// Test the `decode` field end-to-end. The decoder is a flow pipeline
// `Blob -> ?Text -> ?Nat` built by mapping `Nat.fromText` over the
// result of `decodeUtf8`. The method's ingress type is `?Nat`; the
// method echoes that value (or 0 on null) back as a plain `Nat` over
// standard Candid.
//
// The //CALL ingress payload below is the raw three ASCII bytes "123"
// (0x313233) — *not* a Candid envelope. With the decoder active that
// blob decodes as `?123`, and the reply is Candid Nat 123. Without
// the decoder Candid would reject 0x313233 as malformed input.
import { decodeUtf8 } "mo:⛔";
import Nat "mo:core/Nat";

persistent actor {
  (with decode = func (b : Blob) : ?Nat = do ? {
    Nat.fromText((decodeUtf8 b)!)!
  })
  public func get(n : ?Nat) : async Nat = async (switch n { case (?v) v; case null 0 });
}

//CALL ingress get 0x313233

//SKIP run
//SKIP run-ir
//SKIP run-low
