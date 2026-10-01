import Prim "mo:⛔";

// The expected type makes these functions async: their bodies run as `async { ... }`
actor a {
  func run(f : () -> async Nat) : async Nat { await f() };
  func runStar(f : () -> async* Nat) : async* Nat { await* f() };
  func apply(f : Nat -> async Nat) : async Nat { await f(20) };
  func forEach<A>(xs : [A], f : A -> async ()) : async () { for x in xs.values() { await f(x) } };

  var log = "";

  public func go() : async () {
    assert (await run(func() { 1 })) == 1;
    assert (await run(func() = 2)) == 2;
    assert (await apply(func(n) { n + 1 })) == 21;
    assert (await* runStar(func() { await run(func() { 3 }) })) == 3;
    await forEach(["a", "b", "c"], func(x) { log #= x });
    assert log == "abc";

    // Declared functions: a block body and `= e` are the same
    func four() : async Nat = 4;
    func five() : async Nat { 5 };
    assert (await four()) + (await five()) == 9;

    // The body runs asynchronously, after the caller continues
    let fut = run(func() { log #= "!"; 0 });
    log #= "?";
    ignore await fut;
    assert log == "abc?!";
    Prim.debugPrint log;
  };
};

a.go(); //OR-CALL ingress go "DIDL\x00\x00"
