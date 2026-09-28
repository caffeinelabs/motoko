import Prim "mo:prim";

// Compatible upgrade
actor {
  type Variant = {
    #two : { key : Nat; var name : Text };
    #one : Nat;
    #three : Text;
  };

  var root : Variant = #three "ERROR";

  public func modify() : async () {
    switch root {
      case (#two record) record.name #= " TEST";
      case _ assert false;
    };
  };

  public func print() : async () {
    Prim.debugPrint(debug_show (root));
  };
};
