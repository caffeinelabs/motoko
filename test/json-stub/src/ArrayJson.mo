import Json "Json";
import Array "mo:core/Array";

module {
  public func _toJson<T>(self : [T], _toJson : (implicit : T -> Json.Json)) : Json.Json {
    #array(self.map(_toJson));
  };
};
