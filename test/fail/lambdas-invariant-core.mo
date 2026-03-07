//MOC-FLAG --package core $MOTOKO_CORE
import Map "mo:core/Map";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

let m = Map.empty<Nat, Text>();

func _map() {
  let _ = m.map(func(k, v) { (k, v) });
};
func _mapViaIter() {
  let _ = m.entries().map(func(k, v) { (k, v) }).toMap();
};
func _mapViaIter2() {
  let _ = m.entries().map(func(k, v) { (k, v) }).toMap(Nat.compare); // Should compile
};

// Regression test case: implicits should not be checked before the last round of solving
func trick<A, B>(a : A, equal : (implicit : (B, B) -> Bool), f : A -> B) : Bool { equal(f(a), f(a)) };
func _trick() {
  let v = { a = 1 };
  assert trick(v, func (r) { r.a })
}
