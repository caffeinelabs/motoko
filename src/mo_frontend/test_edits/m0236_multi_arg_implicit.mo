import Map "mo:core/Map";
import Nat "mo:core/Nat";

let m = Map.empty<Nat, Text>();
ignore Map.get(m, Nat.compare, 1);
