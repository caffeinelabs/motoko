import P "mo:⛔";

actor {
  var a : [var Nat] = [var];

  system func preupgrade() {
    a := P.Array_init<Nat>(268435456 / 4, 0x0F); // 0.25 GB array (I think)
    P.debugPrint("pre");
  };

  system func postupgrade() {
    // it is expected that we get here, which shows that deserialising
    // small `Nat`s doesn't allocate on the heap
    P.debugPrint("post");
    P.trap("deliberate trap");
  }
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL upgrade ""
