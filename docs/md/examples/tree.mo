type Tree<T> = {
  #leaf : T;
  #branch : {left : Tree<T>; right : Tree<T>};
};

func iterTree<T>(tree : Tree<T>, f : T -> ()) {
  switch (tree) {
    case (#leaf(x)) { f(x) };
    case (#branch{left; right}) {
      iterTree(left, f);
      iterTree(right, f);
    };
  }
};

// Compute the sum of all leaf nodes in a tree
let tree = #branch { left = #leaf 1; right = #leaf 2 };
var sum = 0;
iterTree<Nat>(tree, func (leaf) { sum += leaf });
sum
