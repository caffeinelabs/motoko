actor A {

  public shared func f() : async(Int, Int) {
    var a : async Int = async 0;

    func set ()  {
      a := async 666; // error: misplaced async
    };

    await (async {
      set();
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
