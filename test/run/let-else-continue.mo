func foo() {
  var b = true;
  label l while b {
    let 2 = 3 else {
      b := false;
      continue l;
    };
    assert false;
  };
};

foo();
