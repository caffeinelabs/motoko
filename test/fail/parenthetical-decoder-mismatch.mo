// Type-mismatch (decoder side): the method's ingress type is `?Nat`,
// so the decoder is expected to be `Blob -> ?Nat`. Here we provide a
// `Blob -> ?Text` instead — should be rejected with M0214.
persistent actor {
  (with decoder = func (_ : Blob) : ?Text = ?"oops")
  public func go(_ : ?Nat) : async () {};
}
