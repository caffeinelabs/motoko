/* This tests checks if messages to actors are really asynchronous, and complete
   before delivery.
*/

actor a {
  flexible var x : Bool = false;

  public func test(b : Bool) : () {
    if (b) { assert (x == false); x := true; assert (x == true); }
    else   { assert (x == false); test(false); assert (x == false); is_true(); }
  };

  public func is_true() : () { assert (x == true); };

  public func go() : () {
    test(true);
  }
};
//a.go(); //OR-CALL ingress go "DIDL\x00\x00"
