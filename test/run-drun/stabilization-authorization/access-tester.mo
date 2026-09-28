import Prim "mo:⛔";

actor class AccessTester(
  other : actor {
    __motoko_stabilize_before_upgrade : () -> async ();
    __motoko_destabilize_after_upgrade : () -> async ();
  }
) {
  public func test() : async () {
    Prim.debugPrint("Test __motoko_stabilize_before_upgrade");
    try {
      await other.__motoko_stabilize_before_upgrade();
    } catch (e) {
      Prim.debugPrint(Prim.errorMessage(e));
    };
    try {
      Prim.debugPrint("Test __motoko_stabilize_after_upgrade");
      await other.__motoko_destabilize_after_upgrade();
    } catch (e) {
      Prim.debugPrint(Prim.errorMessage(e));
    };
  };
};
