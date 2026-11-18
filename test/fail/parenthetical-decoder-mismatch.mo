// The method takes `?Nat`, so the decoder must be `Blob -> ?Nat`;
// a `Blob -> ?Text` is rejected.
actor {
  (with decoder = func (_ : Blob) : ?Text = ?"oops")
  public func go(_ : ?Nat) : async () {}
}
