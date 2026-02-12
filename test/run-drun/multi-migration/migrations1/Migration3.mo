import Prim "mo:prim";

module {

  public func run(old : { var zero : Nat; var three : [var (Nat, Text)]; var four : Text }) : {
    var zero : Nat;
    var three : [var (Nat, Text)];
    var four : Text;
    var five : Text;
  } {
    let new = {
      var zero = old.zero;
      var three = old.three;
      var four = old.four;
      var five = "5";
    };
    Prim.debugPrint(debug_show { migration = { old; new } });
    Prim.debugPrint(debug_show "Migration3");
    new;
  }

};
