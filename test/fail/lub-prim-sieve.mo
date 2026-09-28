do {
  type T2  = ??T2;
  type T3  = ???T3;
  type T5  = ?????T5;
  type T7  = ???????T7;
  type T11 = ???????????T11;

  // Using `^` to make the compiler infer the type and spit it out
  ignore (^[null : T2, null : T3, null : T5, null : T7, null : T11]);
};

do {
  type T2  = ?(Nat, ?(Int, T2));
  type T3  = ?(Nat, ?(Nat, ?(Int, T3)));
  type T5  = ?(Nat, ?(Nat, ?(Nat, ?(Nat, ?(Int, T5)))));
  type T7  = ?(Nat, ?(Nat, ?(Nat, ?(Nat, ?(Nat, ?(Nat, ?(Int, T7)))))));
  type T11 = ?(Nat, ?(Nat, ?(Nat, ?(Nat, ?(Nat, ?(Nat, ?(Nat, ?(Nat, ?(Nat, ?(Nat, ?(Int, T11)))))))))));

  // Fun fact: inferred type has `Nat` in those positions that are prime numbers (until 169)
  ignore (^[null : T2, null : T3, null : T5, null : T7, null : T11]);
};
