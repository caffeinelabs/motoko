import Prim "mo:prim";

// migration should not run on fresh install
(with migration = func (_:{}) : {} {
  Prim.debugPrint "unexpectedly migrating";
  assert false;
  {}
})
actor
{

  Prim.debugPrint("installed actor");

};
