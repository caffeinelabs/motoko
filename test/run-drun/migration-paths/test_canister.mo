import Prim "mo:prim";

actor class TestCanister() {
  let length = 8 * 1024 * 1024;
  func initialize() : [var Nat] {
    let array = Prim.Array_init<Nat>(length, 0xfff_ffff);
    Prim.debugPrint("array initialized");
    array;
  };

  var array = initialize();

  var version = 0;
  version += 1;

  assert (array.size() == length);
  Prim.debugPrint("version: " # debug_show (version));
};
