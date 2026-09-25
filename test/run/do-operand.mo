// `do { }` is the block-as-expression, so it works as an operator operand

assert 1 + do { 2 } == 3;
assert do { 1 } + 2 == 3;
assert -do { 1 } == -1;
assert not do { false };
assert ?do { 1 } == ?1;
assert debug_show do { 5 } == "5";
assert do { 1 } : Nat == 1;
assert true and do { true };
assert do ? { (?1)! + 1 } == ?2;
assert (do ? { (null : ?Nat)! } ?? 7) == 7;

// a postfix form needs parentheses around the block
assert (do { { a = 4 } }).a == 4;
assert (do { func(n : Nat) : Nat { n + 1 } })(1) == 2;

// a `do` head is not atomic, so its branches stay braced
let v = if do { true } { 1 } else { 2 };
assert v == 1;
