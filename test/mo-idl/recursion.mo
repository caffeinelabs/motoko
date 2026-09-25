actor {
  public type List<A> = {
    head:A;
    tail:?List<A>;
  };
  public func f(x:Nat):async List<Nat> {
    { head=x; tail=null };
  };
  public func g(x:Int):async List<Int> {
    { head=x; tail=null };
  };
}
