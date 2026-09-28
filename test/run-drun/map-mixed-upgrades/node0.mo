import Prim "mo:⛔";
actor class Node(i : Nat) {

  var upgrades = 0;

  type Key = Nat;
  type Value = Text;

  type List = ?(Key, Value, List);

  var map : List = null;

  Prim.debugPrint(debug_show { node = i; upgrades = upgrades; state = map });

  public func lookup(k : Key) : async ? Value {
    Prim.debugPrint(debug_show i # ": lookup " # debug_show k);
    var m = map;
    loop {
      switch m {
        case ?(k1, v, m1) {
          if k == k1 { return ?v }
          else {
            m := m1;
          };
        }
        case null {
          return null;
        }
      }
    }
  };

  system func preupgrade () {
    upgrades += 1;
  };

  public func insert(k : Key, v : Value) : async () {
    Prim.debugPrint(debug_show i # ": insert " # debug_show (k, v));
    map := ?(k, v, map);
  };

};
