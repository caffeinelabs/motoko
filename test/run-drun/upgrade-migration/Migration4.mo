import Prim "mo:prim";

module {

  func extend<T, U, V>(tus : [var (T, U)], v: V) : [var (T, U, V)] {
    let size = tus.size();
    if size == 0 { return [var] };
    let res = Prim.Array_init<(T, U, V)>(size, (tus[0].0, tus[0].1, v));
    for i in res.keys() {
      res[i] := (tus[i].0, tus[i].1, v);
    };
    res
  };

  public func run(old : { var four : [var (Text, Nat)] }) :
      { var four : [var (Text, Nat, Bool)] } {
    let new =
    {
      var four = extend<Text, Nat, Bool>(old.four, false);
    };
    Prim.debugPrint(debug_show { migration = { old; new } });
    new
  }

}
