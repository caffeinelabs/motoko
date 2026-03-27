// Practical corner cases when json-stub modules are not imported.
//MOC-FLAG --package core $MOTOKO_CORE --package json ../json-stub/src --all-libs
import Map "mo:core/Map";
import Text "mo:core/Text";
import Json "mo:json/Json";
import TextJson "mo:json/TextJson";
// Deliberately NOT importing: IntJson, Tuple2Json, MapJson

ignore ({ name = "Alice"; age = 42 : Int; active = true }).toJson();

let m = Map.empty<Text, Int>();
m.add("k", 1);
ignore ({ data = m; tag = "v" }).toJson();

let m2 = Map.empty<Text, Int>();
m2.add("x", 1);
ignore m2.toJson();

ignore ({ inner = { value = 42 }; outer = "top" }).toJson();
