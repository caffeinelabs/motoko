do {
  type A = B and C;
  type B = {};
  type C = {};
};

do {
  type A = B or C;
  type B = {};
  type C = {};
};

do {
  type A = (Int, { a : Nat; b : [B] }) and (Nat, { b : [C]; c : () });
  type B = {};
  type C = {};
};

do {
  type A = (Int, { a : Nat; b : [B] }) and (Nat, { d : [C]; c : () });
  type B = {};
  type C = {};
};

do {
  type A = (B, B);
  type B = A and (A, A);
};

do {
  type A = B and {};
  type B = { b : A };
};

do {
  type A = { a : B };
  type B = A and {};
};
