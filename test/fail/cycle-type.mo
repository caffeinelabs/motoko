actor {
  func _bad(a : actor { foo : () -> async (); oneway : () -> () }) : async () {
    let defaults = { moot = 9 };
    await (defaults with cycles = 'C') a.foo();
    await (defaults with cycles = "Can't") async ();
    func nonSend() : Nat = 42;
    ignore (with) nonSend();
    (with cycles = 999) a.oneway(); // should not warn
    ({} with) a.oneway();
    await (with timeout = 'T') a.foo();
  }
}
