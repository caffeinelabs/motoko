// This single-cell calculator defines one calculator instruction per
// public entry point (add, sub, mul, div).

// Create a simple Calc actor.
persistent actor Calc {

  var cell : Int = 0;

  // Define functions to add, subtract, multiply, and divide
  public func add(n:Int) : async Int {
    cell += n;
    cell
  };

  public func sub(n:Int) : async Int {
    cell -= n;
    cell
  };

  public func mul(n:Int) : async Int {
    cell *= n;
    cell
  };

  public func div(n:Int) : async ?Int {
    if ( n == 0 ) {
      null // null indicates div-by-zero error
    } else {
      cell /= n;
      ?cell
    }
  };

  // Clear the cell, resetting to zero
  public func clearall() : async Int {
    cell := 0;
    cell
  };
};
