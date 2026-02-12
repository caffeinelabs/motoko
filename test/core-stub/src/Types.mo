/// Stub for Types.

module {
  public type Iter<T> = { next : () -> ?T };
  public type Order = { #less; #equal; #greater };
  public type Result<T, E> = { #ok : T; #err : E };

  public module MapT {
    public type Node<K, V> = {
      #leaf : Leaf<K, V>;
      #internal : Internal<K, V>
    };

    public type Data<K, V> = {
      kvs : [var ?(K, V)];
      var count : Nat
    };

    public type Internal<K, V> = {
      data : Data<K, V>;
      children : [var ?Node<K, V>]
    };

    public type Leaf<K, V> = {
      data : Data<K, V>
    };

    public type MapInternals<K, V> = {
      var root : Node<K, V>;
      var size_ : Nat
    }
  };

  // public newtype Map<K, V> = MapT.MapInternals<K, V>;
}
