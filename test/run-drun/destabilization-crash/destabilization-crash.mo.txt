import Prim = "mo:prim";

// test destabilization of stable variables, without rts stack overflow
actor a {

   stable let x = Prim.stableMemoryGrow(1);
   assert Prim.stableMemorySize() == 1;

   type List<T> = ?(T, List<T>);

   stable var map : List<(Blob, Blob)> = null;

   var count : Nat32 = 0;

   func fillMB(mb : Nat) : () {
     var c = 1024;
     while (c > 0) {
       count += 1;
       Prim.stableMemoryStoreNat32(0, count);
       var k = Prim.stableMemoryLoadBlob(0, 32);
       var v = Prim.stableMemoryLoadBlob(0, 65536);
       map := ?((k,v), map);
       c -= 1;
     };
     if (Prim.rts_heap_size() < mb * 1024 * 1024) {
      // Difference between incremental and non-incremental GC (due to different object header lengths).
      let toleranceMB = 4;
      Prim.debugPrint(debug_show({heap_MB = Prim.rts_heap_size()/1024/1024/toleranceMB*toleranceMB}));
      fillMB(mb);
     }
   };

   system func preupgrade() {
     Prim.debugPrint "preupgrade!";
     fillMB(500); // Reduce because `drun` otherwise fails with `Ingress message did not finish executing within 10000 batches`.
   };

   system func postupgrade() {
     // if we get here, destabilization has succeeded
     Prim.debugPrint "postupgrade!";
     assert false; // trap to avoid saving state to disk
   }

}
