import Prim "mo:prim";

actor {
   Prim.debugPrint("Version 2");

   let allocationSize = 64 * 1024 * 1024;

   func largeAllocation(name: Text): [var Nat] {
      Prim.trap("Should not be called");
   };

   var firstVariable : Any = largeAllocation("first variable");
   var secondVariable : Any = largeAllocation("second variable");

   public func check(): async() {
      // Extra GC increments.
      await async {};
      await async {};
      await async {};
      // Check that both variables have been cleared and both arrays have been reclaimed.
      assert(Prim.rts_heap_size() < allocationSize);
   }
};
