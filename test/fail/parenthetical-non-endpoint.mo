// A parenthetical before `public` is only allowed on public actor methods
// (endpoints); on anything else it is an error (M0282).
mixin() {
  (with encoder = undefinedInMixin)
  public type U = Nat
};

actor {
  (with encoder = undefined)
  public type T = Nat;
  public func ok() : async () {}
}
