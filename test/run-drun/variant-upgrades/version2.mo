import Prim "mo:prim";

// Incompatible upgrade
actor {
  type Variant = {
    #two : { key : Nat; var name : Text };
    #one : Nat;
  };

  var root: Variant = #one 0;

  public func modify() : async () {
    switch root {
      case (#two record) record.name #= " TEST";
      case _ assert false;
    };
  };

  public func print() : async () {
    Prim.debugPrint(debug_show(root));
  };
};
