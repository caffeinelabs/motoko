// Two different functions in nested modules are ambiguous
module Top {
  public module A { public func twice(self : Nat) : Nat = self * 2 };
  public module B { public func twice(self : Nat) : Nat = self + self };
};

ignore (1 : Nat).twice();

// The search for nested modules has a depth limit
module L1 { public module L2 { public module L3 { public module L4 {
  public module L5 { public module L6 { public module L7 { public module L8 {
    public func deepest(self : Nat) : Nat = self;
    public module L9 {
      public func tooDeep(self : Nat) : Nat = self;
    };
  } } } }
} } } };

ignore (1 : Nat).deepest(); // Resolves fine
ignore (1 : Nat).tooDeep();

// Instances of one module declaration are different modules
func adder(n : Nat) : module { plus : (self : Nat) -> Nat } {
  module { public func plus(self : Nat) : Nat = self + n }
};
let One = adder(1);
let Two = adder(2);

ignore (1 : Nat).plus();
