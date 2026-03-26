import Json "Json";
import Map "mo:core/Map";
import Iter "mo:core/Iter";
module {
  public func _toJson<K, V>(
    self : Map.Map<K, V>,
    toText : (implicit : K -> Text),
    _toJson : (implicit : V -> Json.Json),
  ) : Json.Json {
    #obj(
      self.entries().map(func(k, v) { (toText(k), _toJson(v)) }).toArray()
    );
  };
}
