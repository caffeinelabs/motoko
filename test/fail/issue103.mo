// A (non-shared) function is not sharable
do {
  func invalid1 (f : shared (() -> Nat) -> ()) {
    f (func foo() : Nat = 1)
  };
};

// An object with function fields is not sharable
do {
  func invalid2 (f : shared { foo : () -> Nat } -> ()) {
    f (object { public func foo() : Nat = 1; })
  };
};

// Cannot return a function in an async
do {
  func invalid3 () : (async (() -> Nat)) {
    async { func foo() : Nat = 1 }
  };
};

// Cannot return an object with function fields in an async
do {
  func invalid4 () : (async ({ foo : () -> Nat })) {
    async { object { public func foo() : Nat = 1; } }
  };
};
