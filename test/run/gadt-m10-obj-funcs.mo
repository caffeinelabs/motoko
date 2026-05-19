// M10 records / `object` form with function fields. The `object`
// body can declare `public func`s that operate on the existential,
// just like `public let` field initialisers — the witness packed at
// construction propagates through the function types.

type Counter = type X in {
  start : X;
  step  : X -> X;
  show  : X -> Text;
};

let c : Counter = object {
  public let start : Nat = 0;
  public func step(x : Nat) : Nat = x + 1;
  public func show(x : Nat) : Text = debug_show x;
};

let { start; step; show } = c;
assert show (step (step start)) == "2";
