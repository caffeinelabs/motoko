//MOC-FLAG -W=M0223,M0236,M0237 --all-libs --package core $MOTOKO_CORE --error-format=json
import Map "mo:core/Map";
import Nat "mo:core/Nat";
import Text "mo:core/Text";
import { type Order } "mo:core/Order";

// --- M0223: redundant type instantiation ---

do {
  func inferred<T>(x : T) : T = x;
  let n1 = inferred<Nat>(1);
  ignore n1;
};

// --- M0236: contextual dot notation ---

do {
  let m = Map.empty<Nat, Text>();
  let m2 = Map.empty<Int, Text>();

  // single arg
  ignore Map.size(m); // warn M0236

  // multi arg, no implicit in scope -> M0230 error + M0236 warn
  ignore Map.get(m2, 1); // warn M0236

  // multi arg with implicit -> M0236 + M0237
  ignore Map.get(m, Nat.compare, 1); // warn M0236 + M0237

  // complex receiver
  ignore Map.size(
    Map.empty<Nat, Text>()
  ); // warn M0236

  // multiline call -> M0236 + M0237
  Map.add(
    m,
    Nat.compare,
    1,
    "John",
  ); // warn M0236 + M0237
};

// --- M0237: implicit argument removal ---

do {
  let m = Map.empty<Nat, Text>();

  // single line
  ignore m.get(Nat.compare, 1); // warn M0237

  // multiline
  ignore m.get(
    Nat.compare,
    1,
  ); // warn M0237
};

// --- M0237: complex implicit patterns ---

module Impl {
  // implicit in the middle: f(self, implicit, key)
  public func get<K, V>(
    self : [(K, V)],
    _cmp : (implicit : (compare : (K, K) -> Order)),
    key : K,
  ) : ?V { ignore self; ignore key; null };

  // two adjacent implicits: f(self, implicit1, implicit2, key, value)
  public func put<K, V>(
    self : [(K, V)],
    _cmpK : (implicit : (compare : (K, K) -> Order)),
    _cmpV : (implicit : (compare : (V, V) -> Order)),
    key : K,
    value : V,
  ) : [(K, V)] { ignore key; ignore value; self };

  // implicit at the end
  public func find<K, V>(
    self : [(K, V)],
    key : K,
    _cmp : (implicit : (compare : (K, K) -> Order)),
  ) : ?V { ignore self; ignore key; null };
  public func sort1<K, V>(
    self : [(K, V)],
    _cmp : (implicit : (compare : (K, K) -> Order)),
  ) : [(K, V)] { self };
  public func sort2<K, V>(
    notSelf : [(K, V)],
    _cmp : (implicit : (compare : (K, K) -> Order)),
  ) : [(K, V)] { notSelf };

  // all implicits: f(implicit1, implicit2)
  public func make<K, V>(
    _cmpK : (implicit : (compare : (K, K) -> Order)),
    _cmpV : (implicit : (compare : (V, V) -> Order)),
  ) : [(K, V)] { [] };

  // non-adjacent implicits: f(self, implicit1, key, implicit2, value)
  public func update<K, V>(
    self : [(K, V)],
    _cmpK : (implicit : (compare : (K, K) -> Order)),
    key : K,
    _cmpV : (implicit : (compare : (V, V) -> Order)),
    value : V,
  ) : [(K, V)] { ignore key; ignore value; self };
};

do {
  let data : [(Nat, Text)] = [];

  // implicit in the middle -> M0236 + M0237
  ignore Impl.get(data, Nat.compare, 1);

  // two adjacent implicits -> M0236 + M0237 x2
  ignore Impl.put(data, Nat.compare, Text.compare, 1, "a");

  // implicit at the end -> M0236 + M0237
  ignore Impl.find(data, 1, Nat.compare);
  ignore Impl.sort1(data, Nat.compare); // -> M0236 + M0237
  ignore Impl.sort2(data, Nat.compare); // no dot suggestion (notSelf), M0237 only

  // all implicits -> M0237 x2
  let _ = Impl.make<Nat, Text>(Nat.compare, Text.compare);

  // non-adjacent implicits -> M0236 + M0237 x2
  ignore Impl.update(data, Nat.compare, 1, Text.compare, "a");

  // multiline: two adjacent implicits -> M0236 + M0237 x2
  ignore Impl.put(
    data,
    Nat.compare,
    Text.compare,
    1,
    "a",
  );
};

// --- Mix: M0223 + M0236 + M0237 ---

do {
  // NB: Must use `let _ = ...` to get the 'redundant type instantiation' error
  let _ = Map.add<Nat, Text>(
    Map.empty<Nat, Text>(),
    Nat.compare,
    1,
    "John",
  ); // warn M0223 + M0236 + M0237
};
