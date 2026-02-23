//MOC-FLAG -A=M0194
import P "mo:⛔";
import StableMemory "stable-mem/StableMemory";

actor {

  stable var n : Nat64 = 0;
  stable let a0 = P.Array_init<Nat>(65536, 1000);
  stable let a1 = P.Array_init<Nat>(65536, 1000);
  stable let b = [a0, a1];
  stable let c = [b, b];

  system func preupgrade() {
    P.debugPrint("upgrading from " # debug_show n);
    let m = StableMemory.grow(1);

    // check all pages clear
    var i : Nat64 = 0;
    let max = StableMemory.size() * 65536;
    while (i < max) {
      assert (StableMemory.loadNat32(i) == 0);
      i += 4
    };

    n += 1;
    a0[42] := 42;
    a1[25] := 25;
  };

  public func testGrow() : async () {
    var i : Nat64 = 0;
    while (i < 10) {
      var pre = StableMemory.size();
      var post = StableMemory.grow(i);
      assert StableMemory.size() == StableMemory.grow(0);
      assert post == pre;
      assert StableMemory.size() == pre + i;
      i += 1;
    }
  };

  system func postupgrade() {
    assert a0[42] == 42;
    assert a1[25] == 25;
    P.debugPrint("to " # debug_show n);
  };

}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL upgrade ""
//CALL ingress testGrow "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress testGrow "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress testGrow "DIDL\x00\x00"
//CALL upgrade ""
