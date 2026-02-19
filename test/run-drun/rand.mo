//MOC-FLAG -A=M0194
import Prim "mo:⛔";

actor a {

  let rand = (actor "aaaaa-aa" : actor { raw_rand : () -> async Blob }).raw_rand;

  public func go() : async () {
    let t1 = Prim.time();
    let b1 = await rand();
    let t2 = Prim.time();
    let b2 = await rand();
//    Prim.debugPrint(debug_show {t1;b1;t2;b2});
    assert b1.size() == 32;
    assert b2.size() == 32;
//    assert b1 != b2 // might fail, but not likely except on drun!
  };
};
await a.go(); //OR-CALL ingress go "DIDL\x00\x00"
