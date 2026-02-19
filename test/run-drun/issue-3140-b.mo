//MOC-FLAG -A=M0194
actor {

  type A<T> = T;
  module X = { public type A<T> = T };
  module Y = { public type A<T> = T };
  type T = (A<Nat>, X.A<Bool>, Y.A<Text>);

  stable var t = (0, true, "abc") : T;

}
