//MOC-FLAG -W M0276
// downgraded to a warning, the comparison is constant
assert ((1 : Nat) == ("x" : Text));
assert not ((1 : Nat) != ("x" : Text));
assert ({ id = 1 } == { userId = 2 });
