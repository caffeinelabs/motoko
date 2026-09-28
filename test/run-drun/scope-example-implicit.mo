actor A {
  public shared func ping() : async Int { 666 }; // normal remote function

  public shared func f() : async(Int, Int) {
    // current scope: Y
    var a : async Int = async 0;

    await (async {
      // current scope: Z
      a := ping (); // this seems well-typed according to “Application (Derived Rule)”;
    });

    let i = await a;
    assert i == 666; // Boom!
    return (0, 0)
  };

  public shared func Test() : async () {
    let (x, y) = await f();
    assert x == 0 and y == 0;
  };

};

A.Test() //OR-CALL ingress Test 0x4449444C0000
