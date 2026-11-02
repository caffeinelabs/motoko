import Json "Json";
import List "mo:core/List";
import Iter "mo:core/Iter";

module {
  public func _toJson<T>(self : List.List<T>, _toJson : (implicit : T -> Json.Json)) : Json.Json {
    #array(self.values().map(_toJson).toArray());
  };
};
