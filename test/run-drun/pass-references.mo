import Prim "mo:⛔";
actor a {
  public func foo() : async Text = async "a";

  public func pass_func(f : shared () -> async Text) : () {
    Prim.debugPrint (await f());
  };

  public func pass_actor(a : actor { foo : shared () -> async Text }) : () {
    Prim.debugPrint (await a.foo());
  };

  public func go() : () {
    actor b {
      public func foo() : async Text = async "b";
    };
    pass_func(foo);
    pass_func(b.foo);
    pass_actor(a);
    pass_actor(b);
  };
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
