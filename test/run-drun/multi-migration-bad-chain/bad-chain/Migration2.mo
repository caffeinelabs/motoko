import Prim "mo:prim";

module {

  public func run(old : { var zero : Nat; var three : [var (Nat, Text)] }) : {
    var zero : Nat;
    var three : [var (Nat, Text)];
    var four : Text;
  } {
    let new = {
      var zero = old.zero;
      var three = old.three;
      var four = "4";
    };
    Prim.debugPrint(debug_show { migration = { old; new } });
    Prim.debugPrint(debug_show ("Migration2"));
    new;
  }

};
