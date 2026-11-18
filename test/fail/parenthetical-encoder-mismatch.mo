// The method returns `Nat`, so the encoder must be `Nat -> Blob`;
// a `Nat -> Text` is rejected.
actor {
  (with encoder = func (_ : Nat) : Text = "oops")
  public func go() : async Nat = 42;
}
