import Prim "mo:prim";

actor {
  var nullVariable = null;
  var boolVariable = false;
  var natVariable = 1234567890123456789012345678901234567890;
  var nat8Variable = 123: Nat8;
  var nat16Variable = 12345: Nat16;
  var nat32Variable = 1234567890: Nat32;
  var nat64Variable = 123456789012345: Nat64;
  let intVariable = -1234567890123456789012345678901234567890;
  let int8Variable = -123: Int8;
  let int16Variable = -12345: Int16;
  let int32Variable = -1234567890: Int32;
  let int64Variable = -123456789012345: Int64;
  var floatVariable = 1.234567890;
  var charVariable = '!';
  var textVariable = "\"TEST\"";
  let blobVariable = "abc": Blob;
  let principalVariable = Prim.principalOfBlob ("\00\00\00\00\00\30\00\D3\01\01": Blob);

  Prim.debugPrint("Initialized");

  public func increase() : async () {
    nullVariable := null;
    boolVariable := not boolVariable;
    natVariable += 1;
    nat8Variable += 1;
    nat16Variable += 1;
    nat32Variable += 1;
    nat64Variable += 1;
    floatVariable += 0.1;
    charVariable := if charVariable == '!' { '?' } else { '!' };
    textVariable #= " \"TEST\"";
  };

  public func print() : async () {
    Prim.debugPrint(debug_show(nullVariable));
    Prim.debugPrint(debug_show(boolVariable));
    Prim.debugPrint(debug_show(natVariable));
    Prim.debugPrint(debug_show(nat8Variable));
    Prim.debugPrint(debug_show(nat16Variable));
    Prim.debugPrint(debug_show(nat32Variable));
    Prim.debugPrint(debug_show(nat64Variable));
    Prim.debugPrint(debug_show(intVariable));
    Prim.debugPrint(debug_show(int8Variable));
    Prim.debugPrint(debug_show(int16Variable));
    Prim.debugPrint(debug_show(int32Variable));
    Prim.debugPrint(debug_show(int64Variable));
    Prim.debugPrint(debug_show(floatVariable));
    Prim.debugPrint(debug_show(charVariable));
    Prim.debugPrint(debug_show(textVariable));
    Prim.debugPrint(debug_show(blobVariable));
    Prim.debugPrint(debug_show(principalVariable));
  };
}

//SKIP run
//SKIP run-low
//SKIP run-ir
//CALL ingress print "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress increase "DIDL\x00\x00"
//CALL ingress print "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress increase "DIDL\x00\x00"
//CALL ingress print "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress print "DIDL\x00\x00"
