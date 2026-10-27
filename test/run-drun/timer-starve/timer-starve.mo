import Prim "mo:⛔";

// Default timers survive cycle starvation: a timer self-call that is sent,
// but rejected before it runs (out of cycles), is retried once cycles are back.
actor Self {
  let ic = actor "aaaaa-aa" : actor {
    update_settings : {
      canister_id : Principal;
      settings : { freezing_threshold : ?Nat }
    } -> async ()
  };

  var ticks = 0;
  var recurring = 0;

  func burnAll<system>() {
    ignore Prim.cyclesBurn<system>(Prim.cyclesBalance())
  };

  public func start() : async () {
    recurring := Prim.setTimer<system>(1_000_000_000, true, func() : async () { ticks += 1 })
  };

  public func burn() : async () { burnAll<system>() };

  // Two one-shot jobs due together: the first starves the second. Without
  // a freezing reserve, the burn leaves no cycles for the second to run on.
  public func starve() : async () {
    Prim.cancelTimer(recurring);
    ticks := 0;
    await ic.update_settings({
      canister_id = Prim.principalOfActor(Self);
      settings = { freezing_threshold = ?0 }
    });
    ignore Prim.setTimer<system>(1_000_000_000, false, func() : async () { burnAll<system>() });
    ignore Prim.setTimer<system>(1_000_000_000, false, func() : async () { ticks += 1 })
  };

  // A recurring job starved the same way is not retried: like an overdue run,
  // the missed run is skipped.
  public func starveRecurring() : async () {
    ticks := 0;
    ignore Prim.setTimer<system>(5_000_000_000, false, func() : async () { burnAll<system>() });
    recurring := Prim.setTimer<system>(5_000_000_000, true, func() : async () { ticks += 1 })
  };

  public query func count() : async Nat { ticks }
}
