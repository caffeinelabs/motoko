//MOC-FLAG --enhanced-orthogonal-persistence
import Prim "mo:⛔";

// An element count that could never be allocated must trap, rather than wrap the
// byte size the RTS computes from it: `(len + 3) * 8` used to wrap to 8, 16 or 24
// bytes for lengths in `2^61-2 .. 2^61`, handing out an array whose header claims
// ~2^61 elements — every bounds-checked access to it was out of bounds, and the
// incremental GC would never finish marking it.

persistent actor {

  func expectTrap(what : Text, n : Nat) : async () {
    try {
      await async { ignore Prim.Array_init<Nat>(n, 0) };
      assert false
    }
    catch e {
      Prim.debugPrint(what # ": " # Prim.errorMessage(e));
    }
  };

  public func go () : async () {
    await expectTrap("2^61 - 2", 2305843009213693950);
    await expectTrap("2^61 - 1", 2305843009213693951);
    await expectTrap("2^61", 2305843009213693952);
    await expectTrap("2^48", 281474976710656);
    // still allocatable
    let a = Prim.Array_init<Nat>(3, 7);
    Prim.debugPrint("small array: " # debug_show (a.size(), a[0]));
  }
}
//SKIP run
//SKIP run-ir
//SKIP run-low
//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//CALL ingress go "DIDL\x00\x00"
