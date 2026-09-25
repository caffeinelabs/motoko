module F {
  public func f(_ : (self : Nat)) {}; // no warning
  public func g(self: Nat) {}; // warn unusued self
  public func h(_self: Nat) {}; // suppress unused self, but also not dot enabled
};

func _testf() {
  (1).f();
};

func _testg() {
  (1).g();
};

func _testh() {
  F.h(1); // works
  // (1).h(); causes an error
};
