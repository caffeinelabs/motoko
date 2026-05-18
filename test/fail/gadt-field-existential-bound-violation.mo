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
