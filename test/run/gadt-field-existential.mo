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

// --- Record subtyping: witness has more fields than the bound ----
// Bound is the record [{x : Nat}].  Witness [{x : Nat; y : Text}]
// has *more* fields — Motoko's width-subtyping makes it a subtype
// of the bound, so the construction is accepted.

type RecBound = {
  shape : type X <: { x : Nat } in X
};

let _rec : RecBound = {
  shape = { x = 5; y = "extra" } : { x : Nat; y : Text }
};

// --- Variant subtyping: witness has fewer tags than the bound ----
// Bound is the variant [{#a; #b; #c}].  Witness [#a : {#a; #b}]
// has *fewer* tags — by variant subtyping, the narrower variant is
// a subtype of the wider one, so the construction is accepted.

type VarBound = {
  choice : type Y <: { #a; #b; #c } in Y
};

let _var : VarBound = {
  choice = (#a : { #a; #b })
};

// --- Dissect-assemble axiom (unbounded field existential) --------
// Destructure a Box, immediately re-construct from the parts.
// Field-level X becomes a site-skolem in the destructure; the
// re-pack mints a fresh schema-X again — round-trip lands at Box.

func roundtrip(b : Box) : Box {
  let { data = (witness, next); meta } = b;
  { data = (witness, next); meta }
};

let orig : Box = {
  data = (10 : Nat, func (n : Nat) : Nat = n + 1);
  meta = "round"
};
let _tripped = roundtrip orig;

// --- Dissect-assemble with a bound -------------------------------
// Same shape, but the field existential carries a bound.  The
// site-skolem inherits the bound at destructure; re-pack supplies
// it back to the schema's bound check.

func roundtripBoxed(b : Boxed) : Boxed {
  let { data = (witness, next); count } = b;
  { data = (witness, next); count }
};

let origB : Boxed = {
  data = (3 : Nat, func (n : Nat) : Nat = n * 2);
  count = 1
};
let _trippedB = roundtripBoxed origB;

// --- Dissect-assemble with an outer-param bound ------------------
// Bag<A>'s field bound is A; the round-trip must preserve A across
// destructure/re-construct.  Specifically for Bag<Int>, the
// site-skolem's bound is Int (the alias-instantiated bound).

func roundtripBag(b : Bag<Int>) : Bag<Int> {
  let { item = (w, fn); tag } = b;
  { item = (w, fn); tag }
};

let origBag : Bag<Int> = {
  item = (4 : Nat, func (n : Nat) : Int = -n);
  tag = "bag-trip"
};
let _trippedBag = roundtripBag origBag;

//SKIP run
//SKIP run-ir
//SKIP run-low
