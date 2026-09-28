import Prim "mo:⛔";
actor class Node(i : Nat) {

  type Key = Nat;
  type Value = Text;

  Prim.debugPrint("installing node" # debug_show i);
  transient var map = func (_k: Nat) : ?Value { return null };

  public func lookup(k : Key) : async ? Value {
    Prim.debugPrint(debug_show i # ": lookup " # debug_show k);
    return map(k);
  };

  public func insert(k : Key, v : Value) : async () {
    Prim.debugPrint(debug_show i # ": insert " # debug_show (k,v));
    let map0 = map;
    map := func(k1 : Nat) : ?Value { if (k1 == k) ?v else map0(k1); };
  };

};

