// Slice-6.5: `<T with type>` auto-injects a value-side `T : @Candid`
// for the body to use.  No manual `let T = ...` needed.  The desugarer
// is responsible for materialising the runtime witness — disabled here
// until that lands.

func f<T with type>(_arg : T) : ?Int {
  switch type T {
    case Nat (?42);
    case Int (null);
  }
};

let _ = f<Nat>(0);

//SKIP run
//SKIP run-ir
//SKIP run-low
