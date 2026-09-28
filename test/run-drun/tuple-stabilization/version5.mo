import Prim "mo:prim";

// Incompatible upgrade
actor {
  //type TreeType = ?(Nat, TreeType, TreeType);
  type TreeType = ?(Nat, TreeType, Null);

  var unit = ();
  var pair = (1, 2);
  var largerTuple = (0, "Test", 1.23, { key = 5; value = '_' }, [-1, 2, -3]);

  //stable var tree: TreeType = ?(2, ?(1, null: TreeType, null: TreeType), ?(4, ?(3, null: TreeType, null: TreeType), ?(5, null: TreeType, null: TreeType)));
  var tree: TreeType = ?(2, ?(1, null: TreeType, null), null);

  public func print() : async () {
    Prim.debugPrint(debug_show (pair));
    Prim.debugPrint(debug_show (largerTuple));
    Prim.debugPrint(debug_show (tree));
  };
};
