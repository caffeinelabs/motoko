import Prim "mo:⛔";

// Tests that a trap in the response callback of a call (here: a Candid
// decoding error of the reply) runs the `finally` blocks of the awaiting
// code regardless of whether that await was already waiting when the
// response arrived or only awaits the future later.
actor Self {

  type T = { x : Nat };
  type T2 = { x2 : Nat };

  public query func foo() : async T2 { { x2 = 0 } };

  // a view of ourselves with a wrong (undecodable) return type for `foo`
  func remote() : actor { foo : query () -> async T } =
    actor (debug_show (Prim.principalOfActor Self));

  var cleaned = 0;

  // the response arrives while `await` is already waiting
  public func waiter() : async () {
    let fut = remote().foo();
    try {
      ignore await fut;
      Prim.debugPrint "waiter: unreachable";
    } finally {
      Prim.debugPrint "waiter: finally";
      cleaned += 1;
    };
  };

  // the response arrives before anyone awaits it
  public func no_waiter() : async () {
    let fut = remote().foo();
    await async {}; // let the response arrive
    try {
      Prim.debugPrint "no_waiter: try";
      ignore await fut;
      Prim.debugPrint "no_waiter: unreachable";
    } finally {
      Prim.debugPrint "no_waiter: finally";
      cleaned += 1;
    };
  };

  // same, with `await?`
  public func no_waiter_short() : async () {
    let fut = remote().foo();
    await async {}; // let the response arrive
    try {
      Prim.debugPrint "no_waiter_short: try";
      ignore await? fut;
      Prim.debugPrint "no_waiter_short: unreachable";
    } finally {
      Prim.debugPrint "no_waiter_short: finally";
      cleaned += 1;
    };
  };

  public query func show() : async () {
    Prim.debugPrint(debug_show { cleaned });
  };

};

//SKIP run
//SKIP run-ir
//SKIP run-low

//CALL ingress waiter "DIDL\x00\x00"
//CALL query show "DIDL\x00\x00"
//CALL ingress no_waiter "DIDL\x00\x00"
//CALL query show "DIDL\x00\x00"
//CALL ingress no_waiter_short "DIDL\x00\x00"
//CALL query show "DIDL\x00\x00"
