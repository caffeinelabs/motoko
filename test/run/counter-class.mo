//MOC-FLAG -A=M0194
actor class Counter(i : Int) {
  flexible var j = i;

  public func dec() : () {
   showCounter(j);
   j -= 1;
  };

  public func read() : async Int { j };
};

func showCounter(c : Int) {};

let c = await Counter(10);

actor Test {
  public func go() : () {
    var i : Int = 10;
    while (i  > 0) {
      c.dec();
      i -= 1;
    }
  }
};

Test.go();

// no support for top-level await, first-class shared functions anywhere yet
//SKIP comp
