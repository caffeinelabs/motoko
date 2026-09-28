module X = {
  public func f() { g(); };
  func g() { f(); };
  let y : Nat = 2;
  type U = Int;
  public type T = U;
};

type Ok = X.T;
type Wrong = X.U;
