//MOC-FLAG --package core $MOTOKO_CORE --package json ../json-stub/src --all-libs
import Json "mo:json/Json";
import RecordJson "mo:json/RecordJson";

ignore ({ flag = true; nat = 1; int = -1 }).toJson();
