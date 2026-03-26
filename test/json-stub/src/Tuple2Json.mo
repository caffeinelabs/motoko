import Json "Json";
module {
  public func _toJson<A, B>(
    self : (A, B),
    _toJsonA : (implicit : (_toJson : A -> Json.Json)),
    _toJsonB : (implicit : (_toJson : B -> Json.Json)),
  ) : Json.Json {
    let (a, b) = self;
    #array([_toJsonA(a), _toJsonB(b)]);
  };
}
