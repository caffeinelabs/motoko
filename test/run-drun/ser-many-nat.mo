//MOC-ENV MOC_UNLOCK_PRIM=yesplease
import Prim "mo:⛔";

actor {
  func serNat(x: Nat) : Blob = (prim "serialize" : Nat -> Blob) x;
  func deserNat(x: Blob) : Nat = (prim "deserialize" : Blob -> Nat) x;

  var n = Prim.nat64ToNat(1<<32);
  var c = n;
  public func go() : async () {
    while n > 0 {
      if n != deserNat(serNat(n)) {
        Prim.debugPrint(debug_show { failure = n });
      };
      if n > 791197 { n -= 791197; } else { return; };
      c -= 1;
      if c % 1024 == 0 {
        await async (); // trigger gc
      }
    }
  }
}
//SKIP run
//SKIP run-ir
//SKIP run-low
//CALL ingress go "DIDL\x00\x00"
