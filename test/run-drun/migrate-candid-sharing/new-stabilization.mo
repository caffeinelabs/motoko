//MOC-FLAG --enhanced-orthogonal-persistence
import Prim "mo:prim";

actor {
  type Data = { field1 : Text; field2 : Nat; var field3: ?Data; };

  var sharedObject : Data = { field1 = "Test"; field2 = 12345; var field3 = null };
  sharedObject.field3 := ?sharedObject;

  var array : [var Data] = Prim.Array_init<Data>(100, sharedObject);

  Prim.debugPrint("INITIALIZED: " # debug_show (array.size()));
};
