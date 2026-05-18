// Field-level existentials enforce their bounds at construction.
// `f : type X <: B in T` rejects witnesses outside [B].  Bounds can
// be either concrete (e.g. [Int]) or parameterised over an outer
// type-parameter (e.g. [<: A] inside `type Bag<A> = ...`); both
// flavours surface M9002 when violated.

// --- Concrete bound: Bool ≮: Nat -------------------------------

type Box = {
  data : type X <: Nat in (X, X -> Nat);
  meta : Text
};

let _b : Box = {
  data = (true, func (n : Bool) : Nat = if n 1 else 0);
  meta = "hi"
};

// --- Parameterised bound: Bool ≮: Int when A = Int ----------------

type Bag<A> = {
  item : type X <: A in (X, X -> A);
  tag : Text
};

let _bag : Bag<Int> = {
  item = (true, func (n : Bool) : Int = if n 1 else 0);
  tag = "boom"
};

// --- Record subtyping violated: witness missing a required field --
// Bound is [{x : Nat; y : Text}].  Witness [{x : Nat}] has *fewer*
// fields than the bound — it's a *supertype*, not a subtype, so
// rejected.

type RecBound = {
  shape : type X <: { x : Nat; y : Text } in X
};

let _rec : RecBound = {
  shape = { x = 5 } : { x : Nat }
};

// --- Variant subtyping violated: witness has tags outside the bound
// Bound is [{#a; #b}].  Witness [#c : {#a; #b; #c}] has *more* tags
// than the bound — it's a *supertype*, not a subtype, so rejected.

type VarBound = {
  choice : type Y <: { #a; #b } in Y
};

let _var : VarBound = {
  choice = (#c : { #a; #b; #c })
};
