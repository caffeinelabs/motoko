//MOC-FLAG --package core $MOTOKO_CORE -W=M0223,M0236,M0237
import Array "mo:core/Array";
import Iter "mo:core/Iter";
import List "mo:core/List";
import Map "mo:core/Map";
import Nat "mo:core/Nat";
import Int "mo:core/Int";
import Text "mo:core/Text";

// ── JSON type ────────────────────────────────────────────────────────────────

type Json = {
  #null_;
  #bool : Bool;
  #number : Int;
  #string : Text;
  #array : [Json];
  #obj : [(Text, Json)];
};

module Json {
  public func toText(self : Json) : Text {
    switch self {
      case (#null_) "null";
      case (#bool b) if b "true" else "false";
      case (#number n) n.toText();
      case (#string t) "\"" # t # "\"";
      case (#array items) {
        "[" # items.map(toText).vals().join(",") # "]";
      };
      case (#obj pairs) {
        "{" # pairs.map(func(k, v) { "\"" # k # "\":" # toText(v) }).vals().join(",") # "}";
      };
    };
  };
};

// ── Implicit instances ───────────────────────────────────────────────────────

module IntJson {
  public func _toJson(self : Int) : Json { #number self };
};

module TextJson {
  public func _toJson(self : Text) : Json { #string self };
};

module ArrayJson {
  public func _toJson<T>(self : [T], _toJson : (implicit : T -> Json)) : Json {
    #array(self.map(_toJson));
  };
};

module ListJson {
  public func _toJson<T>(self : List.List<T>, _toJson : (implicit : T -> Json)) : Json {
    #array(self.values().map(_toJson).toArray());
  };
};

// Map serialises to a JSON object; keys need a `toText` conversion.
// Nat.toText and Text.toText from mo:core serve as leaf implicits for `toText`.
module MapJson {
  public func _toJson<K, V>(
    self : Map.Map<K, V>,
    toText : (implicit : K -> Text),
    _toJson : (implicit : V -> Json),
  ) : Json {
    #obj(
      self.entries().map(func(k, v) { (toText(k), _toJson(v)) }).toArray()
    );
  };
};

// Tuples: all element implicits share the `_toJson` search label; the concrete
// element type differentiates which instance is resolved for each slot.
module Tuple2Json {
  public func _toJson<A, B>(
    self : (A, B),
    _toJsonA : (implicit : (_toJson : A -> Json)),
    _toJsonB : (implicit : (_toJson : B -> Json)),
  ) : Json {
    let (a, b) = self;
    #array([_toJsonA(a), _toJsonB(b)]);
  };
};

module Tuple3Json {
  public func _toJson<A, B, C>(
    self : (A, B, C),
    _toJsonA : (implicit : (_toJson : A -> Json)),
    _toJsonB : (implicit : (_toJson : B -> Json)),
    _toJsonC : (implicit : (_toJson : C -> Json)),
  ) : Json {
    let (a, b, c) = self;
    #array([_toJsonA(a), _toJsonB(b), _toJsonC(c)]);
  };
};

// ── Entry point ──────────────────────────────────────────────────────────────

// Bridges dot-syntax `value.toJson()` to the `_toJson` implicit search label.
func toJson<R>(self : R, _toJson : (implicit : R -> Json)) : Json { _toJson(self) };

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

// Map<Nat, Nat>
let m1 = Map.empty<Nat, Nat>();
m1.add(1, 10);
m1.add(2, 20);
assert m1.toJson().toText() == "{\"1\":10,\"2\":20}";

// Map<Text, Nat> — Text keys serialise via Text.toText (identity)
let m2 = Map.empty<Text, Nat>();
m2.add("a", 1);
m2.add("b", 2);
assert m2.toJson().toText() == "{\"a\":1,\"b\":2}";

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
assert deep.toJson().toText() == "{\"1\":[[-1,\"hello\",{\"score\":99}]]}";

//SKIP comp
