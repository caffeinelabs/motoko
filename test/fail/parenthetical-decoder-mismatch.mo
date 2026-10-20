// The method takes `?Nat`, so the decoder must be `Blob -> ?Nat`;
// a `Blob -> ?Text` is rejected.
actor {
  public (with decoder = func (_ : Blob) : ?Text = ?"oops") func go(_ : ?Nat) : async () {};
}
