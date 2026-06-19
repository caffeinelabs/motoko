// Structural combiner lives in mo:json/RecordJson (not imported).
// The compiler should suggest importing it.
//MOC-FLAG --package core $MOTOKO_CORE --package json ../json-stub/src --all-libs

import IntJson "mo:json/IntJson";

type Json = {
  #null_;
  #bool : Bool;
  #number : Int;
  #string : Text;
  #array : [Json];
  #obj : [(Text, Json)];
};

func toJson<R>(self : R, _toJson : (implicit : R -> Json)) : Json { _toJson(self) };

ignore ({ age = 30 : Int }).toJson();
