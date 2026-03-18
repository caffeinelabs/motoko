//MOC-FLAG -A=M0194

module X {
  public func f() { g() };
  func g() { f() };
  type U = Int -> Int;
  public type T = U; // ok export, because U, though private, is avoidable
  public func h() : T { h() }; // ditto
};

type Ok = X.T;
let ok = X.h;
