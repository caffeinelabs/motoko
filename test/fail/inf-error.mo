type Order = {
  #less; #equal; #greater;
};

module Any {
//  public func compare(n : Any, m : Any) : Order { #equal };
};

module Nat {
  public func compare(n : Nat, m : Nat) : Order { #equal };
};

module Text {
  public func compare(n : Text, m : Text) : Order { #equal };
};

module Map {
  public type Map<K,V> = {map : [(K, [var V])]};
  public func empty<K, V>() : Map<K,V> = { map= []};

  public func get<K, V>(
    self : Map<K, V>,
    compare : (implicit : (K, K) -> Order),
    n : K)
  : ?V {
    null
  };

  public func set<K, V>(
    self : Map<K, V>,
    compare : (implicit : (K, K) -> Order),
    n : K,
    v : V)
  : Map<K, V> {
    self
  };
};

actor {
  let peopleMap = Map.empty<Nat, Text>();

  func test1() {
     // get
     ignore Map.get(peopleMap, Nat.compare, 1); // ok
     ignore Map.get(peopleMap, 1); // ok
     ignore Map.get(peopleMap, Nat.compare, "test"); // bad
     ignore Map.get(peopleMap, "test"); // bad

     ignore peopleMap.get(Nat.compare, 1); // ok
     ignore peopleMap.get(1); // ok
     ignore peopleMap.get(Nat.compare, "test"); // bad
     ignore peopleMap.get("test"); // bad
  };

  func test2() {
     // set
     ignore Map.set(peopleMap, Nat.compare, 1, ""); // ok
     ignore Map.set(peopleMap, 1, ""); // ok
     ignore Map.set(peopleMap, Nat.compare, "test", ""); // bad
     ignore Map.set(peopleMap, "test", ""); // bad

     ignore peopleMap.set(Nat.compare, "test", ""); // ok
     ignore peopleMap.set(1, ""); // ok
     ignore peopleMap.set(Nat.compare, "test", ""); // bad
     ignore peopleMap.set("test", ""); // bad

   }
}
