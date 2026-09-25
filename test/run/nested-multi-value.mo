func returns_tuple() : (Nat, Nat) = (1, 2);

func multi_value_blocks(n : Nat) : (Nat, Nat) {
  return (
    if n == 0 {
      returns_tuple()
    } else {
      if n == 1 {
        returns_tuple()
      } else {
        returns_tuple()
      }
    }
  );
};

assert (multi_value_blocks(3) == (1, 2));
