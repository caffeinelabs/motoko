import Prim "mo:⛔";

// `actor { … } <: Principal`: an actor reference upcasts to its canister-id
// principal. One-directional and Actor-sort-only. The static checks below run
// across positions; the runtime section then materialises an actor value and
// confirms the upcast is coercion-free identity.

// argument / return / first-class positions
func _arg(a : actor {}) : Principal = a;
func _ret(a : actor { m : shared () -> async () }) : Principal = a;
let _upcast : (actor {}) -> Principal = _arg;

// a more specific actor still upcasts
func _specific(a : actor { foo : shared () -> async Nat }) : Principal = a;

// lub(actor, Principal) = Principal
func _lub(c : Bool, a : actor {}, p : Principal) : Principal = if c a else p;

// lub through the array element type
func _arr(a : actor {}, p : Principal) : [Principal] = [a, p];

// Runtime: the upcast is identity -- same reference, equal by `==`, same blob.
let a : actor {} = actor "rwlgt-iiaaa-aaaaa-aaaaa-cai";
let p : Principal = a;

assert (p == (a : Principal));                    // repeating it is stable
assert (Prim.blobOfPrincipal p == Prim.blobOfPrincipal (a : Principal));
assert (p == Prim.principalOfBlob (Prim.blobOfPrincipal p));  // blob round-trip

// the management canister's principal is the empty blob
let mgmt : Principal = (actor "aaaaa-aa" : actor {});
assert (Prim.blobOfPrincipal mgmt == "");
assert (not (p == mgmt));

Prim.debugPrint(debug_show p);
Prim.debugPrint(debug_show (Prim.blobOfPrincipal p));
