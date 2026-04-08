// Tests that GC properly looks through mutable things in global memory 
// (This uses messages to trigger GC. If we don’t run GC after message, other ways need to be found.)

actor {
  flexible var x = ("Foo", "Bar");

  public func foo1() : () {
    ignore [1,2,3,4,5];
    x := ("abc", "def");
    ignore [1,2,3,4,5];

  };
  public func foo2() : () {
    ignore [1,2,3,4,5,6,7,8,9];
    assert (x.0 == "abc");
    assert (x.1 == "def");
  }

}
//CALL ingress foo1 0x4449444C0000
//CALL ingress foo2 0x4449444C0000


// SKIP run
// SKIP run-ir
// SKIP run-low
