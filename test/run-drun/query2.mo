import Prim "mo:⛔";
actor counter = {
  flexible var c = 1;
  public func inc() : async () {
    c += 1;
    Prim.debugPrintNat c;
  };
  public func printCounter () : async () {
    assert 0 : Nat64 == Prim.replyDeadline();
    Prim.debugPrintNat c;
  };
  public func get() : async Nat {
    return c
  };
  public query func read() : async Nat {
    assert 0 : Nat64 == Prim.replyDeadline();
    let tmp = c;
    c += 1;
    Prim.debugPrint "In read:";
    Prim.debugPrintNat c;
    return tmp;
  };

  public func go() : async () {
    await counter.inc();
    await counter.inc();
    await counter.inc();
    await counter.printCounter();
    let c1 = await counter.get();
    assert c1 == 4;
    let c2 = await counter.read();
    await counter.printCounter();
    assert c2 == 4;
    let c3 = await counter.read();
    await counter.printCounter();
    Prim.debugPrint("The following fails in the interpreter, for lack of query semantics");
    assert c3 == 4;
  };
};
counter.go(); //OR-CALL ingress go "DIDL\x00\x00"

/* Disabled, while we don’t have first-class shared functions

  { // fully explicit syntax

  let _ : actor { read : shared query () -> async Nat } = counter;

  shared query func f () : async Nat { 666; };

  type Query = shared query () -> async Nat;

  let _ : Query = counter.read;

  };

  {

  // sugar, suppressing shared

  let _ : actor { read: query () -> async Nat } = counter;

  query func f () : async Nat { 666; };

  type Query = query () -> async Nat;

  let _ : Query = counter.read;

  };
*/
