import { principalOfActor; cyclesBurn; cyclesBalance } = "mo:⛔";

// A late `await` of a future whose reply trapped must terminate, running only
// `finally`, even if the self-call that schedules the cleanup can't be sent.
actor Self {
  let ic = actor "aaaaa-aa" : actor {
    update_settings : {
      canister_id : Principal;
      settings : { freezing_threshold : ?Nat }
    } -> async ()
  };

  var cleaned = 0;
  var caught = false;
  var resumed = false;

  public query func reply() : async { x2 : Nat } { { x2 = 0 } };

  // without a freezing reserve, burning the balance leaves nothing to send with
  public func setup() : async () {
    await ic.update_settings({
      canister_id = principalOfActor Self;
      settings = { freezing_threshold = ?0 }
    })
  };

  public func go() : async () {
    let remote = actor (debug_show (principalOfActor Self)) : actor {
      reply : query () -> async { x : Nat }
    };
    let fut = remote.reply();
    var n = 0;
    while (n < 5) { await async {}; n += 1 }; // the reply traps before the await
    ignore cyclesBurn<system>(cyclesBalance());
    try {
      ignore await fut;
      resumed := true
    } catch _ {
      caught := true
    } finally {
      cleaned += 1
    };
    resumed := true
  };

  public query func check() : async () {
    assert cleaned == 1;
    assert not caught;
    assert not resumed
  };
};
