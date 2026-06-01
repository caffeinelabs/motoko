import Json "Json";
import Array "mo:core/Array";

module {
  // Structural tuple combiner: handles tuples of any arity (≥ 2) via __tuple.
  // Each element is a thunk — evaluated eagerly here since serialization needs all elements.
  public func _toJson(__tuple : [() -> Json.Json]) : Json.Json =
  #array(__tuple.map(func(t) = t()));
};
