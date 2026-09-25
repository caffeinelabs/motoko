actor {
  public type Foo = { #nil; };
  public query func foo(#nil) : async Foo {
    #nil;
  }
}

//CALL query foo 0x4449444c016b01d1a7cf027f010000

//SKIP run
//SKIP run-ir
//SKIP run-low
