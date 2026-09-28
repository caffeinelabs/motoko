let a : [[var Int]] = [[var 1]];
assert (a[0][0] == 1);
a[0][0] := 2;
assert (a[0][0] == 2);

let b : [{ var x : Int }] = [{ var x = 1 : Int }];
assert (b[0].x == 1);
b[0].x := 2;
assert (b[0].x == 2);
