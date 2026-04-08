//MOC-FLAG -A=M0194
actor {

  type A = Nat;
  type B = Bool;
  type pair = (A, B);
  type alias = (pair, pair);

  stable var p  = ((1,true),(1,true)) : alias;

}
