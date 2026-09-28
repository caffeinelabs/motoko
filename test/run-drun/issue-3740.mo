actor {
  type Unit = ();

  public func f() : async Unit {};

  public func go() : async () {
    await f();
  };

}
