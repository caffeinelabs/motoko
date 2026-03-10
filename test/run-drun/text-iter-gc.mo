//MOC-FLAG -A=M0194
import Prim "mo:⛔";

actor a {
  public func go() : async () {
   // Allocate enough memory to regularly schedule GC run.
   let array = Prim.Array_init<Nat>(32 * 1024 * 1024, 0);
   
   var text = "";
   let iterator = text.chars();
   await async {}; // Trigger GC.
   ignore iterator.next();
  }
};

a.go(); //OR-CALL ingress go "DIDL\x00\x00"

//SKIP run-ir
//SKIP run-low
