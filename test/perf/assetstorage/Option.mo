/// Typesafe nulls
///
/// Optional values can be seen as a typesafe `null`. A value of type `?Int` can
/// be constructed with either `null` or `?42`. The simplest way to get at the
/// contents of an optional is to use pattern matching:
///
/// ```motoko
/// let optionalInt1 : ?Int = ?42;
/// let optionalInt2 : ?Int = null;
///
/// let int1orZero : Int = switch(optionalInt1) {
///   case null 0;
///   case (?int) int;
/// };
/// assertEquals(int1OrZero, 42);
///
/// let int2orZero : Int = switch(optionalInt2) {
///   case null 0;
///   case (?int) int;
/// };
/// assertEquals(int2OrZero, 0);
/// ```
///
/// The functions in this module capture some common operations when working
/// with optionals that can be more succinct than using pattern matching.

import P "Prelude";

module {

  /// Unwraps an optional value, with a default value, i.e. `get(?x, d) = x` and
  /// `get(null, d) = d`.
  public func get<T>(x : ?T, default : T) : T =
    switch x {
      case null { default }
      case ?x_ { x_ }
    };

  /// Unwraps an optional value using a function, or returns the default, i.e.
  /// `option(?x, f, d) = f x` and `option(null, f, d) = d`.
  public func getMapped<A, B>(x : ?A, f : A -> B, default : B) : B =
    switch x {
      case null { default }
      case ?x_ { f(x_) }
    };

  /// Applies a function to the wrapped value. `null`'s are left untouched.
  /// ```
  /// map(?(x), f) = ?(f(x))
  /// map(null, f) = null
  /// ```
  public func map<A, B>(x : ?A, f : A -> B) : ?B =
    switch x {
      case null { null }
      case ?x_ { ?f(x_) }
    };

  /// Applies a function to the wrapped value, but discards the result. Use
  /// `iterate` if you're only interested in the side effect `f` produces.
  ///
  /// ```
  /// var counter : Nat = 0;
  /// iterate(?(5), func (x : Nat) { counter += x });
  /// assert(counter == 5);
  /// iterate(null, func (x : Nat) { counter += x });
  /// assert(counter == 5);
  /// ```
  public func iterate<A>(x : ?A, f : A -> ()) =
    switch x {
      case null {}
      case ?x_ { f(x_) }
    };

  /// Applies an optional function to an optional value. Returns `null` if at
  /// least one of the arguments is `null`.
  public func apply<A, B>(x : ?A, f : ?(A -> B)) : ?B {
    switch (f, x) {
      case (?f_, ?x_) {
        ?f_(x_);
      }
      case (_, _) {
        null;
      }
    };
  };

  /// Applies a function to an optional value. Returns `null` if the argument is
  /// `null`, or the function returns `null`.
  ///
  /// NOTE: Together with [`make`](#value.make), this forms a “monad”.
  public func chain<A, B>(x : ?A, f : A -> ?B) : ?B {
    switch x {
      case ?x_ {
        f(x_);
      }
      case null {
        null;
      }
    };
  };

  /// Given an optional optional value, removes one layer of optionality.
  /// ```
  /// flatten(?(?(42)))) = ?(42)
  /// flatten(?(null))) = null
  /// flatten(null)) = null
  /// ```
  public func flatten<A>(x : ??A) : ?A {
    chain<?A, A>(x, func (x_ : ?A) : ?A {
      x_;
    });
  };

  /// Creates an optional value from a definite value.
  /// ```
  /// make(42) = ?(42)
  /// ```
  public func make<A>(x: A) : ?A = ?x;

  /// Returns true if the argument is not `null`, otherwise returns false.
  public func isSome(x : ?Any) : Bool =
    switch x {
      case null { false }
      case _ { true }
    };

  /// Returns true if the argument is `null`, otherwise returns false.
  public func isNull(x : ?Any) : Bool =
    switch x {
      case null { true }
      case _ { false }
    };

  /// Asserts that the value is not `null`; fails otherwise.
  /// Deprecated.
  public func assertSome(x : ?Any) =
    switch x {
      case null { P.unreachable() }
      case _ {}
    };

  /// Asserts that the value _is_ `null`; fails otherwise.
  /// Deprecated.
  public func assertNull(x : ?Any) =
    switch x {
      case null {}
      case _ { P.unreachable() }
    };

  /// Unwraps an optional value, i.e. `unwrap(?x) = x`.
  ///
  /// WARNING: `unwrap(x)` will fail if the argument is `null`, and is generally considered bad style. Use `switch x` instead.
  public func unwrap<T>(x : ?T) : T =
    switch x {
      case null { P.unreachable() }
      case ?x_ { x_ }
    };
}
