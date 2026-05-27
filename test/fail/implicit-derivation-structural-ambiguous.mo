// Structural derivation fails when two structural combiners with the same
// __record shape are in scope — the compiler cannot pick one.
//MOC-FLAG --package core $MOTOKO_CORE --package json ../json-stub/src

import Json "mo:json/Json";
import RecordJson "mo:json/RecordJson";
import IntJson "mo:json/IntJson";

type Json = Json.Json;

// A second combiner with the same shape creates ambiguity.
module AltRecordJson {
  public func _toJson(__record : [(Text, () -> Json)]) : Json { #null_ };
};

ignore ({ x = 1 : Int }).toJson();
