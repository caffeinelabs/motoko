import Prim "mo:prim";
let { field1; field2; field3 = _; field4 = _field4 } = {
  field1 = 1;
  field2 = 2;
  field3 = 3;
  field4 = 4;
  field5 = 5;
};
Prim.debugPrint(debug_show (field1));
// field2 is reported as unused
