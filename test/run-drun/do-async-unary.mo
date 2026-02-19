//MOC-FLAG -A=M0194
import P "mo:prim";
// test unary async compilation, using null as token value
actor a {

  private func doUnit() : async* Null {
    let t = await async null; // await at unit type
    return t;
  };

  private func doText() : async* Null {
    let t = await async "text"; // await at different type
    return null;
  };

  private func doReturn() : async* Null {
    return null;
  };

  private func doExit() : async* Null {
    null;
  };

  private func doThrow() : async* Null {
    throw P.error("oops");
  };

  public func go() : async () {
    let null = await* doUnit();
    let null = await* doReturn();
    let null = await* doExit();
    try {
      let null = await* doThrow();
      assert(false);
    } catch (e) { assert P.errorMessage(e) == "oops";};
  }

};


a.go(); //OR-CALL ingress go "DIDL\x00\x00"
