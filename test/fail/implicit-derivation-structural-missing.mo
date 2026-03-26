// Structural derivation fails when a field type has no instance in scope.
// Bool has no _toJson, so the record { flag : Bool } cannot be serialized.
//MOC-FLAG --package core $MOTOKO_CORE --package json ../json-stub/src

import Json "mo:json/Json";
import IntJson "mo:json/IntJson";
import TextJson "mo:json/TextJson";

type Json = Json.Json;

ignore ({ flag = true }).toJson();
