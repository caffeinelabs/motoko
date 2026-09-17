(with migration =
   func({
     }) :
   { } =
   { }
)
actor {
  var g : Any = 0; // reject, a lossy supertype
  assert false;
}
