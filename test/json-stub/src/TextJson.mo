import Json "Json";

module {
  public func _toJson(self : Text) : Json.Json { #string(self) };
};
