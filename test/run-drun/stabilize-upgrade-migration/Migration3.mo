import Prim "mo:prim";

module {

  func swap<T, U>(tus : [var (T, U)]) : [var (U, T)] {
    let size = tus.size();
    if size == 0 { return [var] };
    let res = Prim.Array_init<(U, T)>(size, (tus[0].1, tus[0].0));
    for i in res.keys() {
      res[i] := (tus[i].1, tus[i].0);
    };
    res
  };

  public func run(old : { var four : [var (Nat, Text)] }) :
      { var four : [var (Text, Nat)] } {
    let new =
    {
      var four = swap(old.four);
    };
    Prim.debugPrint(debug_show { migration = { old; new } });
    new
  }

}
