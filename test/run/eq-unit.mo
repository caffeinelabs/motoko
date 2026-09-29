//MOC-FLAG -W M0276
// equality on singletons should not hide side effects
import Prim "mo:⛔";
ignore (Prim.debugPrint "Look, ma!" == ());
