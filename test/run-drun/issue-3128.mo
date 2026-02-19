//MOC-FLAG -A=M0194
actor {

  type fst = Nat;
  type snd = Bool;
  type pair = (fst, snd);
  type alias = (pair, pair);

  stable var p  = ((1,true),(1,true)) : alias;

}
