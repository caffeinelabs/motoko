// Type-mismatch (encoder side): the method returns `Nat`, so the
// encoder is expected to be `Nat -> Blob`. Here we provide a
// `Nat -> Text` instead — should be rejected.
persistent actor {
  (with encoder = func (_ : Nat) : Text = "oops")
  public func go() : async Nat = async 42;
}
