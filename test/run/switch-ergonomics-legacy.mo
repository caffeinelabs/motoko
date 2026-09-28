// The legacy forms that #6348 keeps parsing next to the north-star syntax of #6352 (see switch-ergonomics.mo):
// parenthesized atomic heads with bare branches, bare case arms, `for (p in e)`, `= e` function bodies.
// Still supported in moc v2; retired at the v3 flip.

func inc(n : Nat) : Nat = n + 1;
let a = true;
var i = 3;

// bare branches after a parenthesized head: a spaced `(`, `[`, `#tag`, `null`, or any other atom starts the branch
if inc(2) == 3 { (assert true) } else { (assert false) };
let cmp = if i > 0 { #pos } else { #zero };
assert (cmp == #pos);
let opt = if 1 > 0 { null } else { (?5) };
assert (opt == null);
let arr = if a { [1] } else { [2] };
assert (arr[0] == 1);

// a branch may start with a prefix-shaped operator: spaced before, glued to its operand
let sgn = if i < 0 { -1 } else { 1 };
assert (sgn == 1);
let neg = if i > 0 { -i } else { i };
assert (neg == -3);
let flipped : Nat8 = if a { ^0 } else { 0 };
assert (flipped == 255);
let plus = if a { +1 } else { 0 };
assert (plus == 1);
let vtag = if a { #yes } else { #no };
assert (vtag == #yes);
let signed = if a { -1 } else { 1 };
assert (signed == -1);
// ... and in operand position the same spellings stay the binary operators they always were
let t = "a" #"b";
assert (t == "ab");
let n : Int = 10 -1 -2 * 2;
assert (n == 5);
let m : Int = 5 -1 * 2 +1;
assert (m == 4);
let masked : Nat8 = 0xff ^0x0f;
assert (masked == 0xf0);

// a bare-branch `if` may continue an `else` chain that started from a parenthesized head
let chain = if i == 0 { 0 } else if a { 1 } else { 2 };
assert (chain == 1);

// bare loop bodies
while i > 0 { i -= 1 };
assert (i == 0);
var sum = 0;
for x in [1, 2, 3].values() { sum += x };
assert (sum == 6);

// parenthesized scrutinee with bare arms, `;`-separated
func sign(n : Int) : Text = switch n {
  case -1 { "neg" }
  case 0 { "zero" }
  case _ { "other" }
};
assert (sign(-1) == "neg");
assert (sign(0) == "zero");

// an unparenthesized head still admits legacy arms: only `if`/`while`/`for` couple head shape to braces
func tag(o : ?Nat) : Nat = switch o {
  case null { 0 }
  case ?n { n + 1 }
};
assert (tag(null) == 0);
assert (tag(?41) == 42);
