let foo = func () : Nat = 1;
assert (foo() == 1);

let fob =
  func fib (b:Bool) { if (b) { fob(false) } else () };
fob(true);
