// A parenthetical before `public` is only allowed on public actor methods
// (endpoints); on anything else it is an error (M0282), which names what it
// is on.
mixin() {
  (with encoder = undefinedInMixin)
  public type U = Nat
};

module M {
  (with decoder = undefinedInModule)
  public func helper(t : Text) : Text { t }
};

actor {
  (with encoder = undefined)
  public type T = Nat;
  transient let o = object {
    (with decoder = undefinedInObject)
    public var counter = 0;
    (with encoder = undefinedInObject)
    public let limit = 10
  };
  public func ok() : async () { ignore o }
}
