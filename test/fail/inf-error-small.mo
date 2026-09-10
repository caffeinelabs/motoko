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
    compare: (implicit : (K, K) -> Order),
    n : K)
  : ?V {
    null
  };

  public func set<K, V>(
    self : Map<K, V>,
    compare: (implicit : (K, K) -> Order),
    n : K,
    v : V)
  : Map<K, V> {
    self
  };
};

actor {
  let peopleMap = Map.empty<Nat, Text>();

  func test1() {
     ignore peopleMap.get("text") : Text; // bad
     ignore peopleMap.get(Nat.compare, "text") : Text; // bad

     ignore peopleMap.get(peopleMap, "text") : Text; // bad
     ignore peopleMap.get(peopleMap, Nat.compare, "text") : Text; // bad
     ignore peopleMap.get(peopleMap, "text") : Text; // bad

     ignore peopleMap.get(1) : Bool; // bad
  };
  func test2() {
    func c0() { let _ = peopleMap.get("text") };
    func c1() { let _ : Text = peopleMap.get("text") }; // wrong ret should be reported before wrong arg
    func c2() { let _ = peopleMap.get(Text.compare, 1) }; // wrong 1nd arg
    func c3() { let _ = peopleMap.get(Nat.compare, "text") }; // wrong 2st arg
    func c4() { let _ = peopleMap.get(Text.compare, "text") }; // wrong both args, should point to the first one

    func x1() { let _ = peopleMap.get() };
    func x2() { let _ = peopleMap.get(Nat.compare, "text") };
    func x3() { let _ = peopleMap.get(peopleMap, "text") };
    func x4() { let _ = peopleMap.get(peopleMap, Nat.compare, "text") };
  };
}
