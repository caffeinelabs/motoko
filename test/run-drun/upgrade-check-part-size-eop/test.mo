import Prim "mo:prim";

persistent actor {
  let _an_array = Prim.Array_init<Nat8>(1024 * 10, 0);
  public func test() : async () {
    Prim.debugPrint(debug_show {mem = Prim.rts_memory_size() / (1024 * 1024); heap = Prim.rts_heap_size() / (1024 * 1024)});
  };
};

