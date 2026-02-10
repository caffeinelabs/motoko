import Map "mo:core/Map";
import Nat "mo:core/Nat";

let m = Map.empty<Nat, Text>();
Map.add(
  m,
  Nat.compare,
  1,
  "John",
);
