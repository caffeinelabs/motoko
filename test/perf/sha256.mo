//MOC-FLAG -A=M0194
import SHA256 "./sha256/SHA256";
import Array "./sha256/Array";
import Nat8 "./sha256/Nat8";
import Prim "mo:⛔";
actor {
  public func go() : async () {
    // 256kb already exceed the instruction limit
    ignore SHA256.sha256(Array.tabulate(48 * 1024, Nat8.fromIntWrap));
  };

  public func getPerfData() : async () {
    Prim.debugPrint("instructions: " # debug_show (Prim.rts_lifetime_instructions()));
  };
};
//CALL ingress go 0x4449444C0000
//CALL ingress getPerfData 0x4449444C0000
