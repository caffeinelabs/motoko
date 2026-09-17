import Prim = "mo:⛔";

(with migration =
   func({
     }) :
   { } =
   { }
)
actor {
  var g : Nat = 0; // ok, exact type
  Prim.debugPrint "version6"
}
