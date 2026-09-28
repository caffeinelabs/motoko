import Prim "mo:⛔";
actor this {

  var version = 0;
  Prim.debugPrint (debug_show({ version = version }));

  system func postupgrade() {
    version += 1;
  };

  public query func check(ver : Int) : async () {
    assert version == ver;
  };

  var n = 0;
  var done = true;

  public func yield() : async () {
    if not done and n < 40 {
      n += 1;
      await yield();
      n -= 1;
    }
  };

  public func repeat() : () {
    done := false;
    ignore yield();
  };

  public func wait() : async () {
    done := true;
    while n > 0 { await async {} };
  };

}
