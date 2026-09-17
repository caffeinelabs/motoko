module Map {
  public type Map<K, V> = { map : [(K, [var V])] };
  public func empty<K, V>() : Map<K, V> = { map = [] };
  public func map<K, V1, V2>(self : Map<K, V1>, project : (K, V1) -> V2) : Map<K, V2> {
    empty();
  };
};

actor {
  let peopleMap = Map.empty<Nat, Text>();

  // Make sure the errors report the `V2` type parameter unsolved!
  // It should not be `V2 := Non`
  func x1() { let _ = peopleMap.map("text") };
  func x2() { let _ = peopleMap.map(peopleMap) };
  func x3() { let _ = peopleMap.map(func(k, _) { 123 })};
};
