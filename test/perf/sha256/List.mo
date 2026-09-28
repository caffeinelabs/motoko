/// Purely-functional, singly-linked lists.

import Array "Array";
import Iter "IterType";
import Option "Option";
import Order "Order";
import Result "Result";

module {

  // A singly-linked list consists of zero or more _cons cells_, wherein
  // each cell contains a single list element (the cell's _head_), and a pointer to the
  // remainder of the list (the cell's _tail_).
  public type List<T> = ?(T, List<T>);

  /// Create an empty list.
  public func nil<T>() : List<T> = null;

  /// Check whether a list is empty and return true if the list is empty.
  public func isNil<T>(l : List<T>) : Bool {
    switch l {
      case null { true }
      case _ { false }
    }
  };

  /// Construct a list by pre-pending a value.
  /// This function is similar to a `list.cons(item)` function.
  public func push<T>(x : T, l : List<T>) : List<T> = ?(x, l);

  /// Return the last element of the list, if present.
  public func last<T>(l : List<T>) : ?T {
    switch l {
      case null { null }
      case ?(x, null) { ?x }
      case ?(_, t) { last<T>(t) }
    }
  };

  /// Treat the list as a stack.
  /// This function combines the `head` and (non-failing) `tail` operations into one operation.
  public func pop<T>(l : List<T>) : (?T, List<T>) {
    switch l {
      case null { (null, null) }
      case ?(h, t) { (?h, t) }
    }
  };

  /// Return the length of the list.
  public func size<T>(l : List<T>) : Nat {
    func rec(l : List<T>, n : Nat) : Nat {
      switch l {
        case null { n }
        case ?(_, t) { rec(t, n + 1) }
      }
    };
    rec(l, 0)
  };
  /// Access any item in a list, zero-based.
  ///
  /// NOTE: Indexing into a list is a linear operation, and usually an
  /// indication that a list might not be the best data structure
  /// to use.
  public func get<T>(l : List<T>, n : Nat) : ?T {
    switch (n, l) {
      case (_, null) { null }
      case (0, (?(h, t))) { ?h }
      case (_, (?(_, t))) { get<T>(t, n - 1) }
    }
  };

  /// Reverses the list
  public func reverse<T>(l : List<T>) : List<T> {
    func rec(l : List<T>, r : List<T>) : List<T> {
      switch l {
        case null { r }
        case ?(h, t) { rec(t, ?(h, r)) }
      }
    };
    rec(l, null)
  };

  /// Call the given function with each list element in turn.
  ///
  /// This function is equivalent to the `app` function in Standard ML Basis,
  /// and the `iter` function in OCaml.
  public func iterate<T>(l : List<T>, f : T -> ()) {
    switch l {
      case null { () }
      case ?(h, t) { f(h); iterate<T>(t, f) }
    }
  };

  /// Call the given function on each list element and collect the results
  /// in a new list.
  public func map<T, S>(l : List<T>, f : T -> S) : List<S> {
    switch l {
      case null { null }
      case ?(h, t) { ?(f(h), map<T, S>(t, f)) }
    }
  };

  /// Create a new list with only those elements of the original list for which
  /// the given function (often called the _predicate_) returns true.
  public func filter<T>(l : List<T>, f : T -> Bool) : List<T> {
    switch l {
      case null { null }
      case ?(h, t) {
        if f(h) {
          ?(h, filter<T>(t, f))
        } else {
          filter<T>(t, f)
        }
      }
    };
  };

  /// Create two new lists from the results of a given function (`f`).
  /// The first list only includes the elements for which the given
  /// function `f` returns true and the second list only includes
  /// the elements for which the function returns false.
  public func partition<T>(l : List<T>, f : T -> Bool) : (List<T>, List<T>) {
    switch l {
      case null { (null, null) }
      case ?(h, t) {
        if f(h) {
          // call f in-order
          let (l, r) = partition<T>(t, f);
          (?(h, l), r)
        } else {
          let (l, r) = partition<T>(t, f);
          (l, ?(h, r))
        }
      }
    };
  };

  /// Call the given function on each list element, and collect the non-null results
  /// in a new list.
  public func mapFilter<T, S>(l : List<T>, f : T -> ?S) : List<S> {
    switch l {
      case null { null }
      case ?(h, t) {
        switch f(h) {
          case null { mapFilter<T, S>(t, f) }
          case ?h_ { ?(h_, mapFilter<T, S>(t, f)) }
        }
      }
    };
  };

  /// Maps a Result-returning function over a List and returns either
  /// the first error or a list of successful values.
  public func mapResult<A, R, E>(xs : List<A>, f : A -> Result.Result<R, E>) : Result.Result<List<R>, E> {
    func go(xs : List<A>, acc : List<R>) : Result.Result<List<R>, E> {
      switch xs {
        case null { #ok(acc) }
        case ?(head, tail) {
          switch f(head) {
            case #err(err) { #err(err) }
            case #ok(ok) { go(tail, ?(ok, acc)) }
          };
        }
      }
    };
    Result.mapOk(go(xs, null), func (xs : List<R>) : List<R> = reverse(xs))
  };

  /// Append the elements from the reverse of one list to another list.
  func revAppend<T>(l : List<T>, m : List<T>) : List<T> {
    switch l {
      case null { m }
      case ?(h, t) { revAppend(t, ?(h, m)) }
    }
  };

  /// Append the elements from one list to another list.
  public func append<T>(l : List<T>, m : List<T>) : List<T> {
    revAppend(reverse(l), m);
  };

  /// Concatenate a list of lists.
  ///
  /// In some languages, this operation is also known as a `list join`.
  public func flatten<T>(l : List<List<T>>) : List<T> {
    foldLeft<List<T>, List<T>>(l, null, func(a, b) { append<T>(a, b) });
  };

  /// Returns the first `n` elements of the given list.
  /// If the given list has fewer than `n` elements, this function returns
  /// a copy of the full input list.
  public func take<T>(l : List<T>, n:Nat) : List<T> {
    switch (l, n) {
      case (_, 0) { null }
      case (null, _) { null }
      case (?(h, t), m) { ?(h, take<T>(t, m - 1)) }
    }
  };

  /// Drop the first `n` elements from the given list.
  public func drop<T>(l : List<T>, n:Nat) : List<T> {
    switch (l, n) {
      case (l_, 0) { l_ }
      case (null, _) { null }
      case ((?(h, t)), m) { drop<T>(t, m - 1) }
    }
  };

  /// Fold the list left-to-right using the given function (`f`).
  public func foldLeft<T, S>(l : List<T>, a : S, f : (S, T) -> S) : S {
    switch l {
      case null { a }
      case ?(h, t) { foldLeft(t, f(a, h), f) }
    };
  };

  /// Fold the list right-to-left using the given function (`f`).
  public func foldRight<T, S>(l : List<T>, a : S, f : (T, S) -> S) : S {
    switch l {
      case null { a }
      case ?(h, t) { f(h, foldRight<T, S>(t, a, f)) }
    };
  };

  /// Return the first element for which the given predicate `f` is true,
  /// if such an element exists.
  public func find<T>(l: List<T>, f:T -> Bool) : ?T {
    switch l {
      case null { null }
      case ?(h, t) { if f(h) { ?h } else { find<T>(t, f) } }
    };
  };

  /// Return true if there exists a list element for which
  /// the given predicate `f` is true.
  public func some<T>(l : List<T>, f : T -> Bool) : Bool {
    switch l {
      case null { false }
      case ?(h, t) { f(h) or some<T>(t, f) }
    };
  };

  /// Return true if the given predicate `f` is true for all list
  /// elements.
  public func all<T>(l : List<T>, f : T -> Bool) : Bool {
    switch l {
      case null { true }
      case ?(h, t) { f(h) and all<T>(t, f) }
    }
  };

  /// Merge two ordered lists into a single ordered list.
  /// This function requires both list to be ordered as specified
  /// by the given relation `lte`.
  public func merge<T>(l1 : List<T>, l2 : List<T>, lte : (T, T) -> Bool) : List<T> {
    switch (l1, l2) {
      case (null, _) { l2 }
      case (_, null) { l1 }
      case (?(h1, t1), ?(h2, t2)) {
        if lte(h1, h2) {
          ?(h1, merge<T>(t1, l2, lte))
        } else {
          ?(h2, merge<T>(l1, t2, lte))
        }
      }
    }
  };

  /// Compare two lists using lexicographic ordering specified by the given relation `lte`.
  public func compare<T>(l1 : List<T>, l2 : List<T>, compElm: (T, T) -> Order.Order) : Order.Order {
    switch (l1, l2) {
      case (null, null) { #equal }
      case (null, _) { #less }
      case (_, null) { #greater }
      case (?(h1, t1), ?(h2, t2)) {
        let hOrder = compElm(h1, h2);
        if Order.isEqual(hOrder) {
          compare<T>(t1, t2, compElm)
        } else {
          hOrder
        }
      }
    };
  };

  /// Compare two lists for equality as specified by the given relation `eq` on the elements.
  ///
  /// The function `isEq(l1, l2)` is equivalent to `lessThanEq(l1, l2) && lessThanEq(l2, l1)`,
  /// but the former is more efficient.
  public func equal<T>(l1 : List<T>, l2 : List<T>, eq :(T, T) -> Bool) : Bool {
    switch (l1, l2) {
      case (null, null) { true }
      case (null, _) { false }
      case (_, null) { false }
      case (?(h1, t1), ?(h2, t2)) { eq(h1, h2) and equal<T>(t1, t2, eq) }
    }
  };

  /// Generate a list based on a length and a function that maps from
  /// a list index to a list element.
  public func tabulate<T>(n : Nat, f : Nat -> T) : List<T> {
    var i = 0;
    var l : List<T> = null;
    while i < n {
      l := ?(f(i), l);
      i += 1;
    };
    reverse(l);
  };

  /// Create a list with exactly one element.
  public func make<X>(x : X) : List<X> = ?(x, null);

  /// Create a list of the given length with the same value in each position.
  public func replicate<X>(n : Nat, x : X) : List<X> {
    var i = 0;
    var l : List<X> = null;
    while i < n {
      l := ?(x, l);
      i += 1;
    };
    l;
  };

  /// Create a list of pairs from a pair of lists.
  ///
  /// If the given lists have different lengths, then the created list will have a
  /// length equal to the length of the smaller list.
  public func zip<X, Y>(xs : List<X>, ys : List<Y>) : List<(X, Y)> =
    zipWith<X, Y, (X, Y)>(xs, ys, func (x, y) { (x, y) });

  /// Create a list in which elements are calculated from the function `f` and
  /// include elements occuring at the same position in the given lists.
  ///
  /// If the given lists have different lengths, then the created list will have a
  /// length equal to the length of the smaller list.
  public func zipWith<X, Y, Z>(
    xs : List<X>,
    ys : List<Y>,
    f : (X, Y) -> Z
  ) : List<Z> {
    switch pop<X>(xs) {
      case (null, _) { null }
      case (?x, xt) {
        switch pop<Y>(ys) {
          case (null, _) { null }
          case (?y, yt) {
            push<Z>(f(x, y), zipWith<X, Y, Z>(xt, yt, f))
          }
        }
      }
    }
  };

  /// Split the given list at the given zero-based index.
  public func split<X>(n : Nat, xs : List<X>) : (List<X>, List<X>) {
    if n == 0 {
      (null, xs)
    } else {
      func rec(n : Nat, xs : List<X>) : (List<X>, List<X>) {
        switch pop<X>(xs) {
          case (null, _) {
            (null, null)
          }
          case (?h, t) {
            if n == 1 {
              (make<X>(h), t)
            } else {
              let (l, r) = rec(n - 1, t);
              (push<X>(h, l), r)
            }
          }
        }
      };
      rec(n, xs)
    }
  };

  /// Split the given list into chunks of length `n`.
  /// The last chunk will be shorter if the length of the given list
  /// does not divide by `n` evenly.
  public func chunks<X>(n : Nat, xs : List<X>) : List<List<X>> {
    let (l, r) = split<X>(n, xs);
    if isNil<X>(l) {
      null
    } else {
      push<List<X>>(l, chunks<X>(n, r))
    }
  };

  /// Convert an array into a list.
  public func fromArray<A>(xs : [A]) : List<A> {
    Array.foldRight<A, List<A>>(
      xs,
      nil<A>(),
      func (x : A, ys : List<A>) : List<A> {
        push<A>(x, ys);
      }
    );
  };

  /// Convert a mutable array into a list.
  public func fromVarArray<A>(xs : [var A]) : List<A> =
    fromArray<A>(Array.freeze<A>(xs));

  /// Create an array from a list.
  public func toArray<A>(xs : List<A>) : [A] {
    let length = size<A>(xs);
    var list = xs;
    Array.tabulate<A>(length, func (i) {
      let popped = pop<A>(list);
      list := popped.1;
      switch popped.0 {
        case null { loop { assert false } }
        case ?x { x }
      }
    });
  };

  /// Create a mutable array from a list.
  public func toVarArray<A>(xs : List<A>) : [var A] =
    Array.thaw<A>(toArray<A>(xs));

  /// Create an iterator from a list.
  public func toIter<A>(xs : List<A>) : Iter.Iter<A> {
    var state = xs;
    object {
      public func next() : ?A =
        switch state {
          case ?(hd, tl) { state := tl; ?hd }
          case _ { null }
        }
    }
  }

}
