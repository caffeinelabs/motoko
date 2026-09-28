import Prim "mo:prim";

actor {
  type Variant = {
    #one : Nat;
    #two : { key : Nat; var name : Text };
  };

  var root : Variant = #two { key = 1; var name = "TEST" };

  public func modify() : async () {
    switch root {
      case #two(record) { record.name #= " TEST" }
      case _ { assert false }
    };
  };

  public func print() : async () {
    Prim.debugPrint(debug_show (root));
  };
};
