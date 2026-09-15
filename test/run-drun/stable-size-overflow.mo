//CLASSICAL-PERSISTENCE-ONLY
import P "mo:⛔";
import Region "stable-region/Region";

actor {

  let r = Region.new();
  ignore Region.grow(r, 1);

  let page : Blob = Region.loadBlob(r, 0,65536);
  assert (page.size() == 65536);

  stable
  let _a : [Blob] = P.Array_tabulate<Blob>(65536,func _ { page });

  system func preupgrade() {
   P.debugPrint("upgrading...");
  };
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL upgrade ""

//MOC-FLAG -A=M0270