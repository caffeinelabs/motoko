import Prim "mo:⛔";
actor a {
  public func sendi(n : Int) : async () {
    Prim.debugPrint(debug_show ("sendi", n));
  };
  public func sendn(n : Nat) : async () {
    Prim.debugPrint(debug_show ("sendn", n));
  };
  public func sendnn(n1 : Nat, n2 : Nat) : async () {
    Prim.debugPrint(debug_show ("sendnn", n1, n2));
  };
  public func sendnested(n1 : Nat, (n2 : Nat, n3 : Nat)) : async () {
    Prim.debugPrint(debug_show ("sendnested", n1, n2, n3));
  };
  public func sendarray(a : [Nat]) : async () {
    Prim.debugPrint(debug_show ("sendarray", a))
  };
  public func sendopt(a : ?Nat) : async () {
    Prim.debugPrint(debug_show ("sendopt", a))
  };
  public func sendrecord(a : { x : Nat; y : Nat }) : async () {
    Prim.debugPrint(debug_show ("sendrecord", a))
  };
  public func sendVariant(v : { #foo : Nat; #bar : Nat }) : async () {
    Prim.debugPrint(debug_show ("sendVariant", v))
  };
  public func sendLabeled(l:Text) : async () {
    Prim.debugPrint(debug_show ("sendLabeled", l))
  };
  public func sendLabeledOpt(lo:?Text) : async () {
    Prim.debugPrint(debug_show ("sendLabeledOpt", lo))
  };
  public func sendnats(n8 : Nat8, n16 : Nat16, n32 : Nat32, n64 : Nat64) : async () {
    Prim.debugPrint(debug_show ("sendnats", n8, n16, n32, n64))
  };
  public func sendints(i8 : Int8, i16 : Int16, i32 : Int32, i64 : Int64) : async () {
    Prim.debugPrint(debug_show ("sendints", i8, i16, i32, i64))
  };
  public func sendfloat(f64 : Float) : async () {
    Prim.debugPrint(debug_show ("sendfloat", f64))
  };

  public func go() : async () {
    await a.sendn(0);
    await a.sendn(1);
    await a.sendn(2);
    await a.sendn(3);
    await a.sendn(4);
    await a.sendn(1000);
    await a.sendnn(5, 6);
    await a.sendnn(2000, 3000);
    await a.sendnested(7, (8, 9));
    await a.sendarray([10, 11, 12, 13]);
    await a.sendopt(null);
    await a.sendopt(?14);
    await a.sendrecord({ x = 15; y = 16 });
    await a.sendrecord({ x = 17; y = 18; z = 19 });
    await a.sendVariant(#foo 20);
    await a.sendVariant(#bar 20);
    await a.sendLabeled("Foo1: ");
    await a.sendLabeled("μεταγράψτε: ");
    await a.sendLabeledOpt(?"Foo2: ");
    await a.sendn(10000000000000);
    await a.sendi(10000000000000);
    await a.sendi(-20000000000000);
    await a.sendnats(1, 2, 3, 4);
    await a.sendints(1, 2, 3, 4);
    await a.sendints(-1, -2, -3, -4);
    await a.sendn(2**100);
    await a.sendi(2**100);
    await a.sendi(-(2**101));
    await a.sendi(-2**30); // lowest compact
    await a.sendi(2**30 - 1); // highest compact
    await a.sendi(0);
    await a.sendi(1);
    await a.sendi(42);
    await a.sendi(-42);
    await a.sendn(2**31 - 1); // highest compact
    await a.sendfloat(42.0);
  };
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
