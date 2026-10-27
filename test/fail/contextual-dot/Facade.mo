import _Vec "Vec";
import _Pair "Pair";

// Re-exports its submodules as fields, like a package's `lib.mo`
module {
  public let Vec = _Vec;
  public let Pair = _Pair;

  public module Deep {
    public module Deeper {
      public func triple(self : Nat) : Nat = self * 3;
    };
  };

  // Deep is in scope here by itself
  public func nine() : Nat = (3 : Nat).triple();
}
