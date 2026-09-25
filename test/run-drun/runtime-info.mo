import Prim "mo:prim";
import Info "runtime-info/info";
actor Self {
  var array : [var Nat] = [var];
  let region = Prim.regionNew();
  ignore Prim.regionGrow(region, 1);

  public func increaseHeap() : async () {
    array := Prim.Array_init<Nat>(array.size() + 1024 * 1024, 0);
  };

  func validGC(strategy : Text) : Bool {
    for name in ["copying", "compacting", "generational", "incremental", "default"].values() {
      if strategy == name # " force" {
        return true;
      };
    };
    return false;
  };

  public func checkInformation() : async () {
    let information = await Info.introspect(Self).__motoko_runtime_information();
    Prim.debugPrint("Ignore Diff: " # debug_show (information));
    // Runtime information differs between GCs and sanity check options.
    assert (validGC(information.garbageCollector));
    assert (information.heapSize > array.size() * 4); // or 8 with 64-bit
    assert (information.heapSize < information.memorySize);
    assert (information.maxStackSize > 1024);
    assert (information.maxStackSize % 1024 == 0);
    assert (information.totalAllocation > 0);
    assert (information.stableMemorySize > 0);
    assert (information.logicalStableMemorySize == information.stableMemorySize);
    assert (information.callbackTableCount <= information.callbackTableSize);
  };
};

// Not calling query __motoko_runtime_information "DIDL\x00\x00" as it differs
// between the compiler options and changes with every compiler/RTS adjustment.

//SKIP run
//SKIP run-low
//SKIP run-ir
//CALL ingress checkInformation "DIDL\x00\x00"
//CALL ingress increaseHeap "DIDL\x00\x00"
//CALL ingress checkInformation "DIDL\x00\x00"
//CALL ingress increaseHeap "DIDL\x00\x00"
//CALL ingress checkInformation "DIDL\x00\x00"
