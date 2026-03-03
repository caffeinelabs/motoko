func _a() {
  let x = ();
  x := ();
};

func _f () {
  let xs : [Nat] = [];
  xs[0] := 10;
};

func _g () {
  let (x, _y) = (0, 0);
  x := 10;
};
