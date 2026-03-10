import Prim "mo:⛔";
actor a {
  public func go() : () {
    ignore(Prim.Array_init<()>(1200*1024/4, ()));
  };
};

a.go(); //OR-CALL ingress go 0x4449444C0000
a.go(); //OR-CALL ingress go 0x4449444C0000
