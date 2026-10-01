module {
  public type Pair = (Nat, Text);
  public func swap(self : Pair) : (Text, Nat) = (self.1, self.0);
}
