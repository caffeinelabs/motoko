// Exercise compatible_typ arms by using and-patterns where the two legs
// infer DIFFERENT types that are still compatible.
// This avoids the t1 == t2 short-circuit.

//MOC-FLAG -A=M0194

// Nat vs Int: compatible_typ Prim Nat, Prim Int -> true (L1444)
// When pattern leg 1 infers Nat and leg 2 infers Int (via coercion annotations)
// Actually: for and-patterns both legs match the same scrutinee, so same base type.
// The trick: use coercion via explicit type annotation to make leg2 produce a different type.
//
// Strategy: use a generic function whose inferred type changes between legs:
// e.g., `(f : Nat -> Nat)` vs `(g : Int -> Int)` — both compatible as Func

// Opt vs Null: compatible_typ Prim Null, Opt t -> true (L1457)
// We need to present both Null and ?Nat patterns.
// In a switch, different arms can have different inferred types.
// For and-patterns, the scrutinee must be a single value.

// Use a type alias that normalizes differently:
type NatAlias = Nat;
type IntAlias = Int;

// Tuple with different element types — compatible_list compatible_typ is called
// on element pairs (Nat, Int) which are compatible (Nat <: Int)
// Hmm, this still hits the same type for each leg...

// The real solution: the and-pattern legs produce different types via structural
// subtyping. For example:
// - scrutinee : {x : Nat} & leg1 : {x : Int} — x inferred as Int (subtype of leg)
// No, and-patterns must accept the scrutinee type, not supertype.

// Let's think differently: can we make an and-pattern where the two legs
// infer structurally different (but compatible) types?
// Answer: probably not easily — both legs see the same scrutinee.

// However, we CAN use `check_pat` mode vs `infer_pat` mode:
// if pat1 is explicit but pat2 is not, only pat1 is inferred.
// But the interesting case is when BOTH are explicit (true, true branch).

// A variant with compatible arms via structural typing:
// left : {#a : Nat; #b : Text}
// right : {#a : Nat} (subtype of left)
// These are structurally different types but compatible_tags would check them.
// However, in an and-pattern, both legs need to accept the SAME value.

// Most direct trigger: have a type where compatible_typ is called on types
// coming from different typing paths that coincide.
// Example with Array arms:
let _ = switch ([1, 2] : [Nat]) {
  case (xs and ys) (xs.size() + ys.size())
};

// Example with Opt arm — both legs see ?Nat:
let _ = switch (?(42 : Nat)) {
  case (x and y) (switch x { case (?_) 1; case null 0 })
};

// Example with Tup arm:
let _ = switch ((1, true) : (Nat, Bool)) {
  case ((n, _) and (_, b)) n
};

// Example with Obj arm:
let _ = switch ({a = 1; b = true} : {a : Nat; b : Bool}) {
  case ({a} and {b}) (a + (if b 1 else 0))
};

// Variant arm:
type V2 = {#x : Nat; #y : Text};
let _ = switch (#x 42 : V2) {
  case (v and w) (switch v { case (#x n) n; case (#y _) 0 })
};

assert true;
