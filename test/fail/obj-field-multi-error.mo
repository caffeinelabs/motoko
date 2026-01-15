module A {
  public type T = ()
};
module B {
  public type T = ()
};
type MyType = {
  a : A.MissingA;
  b : B.MissingB;
}
