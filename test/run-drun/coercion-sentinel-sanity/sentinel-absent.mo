//MOC-FLAG --enhanced-orthogonal-persistence --sanity-checks
import Prim "mo:⛔";

// A Candid argument or blob whose wire type does not match the expected type
// fails to coerce; the freshly decoded aggregate must not be left holding the
// dummy coercion marker, which denotes no heap object and which the
// incremental GC would otherwise trace as if it were an ordinary field. Under
// `--sanity-checks` the allocation barrier validates every fresh aggregate, so
// a regression is caught at decode time.
//
// One case per aggregate arm of the deserializer that stores decoded elements:
// immutable array, record/object, variant, and (nested) tuple -- each both
// for a field present but uncoercible and, for record/tuple, a field absent
// from the wire. The mutable
// array arm is only reachable via stable/upgrade deserialization -- `[var T]`
// is not a shared type -- so it is not exercised here.
persistent actor {
  // immutable array, via a message argument: `opt vec text` into `?[Nat]`
  public func submit(xs : ?[Nat]) : async Nat {
    switch xs { case null { 0 }; case (?a) { a.size() } }
  };

  // immutable array, via `from_candid` on a caller-supplied blob
  public func fromCandid(b : Blob) : async Nat {
    switch (from_candid(b) : ?[Nat]) { case null { 0 }; case (?a) { a.size() } }
  };

  // record/object: `record { x = text }` into `?{ x : Nat }`
  public func recVal() : async Nat {
    let b = to_candid({ x = "probe" });
    switch (from_candid(b) : ?{ x : Nat }) { case null { 0 }; case (?r) { r.x } }
  };

  // variant: `variant { tag = text }` into `?{ #tag : Nat }`
  public func varVal() : async Nat {
    let b = to_candid(#tag "probe" : { #tag : Text });
    switch (from_candid(b) : ?{ #tag : Nat }) { case null { 0 }; case (?(#tag n)) { n } }
  };

  // tuple (nested, so it is a heap object): `record { 0 = text; 1 = text }`
  public func tupVal() : async Nat {
    let b = to_candid({ pair = ("a", "b") });
    switch (from_candid(b) : ?{ pair : (Nat, Nat) }) { case null { 0 }; case (?r) { r.pair.0 } }
  };

  // record with a *required field absent* from the wire: a recoverable
  // coercion failure that produces no decoded value for the slot
  public func recMissing() : async Nat {
    let b = to_candid({});
    switch (from_candid(b) : ?{ x : Nat }) { case null { 0 }; case (?r) { r.x } }
  };

  // same, for a tuple field
  public func tupMissing() : async Nat {
    let b = to_candid({ pair = ("a", "b") });
    switch (from_candid(b) : ?{ pair : (Nat, Nat, Nat) }) { case null { 0 }; case (?r) { r.pair.0 } }
  };

  public func peek() : async Nat { 0 };
}
