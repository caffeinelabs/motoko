import Prim "mo:⛔";

Prim.debugPrint (debug_show (Prim.blobToArray ""));
Prim.debugPrint (debug_show (Prim.blobToArray "\00\01\02"));
Prim.debugPrint (debug_show (Prim.blobToArray "\FF"));
Prim.debugPrint (debug_show (Prim.blobToArray "\u{FF}"));
Prim.debugPrint (debug_show (Prim.blobToArray "☃"));

Prim.debugPrint (debug_show (Prim.blobToArrayMut ""));
Prim.debugPrint (debug_show (Prim.blobToArrayMut "\00\01\02"));
Prim.debugPrint (debug_show (Prim.blobToArrayMut "\FF"));
Prim.debugPrint (debug_show (Prim.blobToArrayMut "\u{FF}"));
Prim.debugPrint (debug_show (Prim.blobToArrayMut "☃"));

Prim.debugPrint (debug_show (Prim.arrayToBlob([])));
Prim.debugPrint (debug_show (Prim.arrayToBlob([0, 1, 2])));

Prim.debugPrint (debug_show (Prim.arrayMutToBlob([var])));
Prim.debugPrint (debug_show (Prim.arrayMutToBlob([var 0, 1, 2])));
