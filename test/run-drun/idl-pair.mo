actor {
  public type Pair = (Int, Int);
  public query func len2(x:Text, y:Text) : async Pair {
    (x.size(), y.size())
  }
}

//CALL query len2 "DIDL\x00\x02\x71\x71\x02Hi\x05World"

//SKIP run
//SKIP run-ir
//SKIP run-low
