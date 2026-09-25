import Prim "mo:⛔";

actor a {

  let bound : Int = 100000;

  func Loop1(n : Int) {
    if n >= bound {
      Prim.debugPrint "done";
      return;
    };
    Loop1(n + 1);
  };

  public query func test_loop1() : async () {
    Loop1(0);
  };

  system func preupgrade() {
    func preupgrade_loop(n : Int) {
      if n >= bound {
        Prim.debugPrint "preupgrade_loop done";
        return;
      };
      preupgrade_loop(n + 1);
    };

    Prim.debugPrint("preupgrade");
    preupgrade_loop(0);
  };

};
