func none() {
  let 2 = 3 else { return };
  assert false;
};
none();

func single() : Nat {
  let 2 = 3 else { return 2 };
  0;
};

assert (2 == single());

func multiple() : (Nat, Nat) {
  let 2 = 3 else { return (2, 3) };
  (0, 0);
};

assert ((2, 3) == multiple());
