/* a *generic* quicksort class
   - inplace sorting of mutable arrays of T, for any type T
*/

type Array<T> = [var T];

class QS<T>(cmp : (T, T) -> Int) {
  public func quicksort(a : Array<T>, lo : Nat, hi : Nat) {
    if lo < hi {
      let p = partition(a, lo, hi);
      quicksort(a, lo, p);
      quicksort(a, p + 1, hi);
    }
  };

  func swap(a : Array<T>, i : Nat, j : Nat) {
    let temp = a[i];
    a[i] := a[j];
    a[j] := temp;
  };

  func trace<T>(v : T) {};

  func partition(a : Array<T>, lo : Nat, hi : Nat) : Nat {
    trace<Array<T>>(a);
    let pivot = a[lo];
    var i = lo;
    var j = hi;
    loop {
      while cmp(a[i], pivot) < 0 {
        i += 1;
      };
      while cmp(a[j], pivot) > 0 {
        j -= 1;
      };
      if i >= j { return j };
      swap(a, i, j);
      i += 1;
      j -= 1;
    };
  };
};

func cmpi(i : Int, j : Int) : Int = i - j;
let qs = QS<Int>(cmpi);
let a : Array<Int> = [var 8, 8, 3, 9, 5, 2];
qs.quicksort(a, 0, a.size() - 1);
