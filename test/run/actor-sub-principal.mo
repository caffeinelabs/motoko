// `actor { … } <: Principal`: an actor reference upcasts to its canister-id
// principal. One-directional and Actor-sort-only. These are static-typing
// checks across positions; no actor value is materialised at runtime, so
// compiling this also confirms codegen accepts the (coercion-free) upcast.

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
