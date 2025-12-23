let x1 = switch 2 {
  case (0) 0;
  case (2) 1;
  case (_) 2;
};
assert (x1 == 1);

let x2 : Int = switch (-3) {
  case 0 0;
  case -1 2;
  case -3 1;
  case x x;
};
assert (x2 == 1);

let x3 = switch 4 {
  case 0 (+0);
  case 2 (-1);
  case x (x-3);
};
assert (x3 == 1);

let x4 = switch (null : ?{}) {
  case null 1;
  case _ 0;
};
assert (x4 == 1);

let x5 = switch (?{}) {
  case null 0;
  case x 1;
};
assert (x5 == 1);

let oo : ?{} = ?{};
let x6 = switch oo {
  case null 0;
  case _ 1;
};
assert (x6 == 1);

let no : ?Nat = ?0;
let x7 = switch no {
  case null 0;
  case (?0) 1;
  case (?n) n;
};
assert (x7 == 1);

let x8 = switch 3 {
  case (0 or 1) 0;
  case (3 or 4) 1;
  case _ 2;
};
assert (x8 == 1);

let x9 = switch 4 {
  case (0 or 1) 0;
  case (3 or 4 or 5) 1;
  case _ 2;
};
assert (x9 == 1);

let x10 = switch true {
  case true 3;
  case false 4;
};
assert (x10 == 3);

let x11 = switch null {
  case null 5;
};
assert (x11 == 5);

let x12 = switch (null : ?Int) {
  case null 6;
};
assert (x12 == 6);

func f() {
  switch 0 {};
  switch 0 { case _ {} };
  switch 0 { case 0 {}; case _ {} };
  switch 0 { case 0 {}; case _ {}; };
  switch (return) {};
  switch (0, return) { case _ {} };
  switch (0, return) {};
};

func g(_ : None) {};

let x13 = switch (?5) {
  case (?n) 7;
};
assert (x13 == 7);

let x14 = switch (null : ?Nat) {
  case null 8;
};
assert (x14 == 8);
