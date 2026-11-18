//MOC-ENV MOC_UNLOCK_PRIM=yesplease
import { debugPrint; setTimer; cancelTimer } = "mo:⛔";

// A job whose dispatch failed is retried under its own id: `cancelTimer` with
// that id stops the retry, and `cancelTimer(0)` reaches nothing.
actor {
  var a = 0;
  var b = 0;

  let hour : Nat64 = 3_600_000_000_000;
  let raw_rand = (actor "aaaaa-aa" : actor { raw_rand : () -> async Blob }).raw_rand;

  public shared func go() : async () {
    // real ids whose nodes are gone, as after a failed dispatch
    let ida = setTimer<system>(hour, false, func() : async () {});
    let idb = setTimer<system>(hour, false, func() : async () {});
    cancelTimer ida;
    cancelTimer idb;
    @retry(ida, func() : async () { a += 1 });
    @retry(idb, func() : async () { b += 1 });

    cancelTimer ida;
    cancelTimer 0;

    var rounds = 0;
    while rounds < 20 { ignore await raw_rand(); rounds += 1 };
    debugPrint(debug_show { a; b })
  }
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress go "DIDL\x00\x00"
