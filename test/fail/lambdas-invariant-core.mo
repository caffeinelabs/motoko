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
