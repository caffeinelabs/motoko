//MOC-FLAG -A=M0194
actor {

  type A = Nat;
  module X = { public type A = Bool };
  module Y = { public type A = Text };
  type T = (A, X.A, Y.A);

  stable var t = (0, true, "abc") : T;

}
