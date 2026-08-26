// The frozen migration under test. Deliberately exercises all three
// relocatable index spaces:
//  - text literals -> data segments + object pool
//  - a runtime-selected function value -> closure + call_indirect -> table
//  - Nat arithmetic -> bigint RTS imports; text concat -> text RTS imports
module {
  func triple(x : Nat) : Nat = x * 3;

  public func migration(old : { greeting : Text }) : { greeting : Text; total : Nat } {
    let double = func(x : Nat) : Nat { x * 2 };
    // runtime-dependent selection defeats the const/direct-call analysis,
    // forcing a real closure value and a call_indirect through the table
    let pick : Nat -> Nat = if (old.greeting.size() % 2 == 0) double else triple;
    var sum = 0;
    var i = 0;
    while (i < 10) {
      sum += pick(i);
      i += 1;
    };
    { greeting = old.greeting # ", frozen world"; total = sum }
  }
}
