/**
[#mod-Option]
= `Option` -- Optional values
*/

import P "prelude";

module {

  /**
Returns true if the argument is not `null`.
*/
  public let isSome : ?Any -> Bool = func(x) =
    switch x {
      case null { false }
      case _ { true }
    };

  /**
Returns true if the argument is `null`.
*/
  public let isNull : ?Any -> Bool = func(x) =
    switch x {
      case null { true }
      case _ { false }
    };

  /**
Unwraps an optional value, with a default value, i.e. `unwrap(?x, d) = x` and `unwrap(null, d) = d`.
*/
  public let unwrapOr : <T> (?T, default : T) -> T =
    func <T>(x: ?T, d: T): T =
      switch x {
        case null { d }
        case ?x_ { x_ }
      };

  /**
Unwraps an optional value using a function, or returns the default, i.e. `option(?x, f, d) = f x` and `option(null, f, d) = d`.
*/
  public let option : <A, B>(?A, f : A -> B, default : B) -> B =
    func<A, B>(x: ?A, f: A->B, d: B): B =
      switch x {
        case null { d }
        case ?x_ { f(x_) }
      };

  /**
Applies a function to the wrapped value.
*/
  public let map : <A, B>(f: A->B, x: ?A) -> ?B =
    func map<A, B>(f: A->B, x: ?A): ?B =
      switch x {
        case null { null }
        case ?x_ { ?f(x_) }
      };

  /**
Applies an optional function to an optional value. Returns `null` if at least one of the arguments is `null`.
*/
  public let apply : <A, B>(?(A -> B), ?A) -> ?B =
    func <A, B>(f : ?(A -> B), x : ?A) : ?B {
      switch (f, x) {
        case (?f_, ?x_) {
          ?f_(x_);
        }
        case (_, _) {
          null;
        }
      };
    };

  /**
Applies an function to an optional value. Returns `null` if the argument is `null`, or the function returns `null`.

NOTE: Together with <<Option_pure,`pure`>>, this forms a “monad”.
*/
  public let bind : <A, B>(?A, A -> ?B) -> ?B =
    func<A, B>(x : ?A, f : A -> ?B) : ?B {
      switch x {
        case ?x_ {
          f(x_);
        }
        case null {
          null;
        }
      };
    };

  /**
Given an optional optional value, removes one layer of optionality.
*/
  public let join : <A> ??A -> ?A =
    func<A>(x : ??A) : ?A {
      bind<?A, A>(x, func (x_ : ?A) : ?A {
        x_;
      });
    };

  /**
Creates an optional value from a definite value.
*/
  public let pure : <A> A -> ?A =
    func <A>(x: A) : ?A = ?x;

  /**
Asserts that the value is not `null`; fails otherwise.
*/
  public let assertSome : ?Any -> () =
    func(x) =
      switch x {
        case null { P.unreachable() }
        case _ {}
      };

  /**
Asserts that the value _is_ `null`; fails otherwise.
*/
  public let assertNull : ?Any -> () =
    func(x) =
      switch x {
        case null {}
        case _ { P.unreachable() }
      };

  /**
Unwraps an optional value, i.e. `unwrap(?x) = x`.

WARNING: `unwrap(x)` will fail if the argument is `null`, and is generally considered bad style. Use `switch x` instead.
*/
  public let unwrap : <T>?T -> T =
    func<T>(x: ?T): T =
      switch x {
        case null { P.unreachable() }
        case ?x_ { x_ }
      };

}
