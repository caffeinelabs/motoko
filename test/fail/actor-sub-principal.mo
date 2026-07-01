// `actor { … } <: Principal` is one-directional and restricted to the Actor
// sort. Everything here must be rejected.

// Downcast: an arbitrary Principal is not an actor (soundness — you cannot
// call methods on a bare principal).
func down(p : Principal) : actor {} = p;

// A plain object stays on its own (structural) typing rail — not an actor,
// hence not a Principal.
func obj(o : { field : Nat }) : Principal = o;

// So does a module.
module M { public let x = 0 };
func modu() : Principal = M;
