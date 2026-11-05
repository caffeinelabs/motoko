import Prim "mo:⛔";

// A version that makes no table and keeps no codec, so it links the runtime without native tables.
// It keeps the ids of the tables another version made, without their codecs.
actor {
  type User = { name : Text; city : Text; amount : Nat };

  var users : [(Nat, User)] = [];
  var kept : ?{ id : Nat64; indexes : [([Text], Nat)] } = null;

  public func put() : async () {
    users := [(7, { name = "Ana"; city = "Basel"; amount = 40 })]
  };

  public query func check() : async () {
    Prim.debugPrint(debug_show (users, kept))
  }
}
