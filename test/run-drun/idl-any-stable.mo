import Prim "mo:⛔";
// This test checks that the IDL decoder properly
// zooms past the any argument and finds the beginning of the string
actor {
  public query func any(_ : Any, x : Text) : async () {
    Prim.debugPrint ("ok: " # x);
  };
}

// This puts the stable memory “shared” type
// in a message, and better gets rejected

//CALL query any 0x4449444C01017102007100000000000003424144

//SKIP run
//SKIP run-ir
//SKIP run-low
