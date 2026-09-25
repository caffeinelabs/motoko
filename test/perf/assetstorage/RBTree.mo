/// Red-Black Trees

import Debug "Debug";
import I "Iter";
import List "List";
import Nat "Nat";
import O "Order";

module {

  /// Node color: Red or black.
  public type Color = { #R; #B };

  /// Ordered, (red-black) tree of entries.
  public type Tree<X, Y> = {
    #node : (Color, Tree<X, Y>, (X, ?Y), Tree<X, Y>);
    #leaf;
  };

  /// Create an order map from an order function for its keys.
  public class RBTree<X, Y>(compareTo:(X, X) -> O.Order) {

    var tree: Tree<X, Y> = (#leaf : Tree<X, Y>);

    /// Tree as share data.
    ///
    /// Get non-OO, purely-functional representation:
    /// for drawing, pretty-printing and non-OO contexts
    /// (e.g., async args and results):
    public func share() : Tree<X, Y> {
      tree
    };

    /// Get the value associated with a given key.
    public func get(x:X) : ?Y =
      getRec(x, compareTo, tree);
    /// Replace the value associated with a given key.
    public func replace(x:X, y:Y) : ?Y {
      let (res, t) = insertRoot(x, compareTo, y, tree);
      tree := t;
      res
    };
    /// Put an entry: A value associated with a given key.
    public func put(x:X, y:Y) {
      let (res, t) = insertRoot(x, compareTo, y, tree);
      tree := t;
    };
    /// Delete the entry associated with a given key.
    public func delete(x:X) {
      let (res, t) = removeRec(x, compareTo, tree);
      tree := t
    };
    /// Remove the entry associated with a given key.
    public func remove(x:X) : ?Y {
      let (res, t) = removeRec(x, compareTo, tree);
      tree := t;
      res
    };

    /// An iterator for the key-value entries of the map, in ascending key order.
    ///
    /// iterator is persistent, like the tree itself
    public func entries() : I.Iter<(X, Y)> = iter(tree, #fwd);

    /// An iterator for the key-value entries of the map, in descending key order.
    ///
    /// iterator is persistent, like the tree itself
    public func entriesRev() : I.Iter<(X, Y)> = iter(tree, #bwd);

  };

  type IterRep<X, Y> = List.List<{ #tr:Tree<X, Y>; #xy:(X, ?Y) }>;

  /// An iterator for the entries of the map, in ascending (`#fwd`) or descending (`#bwd`) order.
  public func iter<X, Y>(t:Tree<X, Y>, dir:{ #fwd; #bwd }) : I.Iter<(X, Y)> {
    object {
      var trees : IterRep<X, Y> = ?(#tr(t), null);
      public func next() : ?(X, Y) {
        switch (dir, trees) {
        case (_, null) { null }
        case (_, ?(#tr(#leaf), ts)){
          trees := ts;
          next()
        }
        case (_, ?(#xy(xy), ts)) {
          trees := ts;
          switch xy.1 {
            case null { next() }
            case ?y { ?(xy.0, y) }
          }
        }
        case (#fwd, ?(#tr(#node(_, l, xy, r)), ts)) {
          trees := ?(#tr(l), ?(#xy(xy), ?(#tr(r), ts)));
          next()
        }
        case (#bwd, ?(#tr(#node(_, l, xy, r)), ts)) {
          trees := ?(#tr(r), ?(#xy(xy), ?(#tr(l), ts)));
          next()
        }
        }
      };
    }
  };

  /// Remove the value associated with a given key.
  func removeRec<X, Y>(x:X, compareTo:(X, X) -> O.Order, t:Tree<X, Y>)
    : (?Y, Tree<X, Y>)
  {
    switch t {
      case #leaf { (null, #leaf) }
      case #node(c, l, xy, r) {
        switch compareTo(x, xy.0) {
          case #less {
            let (yo, l2) = removeRec(x, compareTo, l);
            (yo, #node(c, l2, xy, r))
          }
          case #equal {
            (xy.1, #node(c, l, (x, null), r))
          }
          case #greater {
            let (yo, r2) = removeRec(x, compareTo, r);
            (yo, #node(c, l, xy, r2))
          }
        }
      }
    }
  };

  func bal<X, Y>(color:Color, lt:Tree<X, Y>, kv:(X, ?Y), rt:Tree<X, Y>) : Tree<X, Y> {
    // thank you, algebraic pattern matching!
    // following notes from [Ravi Chugh](https://www.classes.cs.uchicago.edu/archive/2019/spring/22300-1/lectures/RedBlackTrees/index.html)
    switch (color, lt, kv, rt) {
      case (#B, #node(#R, #node(#R, a, x, b), y, c), z, d) { #node(#R, #node(#B, a, x, b), y, #node(#B, c, z, d)) }
      case (#B, #node(#R, a, x, #node(#R, b, y, c)), z, d) { #node(#R, #node(#B, a, x, b), y, #node(#B, c, z, d)) }
      case (#B, a, x, #node(#R, #node(#R, b, y, c), z, d)) { #node(#R, #node(#B, a, x, b), y, #node(#B, c, z, d)) }
      case (#B, a, x, #node(#R, b, y, #node(#R, c, z, d))) { #node(#R, #node(#B, a, x, b), y, #node(#B, c, z, d)) }
      case _ { #node(color, lt, kv, rt) }
    }
  };

  func insertRoot<X, Y>(x:X, compareTo:(X, X) -> O.Order, y:Y, t:Tree<X, Y>)
    : (?Y, Tree<X, Y>)
  {
    switch insertRec(x, compareTo, y, t) {
      case (_, #leaf) { assert false; loop {} }
      case (yo, #node(_, l, xy, r)) { (yo, #node(#B, l, xy, r)) }
    }
  };

  func insertRec<X, Y>(x:X, compareTo:(X, X) -> O.Order, y:Y, t:Tree<X, Y>)
    : (?Y, Tree<X, Y>)
  {
    switch t {
      case #leaf { (null, #node(#R, #leaf, (x, ?y), #leaf)) }
      case #node(c, l, xy, r) {
        switch compareTo(x, xy.0) {
          case #less {
            let (yo, l2) = insertRec(x, compareTo, y, l);
            (yo, bal(c, l2, xy, r))
          }
          case #equal {
            (xy.1, #node(c, l, (x, ?y), r))
          }
          case #greater {
            let (yo, r2) = insertRec(x, compareTo, y, r);
            (yo, bal(c, l, xy, r2))
          }
        }
      }
    }
  };

  func getRec<X, Y>(x:X, compareTo:(X, X) -> O.Order, t:Tree<X, Y>) : ?Y {
    switch t {
      case #leaf { null }
      case #node(c, l, xy, r) {
        switch compareTo(x, xy.0) {
          case #less { getRec(x, compareTo, l) }
          case #equal { xy.1 }
          case #greater { getRec(x, compareTo, r) }
        }
      }
    }
  };

  func height<X, Y>(t:Tree<X, Y>) : Nat {
    switch t {
      case #leaf { 0 }
      case #node(_, l, _, r) {
        Nat.max(height(l), height(r)) + 1
      }
    }
  };

  /// The size of the tree as the number of key-value entries.
  public func size<X, Y>(t:Tree<X, Y>) : Nat {
    switch t {
      case #leaf { 0 }
      case #node(_, l, _, r) {
        size(l) + size(r) + 1
      }
    }
  };

}
