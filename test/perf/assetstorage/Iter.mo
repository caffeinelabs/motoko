/// Iterators

import Array "Array";
import Buffer "Buffer";
import List "List";

module {
  /// An iterator that produces values of type `T`. Calling `next` returns
  /// `null` when iteration is finished.
  ///
  /// Iterators are inherently stateful. Calling `next` "consumes" a value from
  /// the Iterator that cannot be put back, so keep that in mind when sharing
  /// iterators between consumers.
  /// 
  /// An iterater `i` can be iterated over using
  /// ```
  /// for (x in i) {
  ///   …do something with x…
  /// }
  /// ```
  public type Iter<T> = { next : () -> ?T };

  /// Creates an iterator that produces all `Nat`'s from `x` to `y` including
  /// both of the bounds.
  /// ```
  /// let iter = range(1, 3);
  /// assertEqual(?1, iter.next());
  /// assertEqual(?2, iter.next());
  /// assertEqual(?3, iter.next());
  /// assertEqual(null, iter.next());
  /// ```
  public class range(x : Nat, y : Int) {
    var i = x;
    public func next() : ?Nat { if (i > y) { null } else {let j = i; i += 1; ?j} };
  };

  /// Like [`range`](#value.range) but produces the values in the opposite
  /// order.
  public class revRange(x : Int, y : Int) {
      var i = x;
      public func next() : ?Int { if (i < y) { null } else {let j = i; i -= 1; ?j} };
  };

  /// Calls a function `f` on every value produced by an iterator and discards
  /// the results. If you're looking to keep these results use
  /// [`map`](#value.map).
  /// ```
  /// var sum = 0;
  /// iterate(range(1, 3), func(x : Nat) {
  ///   sum += x;
  /// });
  /// assertEquals(6, sum)
  /// ```
  public func iterate<A>(
    xs : Iter<A>,
    f : (A, Nat) -> ()
  ) {
    var i = 0;
    loop {
      switch (xs.next()) {
        case (?next) {
          f(next, i);
        };
        case (null) {
          break;
        };
      };
      i += 1;
      continue;
    };
  };

  /// Consumes an iterator and counts how many elements were produced
  /// (discarding them in the process).
  public func size<A>(xs : Iter<A>) : Nat {
    var len = 0;
    iterate<A>(xs, func (x, i) { len += 1; });
    len;
  };

  /// Takes a function and an iterator and returns a new iterator that lazily applies
  /// the function to every element produced by the argument iterator.
  /// ```
  /// let iter = range(1, 3);
  /// let mappedIter = map(iter, func (x : Nat) : Nat { x * 2 });
  /// assertEqual(?2, mappedIter.next());
  /// assertEqual(?4, mappedIter.next());
  /// assertEqual(?6, mappedIter.next());
  /// assertEqual(null, mappedIter.next());
  /// ```
  public func map<A, B>(xs : Iter<A>, f : A -> B) : Iter<B> = object {
    public func next() : ?B {
      loop {
        switch (xs.next()) {
          case (?next) {
            return ?f(next);
          };
          case (null) {
            break;
          };
        };
        continue;
      };
      null;
    };
  };

  /// Creates an iterator that produces an infinite sequence of `x`.
  /// ```
  /// let iter = make(10);
  /// assertEquals(?10, iter.next())
  /// assertEquals(?10, iter.next())
  /// assertEquals(?10, iter.next())
  /// // ...
  /// ```
  public func make<A>(x : A) : Iter<A> = object {
    public func next() : ?A {
      ?x;
    };
  };

  /// Creates an iterator that produces the elements of an Array in ascending index order.
  /// ```
  /// let iter = fromArray([1, 2, 3]);
  /// assertEquals(?1, iter.next())
  /// assertEquals(?2, iter.next())
  /// assertEquals(?3, iter.next())
  /// assertEquals(null, iter.next())
  /// ```
  public func fromArray<A>(xs : [A]) : Iter<A> {
    var ix : Nat = 0;
    let size = xs.size();
    object {
      public func next() : ?A {
        if (ix >= size) {
          return null
        } else {
          let res = ?(xs[ix]);
          ix += 1;
          return res
        }
      }
    }
  };

  /// Like [`fromArray`](#value.fromArray) but for Arrays with mutable elements.
  /// Captures the elements of the Array at the time the iterator is created, so
  /// further modifications won't be reflected in the iterator.
  public func fromArrayMut<A>(xs : [var A]) : Iter<A> {
    fromArray<A>(Array.freeze<A>(xs));
  };

  /// Like [`fromArray`](#value.fromArray) but for Lists.
  public func fromList<A>(xs : List.List<A>) : Iter<A> {
    List.toArray<A>(xs).values();
  };

  /// Consumes an iterator and collects its produced elements in an Array.
  /// ```
  /// let iter = range(1, 3);
  /// assertEquals([1, 2, 3], toArray(iter));
  /// ```
  public func toArray<A>(xs : Iter<A>) : [A] {
    let buffer = Buffer.Buffer<A>(8);
    iterate(xs, func(x : A, ix : Nat) { buffer.add(x) });
    return buffer.toArray()
  };

  /// Like [`toArray`](#value.toArray) but for Arrays with mutable elements.
  public func toArrayMut<A>(xs : Iter<A>) : [var A] {
    Array.thaw<A>(toArray<A>(xs));
  };

  /// Like [`toArray`](#value.toArray) but for Lists.
  public func toList<A>(xs : Iter<A>) : List.List<A> {
    var result = List.nil<A>();
    iterate<A>(xs, func (x, _i) {
      result := List.push<A>(x, result);
    });
    List.reverse<A>(result);
  };
}
