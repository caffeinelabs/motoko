import Prim "mo:prim";

// Incompatible upgrade
actor {
  type TreeType = ?(Nat, TreeType, TreeType);

  var unit = ();
  //stable var pair = (1, 2);
  var pair = (1);
  var largerTuple = (0, "Test", 1.23, { key = 5; value = '_' }, [-1, 2, -3]);
  var tree: TreeType = ?(2, ?(1, null: TreeType, null: TreeType), ?(4, ?(3, null: TreeType, null: TreeType), ?(5, null: TreeType, null: TreeType)));

  public func print() : async () {
    Prim.debugPrint(debug_show (pair));
  };
};
