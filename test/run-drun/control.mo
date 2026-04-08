//MOC-FLAG -A=M0194
// Nothing to run here (but do compile and validate)

//SKIP drun-run

actor control {

  flexible func condition() : Bool = false;

  public func testBlock() : () {
    label l1 {
      break l1
    };
    label l2 break l2();
    label l2 (let m = 1 + (break l2) : Int);
    let n = label l3 : Int { break l3(2) };
    let (x, y, z) = label l3 : (Int, Bool, Text) (break l3(2, true, ""));
  };

  public func testWhile() : () {
    label l while (condition()) {
      if true break l
      else continue l;
    };
  };

  public func testLoop() : () {
    label l loop {
      if true break l
      else continue l;
    };
  };

  public func testLoopWhile() : () {
    label l loop {
      if true break l
      else continue l;
    } while (condition());
  };

  public func testLoopWhile2() : () {
    loop { } while (false);
  };

  public func testLoopWhile3() : () {
    label l {
      loop { }
      while (false and true)
    };
  };

  public func testLoopWhile4() : () {
    label l loop {
    } while (true and false);
  };

  public func testNestedWhile() : () {
    label l while (condition()) {
      if true break l
      else continue l;
      label m while (condition()) {
        if true continue l
        else break m;
      };
    };
  };
};
