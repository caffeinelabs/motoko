// Slice-6.6 demo: `<T with type>` threads a real `@Candid` witness
// param through call-sites.  The leg bodies use the refined arg to
// build a variant value whose tag depends on T.
import Prim "mo:⛔";

func f<T with type>(_arg : T) : { #int : Int; #nat : Nat } {
  switch type T {
    case Nat (#nat 42);
    case Int (#int 0);
  }
};

Prim.debugPrint(debug_show (f<Nat>(7)));

//SKIP run
//SKIP run-ir
//SKIP run-low
