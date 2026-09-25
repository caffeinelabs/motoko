/**
[#mod-Iter]
= `Iter` -- Iterators
*/

import Array "array";
import List "list";

module {
  public type Iter<T> = { next : () -> ?T };

  public class range(x : Nat, y : Nat) {
    var i = x;
    public func next() : ?Nat { if i > y { null } else { let j = i; i += 1; ?j } };
  };

  public func forIn<A>(
    f : (A, Nat) -> (),
    xs : Iter<A>
  ) {
    var i = 0;
    label l loop {
      switch xs.next() {
        case ?next {
          f(next, i);
        }
        case null {
          break l;
        }
      };
      i += 1;
      continue l;
    };
  };

  func length<A>(xs : Iter<A>) : Nat {
    var len = 0;
    forIn<A>(func (x, i) { len += 1; }, xs);
    len;
  };

  public func map<A, B>(f : A -> B, xs : Iter<A>) : Iter<B> = object {
    var i = 0;
    public func next() : ?B {
      label l loop {
        switch xs.next() {
          case ?next {
            return ?f(next);
          }
          case null {
            break l;
          }
        };
        i += 1;
        continue l;
      };
      null;
    };
  };

  public func pure<A>(x : A) : Iter<A> = object {
    public func next() : ?A {
      ?x;
    };
  };

  public func fromArray<A>(xs : [A]) : Iter<A> {
    fromList<A>(List.fromArray<A>(xs));
  };

  public func fromArrayMut<A>(xs : [var A]) : Iter<A> {
    fromArray<A>(Array.freeze<A>(xs));
  };

  public func fromList<A>(xs : List.List<A>) : Iter<A> {
    List.toArray<A>(xs).values();
  };

  public func toArray<A>(xs : Iter<A>) : [A] {
    List.toArray<A>(toList<A>(xs));
  };

  public func toArrayMut<A>(xs : Iter<A>) : [var A] {
    Array.thaw<A>(toArray<A>(xs));
  };

  public func toList<A>(xs : Iter<A>) : List.List<A> {
    toListWithLength<A>(xs).list;
  };

  public func toListWithLength<A>(
    xs : Iter<A>,
  ) : ({
    length : Nat;
    list : List.List<A>;
  }) {
    var _length = 0;
    var _list = List.nil<A>();
    forIn<A>(func (x, i) {
      _length += 1;
      _list := List.push<A>(x, _list);
    }, xs);
    { length = _length; list = List.rev<A>(_list); };
  };
}
