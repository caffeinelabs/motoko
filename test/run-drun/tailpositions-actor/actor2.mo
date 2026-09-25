import Prim "mo:⛔";

actor a {

  let bound : Int = 100000;

  system func postupgrade() {
    func postupgrade_loop(n : Int) {
      if n >= bound {
        Prim.debugPrint "postupgrade_loop done";
        return;
      };
      postupgrade_loop(n + 1);
    };

    Prim.debugPrint("postupgrade");
    postupgrade_loop(0);
  };

};
