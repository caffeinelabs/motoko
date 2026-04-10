module Nat {
  public func abs(self : Int) : Int { -self };
};

let x = -2.abs() ; syntax error;
assert x == 2;
