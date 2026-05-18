// Slice-7 demo: real one-byte SLEB128 decoder for @typCode and a
// call-site massager that picks the right Candid sample value per
// type-arg, so different `f<...>` instantiations route to different
// arms of the prim_type's switch.
import Prim "mo:⛔";

func f<T with type>(arg : T) : { #int : Int; #nat : Nat; #char : Char } {
  switch type T {
    case Nat  (#nat arg);
    case Int  (#int arg);
    case Char (#char arg);
  }
};

Prim.debugPrint(debug_show (f<Nat>(7)));
Prim.debugPrint(debug_show (f<Int>(-3)));
Prim.debugPrint(debug_show (f<Char>('Z')));

//SKIP run
//SKIP run-ir
//SKIP run-low
