// Bounded existential: `type X <: B in body` constrains the packed
// witness to be a subtype of B.  Destructuring binds the witness to
// a fresh skolem whose bound is B too, so the body can use the
// B-shaped API on the witness without revealing the concrete witness
// type to the outer scope.

// --- Tuple body --------------------------------------------------

type Boxed = type X <: Nat in (X, X -> Nat);

let b1 : Boxed = (42, func (n : Nat) : Nat = n + 1);
let (witness, next) = b1;
assert next witness == 43;

let b2 : Boxed = (7, func (n : Nat) : Nat = n * n);
let (w2, sq) = b2;
assert sq w2 == 49;

// `<: Int` admits Nat (Nat <: Int).
type Signed = type X <: Int in (X, X -> Int);

let s1 : Signed = (5 : Nat, func (n : Nat) : Int = -n);
let (sw, neg) = s1;
assert neg sw == -5;

// --- Record body (>= 2 fields) ----------------------------------

type Pair = type X <: Int in { fst : X; snd : X -> Int };

let p1 : Pair = { fst = 9 : Nat; snd = func (n : Nat) : Int = -n };
let { fst; snd } = p1;
assert snd fst == -9;

let p2 : Pair = { fst = 11 : Nat; snd = func (n : Nat) : Int = n * 2 };
let { fst = a; snd = f } = p2;
assert f a == 22;

//SKIP run
//SKIP run-ir
//SKIP run-low
