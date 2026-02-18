//MOC-FLAG -A=M0194
import Prim "mo:⛔";

func isEq(x : Principal, y : Principal) : Bool { x == y };


Prim.debugPrint(debug_show(Prim.principalOfBlob("")));
Prim.debugPrint(debug_show(Prim.principalOfBlob("\00")));
Prim.debugPrint(debug_show(Prim.principalOfBlob("\2a\01")));
Prim.debugPrint(debug_show(Prim.principalOfBlob("\ff\ff\ff")));

Prim.debugPrint(debug_show(Prim.principalOfActor(actor "aaaaa-aa")));
Prim.debugPrint(debug_show(Prim.principalOfActor(actor "lg264-qjkae")));

Prim.debugPrint(debug_show(Prim.blobOfPrincipal(Prim.principalOfActor(actor "aaaaa-aa"))));
Prim.debugPrint(debug_show(Prim.blobOfPrincipal(Prim.principalOfActor(actor "lg264-qjkae"))));
