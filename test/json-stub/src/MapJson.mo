import Json "Json";
import Map "mo:core/Map";
import Iter "mo:core/Iter";

module {
  public func _toJson<K, V>(
    self : Map.Map<K, V>,
    _toJsonK : (implicit : (_toJson : K -> Json.Json)),
    _toJsonV : (implicit : (_toJson : V -> Json.Json)),
  ) : Json.Json {
    #array(
      self.entries().map(func((k, v)) : Json.Json { #array([_toJsonK(k), _toJsonV(v)]) }).toArray()
    );
  };
};
