import Json "Json";
import Array "mo:core/Array";

module {
  // Structural record combiner: compiler detects __record and synthesizes per-field resolution.
  // Each field is a thunk — evaluated eagerly here since serialization needs all fields.
  public func _toJson(__record : [(Text, () -> Json.Json)]) : Json.Json =
    #obj(__record.map(func((k, v)) = (k, v())));
};
