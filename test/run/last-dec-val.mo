assert ((do { let x = 5 }) == 5);
assert ((do { let (x, y) = (1, 2) }).1 == 2);
