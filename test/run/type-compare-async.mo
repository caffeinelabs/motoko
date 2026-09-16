//SKIP comp
// compare_typ Async arm (and compare_async_sort) via LUB of async expressions.

persistent actor A {
  public func getAsync(b : Bool) : async Nat {
    if b { await async 1 } else { await async 2 }
  };
  public func nested() : async () {
    ignore await async 42;
    ignore await async "hello";
  };
};
