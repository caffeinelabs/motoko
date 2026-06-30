// Coverage for desugar.ml `neutral` function (L375–402).
//
// `neutral` elides identity-element operands from `BinE` at lowering time:
//   BinE(_, e1, op, e2) when neutral(Left op) e1  → (exp e2).it
//   BinE(_, e1, op, e2) when neutral(Right op) e2 → (exp e1).it
//
// The fixed-width literal arms (L387–400) were dark before this file:
//   Int8Lit/Nat8Lit/…/Int64Lit/Nat64Lit for add_like (0) and mul_like (1).
//
// Note on apply_sign (L28–31): the NegOp/Int8Lit … NegOp/Int64Lit arms are
// NOT reachable from valid source — the parser converts `NegOp, PreLit(s,Nat)`
// directly to `LitP(PreLit("-"^s, Int))`, so Int8Lit never appears inside a
// SignP node.  Those arms are structurally unreachable (like the defensive
// `raise` lines), so we do not attempt to cover them here.

// --- Nat8 add_like: literal 0 on right (Right AddOp, L385) ---
do {
  let x : Nat8 = 42;
  // (0 : Nat8) is NatLit→Nat8Lit after typing; Right AddOp → neutral → elided
  assert (x + (0 : Nat8) == x);
  // Left AddOp: neutral left operand eliminated
  assert ((0 : Nat8) + x == x);
};

// --- Nat8 mul_like: literal 1 on right (Right MulOp, L386) ---
do {
  let x : Nat8 = 7;
  assert (x * (1 : Nat8) == x);   // Right MulOp neutral
  assert ((1 : Nat8) * x == x);   // Left MulOp neutral
};

// --- Int8 add_like: literal 0 (L387) ---
do {
  let x : Int8 = -13;
  assert (x + (0 : Int8) == x);
  assert ((0 : Int8) + x == x);
  // SubOp right-neutral: x - 0 == x  (Right SubOp, add_like)
  assert (x - (0 : Int8) == x);
};

// --- Int8 mul_like: literal 1 (L388) ---
do {
  let x : Int8 = 3;
  assert (x * (1 : Int8) == x);
  assert ((1 : Int8) * x == x);
};

// --- Nat16 add_like (L389) ---
do {
  let x : Nat16 = 1000;
  assert (x + (0 : Nat16) == x);
  assert ((0 : Nat16) + x == x);
};

// --- Nat16 mul_like (L390) ---
do {
  let x : Nat16 = 9;
  assert (x * (1 : Nat16) == x);
  assert ((1 : Nat16) * x == x);
};

// --- Int16 add_like (L391) ---
do {
  let x : Int16 = -500;
  assert (x + (0 : Int16) == x);
  assert ((0 : Int16) + x == x);
  assert (x - (0 : Int16) == x);
};

// --- Int16 mul_like (L392) ---
do {
  let x : Int16 = 11;
  assert (x * (1 : Int16) == x);
  assert ((1 : Int16) * x == x);
};

// --- Nat32 add_like (L393) ---
do {
  let x : Nat32 = 100_000;
  assert (x + (0 : Nat32) == x);
  assert ((0 : Nat32) + x == x);
};

// --- Nat32 mul_like (L394) ---
do {
  let x : Nat32 = 31;
  assert (x * (1 : Nat32) == x);
  assert ((1 : Nat32) * x == x);
};

// --- Int32 add_like (L395) ---
do {
  let x : Int32 = -99_999;
  assert (x + (0 : Int32) == x);
  assert ((0 : Int32) + x == x);
  assert (x - (0 : Int32) == x);
};

// --- Int32 mul_like (L396) ---
do {
  let x : Int32 = 17;
  assert (x * (1 : Int32) == x);
  assert ((1 : Int32) * x == x);
};

// --- Nat64 add_like: already partially covered (Nat64Lit, L397) ---
do {
  let x : Nat64 = 1_000_000;
  assert (x + (0 : Nat64) == x);
  assert ((0 : Nat64) + x == x);
};

// --- Nat64 mul_like: `true` branch was dark (L398) ---
do {
  let x : Nat64 = 2;
  assert (x * (1 : Nat64) == x);   // Right MulOp hits the Nat64 mul_like `true`
  assert ((1 : Nat64) * x == x);
};

// --- Int64 add_like (L399) ---
do {
  let x : Int64 = -1_234_567;
  assert (x + (0 : Int64) == x);
  assert ((0 : Int64) + x == x);
  assert (x - (0 : Int64) == x);
};

// --- Int64 mul_like (L400) ---
do {
  let x : Int64 = 13;
  assert (x * (1 : Int64) == x);
  assert ((1 : Int64) * x == x);
};
