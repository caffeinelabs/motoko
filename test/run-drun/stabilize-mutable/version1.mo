import Prim "mo:prim";

actor {
   type Entry = { key : Nat; value : Text };
   type SubEntry = { key : Nat; };
   
   var array1 : [var Entry] = [var { key = 1; value = "1" }];
   var array2 : [var SubEntry] = [var];

   public func modify() : async () {
      array2[0] := { key = 3; };
      assert (array1[0].key == 3);
      Prim.debugPrint(array1[0].value);
   };

   public func print() : async () {
      Prim.debugPrint(debug_show (array1));
      Prim.debugPrint(debug_show (array2));
   };
};
