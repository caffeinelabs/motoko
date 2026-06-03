import Json "Json";

module {
  public func _toJson(self : Int) : Json.Json { #number(self) };
};
