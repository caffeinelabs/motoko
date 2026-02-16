//MOC-FLAG -A=M0194

module X {
  public func f() { g() };
  func g() { f() };
};

let ok = X.f;
