// A `public (…)` parenthetical is only allowed on public actor methods
// (endpoints); on anything else it is an error (M0282).
mixin() {
  public (with encoder = undefinedInMixin) type U = Nat;
};

actor {
  public (with encoder = undefined) type T = Nat;
  public func ok() : async () {};
}
