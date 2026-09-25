module Wrong = {
  func f() { assert false; };
  let _ = f(); // non_static!
  let _ = [var 0]; // non static
  var x = 0; // non-static
  let a = [0];

  object WrongO = {
    let _ = f(); // non_static!
    let _ = [var 0]; // non static
    var x = 0; // non-static
  };

  object OkO = {
    func f(){};
    let a = [0];
  };

  module WrongM = {
    let _ = f(); // non_static!
    let _ = [var 0]; // non static
    var x = 0; // non-static
  };

  object OkM = {
    func f(){};
    let a = [0];
  };

};
