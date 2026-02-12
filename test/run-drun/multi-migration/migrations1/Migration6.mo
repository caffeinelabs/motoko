import Prim "mo:prim";

module {

  public func run(old : { zero : Nat; var three : [var (Nat, Text)]; var four : Text; var five : Text; var six : Text }) : {
    zero : Nat;
    var three : [var (Nat, Text)];
    var four : Text;
    var five : Text;
    var six : Text;
  } {
    Prim.debugPrint(debug_show "Migration6");
    old;
  }

};
