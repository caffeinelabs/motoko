/// Stub for Map.

import Types "Types";

module {
  public newtype Map<K, V> = Types.MapT.MapInternals<K, V>;
  // public type Map<K, V> = Types.Map<K, V>;

  public func empty<K, V>() : Map<K, V> {
    // TODO: when `Map` is a type alias for a Map newtype, `Map` cannot be used here, as there is no value constructor called `Map`, the type alias is not introducing a constructor.
    Map<K, V>({
      var root = #leaf({
        data = {
          kvs = [var null];
          var count = 0
        }
      });
      var size_ = 0
    })
  };


  public func get<K, V>(self : Map<K, V>, compare : (implicit : (K, K) -> Types.Order), key : K) : ?V {
    switch (self.unwrap.root) {
      case (#internal _) { null };
      case (#leaf(leafNode)) {
        let ?x = leafNode.data.kvs[0] else return null;
        if (compare(key, x.0) == #equal) ?x.1 else null
       }
    }
  };


  public func add<K, V>(self : Map<K, V>, compare : (implicit : (K, K) -> Types.Order), key : K, value : V) {
    switch (self.unwrap.root) {
      case (#internal _) { };
      case (#leaf(leafNode)) {
        switch (leafNode.data.kvs[0]) {
          case (?x) {
            if (compare(key, x.0) == #less) return;
            leafNode.data.kvs[0] := ?(key, value);
          };
          case null {
            leafNode.data.kvs[0] := ?(key, value);
           }
        }
       }
    }
  };

}
