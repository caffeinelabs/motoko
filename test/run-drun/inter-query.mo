actor {
  public query func go1() : async () = async ();
  public query func go2() : async () = async { await go1(); };
};

//CALL ingress go2 "DIDL\x00\x00"
//CALL query go2 "DIDL\x00\x00"
