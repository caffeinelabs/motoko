//MOC-ENV MOC_UNLOCK_PRIM=yesplease
import Prim "mo:⛔";

actor {
  func serInt(x: Int) : Blob = (prim "serialize" : Int -> Blob) x;
  func deserInt(x: Blob) : Int = (prim "deserialize" : Blob -> Int) x;

  var n = Prim.int64ToInt(1<<32);
  let l = -n;
  var c = n;
  public func go() : async () {
    while n > l {
      if n != deserInt(serInt(n)) {
        Prim.debugPrint(debug_show { failure = n });
      };
      n -= 104729;
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
