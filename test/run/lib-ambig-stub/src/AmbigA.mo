module {
  public func show(self : { a : Nat; b : Nat }, show : (implicit : Nat -> Text)) : Text =
    "A:" # show(self.a);
};
