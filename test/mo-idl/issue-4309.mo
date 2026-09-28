//MOC-FLAG --actor-idl issue-4309

import Types = "./import/issue-4309-types";
import Types2 = "./import/issue-4309-types";

actor {
  module Inner {
    public type Foo = Types.Foo;
    public type Credit<T> = Nat;
    public type Result<T, E> = Types.Result<T, E>;
    public type Cons<T> = ?(T, Snoc<T>);
    public type Snoc<T> = ?(Cons<T>, T);
    public type List<T> = ?(T, List2<T>);
    public type List2<T> = ?(T, List<T>);
  };

  module Innerer {
    public type Credit<T> = Inner.Credit<T>;
    public type List<T> = Inner.List<T>;
  };

  public type Credit = Innerer.Credit<Nat>;
  public type Foo = Inner.Foo;
  public func get() : async (Credit, Inner.Credit<Nat>, Innerer.Credit<Nat>) {
    (0, 0, 0)
  };
  public func silly(_ : Foo, _ : Inner.Foo, _ : Types.Foo, _ : Types2.Foo) : async Inner.Result<(), ()> { #ok };
  public func erroring() : async Inner.Result<(), Text> { #err "feel good" };
  public func consful(snoc : Inner.Snoc<Nat>) : async Inner.Cons<Nat> { ?(42, snoc) };
  public func listful(list : Inner.List2<Char>) : async Innerer.List<Char> { ?('Y', list) }
}
