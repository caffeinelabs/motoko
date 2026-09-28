//MOC-FLAG -W M0237
type Order = {
  #less; #equal; #greater;
};

module Nat {
  public func compare(_ : Nat, _ : Nat) : Order { #equal };
};

module Text {
  public func compare(_ : Text, _ : Text) : Order { #equal };
};

module Map {
  public type Map<K,V> = {map : [(K, [var V])]};
  public func empty<K, V>() : Map<K,V> = { map= []};

  public func get<K, V>(
    self : Map<K, V>,
    _compare: (implicit : (compare : (K, K) -> Order)),
    _n : K)
  : ?V {
    null
  };

  public func set<K, V>(
    self : Map<K, V>,
    _compare: (implicit : (compare : (K, K) -> Order)),
    _n : K,
    _v : V)
  : Map<K, V> {
    self
  };
};

actor {
  let peopleMap = Map.empty<Nat, Text>();

  func _test1() {
     // get
     ignore Map.get(peopleMap, Nat.compare, 1); // warn
     ignore Map.get(peopleMap, 1); // ok

     ignore peopleMap.get(Nat.compare, 1); // warn
     ignore peopleMap.get(1); // ok

  };

  func _test2() {
     func compare(_ : Nat, _: Nat) : Order {#equal}; // overrides Nat.compare

     ignore Map.get(peopleMap, compare, 1); // warn
     ignore Map.get(peopleMap, Nat.compare, 1); // ok
     ignore Map.get(peopleMap, 1); // ok

     ignore peopleMap.get(compare, 1); // warn
     ignore Map.get(peopleMap, Nat.compare, 1); // ok
     ignore peopleMap.get(1); // ok
   };

  func _test3() {
    module Amb1 {
      public type T = {#amb};
      public func compare(_ : T, _ : T) : Order { #equal };
    };

    module _Amb2 {
      public type T = {#amb};
      public func compare(_ : T, _ : T) : Order { #equal };
    };

    let ambMap = Map.empty<Amb1.T, Text>();
    ignore Map.get(ambMap, Amb1.compare, #amb); // don't warn (suggestion would be ambiguous)
    ignore Map.get(ambMap, #amb); // reject // ambiguous
   }

}
