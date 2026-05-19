// Bounded existential rejects witnesses outside the bound.
// `type X <: Int in body` admits only [Int]-subtypes (e.g., Nat).
// Bool, Text, etc. are rejected at construction with M9002.

// --- Tuple body --------------------------------------------------

type Signed = type X <: Int in (X, X -> Int);

// Bool ≮: Int: bound violated on the first element.
let _t1 : Signed = (true, func (n : Bool) : Int = if n 1 else 0);

// Text ≮: Int: bound violated on the first element.
let _t2 : Signed = ("hi", func (n : Text) : Int = 0);

// --- Record body (>= 2 fields) ----------------------------------

type Pair = type X <: Int in { fst : X; snd : X -> Int };

let _r1 : Pair = { fst = true; snd = func (n : Bool) : Int = 0 };

let _r2 : Pair = { fst = "hi"; snd = func (n : Text) : Int = 0 };
