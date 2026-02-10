import Map "mo:core/Map";
import Nat "mo:core/Nat";

let m = Map.empty<Nat, Text>();
ignore m.get(
  Nat.compare,
  1,
);
