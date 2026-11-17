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

  (with encoder = unbound)
  public func broken() : async () {};

  module M {
    (with deprecated = "")
    public let x = 1
  };

  func local() {};

  // still a call
  (with cycles = 1) local()
}
