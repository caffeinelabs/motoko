// The method returns `Nat`, so the encoder must be `Nat -> Blob`;
// a `Nat -> Text` is rejected.
actor {
  public (with encoder = func (_ : Nat) : Text = "oops") func go() : async Nat = 42;
}
