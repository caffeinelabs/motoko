//MOC-FLAG --max-stable-pages 65536
import StableMemory "stable-mem/StableMemory";

actor {
  let max : Nat64 = 65536;
  public query func testGrow() : async () {
    assert 0  == StableMemory.grow(max - 1);
    assert (max - 1) == StableMemory.grow(1);
    assert max == StableMemory.size();
    assert max == StableMemory.grow(0);
    assert 0xFFFF_FFFF_FFFF_FFFF == StableMemory.grow(1);
    assert 0xFFFF_FFFF_FFFF_FFFF == StableMemory.grow(1024);
  }
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL query testGrow "DIDL\x00\x00"


