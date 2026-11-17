actor {
  public func baz() : async () = (with) async { };

  func utf8(_ : Text) : Blob { "" };
  transient let codecs = { encoder = utf8 };

  // a parenthetical before `public`, on its own line, sees the actor's fields
  (with encoder = utf8)
  public func shout(_ : Text) : async Text { "" };

  (with)
  public func quiet() : async () {};

  (codecs with)
  public query func peek() : async Text { "" };

  // each erroneous parenthetical is reported
  (with encoder = unbound)
  public func broken() : async () {};

  (with decoder = unbound2)
  public func broken2() : async () {};

  module M {
    /// @deprecated use `y`
    (with)
    public let x = 1;

    (with)
    /// @deprecated use `x`
    public let y = 2
  };

  transient let _ = M.x + M.y;

  class C() {
    (with)
    public let c = 0
  };

  transient let o = object {
    (with)
    public let p = 0
  };

  func local() {};

  // still a call
  (with cycles = 1) local()
}
