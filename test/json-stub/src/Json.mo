import Int "mo:core/Int";

module {
  public type Json = {
    #null_;
    #bool : Bool;
    #number : Int;
    #string : Text;
    #array : [Json];
    #obj : [(Text, Json)];
  };

  public func toText(self : Json) : Text {
    switch self {
      case (#null_) "null";
      case (#bool b) if b "true" else "false";
      case (#number n) Int.toText(n);
      case (#string t) "\"" # t # "\"";
      case (#array items) {
        var s = "[";
        var first = true;
        for (item in items.vals()) {
          if (not first) { s #= "," };
          s #= toText(item);
          first := false;
        };
        s # "]";
      };
      case (#obj pairs) {
        var s = "{";
        var first = true;
        for ((k, v) in pairs.vals()) {
          if (not first) { s #= "," };
          s #= "\"" # k # "\":" # toText(v);
          first := false;
        };
        s # "}";
      };
    };
  };

  // Entry point for dot-syntax: value.toJson()
  public func toJson<R>(self : R, _toJson : (implicit : R -> Json)) : Json {
    _toJson(self);
  };
};
