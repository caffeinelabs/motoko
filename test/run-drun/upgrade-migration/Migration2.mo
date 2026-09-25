import Prim "mo:prim";

module {

  public func run(old : { var three : [var (Nat, Text)] }) :
      { var four : [var (Nat, Text)] } {
    let new =
    {
      var four = old.three;
    };
    Prim.debugPrint(debug_show { migration = { old; new } });
    new
  }

}
