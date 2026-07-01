//SKIP comp
// Exercise compare_typ Async arm via async/await patterns.
// compare_typ (Async ...) (Async ...) fires when LUB/GLB of async types
// is computed (e.g., in an if-then-else where both branches are async).
// Also exercises compare_async_sort.

persistent actor A {
  // Async arm: LUB of two async expressions with same type.
  // The if-then-else here forces LUB computation of async Nat, async Nat.
  public func getAsync(b : Bool) : async Nat {
    if b { await async 1 } else { await async 2 }
  };

  // Async arm with different scope types (nested async in actor)
  public func nested() : async () {
    ignore await async 42;
    ignore await async "hello";
  };
};
