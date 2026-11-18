// A oneway method never replies, so an encoder on it is rejected (M0280).
// A decoder alone is fine: oneway arguments are decoded like any other.
actor {
  func none(_ : ()) : Blob = "";
  func nothing(_ : Blob) : Nat = 0;
  (with encoder = none)
  public func ping() : () {};
  (with decoder = nothing)
  public func poke(_ : Nat) : () {}
}
