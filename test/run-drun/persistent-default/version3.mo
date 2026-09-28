
import Prim "mo:⛔";
actor {

  var x : {#x} = Prim.trap "ohoh";
  var y : {#y} = Prim.trap "ohoh";
  var z = #z;
  Prim.debugPrint(debug_show {x;y;z});

}
