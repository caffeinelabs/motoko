func foo(n:Nat) {
  let f = do { var x : Nat = n; func () : Nat { x+=1; return x } };
  assert(f() == n + 1);
  assert(f() == n + 2);
};

foo(5);
