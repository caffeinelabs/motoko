import Map "mo:core/Map";
import Nat "mo:core/Nat";

Map.add(
  Map.empty<Nat, Text>(),
  Nat.compare,
  1,
  "John",
);
