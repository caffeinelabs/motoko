/// Stub for Map.

import Types "Types";

module {
  public type Map<K, V> = Types.Map<K, V>;

  type Node<K, V> = Types.Map.Node<K, V>;
  type Data<K, V> = Types.Map.Data<K, V>;
  type Internal<K, V> = Types.Map.Internal<K, V>;
  type Leaf<K, V> = Types.Map.Leaf<K, V>;

  public func empty<K, V>() : Map<K, V> {
    {
      var root = #leaf({
        data = {
          kvs = [var null];
          var count = 0
        }
      });
      var size_ = 0
    }
  };

  public func get<K, V>(self : Map<K, V>, compare : (implicit : (K, K) -> Types.Order), key : K) : ?V {
    switch self.root {
      case #internal(_) { null }
      case #leaf(leafNode) {
        let ?x = leafNode.data.kvs[0] else return null;
        if compare(key, x.0) == #equal { ?x.1 } else { null }
      }
    }
  };

  public func add<K, V>(self : Map<K, V>, compare : (implicit : (K, K) -> Types.Order), key : K, value : V) {
    switch self.root {
      case #internal(_) {}
      case #leaf(leafNode) {
        switch leafNode.data.kvs[0] {
          case ?x {
            if compare(key, x.0) == #less { return };
            leafNode.data.kvs[0] := ?(key, value);
          }
          case null {
            leafNode.data.kvs[0] := ?(key, value);
          }
        }
      }
    }
  };

}
