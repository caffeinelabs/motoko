/// Functions on Arrays

import Buffer "Buffer";
import I "IterType";
import Option "Option";
import Prim "mo:⛔";
import Result "Result";

module {
  /// Test if two arrays contain equal values
  public func equal<A>(a : [A], b : [A], eq : (A, A) -> Bool) : Bool {
    if a.size() != b.size() {
      return false;
    };
    var i = 0;
    while i < a.size() {
      if not eq(a[i], b[i]) {
        return false;
      };
      i += 1;
    };
    return true;
  };
  /// Append the values of two input arrays
  public func append<A>(xs : [A], ys : [A]) : [A] {
    switch(xs.size(), ys.size()) {
      case (0, 0) { []; }
      case (0, _) { ys; }
      case (_, 0) { xs; }
      case (xsSize, ysSize) {
        Prim.Array_tabulate<A>(xsSize + ysSize, func (i : Nat) : A {
          if i < xsSize {
            xs[i];
          } else {
            ys[i - xsSize];
          };
        });
      }
    };
  };
  /// Transform each array value into zero or more output values, appended in order
  public func chain<A, B>(xs : [A], f : A -> [B]) : [B] {
    var ys : [B] = [];
    for i in xs.keys() {
      ys := append<B>(ys, f(xs[i]));
    };
    ys;
  };
  /// Output array contains each array-value if and only if the predicate is true; ordering retained.
  public func filter<A>(xs : [A], f : A -> Bool) : [A] {
    let ys : Buffer.Buffer<A> = Buffer.Buffer(xs.size());
    for x in xs.values() {
      if f(x) {
        ys.add(x);
      };
    };
    ys.toArray();
  };
  /// Output array contains each transformed optional value; ordering retained.
  public func mapFilter<A, B>(xs : [A], f : A -> ?B) : [B] {
    let ys : Buffer.Buffer<B> = Buffer.Buffer(xs.size());
    for x in xs.values() {
      switch f(x) {
        case null {}
        case ?y { ys.add(y) }
      }
    };
    ys.toArray();
  };
  /// Aggregate and transform values into a single output value, by increasing indices.
  public func foldLeft<A, B>(xs : [A], initial : B, f : (B, A) -> B) : B {
    var acc = initial;
    let size = xs.size();
    var i = 0;
    while i < size {
      acc := f(acc, xs[i]);
      i += 1;
    };
    acc;
  };
  /// Aggregate and transform values into a single output value, by decreasing indices.
  public func foldRight<A, B>(xs : [A], initial : B, f : (A, B) -> B) : B {
    var acc = initial;
    let size = xs.size();
    var i = size;
    while i > 0 {
      i -= 1;
      acc := f(xs[i], acc);
    };
    acc;
  };
  /// Returns optional first value for which predicate is true
  public func find<A>(xs : [A], f : A -> Bool) : ?A {
    for x in xs.values() {
      if f(x) {
        return ?x;
      }
    };
    return null;
  };
  /// Transform mutable array into immutable array
  public func freeze<A>(xs : [var A]) : [A] {
    Prim.Array_tabulate<A>(xs.size(), func (i : Nat) : A {
      xs[i];
    });
  };
  /// Transform an array of arrays into a single array, with retained array-value order.
  public func flatten<A>(xs : [[A]]) : [A] {
    chain<[A], A>(xs, func (x : [A]) : [A] {
      x;
    });
  };
  /// Transform each value using a function, with retained array-value order.
  public func map<A, B>(xs : [A], f : A -> B) : [B] {
    Prim.Array_tabulate<B>(xs.size(), func (i : Nat) : B {
      f(xs[i]);
    });
  };
  /// Transform each entry (index-value pair) using a function.
  public func mapEntries<A, B>(xs : [A], f : (A, Nat) -> B) : [B] {
    Prim.Array_tabulate<B>(xs.size(), func (i : Nat) : B {
      f(xs[i], i);
    });
  };

  /// Maps a Result-returning function over an Array and returns either
  /// the first error or an array of successful values.
  ///
  /// ```motoko
  /// func makeNatural(x : Int) : Result.Result<Nat, Text> =
  ///   if (x >= 0) {
  ///     #ok(Int.abs(x))
  ///   } else {
  ///     #err(Int.toText(x) # " is not a natural number.")
  ///   };
  ///
  /// mapResult([0, 1, 2], makeNatural) = #ok([0, 1, 2]);
  /// mapResult([-1, 0, 1], makeNatural) = #err("-1 is not a natural number.");
  /// ```
  public func mapResult<A, R, E>(xs : [A], f : A -> Result.Result<R, E>) : Result.Result<[R], E> {
    let len : Nat = xs.size();
    var target : [var R] = [var];
    var i : Nat = 0;
    var isInit = false;
    while i < len {
      switch f(xs[i]) {
        case #err(err) { return #err(err) }
        case #ok(ok) {
          if not isInit {
            isInit := true;
            target := init(len, ok);
          } else {
            target[i] := ok
          }
        }
      };
      i += 1;
    };
    #ok(freeze(target))
  };

  /// Make an array from a single value.
  public func make<A>(x: A) : [A] {
    [x];
  };
  /// Returns `xs.values()`.
  public func values<A>(xs : [A]) : I.Iter<A> {
    xs.values()
  };
  /// Returns `xs.keys()`.
  public func keys<A>(xs : [A]) : I.Iter<Nat> {
    xs.keys()
  };
  /// Transform an immutable array into a mutable array.
  public func thaw<A>(xs : [A]) : [var A] {
    let xsSize = xs.size();
    if xsSize == 0 {
      return [var];
    };
    let ys = Prim.Array_init<A>(xsSize, xs[0]);
    for i in ys.keys() {
      ys[i] := xs[i];
    };
    ys;
  };
  /// Initialize a mutable array with `size` copies of the initial value.
  public func init<A>(size : Nat, initVal : A) : [var A] {
    Prim.Array_init<A>(size, initVal);
  };
  /// Initialize a mutable array of the given size, and use the `gen` function to produce the initial value for every index.
  public func tabulate<A>(size : Nat, gen : Nat -> A) : [A] {
    Prim.Array_tabulate<A>(size, gen);
  };

  // copy from iter.mo, but iter depends on array
  class range(x : Nat, y : Int) {
    var i = x;
    public func next() : ?Nat {
      if i > y {
        null
      } else {
        let j = i;
        i += 1;
        ?j
      }
    };
  };
  /// Initialize a mutable array using a generation function
  public func tabulateVar<A>(size : Nat, gen : Nat -> A) : [var A] {
    if size == 0 { return [var] };
    let xs = Prim.Array_init<A>(size, gen(0));
    for i in range(1, size - 1) {
      xs[i] := gen(i);
    };
    return xs;
  };
}
