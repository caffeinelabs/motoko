// Field-level existential: `name : type X [<: B] in T` puts the
// existential at field scope.  Each field's existentials are
// independent — same surface name in two fields mints two distinct
// schema cons.  Construction-side σ derivation is per-field.
//
// Internally the existential cons stays on the field's [binds] slot;
// it gets "hoisted" at construction time when each field's σ is
// derived from the supplied value.

// --- Basic field existential (one field, tuple payload) ----------

type Box = {
  data : type X in (X, X -> Nat);
  meta : Text
};

let _b : Box = {
  data = (42, func (n : Nat) : Nat = n + 1);
  meta = "hi"
};

// --- Field existential with explicit bound -----------------------

type Boxed = {
  data : type X <: Nat in (X, X -> Nat);
  count : Nat
};

let _b2 : Boxed = {
  data = (7, func (n : Nat) : Nat = n * n);
  count = 1
};

// --- Field existential bounded by an outer type-parameter --------

type Bag<A> = {
  item : type X <: A in (X, X -> A);
  tag : Text
};

let _bag_int : Bag<Int> = {
  item = (5 : Nat, func (n : Nat) : Int = -n);
  tag = "negate"
};

let _bag_nat : Bag<Nat> = {
  item = (3 : Nat, func (n : Nat) : Nat = n + 1);
  tag = "succ"
};

// --- Two independent field existentials with same surface name ---

type Pair = {
  fst : type X in (X, X -> Nat);
  snd : type X in (X, X -> Text)
};

let _p : Pair = {
  fst = (10 : Nat, func (n : Nat) : Nat = n);
  snd = (true, func (b : Bool) : Text = if b "yes" else "no")
};

// --- Chained bounds: sibling existentials in the same pack -------
// G's bound is OUTER (alias-level existential), H's bound is G.
// At construction OUTER, G, H are inferred sequentially; each
// witness must satisfy the previous bound.

type Chain = type OUTER in {
  link : type G <: OUTER, type H <: G in (OUTER, G, H);
  tag : Text
};

let _c : Chain = {
  link = (5 : Int, 3 : Nat, 7 : Nat);  // OUTER=Int, G=Nat<:Int, H=Nat<:Nat
  tag = "demo"
};

//SKIP run
//SKIP run-ir
//SKIP run-low
