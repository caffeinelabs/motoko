module Map {
  public class Map<T, U>(k_ : T, u_ : U) {
    public let k = k_;
    public let u = u_;
  };

  public func get<T, U>(self : Map<T, U>, x : T, compare : (implicit : (T, T) -> Int)) : ?U {
    if compare(self.k, x) == 0 { ?self.u } else { null };
  };
};

func compare(n : Nat, m : Nat) : Int = n - m;

let map : Map.Map<Nat, Text> = Map.Map(1, "abc");

let t0 : Nat = 0;
let t1 : Nat = 1;

let n1 = map.get(t0);
let n2 = map.get t0;
let n3 = map.get(t0, compare);
let n4 = map.get<Nat, Text>(t0);
assert n1 == null and n2 == null and n3 == null and n4 == null;

let o1 = map.get(t1);
let o2 = map.get t1;
let o3 = map.get(t1, compare);
let o4 = map.get<Nat, Text>(t1);
assert o1 == ?"abc" and o2 == ?"abc" and o3 == ?"abc" and o4 == ?"abc";
