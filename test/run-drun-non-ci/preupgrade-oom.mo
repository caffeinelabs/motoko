import P "mo:⛔";

// exercise OOM detection during upgrade.
// allocate up to last page, and then try to
// trigger OOM incrementally on last page.
// post-upgrade traps to avoid hitting disk
// NB: the oom would not be detected pre 0.6.21
actor {

  let r = P.regionNew();
  ignore P.regionGrow(r, 1);

  system func preupgrade() {
   // allocate up to last page
   P.debugPrint("(pre");
   while (P.rts_memory_size() / 65536 < 65534) {
     ignore P.regionLoadBlob(r, 0, 65536);
   };
   P.debugPrint("filled");

   P.debugPrint(debug_show(P.rts_memory_size() / 65536));
   // try to trigger oom on last page
   P.debugPrint("try to trigger OOM");
   do {
     var i = 32768;
     while (i > 0) {
       ignore P.regionLoadBlob(r, 0, i);
       i /= 2;
     };
   };
   P.debugPrint("pre)");
  };

  system func postupgrade() {
   P.debugPrint("(post");
   P.trap("deliberate trap - this code should not be reached!");
   P.debugPrint("post)");
  };

}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL upgrade ""
