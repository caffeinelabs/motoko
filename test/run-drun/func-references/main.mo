import { debugPrint } = "mo:⛔";

persistent actor self {
  // (1) actor-public: a `public shared` method of self. The wire
  // encoding carries self's own canister principal.
  public shared func nullary() : async () {};

  // (2) mgmt-public: a reference to the IC management canister's
  // `raw_rand` method. Principal `aaaaa-aa` (empty blob).
  transient let raw_rand =
    (actor "aaaaa-aa" : actor { raw_rand : () -> async Blob }).raw_rand;

  // (3) let-bound-shared: a `transient let` binding of a shared
  // function reference to a third external canister. We use the
  // anonymous principal (`2vxsx-fae`, blob `\04`) so the three func
  // refs span three distinct principals on the wire.
  transient let elsewhere : shared Text -> async Nat =
    (actor "2vxsx-fae" : actor { lookup : Text -> async Nat }).lookup;

  public shared func getFuncs() : async (
    shared () -> async (),
    shared () -> async Blob,
    shared Text -> async Nat,
  ) {
    debugPrint "returning three shared func references";
    (nullary, raw_rand, elsewhere)
  };
}
