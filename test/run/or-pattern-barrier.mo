import Prim "mo:⛔";

// `v` is `Null` in the left arm and `?Text` in the right one (no warning,
// `Null <: ?Text`). The closure `get` captures `v` before its definition,
// so `v` lives in a MutBox. Storing a `?Text` there is a pointer write and
// must go through the write barrier, whichever arm matched.

func h(x : { #a : Null; #b : ?Text }) : ?Text {
  func get() : ?Text { v };
  let (#a v) or (#b v) = x;
  get()
};

assert h(#a null) == null;
assert h(#b (?"hi")) == ?"hi";
Prim.debugPrint("ok")

// CHECK-LABEL: (func $h
// CHECK: call $write_with_barrier
// CHECK: call $write_with_barrier
// CHECK-LABEL: (func $get
