module {
  public type Tree<T> = {
    #leaf;
    #node : { left : Tree<T>; value : T; right : Tree<T> };
  };

  public func leaf<T>() : Tree<T> = #leaf;

  public func node<T>(l : Tree<T>, v : T, r : Tree<T>) : Tree<T> =
    #node { left = l; value = v; right = r };

  public func size<T>(t : Tree<T>) : Nat {
    switch t {
      case (#leaf) 0;
      case (#node n) 1 + size(n.left) + size(n.right);
    }
  };

  public func map<A, B>(t : Tree<A>, f : A -> B) : Tree<B> {
    switch t {
      case (#leaf) #leaf;
      case (#node n) #node {
        left = map(n.left, f);
        value = f(n.value);
        right = map(n.right, f);
      };
    }
  };
}
