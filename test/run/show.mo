//MOC-FLAG -A=M0194
import Prim "mo:⛔";

Prim.debugPrint(debug_show (true));
Prim.debugPrint(debug_show (false));

// nat
Prim.debugPrint(debug_show (-42000000000000));
Prim.debugPrint(debug_show (-42000000));
Prim.debugPrint(debug_show (-42000));
Prim.debugPrint(debug_show (-42));
Prim.debugPrint(debug_show (-2));
Prim.debugPrint(debug_show (-1));
Prim.debugPrint(debug_show (-0));
Prim.debugPrint(debug_show (+0));
Prim.debugPrint(debug_show (+1));
Prim.debugPrint(debug_show (+2));
Prim.debugPrint(debug_show (+42));
Prim.debugPrint(debug_show (+42000));
Prim.debugPrint(debug_show (+42000000));
Prim.debugPrint(debug_show (+420000000000000));

// int
Prim.debugPrint(debug_show (0));
Prim.debugPrint(debug_show (1));
Prim.debugPrint(debug_show (2));
Prim.debugPrint(debug_show (42));
Prim.debugPrint(debug_show (42000));
Prim.debugPrint(debug_show (42000000));
Prim.debugPrint(debug_show (420000000000000));


// int corner cases
Prim.debugPrint(debug_show (+0x7FFFFFFE));
Prim.debugPrint(debug_show (+0x7FFFFFFF));
Prim.debugPrint(debug_show (+0x80000000));
Prim.debugPrint(debug_show (+0x80000001));
Prim.debugPrint(debug_show (+0x8FFFFFFF));
Prim.debugPrint(debug_show (+0x100000000));
Prim.debugPrint(debug_show (+0x100000001));
Prim.debugPrint(debug_show (-0x7FFFFFFE));
Prim.debugPrint(debug_show (-0x7FFFFFFF));
Prim.debugPrint(debug_show (-0x80000000));
Prim.debugPrint(debug_show (-0x80000001));
Prim.debugPrint(debug_show (-0x8FFFFFFF));
Prim.debugPrint(debug_show (-0x100000000));
Prim.debugPrint(debug_show (-0x100000001));

// nat corner cases
Prim.debugPrint(debug_show (0x7FFFFFFE));
Prim.debugPrint(debug_show (0x7FFFFFFF));
Prim.debugPrint(debug_show (0x80000000));
Prim.debugPrint(debug_show (0x80000001));
Prim.debugPrint(debug_show (0x8FFFFFFF));
Prim.debugPrint(debug_show (0x100000000));
Prim.debugPrint(debug_show (0x100000001));

Prim.debugPrint(debug_show (42,-42,()));
Prim.debugPrint(debug_show ("Foobar", null, null, ?23));
Prim.debugPrint(debug_show ([]));
Prim.debugPrint(debug_show ([1,2,3]));
Prim.debugPrint(debug_show ([var]));
Prim.debugPrint(debug_show ([var 1,2,3]));
class Foo() { public let foo : Int = 42; public var bar : Bool = true ; let hidden = [1,2] };
Prim.debugPrint(debug_show (Foo()));
Prim.debugPrint(debug_show (#foo ()));
Prim.debugPrint(debug_show (#bar (1, 2)));
Prim.debugPrint(debug_show (#bar {}));
Prim.debugPrint(debug_show (#bar ([])));
Prim.debugPrint(debug_show (#bar 42));
Prim.debugPrint(debug_show (#bar (-42)));
Prim.debugPrint(debug_show (#foo 42 : {#foo : Int; #bar : Text}));
Prim.debugPrint(debug_show (42 : Nat8));
Prim.debugPrint(debug_show (42 : Int32));
Prim.debugPrint(debug_show (Prim.intToInt64 (-42)));
Prim.debugPrint(debug_show '☃');
Prim.debugPrint(debug_show 0.0);
Prim.debugPrint(debug_show (-0.0));
Prim.debugPrint(debug_show (-42.0));
Prim.debugPrint(debug_show (1000000.12345));
Prim.debugPrint(debug_show ("Test" : Blob));
Prim.debugPrint(debug_show ("\00\01\02\03\04" : Blob));

// Make sure blob-to-string conversion can handle invalid UTF-8 strings
Prim.debugPrint(debug_show ("\FF" : Blob)); // 0xFF is never valid in a UTF-8 string
Prim.debugPrint(debug_show ("\D8\00" : Blob)); // unpaired surrogate
