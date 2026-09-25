do {
  var x : Nat = 0;
  assert(x == 0);
  x += 3;
  assert(x == 3);
  x -= 1;
  assert(x == 2);
  x *= 4;
  assert(x == 8);
  x **= 3;
  assert(x == 512);
  x /= 4;
  assert(x == 128);
  x %= 3;
  assert(x == 2);
};

do {
  var x : Int = 0;
  assert(x == 0);
  x += 3;
  assert(x == 3);
  x -= 1;
  assert(x == 2);
  x *= 4;
  assert(x == 8);
  x **= 3;
  assert(x == 512);
  x /= 4;
  assert(x == 128);
  x %= 3;
  assert(x == 2);
};

do {
  var x : Nat8 = 0;
  assert(x == (0:Nat8));
  x += 3;
  assert(x == (3:Nat8));
  x -= 1;
  assert(x == (2:Nat8));
  x *= 2;
  assert(x == (4:Nat8));
  x **= 2;
  assert(x == (16:Nat8));
  x /= 2;
  assert(x == (8:Nat8));
  x %= 3;
  assert(x == (2:Nat8));
};
