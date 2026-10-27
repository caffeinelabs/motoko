import P "mo:prim";
// test polymorphic async* compilation, at shared and non-shared types
actor a {

  private func doUnit<T>(t : T) : async* T {
    await async { () }; // await at unit type
    t
  };

  private func doText<T>(t : T) : async* T {
    ignore await async { "text" }; // await at different type
    t
  };

  private func doReturn<T>(t : T) : async* T {
    return t
  };

  private func doExit<T>(t : T) : async* T {
    t
  };

  private func doThrow<T>(_ : T) : async* T {
    throw P.error("oops")
  };

  type Comp<T> = () -> async* T;

  func retry<T>(call : Comp<T>) : async* T {
    try { await* call() } catch _ { await* call() }
  };

  func twice<T>(call : Comp<T>) : async* (T, T) {
    (await* retry(call), await* retry(call))
  };

  type Counter = { var count : Nat; inc : () -> () };

  func counter() : Counter {
    object {
      public var count = 0;
      public func inc() { count += 1 }
    }
  };

  func cell() : async* [var Nat] { [var 0] };

  public func double(n : Nat) : async Nat { 2 * n };

  public func go() : async () {
    let n = await* doUnit<Nat>(1);
    assert n == 1;
    let (x, y) = await* doText<(Nat, Nat)>((1, 2));
    assert x == 1 and y == 2;
    await* doReturn<()>(());
    await* doExit(());

    // non-shared results keep their identity
    let arr = await* cell();
    let arr1 = await* doText<[var Nat]>(arr);
    arr1[0] := 1;
    assert arr[0] == 1;
    let c = counter();
    let c1 = await* doUnit<Counter>(c);
    c1.inc();
    c1.count += 1;
    assert c.count == 2;

    // exceptions through generic helpers
    try {
      ignore await* doThrow<[var Nat]>(arr);
      assert false
    } catch e { assert P.errorMessage(e) == "oops" };
    var attempts = 0;
    let arr2 = await* retry<[var Nat]>(func() {
      attempts += 1;
      if attempts == 1 { throw P.error("transient") };
      arr
    });
    arr2[0] += 1;
    assert arr[0] == 2 and attempts == 2;
    try {
      ignore await* retry<Counter>(func() { throw P.error("permanent") });
      assert false
    } catch e { assert P.errorMessage(e) == "permanent" };

    // thunks that send messages to this actor
    let c2 = await* retry<Counter>(func() {
      let c = counter();
      c.count := await double(1);
      c
    });
    c2.inc();
    assert c2.count == 3;
    let (p, q) = await* twice<(Nat, Nat)>(func() { (await double(1), await a.double(2)) });
    assert p.0 == 2 and q.1 == 4;
    let (r, s) = await* twice<[var Nat]>(func() { arr });
    r[0] := 7;
    assert s[0] == 7 and arr[0] == 7;
    let ((), ()) = await* twice<()>(func() { ignore await double(0) });

    // each await* reruns the computation, so each result is a fresh array
    let fresh = async* { [var 0] };
    let arr3 = await* fresh;
    let arr4 = await* fresh;
    arr3[0] := 1;
    assert arr4[0] == 0;
    let c3 = await* async* { counter() };
    c3.inc();
    assert c3.count == 1;

    // futures and computations as results, awaited in the scope that created them
    let mk = async* { async { 1 } };
    assert (await (await* mk)) == 1;
    let mkStar = async* { async* { 2 } };
    assert (await* (await* mkStar)) == 2;
    let fut = async { 3 };
    assert (await (await* doReturn<async Nat>(fut))) == 3;
    assert (await* (await* doExit<async* Nat>(async* { 4 }))) == 4
  }

};

a.go() //OR-CALL ingress go "DIDL\x00\x00"
