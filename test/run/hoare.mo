func swap(a : [var Int], i : Nat, j : Nat) {
  let temp = a[i];
  a[i] := a[j];
  a[j] := temp;
};

func partition(a : [var Int], lo : Nat, hi : Nat) : Nat {
  let pivot = a[lo];
  var i : Nat = lo;
  var j : Nat = hi;

  loop {
    while a[i] < pivot { i += 1 };
    while a[j] > pivot { j -= 1 };
    if i >= j { return j };
    swap(a, i, j);
    i += 1;
    j -= 1;
  };
};

func quicksort(a : [var Int], lo : Nat, hi : Nat) {
  if lo < hi {
    let p = partition(a, lo, hi);
    quicksort(a, lo, p);
    quicksort(a, p + 1, hi);
  };
};

let a : [var Int] = [var 8, 8, 3, 9, 5, 2];

quicksort(a, 0, a.size() - 1);

assert(a[0] == 2);
assert(a[1] == 3);
assert(a[2] == 5);
assert(a[3] == 8);
assert(a[4] == 8);
assert(a[5] == 9);
