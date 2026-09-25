func foo() {
  label l loop {
    let 2 = 3 else { break l };
    assert false;
  }
};
foo()
