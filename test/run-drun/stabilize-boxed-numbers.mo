import Prim "mo:prim";

actor {
  let boxedNat32 : Nat32 = 4294967295;
  let boxedInt32 : Int32 = -2147483648;
  let boxedNat64 : Nat64 = 18446744073709551614;
  let boxedInt64 : Int64 = -9223372036854775808;

  public func print() : async () {
    Prim.debugPrint(debug_show (boxedNat32));
    Prim.debugPrint(debug_show (boxedInt32));
    Prim.debugPrint(debug_show (boxedNat64));
    Prim.debugPrint(debug_show (boxedInt64));
  };
};

//CALL ingress print "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress print "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress print "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress print "DIDL\x00\x00"
