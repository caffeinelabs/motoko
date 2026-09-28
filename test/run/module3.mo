// test equivalence of various type references
module List = {
  public type List<A> = ?(A, List.List<A>);

  func rev<A>(x : List<A>, acc : List.List<A>) : List<A> {
    switch x {
      case null { acc }
      case ?(h, t) { rev<A>(t, ?(h, acc)) }
    };
  };
};
