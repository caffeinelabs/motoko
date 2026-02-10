import Map "mo:core/Map";
import Nat "mo:core/Nat";

let _ = Map.add<Nat, Text>(
  Map.empty<Nat, Text>(),
  Nat.compare,
  1,
  "John",
);
