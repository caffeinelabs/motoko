//MOC-FLAG --package core $MOTOKO_CORE
import Text "mo:core/Text";
import Iter "mo:core/Iter";

let chars = "a b c".chars().map(
  func(c) {
    if (c == ' ') return #space else return #char(c);
  }
);

assert Text.fromIter(
  chars.filterMap(
    func(v) {
      switch v { case (#space) return null; case (#char(c)) return ?c };
    }
  )
) == "abc";

// A return type annotation can be a subtype of the expected return type
func applyInt<T>(x : T, f : T -> Int) : Int = f(x);
assert applyInt(5, func(x) : Nat { return x + 1 }) == 6
