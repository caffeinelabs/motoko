//MOC-FLAG -A=M0194
let a = [1, 2, 42];
let aa = a : [Nat];

assert(a.size() == 3);

assert(a[0] == 1);
assert(a[1] == 2);
assert(a[2] == 42);

assert(a.get(0) == 1);
assert(a.get(1) == 2);
assert(a.get(2) == 42);

let b = [var 2, 3, 23];
let bb = b : [var Nat];

assert(b.size() == 3);

assert(b[0] == 2);
assert(b[1] == 3);
assert(b[2] == 23);

assert(b.get(0) == 2);
assert(b.get(1) == 3);
assert(b.get(2) == 23);

b[1] := 6;
assert(b[1] == 6);

b.put(2, 7);
assert(b[2] == 7);

func opt_eq(x : ?Nat, y : Nat) : Bool {
  switch x { case null { false };
             case (?i) { i == y } }
};

var emptyit = ([] : [Nat]).keys();
switch (emptyit.next()) { case null {}; case _ {assert false} };

var it = a.keys();
assert (opt_eq(it.next(), 0));
assert (opt_eq(it.next(), 1));
assert (opt_eq(it.next(), 2));
switch (it.next()) { case null {}; case _ {assert false} };

var it_again = a.keys();
assert (opt_eq(it_again.next(), 0));
assert (opt_eq(it_again.next(), 1));
assert (opt_eq(it_again.next(), 2));
switch (it_again.next()) { case null {}; case _ {assert false} };

var it2 = a.vals();
assert (opt_eq(it2.next(), 1));
assert (opt_eq(it2.next(), 2));
assert (opt_eq(it2.next(), 42));
switch (it2.next()) { case null {}; case _ {assert false} };

var it3 = a.values();
assert (opt_eq(it3.next(), 1));
assert (opt_eq(it3.next(), 2));
assert (opt_eq(it3.next(), 42));
switch (it3.next()) { case null {}; case _ {assert false} };

var i = 0;

i := 0;
for (j in a.keys()) {
  assert(j == i);
  i += 1;
};
assert(i == a.size());

i := 0;
for (n in a.vals()) {
  assert(n == a[i]);
  i += 1;
};
assert(i == a.size());

i := 0;
for (n in a.values()) {
  assert(n == a[i]);
  i += 1;
};
assert(i == a.size());

i := 0;
for (j in b.keys()) {
  assert(j == i);
  i += 1;
};
assert(i == b.size());

i := 0;
for (n in b.vals()) {
  assert(n == b[i]);
  i += 1;
};
assert(i == b.size());

i := 0;
for (n in b.values()) {
  assert(n == b[i]);
  i += 1;
};
assert(i == b.size());

let heterogeneous : Any = [1, 2 : Int8, 3 : Nat16, false, true];
let homogeneous = [true, false, true];

assert not homogeneous[1];
assert homogeneous[2]
