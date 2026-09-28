// A simple Counter actor.

persistent actor class Counter(i : Int) {
  transient var c = i;

  // Decrement counter
  public func dec() {
    show("dec", c);
    c -= 1;
  };

  // Read counter, asynchronous
  public func read() : async Int { c };
};

// Dummy function to show intermediate value in trace.
func show(note : Text, c : Int) {};

// Create an actor.
let c = await Counter(10);

// Issue ten `dec` messages.
func testDec() : async () {
  var i : Int = 10;
  while i > 0 {
    c.dec();
    i -= 1;
  }
};

ignore testDec();

// Issue ten `dec` & `read` messages.
func testRead() : async () {

  // Dummy function to show intermediate value in trace.
  func showAsync(note : Text, a : async Int) {};

  var i : Int = 10;
  while i > 0 {
    c.dec();
    let t = c.read();
    showAsync("before", t);
    show("await", await t);
    showAsync("after", t);
    i -= 1;
  }
};

ignore testRead();
