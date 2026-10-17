import { principalOfActor; error } = "mo:⛔";

// Regression for #5909: a reply that traps before a waiter is registered
// must not leave that waiter suspended forever.
actor A {
  var cleaned = 0;
  var caught = false;
  var resumed = false;
  var entered = false;
  let concurrentDone : [var Nat] = [var 0, 0, 0, 0, 0, 0];
  var concurrentResumed = false;

  public query func reply() : async { x2 : Nat } { { x2 = 0 } };

  func run(late : Bool, short : Bool) : async* () {
    cleaned := 0;
    caught := false;
    resumed := false;
    entered := false;
    let remote = actor (debug_show (principalOfActor A)) : actor {
      reply : query () -> async { x : Nat };
    };
    let fut = remote.reply();
    if late {
      // Let the reply callback run before entering the try/await below.
      var n = 0;
      while (n < 5) { await async {}; n += 1 };
    };
    try {
      try {
        entered := true;
        if short { ignore await? fut } else { ignore await fut };
        resumed := true;
      } catch _ {
        caught := true;
      } finally {
        cleaned += 1;
      };
      resumed := true;
    } finally {
      cleaned += 1;
    };
    resumed := true;
  };

  public func early() : async () { await* run(false, false) };
  public func late() : async () { await* run(true, false) };
  public func earlyShort() : async () { await* run(false, true) };
  public func lateShort() : async () { await* run(true, true) };

  public query func check() : async () {
    for (count in concurrentDone.values()) { assert count == 1 };
    assert not concurrentResumed;
    assert entered;
    assert cleaned == 2;
    assert not caught;
    assert not resumed;
  };

  // Start all three calls before awaiting any of them. Test a decoding trap
  // at each position, after the replies have had time to arrive. Each case
  // records completion only after both finally clauses ran in order.
  func concurrent(bad : Nat, short : Bool, index : Nat) : async* () {
    let remote = actor (debug_show (principalOfActor A)) : actor {
      reply : query () -> async { x : Nat };
    };
    let a : async Any = if (bad == 0) remote.reply() else A.reply();
    let b : async Any = if (bad == 1) remote.reply() else A.reply();
    let c : async Any = if (bad == 2) remote.reply() else A.reply();
    var n = 0;
    while (n < 5) { await async {}; n += 1 };
    var completed = 0;
    var innerCleaned = false;
    try {
      try {
        for (fut in [a, b, c].values()) {
          if short { ignore await? fut } else { ignore await fut };
          completed += 1;
        };
        concurrentResumed := true;
      } catch _ {
        concurrentResumed := true;
      } finally {
        assert not innerCleaned;
        assert completed == bad;
        innerCleaned := true;
      };
      concurrentResumed := true;
    } finally {
      assert innerCleaned;
      assert completed == bad;
      assert not concurrentResumed;
      concurrentDone[index] += 1;
    };
    concurrentResumed := true;
  };

  public func first() : async () { await* concurrent(0, false, 0) };
  public func middle() : async () { await* concurrent(1, false, 1) };
  public func last() : async () { await* concurrent(2, false, 2) };
  public func firstShort() : async () { await* concurrent(0, true, 3) };
  public func middleShort() : async () { await* concurrent(1, true, 4) };
  public func lastShort() : async () { await* concurrent(2, true, 5) };

  public func unawaited() : async () {
    let remote = actor (debug_show (principalOfActor A)) : actor {
      reply : query () -> async { x : Nat };
    };
    let before = cleaned;
    ignore remote.reply();
    var n = 0;
    while (n < 5) { await async {}; n += 1 };
    assert cleaned == before;
  };

  // Successful and rejected futures still resume normally after settling.
  public func settled() : async () {
    let ok = A.reply();
    let err : async () = async { throw error "expected" };
    var n = 0;
    while (n < 5) { await async {}; n += 1 };
    assert (await ok).x2 == 0;
    assert (await? ok).x2 == 0;
    var catches = 0;
    try { await err } catch _ { catches += 1 };
    try { await? err } catch _ { catches += 1 };
    assert catches == 2;

    // All-success control: starting three calls together does not trigger
    // cleanup until all three sequential awaits have resumed.
    for (short in [false, true].values()) {
      let a = A.reply();
      let b = A.reply();
      let c = A.reply();
      var completed = 0;
      var cleanups = 0;
      try {
        for (fut in [a, b, c].values()) {
          let value = if short { await? fut } else { await fut };
          assert value.x2 == 0;
          completed += 1;
          assert cleanups == 0;
        };
      } finally {
        assert completed == 3;
        cleanups += 1;
      };
      assert cleanups == 1;
    };
  };
};
