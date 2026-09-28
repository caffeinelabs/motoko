let a = [var 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0];

var i = 1;
while i < 10 {
  i += 1;
  a[i] := a[i - 1] + a[i - 2];
};

assert (i == 10);
assert (a[0] == 0);
assert (a[1] == 1);
assert (a[2] == 1);
assert (a[3] == 2);
assert (a[4] == 3);
assert (a[5] == 5);
assert (a[6] == 8);
assert (a[7] == 13);
assert (a[8] == 21);
assert (a[9] == 34);
assert (a[10] == 55);
