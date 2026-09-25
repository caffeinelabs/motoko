/**
[#mod-List]
= `List` -- Lists

This module provides purely-functional, singly-linked lists.


*/

import Array "array";
import Option "option";

module {

  /**
  A singly-linked list consists of zero or more _cons cells_, wherein
  each cell contains a single list element (the cell's _head_), and a pointer to the
  remainder of the list (the cell's _tail_).
  */
  public type List<T> = ?(T, List<T>);

  /**
  The empty list
  */
  public let nil : <T> () -> List<T> =
    func<T>() : List<T> = null;

  /**
  Returns true if the list is empty.
  */
  public let isNil : <T> List<T> -> Bool =
    func<T>(l : List<T>) : Bool {
      switch l {
        case null { true }
        case _ { false }
      }
    };

  /**
  also known as "list cons"
  */
  public let push : <T> (T, List<T>) -> List<T> =
    func<T>(x : T, l : List<T>) : List<T> = ?(x, l);

  /**
  The last element of the list, if present.
  */
  public let last : <T> List<T> -> ?T =
    func<T>(l : List<T>) : ?T {
      switch l {
        case null { null }
        case ?(x, null) { ?x }
        case ?(_, t) { last<T>(t) }
      }
    };

  /**
  Treating the list as a stack; this combines the usual operations `head` and (non-failing) `tail` into one operation.
  */
  public let pop : <T> List<T> -> (?T, List<T>) =
    func<T>(l : List<T>) : (?T, List<T>) {
      switch l {
        case null { (null, null) }
        case ?(h, t) { (?h, t) }
      }
    };

  /**
  The length of the list
  */
  public let len : <T> List<T> -> Nat =
    func<T>(l : List<T>) : Nat {
      func rec(l : List<T>, n : Nat) : Nat {
        switch l {
          case null { n }
          case ?(_, t) { rec(t, n + 1) }
        }
      };
      rec(l, 0)
    };

  /**
  Tests the length against a maximum value
  */
  public let lenIsEqLessThan : <T> (List<T>, Nat) -> Bool =
    func<T>(l : List <T>, i : Nat) : Bool {
      switch l {
        case null { true }
        case ?(_, t) {
          if i == 0 { false }
          else { lenIsEqLessThan<T>(t, i - 1) }
        }
      };
    };

  /**
  Get the length, unless greater than a maximum value, in which return `null`.
  */
  public let lenClamp : <T> (List<T>, max : Nat) -> ?Nat =
    func<T>(l : List<T>, max : Nat) : ?Nat {
      func rec(l : List<T>, max : Nat, i : Nat) : ?Nat {
        switch l {
          case null { ?i }
          case ?(_, t) {
            if i > max { null }
            else { rec(t, max, i + 1) }
          }
        }
      };
      rec(l, max, 0)
    };

  /**
  Random list access, zero-based.

  NOTE: Indexing into a list is a linear operation, and usually an indication
  that a list might not be the best data structure to use.
  */
  public let nth : <T>(List<T>, Nat) -> ?T =
    func<T>(l : List<T>, n : Nat) : ?T {
      switch (n, l) {
        case (_, null) { null }
        case (0, (?(h, t))) { ?h }
        case (_, (?(_, t))) { nth<T>(t, n - 1) }
      }
    };

  /**
  reverse the list; tail recursive
  */
  public let rev : <T> List<T> -> List<T> =
    func<T>(l : List<T>) : List<T> {
      func rec(l : List<T>, r : List<T>) : List<T> {
        switch l {
          case null { r }
          case ?(h, t) { rec(t, ?(h, r)) }
        }
      };
      rec(l, null)
    };

  /**
  Calls the given function with each list element in turn.

  This is called `app` in SML Basis, and `iter` in OCaml.
  */
  public let iter : <T>(List<T>, f : T -> ()) -> () =
    func iter<T>(l : List<T>, f : T -> ()) {
      switch l {
        case null { () }
        case ?(h, t) { f(h); iter<T>(t, f) }
      }
    };

  /**
  Calls the given function on each list element, collecing the results in a new
  list.
  */
  public let map : <T, S>(List<T>, f : T -> S) -> List<S> =
    func<T, S>(l : List<T>, f:T -> S) : List<S> {
      switch l {
        case null { null }
        case ?(h, t) { ?(f(h), map<T, S>(t, f)) }
      }
    };

  /**
  Creates a new list with only those elements of the original list for which
  the given function (often called the _predicate_) is returns true.
  */
  public let filter : <T>(List<T>, p : T -> Bool) -> List<T> =
    func<T>(l : List<T>, f:T -> Bool) : List<T> {
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

  /**
  Creates two new lists. The first has those elements for which the given
  function `f` returns true, the second those for which it returns false.

  Also known as `partition`.
  */
  public let split : <T>(List<T>, f : T -> Bool) -> (List<T>, List<T>) =
    func<T>(l : List<T>, f:T -> Bool) : (List<T>, List<T>) {
      switch l {
        case null { (null, null) }
        case ?(h, t) {
          if f(h) {
            // call f in-order
            let (l, r) = split<T>(t, f);
            (?(h, l), r)
          } else {
            let (l, r) = split<T>(t, f);
            (l, ?(h, r))
          }
        }
      };
    };

  /**
  Calls the given function on each list element, collecing the non-null results
  in a new list.
  */
  public let mapFilter : <T, S>(List<T>, f : T -> ?S) -> List<S> =
    func<T, S>(l : List<T>, f:T -> ?S) : List<S> {
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

  /**
  Appends two lists.
  */
  public let append : <T>(List<T>, List<T>) -> List<T> =
    func <T>(l : List<T>, m : List<T>) : List<T> {
      func rec(l : List<T>) : List<T> {
        switch l {
          case null { m }
          case ?(h, t) { ?(h, rec(t)) }
        }
      };
      rec(l)
    };

  /**
  concatenations a list of lists (Also known as "list join")
  */
  public let concat : <T>(List<List<T>>) -> List<T> =
    // tail recursive, but requires "two passes"
    func<T>(l : List<List<T>>) : List<T> {
      // 1/2: fold from left to right, reverse-appending the sublists...
      let r = foldLeft<List<T>, List<T>>(l, null, func(a, b) { revAppend<T>(a, b) });
      // 2/2: ...re-reverse the elements, to their original order:
      rev<T>(r)
    };

  // Internal utility-function
  func revAppend<T>(l1 : List<T>, l2 : List<T>) : List<T> {
    switch l1 {
      case null { l2 }
      case ?(h, t) { revAppend<T>(t, ?(h, l2)) }
    }
  };

  /**
  "takes" `n` elements from the prefix of the given list.
  If the given list has fewer than `n` elements, this returns (a copy of) the
  full input list.
  */
  public let take : <T>(List<T>, n:Nat) -> List<T> =
    func<T>(l : List<T>, n:Nat) : List<T> {
      switch (l, n) {
        case (_, 0) { null }
        case (null, _) { null }
        case (?(h, t), m) { ?(h, take<T>(t, m - 1)) }
      }
    };

  /**
  "drops" an `n` element prefix from the given list.
  */
  public let drop : <T>(List<T>, n:Nat) -> List<T> =
    func<T>(l : List<T>, n:Nat) : List<T> {
      switch (l, n) {
        case (l_, 0) { l_ }
        case (null, _) { null }
        case ((?(h, t)), m) { drop<T>(t, m - 1) }
      }
    };

  /**
  fold list left-to-right using function `f`.
  */
  public let foldLeft : <T, S>(List<T>, S, f : (T, S) -> S) -> S =
    func<T, S>(l : List<T>, a:S, f:(T, S) -> S) : S {
      switch l {
        case null { a }
        case ?(h, t) { foldLeft<T, S>(t, f(h, a), f) }
      };
    };

  /**
  fold the list right-to-left using function `f`.
  */
  public let foldRight : <T, S>(List<T>, S, f : (T, S) -> S) -> S =
    func<T, S>(l : List<T>, a:S, f:(T, S) -> S) : S {
      switch l {
        case null { a }
        case ?(h, t) { f(h, foldRight<T, S>(t, a, f)) }
      };
    };

  /**
  Returns the first element for which given predicate `f` is true, if such an element exists.
  */
  public let find : <T>(l: List<T>, f : T -> Bool) -> ?T =
    func<T>(l: List<T>, f:T -> Bool) : ?T {
      switch l {
        case null { null }
        case ?(h, t) { if f(h) { ?h } else { find<T>(t, f) } }
      };
    };

  /**
  Returns true if there exists list element for which given predicate `f` is true
  */
  public let exists : <T>(List<T>, f : T -> Bool) -> Bool =
    func<T>(l: List<T>, f:T -> Bool) : Bool {
      switch l {
        case null { false }
        case ?(h, t) { f(h) or exists<T>(t, f) }
      };
    };

  /**
  Returns true if for all list element the given predicate `f` is true
  */
  public let all : <T>(List<T>, f : T -> Bool) -> Bool =
    func<T>(l: List<T>, f:T -> Bool) : Bool {
      switch l {
        case null { true }
        case ?(h, t) { f(h) and all<T>(t, f) }
      }
    };

  /**
  Given two lists that are orderd (with regard to the given relation `lte`),
  merge them into a single ordered list
  */
  public let merge : <T>(List<T>, List<T>, lte : (T, T) -> Bool) -> List<T> =
    func<T>(l1: List<T>, l2: List<T>, lte:(T, T) -> Bool) : List<T> {
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

  /**
  Compares two lists using lexicographic ordering (with regard to the given relation `lte`).

  // To do: Eventually, follow `collate` design from SML Basis, with real sum
  // types, use 3-valued `order` type here.
  */
  public let lessThanEq : <T>(List<T>, List<T>, lte: (T, T) -> Bool) -> Bool =
    func<T>(l1: List<T>, l2: List<T>, lte:(T, T) -> Bool) : Bool {
      switch (l1, l2) {
        case (null, _) { true }
        case (_, null) { false }
        case (?(h1, t1), ?(h2, t2)) { lte(h1, h2) and lessThanEq<T>(t1, t2, lte) }
      };
    };

  /**
  Compares two lists for equality (with regard to the given relation `eq` on elements).

  // `isEq(l1, l2)` is equivalent to `lessThanEq(l1,l2) && lessThanEq(l2,l1)`, but the former is more efficient.
  */
  public let isEq : <T>(List<T>, List<T>, eq : (T, T) -> Bool) -> Bool =
    func<T>(l1: List<T>, l2: List<T>, eq:(T, T) -> Bool) : Bool {
      switch (l1, l2) {
        case (null, null) { true }
        case (null, _) { false }
        case (_, null) { false }
        case (?(h1, t1), ?(h2, t2)) { eq(h1, h2) and isEq<T>(t1, t2, eq) }
      }
    };

  /**
  generates a list based on a length, and a function from list index to list element.
  */
  public let tabulate : <T>(Nat, f : Nat -> T) -> List<T> =
    func<T>(n:Nat, f:Nat -> T) : List<T> {
      func rec(i:Nat, n: Nat, f : Nat -> T) : List<T> {
        if i == n { null } else { ?(f(i), rec(i + 1, n, f)) }
      };
      rec(0, n, f)
    };

  /**
  creates a list with exactly one element.
  */
  public let singleton : <X> X -> List<X> =
    func<X>(x : X) : List<X> {
      ?(x, null)
    };

  /**
  creates a list of the given length with the same value in each position.
  */
  public let replicate : <X>(Nat, X) -> List<X> =
    func<X>(n : Nat, x : X) : List<X> {
      tabulate<X>(n, func _ { x })
    };

  /**
  creates a list of pairs from a pair of lists.

  If the given lists have different lengths, then the created list will have a
  length equal to the lenght of the smaller list.
  */
  public let zip : <X, Y>(List<X>, List<Y>) -> List<(X, Y)> =
    func<X, Y>(xs : List<X>, ys : List<Y>) : List<(X, Y)> {
      zipWith<X, Y, (X, Y)>(xs, ys, func (x, y) { (x, y) })
    };

  /**
  Creates a list whose elements are calculated from the function `f` and
  elements occuring at the same position in the given lists.

  If the given lists have different lengths, then the created list will have a
  length equal to the length of the smaller list.
  */
  public let zipWith : <X, Y, Z>(List<X>, List<Y>, f : (X, Y) -> Z) -> List<Z> =
    func<X, Y, Z>(xs : List<X>, ys : List<Y>, f : (X, Y) -> Z) : List<Z> {
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

  /**
  splits the given list at the given zero-based index.
  */
  public let splitAt : <X>(Nat, List<X>) -> (List<X>, List<X>) =
    func<X>(n : Nat, xs : List<X>) : (List<X>, List<X>) {
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
                (singleton<X>(h), t)
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

  /**
  split the given list into length-n chunks. The last chunk will be shorter if
  n does not evenly divide the length of the given list.
  */
  public let chunksOf : <X>(Nat, List<X>) -> List<List<X>> =
    func<X>(n : Nat, xs : List<X>) : List<List<X>> {
      let (l, r) = splitAt<X>(n, xs);
      if isNil<X>(l) {
        null
      } else {
        push<List<X>>(l, chunksOf<X>(n, r))
      }
    };

  /**
  converts an array into a list.
  */
  public let fromArray : <A>[A] -> List<A> =
    func<A>(xs : [A]) : List<A> {
      Array.foldr<A, List<A>>(func (x : A, ys : List<A>) : List<A> {
        push<A>(x, ys);
      }, nil<A>(), xs);
    };

  /**
  converts a mutable array into a list.
  */
  public let fromArrayMut : <A>[var A] -> List<A> =
    func<A>(xs : [var A]) : List<A> {
      fromArray<A>(Array.freeze<A>(xs));
    };

  /**
  creates an array from the list
  */
  public let toArray : <A> List<A> -> [A] =
    func<A>(xs : List<A>) : [A] {
      let length = len<A>(xs);
      var list = xs;
      Array.tabulate<A>(length, func (i) {
        let popped = pop<A>(list);
        list := popped.1;
        Option.unwrap<A>(popped.0);
      });
    };

  /**
  creates a mutable array from the list
  */
  public let toArrayMut : <A> List<A> -> [var A] =
    func<A>(xs : List<A>) : [var A] {
      Array.thaw<A>(toArray<A>(xs));
    };

  /*

To do:
--------
- iterator objects, for use in `for ... in ...` patterns
- operations for lists of pairs and pairs of lists: split, etc
- more regression tests for everything that is below

*/
}
