//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration-fast-forward/migrations

import Prim "mo:prim";
import Info "../runtime-info/info";

actor Self {
  let a : Text;
  var b : Bool;

  public func check() : async () {
    Prim.debugPrint(debug_show "Fast-forwarded migrations:");
    Prim.debugPrint(debug_show { a; b });

    // Check migration version information.
    let information = await Info.introspect(Self).__motoko_runtime_information();
    switch (information.version) {
      case (null) assert false; // Should not happen, there have been 5 migrations.
      case (?list) assert list.0 == "5"; // Last migration version is 5.
    };
  };
};
