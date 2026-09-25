func test() : Int {
  var i : Int = 10;
  label count while i > -10 {
    i -= 1;
    if i == 0 { break count };
    continue count;
    return -1;
  };
  return i;
};

let passed = test() == 0;
assert passed;
