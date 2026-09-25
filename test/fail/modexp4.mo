module X = {
  public func f() { g(); };
  func g() { f(); };
  module Y = {
    public type U = U -> U;
  };
  public type T = Y.U; // ok public field even though Y private and U "unavoidable"
  public func h():T { h() }; // ditto
};
