// A function body is implicitly `async` when the function returns `async`, written as a block or `= e` alike
actor {
  func block() : async Nat = async { 1 };
  func exp() : async Nat = async 1;
  func star() : async* Nat = async* { 1 };
  func par() : async () = (with cycles = 1) async {};
  func run(f : () -> async Nat) : async Nat { await f() };
  public func go() : async Nat { await run(func() = async { 1 }) };
  public func notify() : () = ignore (async {});

  // not redundant
  func eq() : async Nat = do { 1 };
  func inner() : async Nat { await async { 1 } };
}
