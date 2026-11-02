// Practical corner cases when json-stub modules are not imported.
//MOC-FLAG --package core $MOTOKO_CORE --package json ../json-stub/src --all-libs
import Map "mo:core/Map";
import Text "mo:core/Text";
import Json "mo:json/Json";
import RecordJson "mo:json/RecordJson";
import TextJson "mo:json/TextJson";
// Deliberately NOT importing: IntJson, TupleJson, MapJson

// 1. Multiple missing leaves: Bool has no library at all, Int needs IntJson.
ignore ({ name = "Alice"; age = 42 : Int; active = true }).toJson();

// 2. Record with a Map field — Map type is normalized (expanded) in the error.
let m = Map.empty<Text, Int>();
m.add("k", 1);
ignore ({ data = m; tag = "v" }).toJson();

// 3. Direct .toJson() on a Map — same expansion, plus inner derivation errors.
let m2 = Map.empty<Text, Int>();
m2.add("x", 1);
ignore m2.toJson();

// 4. Nested records — derivation recurses into the inner record's fields.
ignore ({ inner = { value = 42 }; outer = "top" }).toJson();
