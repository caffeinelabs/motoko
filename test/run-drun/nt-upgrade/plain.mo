import Prim "mo:⛔";

// A program without native tables: its state survives a graph-copy upgrade
actor {
  type User = { name : Text; city : Text; amount : Nat };

  var users : [(Nat, User)] = [];

  public func put() : async () {
    users := [(7, { name = "Ana"; city = "Basel"; amount = 40 }), (2 ** 100, { name = "Ed"; city = "Ems"; amount = 2 ** 100 })]
  };

  public query func check() : async () {
    Prim.debugPrint(debug_show users)
  }
}
