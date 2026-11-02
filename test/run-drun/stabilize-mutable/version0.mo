import Prim "mo:prim";

actor {
   type Entry = { key: Nat; value: Text; };
   var array1: [var Entry] = [ var { key = 1; value = "1" }];
   var array2: [var Entry] = array1;
   
   public func modify() : async () {
      array1[0] := { key = 2; value = "2" };
      assert(array2[0].key == 2);
      assert(array2[0].value == "2");
   };

   public func print() : async () {
      Prim.debugPrint(debug_show(array1));
      Prim.debugPrint(debug_show(array2));
   };
};
