//MOC-FLAG --package core $MOTOKO_CORE --package json ../json-stub/src -W=M0223,M0236,M0237
import List "mo:core/List";
import Map "mo:core/Map";
import Nat "mo:core/Nat";
import Int "mo:core/Int";
import Text "mo:core/Text";
import Json "mo:json/Json";
import RecordJson "mo:json/RecordJson";
import IntJson "mo:json/IntJson";
import TextJson "mo:json/TextJson";
import ArrayJson "mo:json/ArrayJson";
import ListJson "mo:json/ListJson";
import MapJson "mo:json/MapJson";
import TupleJson "mo:json/TupleJson";

type Json = Json.Json;

// ── Tests ────────────────────────────────────────────────────────────────────

// Primitives
assert (42 : Nat).toJson().toText() == "42";
assert (-7 : Int).toJson().toText() == "-7";
assert "hello".toJson().toText() == "\"hello\"";

// Array<Nat>
assert ([1, 2, 3] : [Nat]).toJson().toText() == "[1,2,3]";

// List<Text>
let lst = List.empty<Text>();
lst.add("a");
lst.add("b");
assert lst.toJson().toText() == "[\"a\",\"b\"]";

// Tuple2<Nat, Text>
assert (1 : Nat, "x").toJson().toText() == "[1,\"x\"]";

// Tuple3<Nat, Text, Int>
assert (42 : Nat, "hello", -3 : Int).toJson().toText() == "[42,\"hello\",-3]";

// Map<Nat, Nat> — keys and values both serialise via _toJson.
let m1 = Map.empty<Nat, Nat>();
m1.add(1, 10);
m1.add(2, 20);
assert m1.toJson().toText() == "[[1,10],[2,20]]";

// Map<Text, Nat>
let m2 = Map.empty<Text, Nat>();
m2.add("a", 1);
m2.add("b", 2);
assert m2.toJson().toText() == "[[\"a\",1],[\"b\",2]]";

// Array<(Nat, Text)>
let tuples : [(Nat, Text)] = [(1, "one"), (2, "two")];
assert tuples.toJson().toText() == "[[1,\"one\"],[2,\"two\"]]";

// Flagship: Map<Nat, List<(Int, Text, Map<Text, Nat>)>>
let inner = Map.empty<Text, Nat>();
inner.add("score", 99);
let items = List.empty<(Int, Text, Map.Map<Text, Nat>)>();
items.add((-1, "hello", inner));
let deep = Map.empty<Nat, List.List<(Int, Text, Map.Map<Text, Nat>)>>();
deep.add(1, items);
assert deep.toJson().toText() == "[[1,[[-1,\"hello\",[[\"score\",99]]]]]]";

// Records — compiler synthesizes a wrapper that serialises each field via _toJson.
// Fields appear in alphabetical order (Motoko sorts object-type fields).

// Flat record: two primitive fields
let r1 : { age : Nat; name : Text } = { age = 30; name = "Alice" };
assert r1.toJson().toText() == "{\"age\":30,\"name\":\"Alice\"}";

// Single-field record
let r2 : { x : Nat } = { x = 7 };
assert r2.toJson().toText() == "{\"x\":7}";

// Record with heterogeneous primitives: Nat, Int, Text fields
let r3 : { count : Nat; name : Text; offset : Int } = {
  count = 3;
  name = "ok";
  offset = -1;
};
assert r3.toJson().toText() == "{\"count\":3,\"name\":\"ok\",\"offset\":-1}";

// Record containing an array field
let r4 : { items : [Nat]; tag : Text } = { items = [1, 2, 3]; tag = "nums" };
assert r4.toJson().toText() == "{\"items\":[1,2,3],\"tag\":\"nums\"}";

// Record containing a map field
let scores = Map.empty<Text, Nat>();
scores.add("alice", 90);
scores.add("bob", 85);
let r5 : { data : Map.Map<Text, Nat>; version : Nat } = {
  data = scores;
  version = 1;
};
assert r5.toJson().toText() == "{\"data\":[[\"alice\",90],[\"bob\",85]],\"version\":1}";

// Nested record: inner field is itself a record
let r6 : { inner : { value : Nat }; outer : Text } = {
  inner = { value = 42 };
  outer = "top";
};
assert r6.toJson().toText() == "{\"inner\":{\"value\":42},\"outer\":\"top\"}";

//SKIP comp
