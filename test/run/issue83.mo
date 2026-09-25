import Prim "mo:⛔";

func Array_thaw<A>(xs : [A]) : [var A] {
  let xsLen = xs.size();
  if xsLen == 0 {
    return [var];
  };
  let ys = Prim.Array_init<A>(xsLen, xs[0]);
  for i in ys.keys() {
    ys[i] := xs[i];
  };
  ys;
};

do {
  let xs : [Int] = [1, 2, 3];

  let actual = Array_thaw<Int>(xs);
  let expected : [var Int] = [var 1, 2, 3];

  assert(actual.size() == expected.size());

  for i in actual.keys() {
    assert(actual[i] == expected[i]);
  };
};
