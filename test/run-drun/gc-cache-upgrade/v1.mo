import Prim "mo:prim";

// Keeps a heap big enough that an incremental GC cycle spans several messages,
// so an upgrade lands with `state.phase() != Pause`. The persisted phase must
// re-seat the backend `__running_gc` cache, since the fresh module's globals
// start at zero — otherwise the barriers below take the paused fast path while
// the GC is marking. Under `--sanity-checks` the RTS asserts the cache against
// the authoritative phase on every barrier, which is what gives this teeth.

persistent actor {

  var live : [var [var Nat]] = Prim.Array_init<[var Nat]>(192, Prim.Array_init<Nat>(0, 0));

  public func build() : async () {
    for (i in live.keys()) {
      live[i] := Prim.Array_init<Nat>(64 * 1024, i);
    };
  };

  // Pointer writes (write barrier) plus fresh allocations (allocation barrier).
  public func churn() : async () {
    for (i in live.keys()) {
      live[i] := Prim.Array_init<Nat>(16 * 1024, i);
    };
    assert (live.size() == 192);
  };

  public func size() : async Nat = async live.size();
}
