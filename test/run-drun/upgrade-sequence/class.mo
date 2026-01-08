import Prim "mo:⛔";
(with migration = func ({}) : {} {
   Prim.debugPrint "migrating...";
   {}
 }
)
actor class C() {
  stable var version = 0;
  Prim.debugPrint "initializing...";
  Prim.debugPrint(debug_show {version});
  version += 1;
}
