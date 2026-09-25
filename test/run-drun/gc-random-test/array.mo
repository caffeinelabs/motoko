// Source: Motoko Base Library

import Prim "mo:prim";

module {
  public func freeze<X>(varArray : [var X]) : [X] = Prim.Array_tabulate<X>(varArray.size(), func i = varArray[i]);

  public func filter<X>(array : [X], predicate : X -> Bool) : [X] {
    var count = 0;
    let keep = Prim.Array_tabulate<Bool>(
      array.size(),
      func i {
        if predicate(array[i]) {
          count += 1;
          true;
        } else {
          false;
        };
      },
    );
    var nextKeep = 0;
    Prim.Array_tabulate<X>(
      count,
      func _ {
        while not keep[nextKeep] {
          nextKeep += 1;
        };
        nextKeep += 1;
        array[nextKeep - 1];
      },
    );
  };
};
