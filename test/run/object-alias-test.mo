func go() {
  var x : Nat = 1;
  let o = { var f = x };
  (func () { x := 2 })();
  (func () { o.f := 3 })();
  assert (x == 2);
  assert (o.f == 3);
};

go();
