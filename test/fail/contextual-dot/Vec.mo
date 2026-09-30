module {
  public type Vec = { x : Nat; y : Nat };
  public func sum(self : Vec) : Nat = self.x + self.y;
}
