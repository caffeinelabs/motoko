//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
import P "mo:⛔";
import SM "stable-mem/StableMemory";

actor {

  ignore SM.grow(1);

  let page : Blob = SM.loadBlob(0,65536);
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
