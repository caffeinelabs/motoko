// A (non-shared) function is not sharable
do {
  func invalid1 (f : shared (() -> Nat) -> ()) {
     f (func foo() : Nat = 1)
  };
};

// An object with function fields is not sharable
do {
  func invalid2 (f : shared {foo : () -> Nat} -> ()) {
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
  func invalid4 () : (async ({foo : () -> Nat})) {
     async { object { public func foo() : Nat = 1; } }
  };
};

// A future's result must be shared, even when a computation produces it
do {
  func cell() : async* [var Nat] { [var 0] };
  func invalid5() : async () {
    ignore async { await* cell() }
  }
};

// A type parameter is not shared
do {
  func invalid6<T>(x : T) : async T { x }
};

// A shared function's result must be shared
do {
  func invalid7(f : shared () -> async [var Nat]) {}
};

// A computation is not shared
do {
  func invalid8(f : shared (async* [var Nat]) -> ()) {}
};

do {
  func invalid9(c : async* [var Nat]) : Blob { to_candid (c) }
}
