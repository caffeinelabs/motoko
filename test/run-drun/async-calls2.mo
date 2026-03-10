/* This tests checks if messages to actors are really asynchronous, and complete
   before delivery.
*/

actor a {
  flexible var x : Bool = false;

  public func bump() : () { assert (x == false); x := true; assert (x == true);  };

  public func is_true() : () { assert (x == true); };

  public func go() : () { assert (x == false); bump(); assert (x == false); is_true(); };
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
