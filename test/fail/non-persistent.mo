//MOC-FLAG --require-persistent-actors
actor {
  let _x = #x; // error
  var _y = #y; // error
  stable let _z = #z;
  type T = ?T;
  class _C() {};
  module _M = {};
  object _O = {}; // error
  func f(){};
  public /*shared*/ func g() : (){};
}
