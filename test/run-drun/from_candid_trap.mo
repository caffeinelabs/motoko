//MOC-FLAG -A=M0194
// test from_candid on an invalid blob traps
// NB: we may wish to return null instead, in future
actor {

  public func go() : async () {
    try {
      await async {
        let ohoh : ?() = from_candid "";
        assert false;
      }
    } catch e {
    };
  }
}

//skip run
//skip run-ir
//skip run-low
//CALL ingress go "DIDL\x00\x00"
