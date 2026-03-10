//MOC-FLAG -A=M0194
// A simple Counter actor.
actor class (i : Int, step : Nat) {
  flexible var c = i;
  func show(note : Text, c : Int) {};
  // Decrement counter
  public func dec() : () {
    show("dec", c);
    c -= step;
  };

  // Read counter, asynchronous
  public func read() : async Int { c };
};
