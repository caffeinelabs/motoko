import P "mo:prim";

actor a {

  public query func int() : async Int {
    666
  };

  public query func text() : async Text {
    "hello"
  };

  private func doInt() : async* Int = async* {
    return await int();
  };

  private func doText() : async* Int = async* {
    let t = await text(); // await at different type
    return t.size();
  };

  private func doReturn() : async* Int = async* {
    return 666;
  };

  private func doExit() : async* Int = async* {
    666;
  };

  private func doThrow() : async* Int = async* {
    throw P.error("oops");
  };

  public func go() : async () {
    let i = await* doInt();
    assert i == 666;
    let s = await* doText();
    assert s == 5;
    let r = await* doReturn();
    assert r == 666;
    let e = await* doExit();
    assert e == 666;
    try {
      let _ = await* doThrow();
      assert(false);
    } catch (e) { assert P.errorMessage(e) == "oops"; };
    ignore async* {};
  }
};

a.go(); //OR-CALL ingress go "DIDL\x00\x00"
