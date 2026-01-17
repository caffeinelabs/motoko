import Prim "mo:prim";

module {

  public func run(old : { var zero : Nat; var three : [var (Nat, Text)]; var four : Text; var five : Text; var six : Text }) : {
    var zero : Nat;
    var three : [var (Nat, Text)];
    var four : Text;
    var five : Text;
    var six : Text;
  } {
    Prim.debugPrint(debug_show "Migration6");
    old;
  }

};
