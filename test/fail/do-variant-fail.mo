// misplaced ! (no enclosing do ? or do #lab)
func wrong1() {
  let _ = (#ok 1)!;
};

// ! on non-variant inside do #ok
func wrong2() {
  let _ = do #ok {
    (42 : Nat)!;
  };
};

// ! on variant without the success tag
func wrong3() {
  let _ = do #ok {
    (#err "bad" : {#err : Text})!;
  };
};

// / # on non-variant
func wrong4() {
  let _ = (42 : Nat) / #ok;
};
