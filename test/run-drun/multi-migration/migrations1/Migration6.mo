import Prim "mo:prim";

module {

  public func migration(old : { zero : Nat; var three : [var (Nat, Text)]; var four : Text; var five : Text; var six : Text }) : {
    zero : [Nat];
    var three : [var (Nat, Text)];
    var four : Text;
    var five : Text;
    var six : [Text];
  } {
    Prim.debugPrint(debug_show "Migration6");
    {
      zero = [old.zero];
      var three = old.three;
      var four = old.four;
      var five = old.five;
      var six = [old.six];
    };
  }

};
