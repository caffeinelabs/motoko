// A oneway method never replies, so an encoder on it is rejected (M0280).
// A decoder alone is fine: oneway arguments are decoded like any other.
actor {
  func none(_ : ()) : Blob = "";
  func nothing(_ : Blob) : Nat = 0;
  public (with encoder = none) func ping() : () {};
  public (with decoder = nothing) func poke(_ : Nat) : () {}
}
