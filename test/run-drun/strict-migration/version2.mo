import Prim = "mo:⛔";

(with migration =
   func({
     f : Nat // ok - exact type
     }) :
   { f : Int} =
   { f = f }
)
actor {
  var f : Int = Prim.trap("impossible");
  Prim.debugPrint("version2");
}
