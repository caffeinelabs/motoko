// test type components in object and actors (not just modules)

object X = {
  public func f() { g();};
  func g() { f();};
  public object Y = {
    public type U = Int;
  };
  public type T = Y.U; // ok type field since Y.U private but avoidable
  func h():T { h() }; // ditto
};

type XT = X.T;
type XYU = X.Y.U;


actor A = {
  public func f() : () { g();};
  flexible shared func g() : () { f();};
  flexible object Y = {
    public type U = Int;
  };
  public type T = Y.U; // ok type field since Y.U private but avoidable
  flexible func h():T { h() }; // ditto
};

type AT = A.T;

//SKIP comp
