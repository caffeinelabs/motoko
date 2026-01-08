import Lib "upgrade-sequence/class";
import Prim "mo:⛔";

actor Driver {

  public func go() : async () {
    var c = await (with cycles = Prim.cyclesBalance() / 2) Lib.C();

    var i = 0;
    while (i < 50) {
      Prim.debugPrint(debug_show {i});
      c := await (system Lib.C)(#upgrade c)();
      i += 1;
    };
  }

}

//CALL ingress go "DIDL\x00\x00"
//SKIP run
//SKIP run-ir
//SKIP run-low

