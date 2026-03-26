import Json "Json";
module {
  public func _toJson<A, B, C>(
    self : (A, B, C),
    _toJsonA : (implicit : (_toJson : A -> Json.Json)),
    _toJsonB : (implicit : (_toJson : B -> Json.Json)),
    _toJsonC : (implicit : (_toJson : C -> Json.Json)),
  ) : Json.Json {
    let (a, b, c) = self;
    #array([_toJsonA(a), _toJsonB(b), _toJsonC(c)]);
  };
}
