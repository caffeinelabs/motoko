import Prim "mo:prim";

actor class TestActor() {
  var version = 0;
  version += 1;
  var trapOnPreUpgrade = false;

  public query func getVersion() : async Nat {
    version;
  };

  public func breakNextUpgrade() : async () {
    trapOnPreUpgrade := true;
  };

  system func preupgrade() {
    if trapOnPreUpgrade {
      Prim.trap("Trap on pre-upgrade");
    };
  };
};
