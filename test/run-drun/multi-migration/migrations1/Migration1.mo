import Prim "mo:prim";

module {

  func zip<T, U>(ts : [var T], us : [var U]) : [var (T, U)] {
    let size = if (ts.size() <= us.size()) ts.size() else us.size();
    if (size == 0) return [var];
    let res = Prim.Array_init<(T, U)>(size, (ts[0], us[0]));
    for (i in res.keys()) {
      res[i] := (ts[i], us[i]);
    };
    res;
  };

  public func run(
    old : {
      var zero : Nat;
      var one : [var Nat];
      var two : [var Text];
    }
  ) : { var zero : Nat; var three : [var (Nat, Text)] } {
    let new = {
      var zero = old.zero;
      var three = zip(old.one, old.two);
    };
    Prim.debugPrint(debug_show { migration = { old; new } });
    Prim.debugPrint(debug_show ("Migration1"));
    new;
  }

};
